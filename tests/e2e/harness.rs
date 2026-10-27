//! Harnais e2e : lance le VRAI binaire `mcp-control` (CARGO_BIN_EXE) dans un
//! répertoire temporaire isolé et lui parle en HTTP brut — uniquement
//! std::net + serde_json, aucune dépendance externe.
//!
//! Isolation garantie :
//! - `MCC_CONFIG` pointe vers un config.json unique par moteur ; mcc.log et
//!   state-cache.json sont écrits à côté (même répertoire temporaire) ;
//! - `"autostart": false` — sinon le moteur écrit une VRAIE clé Run (HKCU) ;
//! - `"network_enabled": false` — sinon il scanne le LAN à +30 s ;
//! - `removed_seeds` liste les 8 seeds embarqués — sinon le premier cycle du
//!   poller AUTO-LANCE les vrais serveurs node de la machine (start_cmd) ;
//! - `MCC_API_ACCESS` sème le code d'accès quand le scénario en demande un.
//!
//! API partagée par tous les modules de scénarios (certains helpers ne servent
//! que scenarios_lifecycle / scenarios_scan) → allow(dead_code) assumé.

#![allow(dead_code)]

use serde_json::{json, Value};
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, UdpSocket};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

/// Compteur pour des répertoires temporaires uniques par process de test.
static NEXT_TMP: AtomicU64 = AtomicU64::new(0);

// ---------- sérialisation globale des scénarios ----------

/// Verrou global e2e : UN SEUL scénario actif à la fois, même quand
/// `cargo test --test e2e` tourne en parallèle (défaut de cargo). Chaque
/// scénario démarre bien SON moteur sur SON port, mais plusieurs scénarios
/// restent sensibles aux ressources partagées de la machine (netstat/scan
/// local, ARP, charge CPU, sonde de mise à jour…) — c'est la cause du flak
/// historique de `scenarios_boot::auth_matrix_lan` en mode parallèle.
/// Const-stable depuis Rust 1.63.
pub static E2E_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Prend le verrou de sérialisation pour toute la durée d'un scénario.
/// À appeler en PREMIÈRE ligne de chaque `#[test]` :
/// `let _garde = serial_guard();`. Tolère le poison (un scénario qui panique
/// en tenant le verrou ne doit pas faire échouer en cascade les suivants).
pub fn serial_guard() -> std::sync::MutexGuard<'static, ()> {
    match E2E_LOCK.lock() {
        Ok(g) => g,
        Err(poisonne) => poisonne.into_inner(),
    }
}

/// Les 8 seeds embarqués par `default_servers()` — chaque seed porte une vraie
/// `start_cmd` (node / hello.exe) + auto_restart : sans retrait explicite, le
/// poller du moteur les AUTO-LANCE au premier cycle. On les retire au chargement
/// (load_config filtre les seeds listés dans settings.removed_seeds).
pub const SEED_IDS: &[&str] = &[
    "seed-overmind",
    "seed-discord",
    "seed-postgresql",
    "seed-debats",
    "seed-x",
    "seed-news-btc",
    "seed-news",
    "seed-litemcp",
];

/// Configuration de démarrage d'un moteur de test.
pub struct EngineCfg {
    /// Port UI demandé (le moteur peut décaler de +1 à +9 si occupé —
    /// wait_ready détecte le port réel en sondant /api/hello).
    pub ui_port: u16,
    /// Bind UI : "127.0.0.1" (loopback) ou "0.0.0.0" (exposé LAN).
    pub ui_bind: &'static str,
    /// Code d'accès API (semé via config + env MCC_API_ACCESS). None = aucun.
    pub api_access: Option<String>,
}

impl Default for EngineCfg {
    fn default() -> Self {
        EngineCfg {
            ui_port: free_port(),
            ui_bind: "127.0.0.1",
            api_access: None,
        }
    }
}

/// Un moteur mcp-control vivant : process enfant + port réel détecté.
/// Drop = kill + wait (le tmp est CONSERVÉ : verrous de fichiers Windows).
pub struct Engine {
    child: Child,
    base_url: String,
    tmp: PathBuf,
    /// Code d'accès API passé au moteur (None si engine sans code).
    pub code: Option<String>,
    /// Port UI RÉEL (base ou base+offset après repli du moteur).
    pub port: u16,
}

/// Lance le binaire mcp-control avec une config isolée dans un tmp unique.
pub fn spawn(cfg: EngineCfg) -> std::io::Result<Engine> {
    let n = NEXT_TMP.fetch_add(1, Ordering::SeqCst);
    let tmp = std::env::temp_dir().join(format!("mcc-e2e-{}-{}", std::process::id(), n));
    std::fs::create_dir_all(&tmp)?;
    let cfg_path = tmp.join("config.json");

    // Config générée : les trois réglages SÛRS sont obligatoires —
    // autostart false (jamais de clé Run réelle), network_enabled false
    // (jamais de scan LAN à +30 s), refresh_secs 5 (plancher) pour des
    // cycles de sondage rapides. removed_seeds neutralise les seeds embarqués.
    let doc = json!({
        "servers": [],
        "settings": {
            "refresh_secs": 5,
            "scan_from": 3000,
            "scan_to": 3000,
            "ui_port": cfg.ui_port,
            "ui_bind": cfg.ui_bind,
            "network_enabled": false,
            "network_ports": [5090, 5091],
            "api_access": cfg.api_access,
            "allowed_origins": [],
            "removed_seeds": SEED_IDS,
            "server_order": [],
            "autostart": false,
            "ssh_hosts": []
        }
    });
    std::fs::write(
        &cfg_path,
        serde_json::to_string_pretty(&doc).unwrap_or_default(),
    )?;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_mcp-control"));
    // mcc.log + state-cache.json sont écrits À CÔTÉ du config.json → tmp isolé.
    cmd.env("MCC_CONFIG", &cfg_path);
    match &cfg.api_access {
        Some(code) => {
            cmd.env("MCC_API_ACCESS", code);
        }
        // on NE hérite PAS d'un MCC_API_ACCESS du shell de l'opérateur
        None => {
            cmd.env_remove("MCC_API_ACCESS");
        }
    }
    // stdout/stderr vers des fichiers du tmp : débogage facile, aucun risque
    // de blocage pipe-buffer (on ne les lit jamais en temps réel).
    let out_log = std::fs::File::create(tmp.join("mcc-stdout.log"))?;
    let err_log = std::fs::File::create(tmp.join("mcc-stderr.log"))?;
    cmd.stdout(Stdio::from(out_log))
        .stderr(Stdio::from(err_log));
    #[cfg(windows)]
    {
        // CREATE_NO_WINDOW : aucune console ne clignote pendant les tests
        use std::os::windows::process::CommandExt;
        cmd.creation_flags(0x0800_0000);
    }
    let child = cmd.spawn()?;
    Ok(Engine {
        child,
        base_url: format!("http://127.0.0.1:{}", cfg.ui_port),
        tmp,
        code: cfg.api_access,
        port: cfg.ui_port,
    })
}

impl Engine {
    /// Port demandé à la config (avant repli éventuel).
    pub fn tmp_dir(&self) -> &PathBuf {
        &self.tmp
    }

    /// Attend que le moteur réponde GET /api/hello → 200 avec un nonce, sur le
    /// port de base puis les ports +1..+9 (repli anti-collision du moteur).
    /// Vrai HTTP via TcpStream — jamais un simple sleep.
    pub fn wait_ready(&mut self, timeout: Duration) -> Result<(), String> {
        let deadline = Instant::now() + timeout;
        loop {
            // moteur mort → inutile d'attendre : message avec le tmp (logs dedans)
            if let Ok(Some(st)) = self.child.try_wait() {
                return Err(format!(
                    "le moteur est mort (code {:?}) — logs dans {}",
                    st.code(),
                    self.tmp.display()
                ));
            }
            for off in 0..=9u16 {
                let port = self.port + off;
                if try_hello(port) == Some(true) {
                    self.port = port;
                    self.base_url = format!("http://127.0.0.1:{}", port);
                    return Ok(());
                }
            }
            if Instant::now() >= deadline {
                return Err(format!(
                    "moteur pas prêt après {:?} (ports {}..{} sondés, tmp {})",
                    timeout,
                    self.port,
                    self.port + 9,
                    self.tmp.display()
                ));
            }
            thread::sleep(Duration::from_millis(150));
        }
    }

    /// URL loopback du moteur (port réel après wait_ready).
    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// IP LAN non-loopback de la machine (astuce UDP : connect("8.8.8.8:80")
    /// n'émet AUCUN paquet, il sélectionne juste l'interface de routage).
    /// None si indisponible (pas d'interface LAN, sandbox…).
    pub fn lan_ip(&self) -> Option<String> {
        let s = UdpSocket::bind("0.0.0.0:0").ok()?;
        s.connect("8.8.8.8:80").ok()?;
        let ip = s.local_addr().ok()?.ip();
        if ip.is_loopback() {
            return None;
        }
        Some(ip.to_string())
    }

    /// URL LAN du moteur (même port que loopback ; bind 0.0.0.0 requis côté moteur).
    pub fn lan_base_url(&self) -> Option<String> {
        self.lan_ip()
            .map(|ip| format!("http://{}:{}", ip, self.port))
    }

    /// Requête HTTP arbitraire vers `host:port` (Host: dérivé de host, sauf si
    /// les en-têtes en contiennent déjà un). Retourne une Resp même en cas
    /// d'échec de connexion (status 0) pour que les assertions parlent clair.
    pub fn request(
        &self,
        host: &str,
        port: u16,
        method: &str,
        path: &str,
        headers: &[(&str, &str)],
        body: Option<&[u8]>,
    ) -> Resp {
        let addr: SocketAddr = match format!("{}:{}", host, port).parse() {
            Ok(a) => a,
            Err(e) => return Resp::dead(format!("adresse invalide {}:{} : {}", host, port, e)),
        };
        let mut req = Vec::with_capacity(512 + body.map(|b| b.len()).unwrap_or(0));
        req.extend_from_slice(format!("{} {} HTTP/1.1\r\n", method, path).as_bytes());
        let has_host = headers.iter().any(|(n, _)| n.eq_ignore_ascii_case("host"));
        if !has_host {
            req.extend_from_slice(format!("Host: {}:{}\r\n", host, port).as_bytes());
        }
        for (n, v) in headers {
            req.extend_from_slice(format!("{}: {}\r\n", n, v).as_bytes());
        }
        match body {
            Some(b) => {
                req.extend_from_slice(
                    format!(
                        "Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                        b.len()
                    )
                    .as_bytes(),
                );
                // LE CORPS : annoncé par Content-Length, il DOIT suivre — sinon
                // le moteur attend (30 s de read timeout) et la réponse n'arrive
                // jamais du côté client.
                req.extend_from_slice(b);
            }
            None => req.extend_from_slice(b"Connection: close\r\n\r\n"),
        }
        let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
            Ok(s) => s,
            Err(e) => return Resp::dead(format!("connexion {}:{} : {}", host, port, e)),
        };
        // lecture 30 s : certaines actions sont SYNCHRONES côté moteur
        // (« scan », « discover » : sondage réel des ports) et peuvent
        // dépasser largement 10 s sous charge — un timeout trop court
        // tronquait la réponse et faisait échouer le POST (ok:false fantôme)
        stream.set_read_timeout(Some(Duration::from_secs(30))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
        if stream.write_all(&req).is_err() {
            return Resp::dead(format!("écriture {}:{} échouée", host, port));
        }
        let mut data = Vec::new();
        let mut tmp = [0u8; 16384];
        loop {
            match stream.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&tmp[..n]),
                Err(_) => break,
            }
            if data.len() > 64 * 1024 * 1024 {
                break;
            }
        }
        if data.is_empty() {
            // connecté mais fermé sans UN SEUL octet : à distinguer d'un échec
            // de connexion (les assertions affichent ce message via .text())
            eprintln!("[harnais] réponse vide de {}:{}", host, port);
            return Resp::dead(format!(
                "réponse vide : {}:{} a fermé sans répondre",
                host, port
            ));
        }
        Resp::parse(&data)
    }

    /// GET simple sur le moteur (loopback).
    pub fn get(&self, path: &str) -> Resp {
        self.request("127.0.0.1", self.port, "GET", path, &[], None)
    }

    /// GET avec en-têtes additionnels (X-MCC-Access, Origin, …).
    pub fn get_with(&self, path: &str, headers: &[(&str, &str)]) -> Resp {
        self.request("127.0.0.1", self.port, "GET", path, headers, None)
    }

    /// POST JSON sur le moteur (loopback).
    pub fn post_json(&self, path: &str, v: &Value) -> Resp {
        self.request(
            "127.0.0.1",
            self.port,
            "POST",
            path,
            &[],
            Some(v.to_string().as_bytes()),
        )
    }

    /// POST /api/action → corps JSON parsé (ok:false en Value si erreur moteur).
    pub fn action(&self, v: &Value) -> Value {
        self.post_json("/api/action", v).json()
    }

    /// Écriture TCP BRUTE (fuzz) : octets envoyés TELS QUELS, lecture jusqu'à
    /// EOF avec 5 s de timeout. Les erreurs d'écriture sont tolérées (le moteur
    /// peut fermer la connexion pendant l'envoi d'un flux hostile).
    /// port_offset permet de visiter les ports de repli du moteur (+1..+9).
    pub fn raw(&self, request_bytes: &[u8], port_offset: u16) -> Resp {
        let port = self.port.saturating_add(port_offset);
        let addr = SocketAddr::from(([127, 0, 0, 1], port));
        let mut stream = match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
            Ok(s) => s,
            Err(e) => return Resp::dead(format!("connexion :{} : {}", port, e)),
        };
        stream.set_read_timeout(Some(Duration::from_secs(5))).ok();
        stream.set_write_timeout(Some(Duration::from_secs(5))).ok();
        let _ = stream.write_all(request_bytes); // erreur possible = fuite d'info utile
        let mut data = Vec::new();
        let mut tmp = [0u8; 16384];
        loop {
            match stream.read(&mut tmp) {
                Ok(0) => break,
                Ok(n) => data.extend_from_slice(&tmp[..n]),
                Err(_) => break,
            }
            if data.len() > 8 * 1024 * 1024 {
                break;
            }
        }
        if data.is_empty() {
            // aucune réponse : connexion fermée / refusée à sec (status 0)
            Resp {
                status: 0,
                headers: vec![],
                body: b"(aucune reponse)".to_vec(),
            }
        } else {
            Resp::parse(&data)
        }
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // kill + wait : pas de zombie. Le tmp est conservé (verrous Windows +
        // pistage post-mortem des logs mcc-*.log).
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

/// Sonde /api/hello rapide (wait_ready) : Some(true) = 200 + nonce ;
/// Some(false) = port ouvert mais pas notre hello ; None = port mort.
fn try_hello(port: u16) -> Option<bool> {
    let addr = SocketAddr::from(([127, 0, 0, 1], port));
    let mut s = TcpStream::connect_timeout(&addr, Duration::from_millis(300)).ok()?;
    s.set_read_timeout(Some(Duration::from_millis(700))).ok();
    let _ = s.write_all(b"GET /api/hello HTTP/1.1\r\nHost: 127.0.0.1\r\nConnection: close\r\n\r\n");
    let mut data = Vec::new();
    let mut tmp = [0u8; 2048];
    loop {
        match s.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => data.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
        if data.len() > 65536 {
            break;
        }
    }
    let text = String::from_utf8_lossy(&data);
    Some(text.starts_with("HTTP/1.1 200") && text.contains("nonce"))
}

// ---------- réponse HTTP ----------

/// Réponse HTTP parsée : status 0 = pas de réponse (connexion fermée à sec,
/// refus de connexion…). Headers en minuscules (noms).
pub struct Resp {
    pub status: u16,
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Resp {
    fn dead(msg: String) -> Resp {
        Resp {
            status: 0,
            headers: vec![],
            body: msg.into_bytes(),
        }
    }

    /// Parse une réponse HTTP brute (status + headers + corps).
    fn parse(raw: &[u8]) -> Resp {
        let sep = find_sub(raw, b"\r\n\r\n").unwrap_or(raw.len());
        let head = String::from_utf8_lossy(&raw[..sep]).to_string();
        let mut lines = head.split("\r\n");
        let status = lines
            .next()
            .unwrap_or("")
            .split_whitespace()
            .nth(1)
            .and_then(|s| s.parse::<u16>().ok())
            .unwrap_or(0);
        let mut headers = Vec::new();
        for l in lines {
            if let Some((n, v)) = l.split_once(':') {
                headers.push((n.trim().to_ascii_lowercase(), v.trim().to_string()));
            }
        }
        let body = raw[sep.saturating_add(4).min(raw.len())..].to_vec();
        Resp {
            status,
            headers,
            body,
        }
    }

    /// Corps parsé en JSON (tolérant : Value::Null si non-JSON).
    pub fn json(&self) -> Value {
        serde_json::from_str(&self.text()).unwrap_or(Value::Null)
    }

    /// Corps en texte (UTF-8 lossy).
    pub fn text(&self) -> String {
        String::from_utf8_lossy(&self.body).to_string()
    }

    /// Premier en-tête correspondant (nom insensible à la casse).
    pub fn header(&self, name: &str) -> Option<String> {
        let name = name.to_ascii_lowercase();
        self.headers
            .iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.clone())
    }

    /// Le status correspond-il ?
    pub fn status_is(&self, s: u16) -> bool {
        self.status == s
    }
}

fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

// ---------- utilitaires d'attente ----------

/// Un port libre (bind :0 → port → drop). Course possible mais improbable ;
/// le repli +1..+9 du moteur couvre le cas.
pub fn free_port() -> u16 {
    TcpListener::bind(("127.0.0.1", 0))
        .expect("bind 127.0.0.1:0")
        .local_addr()
        .expect("local_addr")
        .port()
}

/// Sonde `f` toutes les ~150 ms jusqu'au délai : Some(true) → succès ;
/// Some(false)/None → on continue ; échéance → false. TOUTE attente e2e est
/// deadline-bounded par ce helper.
pub fn wait_until(timeout: Duration, mut f: impl FnMut() -> Option<bool>) -> bool {
    let deadline = Instant::now() + timeout;
    loop {
        if f() == Some(true) {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        thread::sleep(Duration::from_millis(150));
    }
}

/// Moteur loopback sans code, prêt (wait_ready 15 s) — le cas le plus courant.
pub fn spawn_loopback() -> Engine {
    let mut e = spawn(EngineCfg::default()).expect("spawn moteur loopback");
    e.wait_ready(Duration::from_secs(15))
        .expect("moteur loopback prêt");
    e
}

/// Moteur loopback avec code d'accès, prêt.
pub fn spawn_loopback_with_code(code: &str) -> Engine {
    let cfg = EngineCfg {
        ui_port: free_port(),
        ui_bind: "127.0.0.1",
        api_access: Some(code.to_string()),
    };
    let mut e = spawn(cfg).expect("spawn moteur loopback+code");
    e.wait_ready(Duration::from_secs(15))
        .expect("moteur loopback+code prêt");
    e
}

/// Moteur exposé au LAN (bind 0.0.0.0) avec code d'accès, prêt.
pub fn spawn_lan_with_code(code: &str) -> Engine {
    let cfg = EngineCfg {
        ui_port: free_port(),
        ui_bind: "0.0.0.0",
        api_access: Some(code.to_string()),
    };
    let mut e = spawn(cfg).expect("spawn moteur LAN");
    e.wait_ready(Duration::from_secs(15))
        .expect("moteur LAN prêt");
    e
}

// ---------- faux serveur MCP (HTTP JSON-RPC en Rust pur) ----------

/// Mode d'un serveur MCP factice.
#[derive(Clone, Copy, PartialEq)]
pub enum FakeMode {
    /// initialize → serverInfo + capabilities ; tools/list → 2 outils ;
    /// notifications/… → result vide ; TOUT le reste (GET /health, /mcp, /sse…)
    /// répond la même réponse initialize — path-agnostique, toute stratégie de
    /// sonde du moteur aboutit.
    Mcp,
    /// Toujours {"result":{}} : JSON-RPC correct mais PAS un serveur MCP
    /// (validate_initialize → NotMcp) — le moteur doit rester à PORT-ONLY.
    ResultEmpty,
    /// Accepte la connexion et ne répond JAMAIS (serveur mort-faux).
    Stall,
}

/// Serveur MCP factice en mémoire : TcpListener + thread d'acceptation,
/// un thread par connexion, réponses 200 JSON + Connection: close.
pub struct Fixture {
    port: u16,
    stop: Arc<AtomicBool>,
    th: Option<thread::JoinHandle<()>>,
}

impl Fixture {
    /// Faux MCP complet (initialize valide + tools/list).
    pub fn spawn_mcp() -> Fixture {
        Self::spawn(FakeMode::Mcp)
    }

    /// Faux serveur qui répond toujours {"result":{}} (pas un MCP).
    pub fn spawn_result_empty() -> Fixture {
        Self::spawn(FakeMode::ResultEmpty)
    }

    /// Faux serveur muet : accepte, ne répond jamais.
    pub fn spawn_stall() -> Fixture {
        Self::spawn(FakeMode::Stall)
    }

    fn spawn(mode: FakeMode) -> Fixture {
        let listener = TcpListener::bind(("127.0.0.1", 0)).expect("bind fixture 127.0.0.1:0");
        let port = listener.local_addr().expect("local_addr fixture").port();
        let stop = Arc::new(AtomicBool::new(false));
        let stop2 = stop.clone();
        let th = thread::Builder::new()
            .name("mcc-fake-mcp".into())
            .spawn(move || {
                let _ = listener.set_nonblocking(true);
                loop {
                    if stop2.load(Ordering::SeqCst) {
                        break;
                    }
                    match listener.accept() {
                        Ok((s, _)) => {
                            // thread détaché : les connexions stall peuvent durer
                            let _ = thread::Builder::new()
                                .name("mcc-fake-conn".into())
                                .spawn(move || fake_conn(s, mode));
                        }
                        Err(_) => thread::sleep(Duration::from_millis(25)),
                    }
                }
                // listener droppé ici → port libéré
            })
            .expect("spawn thread fixture");
        Fixture {
            port,
            stop,
            th: Some(th),
        }
    }

    /// Port éphémère du faux serveur.
    pub fn port(&self) -> u16 {
        self.port
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        // join du thread d'acceptation → listener droppé → port fermé
        if let Some(h) = self.th.take() {
            let _ = h.join();
        }
    }
}

/// Initialise MCP valide (passe validate_initialize : serverInfo.name présent).
fn init_json() -> String {
    json!({
        "jsonrpc": "2.0", "id": 1,
        "result": {
            "protocolVersion": "2025-06-18",
            "capabilities": {"tools": {}},
            "serverInfo": {"name": "fake-e2e-mcp", "version": "0.2.0"}
        }
    })
    .to_string()
}

/// tools/list : deux outils, comme un vrai serveur.
fn tools_json() -> String {
    json!({
        "jsonrpc": "2.0", "id": 2,
        "result": {"tools": [
            {"name": "ping", "description": "ping tool", "inputSchema": {"type": "object"}},
            {"name": "echo", "description": "echo tool", "inputSchema": {"type": "object"}}
        ]}
    })
    .to_string()
}

/// Traite UNE connexion du faux serveur : lit l'en-tête + le corps, répond
/// selon le mode. Path-agnostique : le moteur sonde /mcp, /, /api/mcp, /health.
fn fake_conn(mut s: TcpStream, mode: FakeMode) {
    let _ = s.set_read_timeout(Some(Duration::from_secs(10)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 4096];
    let head_end = loop {
        match s.read(&mut tmp) {
            Ok(0) => return,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(p) = find_sub(&buf, b"\r\n\r\n") {
                    break p;
                }
                if buf.len() > 65536 {
                    return;
                }
            }
            Err(_) => return,
        }
    };
    // Content-Length (première occurrence — nos clients n'en envoient qu'un)
    let head = String::from_utf8_lossy(&buf[..head_end]).to_ascii_lowercase();
    let mut cl = 0usize;
    for line in head.lines().skip(1) {
        if let Some(v) = line.strip_prefix("content-length:") {
            cl = v.trim().parse().unwrap_or(0);
            break;
        }
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < cl {
        match s.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => body.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
    }
    body.truncate(cl);

    if mode == FakeMode::Stall {
        // accepté mais JAMAIS de réponse : on avale jusqu'à la fin (le client
        // fermant ou le timeout de lecture met fin au thread)
        let mut sink = Vec::new();
        let _ = s.read_to_end(&mut sink);
        return;
    }

    let payload = match mode {
        FakeMode::Mcp => {
            let method = serde_json::from_slice::<Value>(&body)
                .ok()
                .and_then(|v| v.get("method").and_then(|m| m.as_str()).map(String::from))
                .unwrap_or_default();
            match method.as_str() {
                "tools/list" => tools_json(),
                // notifications/initialized et toute autre méthode : result vide
                "notifications/initialized" => {
                    json!({"jsonrpc": "2.0", "id": 3, "result": {}}).to_string()
                }
                // initialize (et GET sans corps : /health, /, /sse…) → initialize
                _ => init_json(),
            }
        }
        FakeMode::ResultEmpty => json!({"jsonrpc": "2.0", "id": 1, "result": {}}).to_string(),
        FakeMode::Stall => unreachable!(),
    };
    let resp = format!(
        "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
        payload.len(),
        payload
    );
    let _ = s.write_all(resp.as_bytes());
    let _ = s.flush();
}
