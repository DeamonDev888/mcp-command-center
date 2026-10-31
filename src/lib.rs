pub mod crypto;
pub mod frameworks;
pub mod gateway;
pub mod logos;
pub mod manage;
pub mod notify;
pub mod probe;
pub mod runtime;
pub mod store_aitmpl;
pub mod store_github;
pub mod store_glama;
pub mod types;
pub mod ui;
pub mod validate;

use crate::types::{lockv, AppState};
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

/// Corps de requête max (anti-DoS : l'ancien code suivait un Content-Length
/// choisi par le client sans plafond → gonflement mémoire infini).
const MAX_BODY: usize = 2 * 1024 * 1024;
/// Connexions simultanées max (un thread par connexion : au-delà on refuse net).
const MAX_CONNS: usize = 64;
/// CSP de la page UI (servie en en-tête HTTP par CE serveur — le webview Tauri
/// charge une URL externe, la config `csp` de tauri.conf.json ne s'y applique
/// pas). Depuis le split en assets servis (v0.7.19), scripts et styles sont
/// des fichiers même origine ('self') ; 'unsafe-inline' reste nécessaire pour
/// les attributs on* des gabarits (la délégation d'événements est le chantier
/// suivant) et pour les styles inline générés par le JS. Une XSS future ne
/// peut toujours ni charger du code externe ni exfiltrer (connect-src 'self').
const UI_CSP: &str = "default-src 'none'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; connect-src 'self' ipc: http://ipc.localhost; img-src 'self' data:; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

/// Version de spec annoncée par litemcp — premier signal du fingerprint :
/// litemcp répond initialize avec 2026-07-28 même quand le client demande
/// 2024-11-05 (stateless strict, pas de négociation).
pub const LITEMCP_PROTOCOL: &str = "2026-07-28";

static OPEN_CONNS: AtomicUsize = AtomicUsize::new(0);

struct ConnGuard;
impl Drop for ConnGuard {
    fn drop(&mut self) {
        OPEN_CONNS.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Vrai si le bind n'expose que la machine locale.
pub fn is_loopback_bind(s: &str) -> bool {
    matches!(s, "127.0.0.1" | "localhost" | "::1" | "[::1]")
}

/// Le code d'accès est-il exigé pour CETTE connexion ? Uniquement quand l'UI
/// est exposée au réseau (bind non-loopback) ET que le client vient du réseau :
/// les connexions locales (fenêtre desktop, navigateur de la machine) n'ont
/// JAMAIS de code à saisir. Les contrôles Host/Origin (anti CSRF/rebinding)
/// restent appliqués à toutes les connexions, locales incluses.
pub fn effective_require_access(bind_is_loopback: bool, peer_is_loopback: bool) -> bool {
    !bind_is_loopback && !peer_is_loopback
}

/// Hex aléatoire de `words` × 64 bits — CSPRNG de l'OS : les nonces de preuve
/// de pair et les codes d'accès exigent une imprévisibilité cryptographique.
/// RandomState (SipHash semé au boot, puis compteur thread-local) ne la
/// fournit PAS — deux appels successifs y sont corrélables. Zéro dépendance :
/// BCryptGenRandom sur Windows, /dev/urandom sur Unix. Repli (hors pratique,
/// les deux sources OS sont toujours disponibles) : hash semé aléatoirement
/// mélangé à l'horloge et à un compteur atomique — strictement mieux que
/// RandomState seul, et le résultat reste improvisible entre processus.
fn random_hex(words: usize) -> String {
    let mut out = String::with_capacity(words * 16);
    let mut buf = [0u8; 8];
    for _ in 0..words {
        if !os_random(&mut buf) {
            use std::hash::{BuildHasher, Hasher};
            static FALLBACK_COUNTER: std::sync::atomic::AtomicU64 =
                std::sync::atomic::AtomicU64::new(0);
            let mut h = std::collections::hash_map::RandomState::new().build_hasher();
            h.write_u64(crate::types::now_ts());
            h.write_u64(FALLBACK_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
            buf = h.finish().to_le_bytes();
        }
        out.push_str(&format!("{:016x}", u64::from_le_bytes(buf)));
    }
    out
}

/// Remplit `buf` avec des octets cryptographiquement aléatoires de l'OS.
/// `true` = rempli ; `false` = source indisponible (l'appelant a son repli).
fn os_random(buf: &mut [u8]) -> bool {
    #[cfg(windows)]
    {
        #[link(name = "bcrypt")]
        extern "system" {
            fn BCryptGenRandom(
                hAlgorithm: usize,
                pBuffer: *mut u8,
                cbBuffer: u32,
                dwFlags: u32,
            ) -> i32;
        }
        // BCRYPT_USE_SYSTEM_PREFERRED_RNG : le RNG préféré du système.
        // NTSTATUS 0 = succès.
        unsafe { BCryptGenRandom(0, buf.as_mut_ptr(), buf.len() as u32, 0x0000_0002) == 0 }
    }
    #[cfg(not(windows))]
    {
        use std::io::Read;
        std::fs::File::open("/dev/urandom")
            .and_then(|mut f| f.read_exact(buf))
            .is_ok()
    }
}

/// Code d'accès aléatoire 128 bits (hex).
fn generate_access() -> String {
    random_hex(2)
}

// ---------- preuve de pair (audit 2026-10-24 S-B) ----------
//
// Avant : discover_peers envoyait NOTRE code d'accès en clair (X-MCC-Access) à
// toutes les cibles sondées du LAN — n'importe quel hôte à l'écoute récupérait
// le secret. Désormais : le sondeur récupère un nonce à usage unique sur
// GET /api/hello (préauthentifié, aucune donnée) et signe ce nonce avec le code
// (HMAC-SHA256, cf. crypto.rs). Le pair vérifie la signature avec SA copie du
// code : un observateur réseau ne récupère qu'une preuve morte après usage.

/// Nonces émis par GET /api/hello → expiration (usage unique + TTL court).
fn hello_nonces_slot() -> &'static std::sync::Mutex<std::collections::HashMap<String, u64>> {
    static SLOT: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, u64>>> =
        std::sync::OnceLock::new();
    SLOT.get_or_init(|| std::sync::Mutex::new(std::collections::HashMap::new()))
}

const HELLO_NONCE_TTL: u64 = 120;
const HELLO_NONCE_MAX: usize = 1024;

fn issue_hello_nonce() -> String {
    let mut slot = hello_nonces_slot()
        .lock()
        .unwrap_or_else(|p| p.into_inner());
    let now = crate::types::now_ts();
    slot.retain(|_, exp| *exp > now);
    if slot.len() >= HELLO_NONCE_MAX {
        slot.clear(); // inondation : on repart à zéro (les preuves en vol expireront)
    }
    let nonce = random_hex(4);
    slot.insert(nonce.clone(), now + HELLO_NONCE_TTL);
    nonce
}

/// Vérifie une preuve de pair : le nonce doit avoir été émis par NOUS (et pas
/// encore consommé), et le HMAC doit correspondre à NOTRE code d'accès.
fn verify_proof(nonce: &str, proof: &str, api_access: &str) -> bool {
    if nonce.len() != 64 || !nonce.chars().all(|c| c.is_ascii_hexdigit()) {
        return false;
    }
    let issued = hello_nonces_slot()
        .lock()
        .unwrap_or_else(|p| p.into_inner())
        .remove(nonce)
        .is_some();
    if !issued {
        return false;
    }
    let expected = crate::crypto::hmac_sha256_hex(
        api_access.as_bytes(),
        format!("mcc-proof-v1:{}", nonce).as_bytes(),
    );
    crate::crypto::ct_eq(expected.as_bytes(), proof.as_bytes())
}

/// Extrait un paramètre de query d'un chemin (ex. access= pour /api/detail).
fn query_param(path: &str, name: &str) -> Option<String> {
    let q = path.split_once('?')?.1;
    for pair in q.split('&') {
        let Some((k, v)) = pair.split_once('=') else {
            continue;
        };
        if k == name {
            return Some(urldecode(v));
        }
    }
    None
}

/// Contrôle d'accès central (pur, testable) :
/// - OPTIONS → 403 : les préflights CORS meurent ici, aucun cross-site ne passe ;
/// - Host hors liste → 403 (anti DNS-rebinding : un domaine rebindexé n'est pas le nôtre) ;
/// - Origin hors liste → 403 (anti CSRF cross-site) ;
/// - /api/* hors loopback → 401 sans le bon code (header X-MCC-Access ou ?access=),
///   SAUF /api/hello (nonce de préauthentification, aucune donnée) et SAUF si une
///   preuve de pair valide a déjà été vérifiée (proof_ok, cf. verify_proof) ;
/// - POST /api/action → 415 si Content-Type ≠ application/json (bloque les POST
///   « simples » text/plain des sites web, qui s'exécutent sans préflight).
///
/// Extrait la partie hote (sans port, sans crochets) d'un header Host ou d'une origine.
/// Gere l'IPv6 brackete et les ports numeriques ; minuscules pour comparaison.
pub fn host_only_of(h: &str) -> String {
    let h = h.trim();
    if let Some(rest) = h.strip_prefix('[') {
        if let Some(idx) = rest.find(']') {
            return rest[..idx].to_ascii_lowercase();
        }
    }
    if let Some((host, port)) = h.rsplit_once(':') {
        if !port.is_empty() && port.chars().all(|c| c.is_ascii_digit()) {
            let host = host.trim_matches(|c| c == '[' || c == ']');
            return host.to_ascii_lowercase();
        }
    }
    h.trim_matches(|c| c == '[' || c == ']')
        .to_ascii_lowercase()
}

/// Hote normalise extrait d'une origine HTTP complete (schema://hote[:port][/...]).
fn origin_host(o: &str) -> String {
    let r = o.split("://").nth(1).unwrap_or(o);
    let r = r.split('/').next().unwrap_or(r);
    host_only_of(r)
}

#[allow(clippy::too_many_arguments)] // signature pur/testable, les 8 paramètres sont utiles
pub fn check_access(
    require_access: bool,
    api_access: Option<&str>,
    allowed_hosts: &[String],
    allowed_origins: &[String],
    method: &str,
    path: &str,
    headers: &[(String, String)],
    proof_ok: bool,
) -> Result<(), (u16, &'static str)> {
    if method == "OPTIONS" {
        return Err((403, "Forbidden"));
    }
    if let Some((_, h)) = headers.iter().find(|(n, _)| n == "host") {
        let host_only = host_only_of(h);
        if !allowed_hosts
            .iter()
            .any(|a| a.to_ascii_lowercase() == host_only)
        {
            return Err((403, "Forbidden"));
        }
    }
    // anti request-smuggling : refuser les en-tetes de cadrage ambigus/dupliques
    let cl_count = headers
        .iter()
        .filter(|(n, _)| n == "content-length")
        .count();
    if cl_count > 1 {
        return Err((400, "Bad Request"));
    }
    if headers.iter().any(|(n, _)| n == "transfer-encoding") {
        return Err((400, "Bad Request"));
    }
    if let Some((_, o)) = headers.iter().find(|(n, _)| n == "origin") {
        if !allowed_origins.iter().any(|a| a == o) {
            return Err((403, "Forbidden"));
        }
    }
    let bare_path = path.split('?').next().unwrap_or("");
    // /api/hello est PRÉ-authentifié : il ne délivre qu'un nonce à usage unique,
    // aucune donnée — c'est l'amorce de la preuve de pair (cf. verify_proof).
    let is_hello = bare_path == "/api/hello";
    // le gateway MCP (/mcp) est protégé comme /api/* : hors loopback, le code
    // d'accès est exigé — c'est le même sas de confiance
    let gated_path = bare_path.starts_with("/api") || bare_path == "/mcp";
    if require_access && gated_path && !is_hello && !proof_ok {
        let hdr = headers
            .iter()
            .find(|(n, _)| n == "x-mcc-access")
            .map(|(_, v)| v.clone());
        let q = query_param(path, "access");
        let provided: Option<&str> = hdr.as_deref().or(q.as_deref());
        let api_access = api_access.filter(|s| !s.is_empty());
        match (api_access, provided) {
            (Some(expected), Some(got))
                if crate::crypto::ct_eq(expected.as_bytes(), got.as_bytes()) => {}
            _ => return Err((401, "Unauthorized")),
        }
    }
    if method == "POST" && bare_path == "/api/action" {
        let ok = headers
            .iter()
            .find(|(n, _)| n == "content-type")
            .map(|(_, v)| v.to_ascii_lowercase().starts_with("application/json"))
            .unwrap_or(false);
        if !ok {
            return Err((415, "Unsupported Media Type"));
        }
    }
    Ok(())
}

fn reason_phrase(status: u16) -> &'static str {
    match status {
        202 => "Accepted",
        400 => "Bad Request",
        200 => "OK",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        406 => "Not Acceptable",
        413 => "Payload Too Large",
        415 => "Unsupported Media Type",
        503 => "Service Unavailable",
        500 => "Internal Server Error",
        _ => "Bad Request",
    }
}

/// Résultat de `run_headless` : l'URL de l'UI, et si un moteur TOURNAIT DÉJÀ
/// (instance unique — cf. la logique de reconnexion dans run_headless).
pub struct RunOutcome {
    pub url: String,
    pub reused: bool,
}

pub fn run_headless() -> RunOutcome {
    let config_path =
        manage::resolve_config_path(env!("CARGO_MANIFEST_DIR"), std::env::var("MCC_CONFIG").ok());
    // ROBUSTESSE (audit 2026-10-25) : un panic dans un thread (poller, sonde)
    // était TOTALEMENT silencieux en mode desktop (windows_subsystem=windows,
    // aucune console) et tuait la supervision sans trace. Le hook écrit le
    // panic sur disque (mcc.log) et sur stderr.
    {
        let log_file = std::path::Path::new(&config_path)
            .with_file_name("mcc.log")
            .to_string_lossy()
            .to_string();
        std::panic::set_hook(Box::new(move |info| {
            let ts = crate::types::iso_utc(crate::types::now_ts());
            let payload = info
                .payload()
                .downcast_ref::<&str>()
                .map(|s| s.to_string())
                .or_else(|| info.payload().downcast_ref::<String>().cloned())
                .unwrap_or_else(|| "?".to_string());
            let loc = info
                .location()
                .map(|l| format!("{}:{}", l.file(), l.line()))
                .unwrap_or_default();
            let line = format!("[{}] [panic] {} ({})\n", ts, payload, loc);
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&log_file)
            {
                let _ = f.write_all(line.as_bytes());
            }
            eprint!("{}", line);
        }));
    }
    let (servers, settings) = manage::load_config(&config_path);
    let state = Arc::new(AppState {
        servers: Mutex::new(servers),
        log: Mutex::new(std::collections::VecDeque::new()),
        settings: Mutex::new(settings.clone()),
        children: Mutex::new(std::collections::HashMap::new()),
        peers: Mutex::new(std::collections::HashMap::new()),
        allowed_origins: Mutex::new(Vec::new()),
        allowed_hosts: Mutex::new(Vec::new()),
        spawn_fails: Mutex::new(std::collections::HashMap::new()),
        runtime: Mutex::new(manage::load_runtime(&config_path)),
        spawning: Mutex::new(std::collections::HashSet::new()),
        update_latest: Mutex::new(None),
        lan_ok: Mutex::new(None),
        config_path,
    });
    // cache de démarrage : restaure le dernier état connu AVANT la première
    // sonde — l'UI est peuplée instantanément, les sondes corrigent ensuite
    manage::load_cache(&state);

    let mut ui_port = settings.ui_port;
    let ui_bind = settings.ui_bind.clone();
    let listener = loop {
        match TcpListener::bind((ui_bind.as_str(), ui_port)) {
            Ok(l) => break l,
            Err(_) => {
                // port occupé : si un AUTRE moteur MCP Control y tourne déjà,
                // on s'y RECONNECTE au lieu d'en lancer un second (instance
                // unique — deux moteurs sur la même config se disputeraient les
                // spawns, réécriraient config.json et doublerraient les scans).
                // Occupant étranger → port suivant (comportement historique).
                // Un moteur peut être détecté puis mourir dans la seconde
                // (relance pendant un build, arrêt manuel) : on ne réutilise
                // une URL que si elle répond DEUX fois de suite, espacées de
                // 300 ms. Un moteur moribond ne survivra pas à la seconde
                // mesure — sinon la fenêtre s'ouvre sur un port mort.
                // Le moteur à détecter écoute sur le bind de la config : on
                // interroge CETTE adresse (un bind 0.0.0.0 ou une IP LAN
                // précise n'est pas joignable par défaut sur 127.0.0.1).
                let bind_host: Option<&str> = match ui_bind.as_str() {
                    "" | "0.0.0.0" | "*" => None,
                    h => Some(h),
                };
                let mut probe_hosts: Vec<&str> = Vec::new();
                if let Some(h) = bind_host {
                    probe_hosts.push(h);
                }
                for h in ["127.0.0.1", "::1"] {
                    if !probe_hosts.contains(&h) {
                        probe_hosts.push(h);
                    }
                }
                let mut dead_motor = false;
                for _ in 0..2 {
                    if let Some(url) = probe::control_ui_at_hosts(ui_port, &probe_hosts) {
                        thread::sleep(Duration::from_millis(300));
                        if probe::control_ui_at_hosts(ui_port, &probe_hosts).is_some() {
                            manage::add_log(
                                &state,
                                "info",
                                &format!(
                                    "moteur déjà en cours sur {} — réutilisé : aucun second moteur lancé (sondes, autostart et scan restent à l'instance existante)",
                                    url
                                ),
                            );
                            return RunOutcome { url, reused: true };
                        }
                        dead_motor = true;
                        break;
                    }
                    thread::sleep(Duration::from_millis(250));
                }
                if dead_motor {
                    // Le moteur détecté était en train de mourir : on attend
                    // que le port se libère pour lancer NOTRE moteur, sinon
                    // la fenêtre pointerait vers un port mort.
                    manage::add_log(
                        &state,
                        "warn",
                        &format!(
                            "moteur précédent sur le port {} : arrêt en cours, attente de libération",
                            ui_port
                        ),
                    );
                    for _ in 0..12 {
                        if TcpListener::bind((ui_bind.as_str(), ui_port)).is_ok() {
                            break;
                        }
                        thread::sleep(Duration::from_millis(250));
                    }
                }
                manage::add_log(
                    &state,
                    "warn",
                    &format!("port {} occupé (étranger), essai {}", ui_port, ui_port + 1),
                );
                ui_port += 1;
                if ui_port > settings.ui_port + 9 {
                    eprintln!(
                        "[fatal] aucun port UI libre ({}..{})",
                        settings.ui_port,
                        settings.ui_port + 9
                    );
                    std::process::exit(1);
                }
            }
        }
    };
    manage::add_log(
        &state,
        "info",
        &format!("mcp-control v{} démarré", env!("CARGO_PKG_VERSION")),
    );
    {
        let mut st = lockv(&state.settings);
        st.ui_port = ui_port;
    }
    // SÉCURITÉ (audit v0.6.6) :
    // - bind non-loopback → code d'accès API généré au boot si absent (persisté) ;
    // - origines/hostes autorisés (anti CSRF / DNS-rebinding) figés au boot.
    let local_ips_list = manage::local_ips();
    let loopback = is_loopback_bind(&ui_bind);
    let mut generated = false;
    if !loopback {
        let mut st = lockv(&state.settings);
        if st.api_access.as_deref().map(str::is_empty).unwrap_or(true) {
            st.api_access = Some(generate_access());
            generated = true;
        }
    }
    {
        let extra = lockv(&state.settings).allowed_origins.clone();
        let mut origins: Vec<String> = vec![
            format!("http://127.0.0.1:{}", ui_port),
            format!("http://localhost:{}", ui_port),
            format!("http://[::1]:{}", ui_port),
        ];
        let mut hosts: Vec<String> = vec![
            "127.0.0.1".to_string(),
            "localhost".to_string(),
            "::1".to_string(),
        ];
        for ip in &local_ips_list {
            origins.push(format!("http://{}:{}", ip, ui_port));
            hosts.push(ip.clone());
        }
        for o in &extra {
            origins.push(o.clone());
            hosts.push(origin_host(o));
        }
        *lockv(&state.allowed_origins) = origins;
        *lockv(&state.allowed_hosts) = hosts;
    }
    if generated {
        manage::save_config(&state);
        manage::add_log(
            &state,
            "info",
            "code d'accès API généré au boot (persisté dans config.json: settings.api_access) — exigé sur /api/* pour tout accès hors loopback ; pour les pairs, mettre la même valeur des deux côtés",
        );
    }
    let ui_host = if ui_bind == "0.0.0.0" {
        "<ip-lan>".to_string()
    } else {
        ui_bind.clone()
    };
    let ui_url_host = if ui_bind == "0.0.0.0" {
        "127.0.0.1".to_string()
    } else {
        ui_bind.clone()
    };
    let ui_url = format!("http://{}:{}", ui_url_host, ui_port);
    println!("[UI] {}  (Ctrl+C pour arrêter)", ui_url);
    manage::add_log(
        &state,
        "info",
        &format!(
            "UI prête sur http://{}:{} (bind {})",
            ui_host, ui_port, ui_bind
        ),
    );

    // poller : première passe immédiate, ensuite refresh_secs.
    // catch_unwind : un panic dans un cycle de sondage ne doit JAMAIS tuer la
    // supervision (c'était le cas avant l'audit 2026-10-25 — le thread mourait
    // en silence et l'app affichait un état figé).
    {
        let st = state.clone();
        thread::spawn(move || loop {
            let cycle =
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| manage::poll_all(&st)));
            if cycle.is_err() {
                manage::add_log(
                    &st,
                    "error",
                    "panic pendant un cycle de sondage — le poller a survécu (détail dans mcc.log)",
                );
            }
            let secs = lockv(&st.settings).refresh_secs;
            thread::sleep(Duration::from_secs(secs));
        });
    }
    // veille réseau : la passerelle LAN répond-elle ? (VPN, câble, Wi-Fi)
    // Mesuré tout de suite puis toutes les 30 s — l'UI explique les serveurs
    // distants injoignables au lieu de les afficher comme en panne.
    {
        let st = state.clone();
        thread::spawn(move || loop {
            manage::refresh_lan_state(&st);
            thread::sleep(Duration::from_secs(30));
        });
    }
    // AUTO-SCAN au boot : tous les ports en écoute de la machine (netstat/ss),
    // plus de plage à deviner — un serveur MCP sur n'importe quel port (8787
    // compris) est découvert même si scan_from/scan_to ne le couvre pas.
    {
        let st = state.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_millis(1200));
            manage::scan_listening(&st);
        });
    }
    // démarrage automatique : aligne le registre/fichier sur la préférence
    // persistée (créé au premier lancement — settings.autostart vaut true)
    manage::sync_autostart_at_boot(&state);
    // vérification de mise à jour (release GitHub) : au boot puis chaque heure
    {
        let st = state.clone();
        thread::spawn(move || loop {
            thread::sleep(Duration::from_secs(8));
            manage::check_update(&st);
            thread::sleep(Duration::from_secs(3600));
        });
    }
    // découverte des pairs MCP Control + réseau (après le scan local)
    {
        let st = state.clone();
        thread::spawn(move || {
            thread::sleep(Duration::from_secs(30));
            manage::discover_peers(&st);
            let enabled = crate::types::lockv(&st.settings).network_enabled;
            if enabled {
                let (from, to) = {
                    let s = crate::types::lockv(&st.settings);
                    (s.scan_from, s.scan_to)
                };
                manage::scan_network(&st, Some((from, to)));
            }
        });
    }

    // GATEWAY MCP : la console est elle-même un serveur MCP — POST /mcp sur
    // le port UI, toute la surface de l'app en tools + proxy vers les
    // serveurs détectés (cf. gateway::serve). Le snapshot des ressources est
    // reconstruit quand l'empreinte de détection change (thread ci-dessous ;
    // les actions passées par le gateway lui-même rafraîchissent immédiatement).
    let gw = gateway::install(&state);
    {
        let msg = format!(
            "gateway MCP prêt : {} outils de supervision + proxy call_server_tool — les clients MCP joignent POST {}/mcp{}",
            gateway::tool_count(&gw.snapshot()),
            ui_url,
            if is_loopback_bind(&ui_bind) {
                String::new()
            } else {
                " (code d'accès exigé hors loopback, comme /api/*)".to_string()
            }
        );
        println!("[MCP] {msg}");
        manage::add_log(&state, "info", &msg);
    }
    {
        let st = state.clone();
        thread::spawn(move || {
            let mut last = gateway::fingerprint(&st);
            loop {
                thread::sleep(Duration::from_secs(5));
                let now = gateway::fingerprint(&st);
                if now != last {
                    last = now;
                    gateway::refresh(&st);
                }
            }
        });
    }

    // serveur HTTP en tâche de fond : run_headless retourne, l'app hôte décide
    // (main = park forever, desktop = boucle d'événements Tauri)
    thread::spawn(move || {
        for stream in listener.incoming() {
            match stream {
                Ok(s) => {
                    let st = state.clone();
                    thread::spawn(move || handle_conn(s, st));
                }
                Err(e) => eprintln!("[accept] {}", e),
            }
        }
    });
    RunOutcome {
        url: ui_url,
        reused: false,
    }
}

/// Bloque le thread courant indéfiniment (mode CLI headless).
pub fn park_forever() {
    loop {
        thread::sleep(Duration::from_secs(3600));
    }
}

pub fn find_sub(hay: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.len() > hay.len() {
        return None;
    }
    hay.windows(needle.len()).position(|w| w == needle)
}

pub fn handle_conn(mut stream: TcpStream, state: Arc<AppState>) {
    // garde de charge : au-delà de MAX_CONNS connexions simultanées, on refuse net
    // (un thread par connexion — sinon épuisement trivial depuis le LAN)
    if OPEN_CONNS.fetch_add(1, Ordering::SeqCst) >= MAX_CONNS {
        OPEN_CONNS.fetch_sub(1, Ordering::SeqCst);
        return;
    }
    let _guard = ConnGuard;
    let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
    let mut buf = Vec::new();
    let mut tmp = [0u8; 16384];
    let mut head_end = None;
    loop {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                buf.extend_from_slice(&tmp[..n]);
                if let Some(p) = find_sub(&buf, b"\r\n\r\n") {
                    head_end = Some(p);
                    break;
                }
                if buf.len() > 1_048_576 {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    let head_end = match head_end {
        Some(p) => p,
        None => return,
    };
    let head = String::from_utf8_lossy(&buf[..head_end]).to_string();
    let mut lines = head.split("\r\n");
    let req_line = lines.next().unwrap_or("");
    let mut parts = req_line.split_whitespace();
    let method = parts.next().unwrap_or("").to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let mut content_length = 0usize;
    let mut headers: Vec<(String, String)> = Vec::new();
    for l in lines {
        if let Some((name, value)) = l.split_once(':') {
            headers.push((name.trim().to_ascii_lowercase(), value.trim().to_string()));
        }
    }
    for (name, value) in &headers {
        if name == "content-length" {
            content_length = value.parse().unwrap_or(0);
        }
    }
    let mut body = buf[head_end + 4..].to_vec();
    while body.len() < content_length {
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => {
                body.extend_from_slice(&tmp[..n]);
                if body.len() > MAX_BODY {
                    break;
                }
            }
            Err(_) => break,
        }
    }
    if body.len() > MAX_BODY {
        let j = serde_json::json!({"ok": false, "error": "body trop grand (max 2 Mo)"}).to_string();
        let resp = format!(
            "HTTP/1.1 413 Payload Too Large\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
            j.len(),
            j
        );
        let _ = stream.write_all(resp.as_bytes());
        return;
    }
    body.truncate(content_length.min(MAX_BODY));

    // contrôle d'accès (preuve de pair/token/Origin/Host/Content-Type) puis routage
    let (loopback_bind, api_access) = {
        let st = lockv(&state.settings);
        (is_loopback_bind(&st.ui_bind), st.api_access.clone())
    };
    // client local (fenêtre desktop, navigateur de la machine) → jamais de code
    let peer_is_loopback = stream
        .peer_addr()
        .map(|a| a.ip().is_loopback())
        .unwrap_or(false);
    let (allowed_hosts, allowed_origins) = (
        lockv(&state.allowed_hosts).clone(),
        lockv(&state.allowed_origins).clone(),
    );
    // preuve de pair : X-MCC-Nonce + X-MCC-Proof (HMAC du nonce avec le code
    // partagé) authentifie sans que le code n'ait jamais voyagé en clair
    let proof_ok = {
        let nonce = headers
            .iter()
            .find(|(n, _)| n == "x-mcc-nonce")
            .map(|(_, v)| v.as_str());
        let proof = headers
            .iter()
            .find(|(n, _)| n == "x-mcc-proof")
            .map(|(_, v)| v.as_str());
        match (nonce, proof, api_access.as_deref()) {
            (Some(n), Some(p), Some(code)) if !code.is_empty() => verify_proof(n, p, code),
            _ => false,
        }
    };
    let gated = check_access(
        effective_require_access(loopback_bind, peer_is_loopback),
        api_access.as_deref(),
        &allowed_hosts,
        &allowed_origins,
        &method,
        &path,
        &headers,
        proof_ok,
    );
    let (status, ctype, payload) = match gated {
        Err((code, reason)) => {
            let j = serde_json::json!({"ok": false, "error": reason}).to_string();
            (code, "application/json", j.into_bytes())
        }
        // Gateway MCP : la console elle-même est un serveur MCP — POST /mcp
        // (émule le transport Streamable HTTP stateless de LiteMCP sur notre
        // socket déjà durci ; cf. gateway::serve)
        Ok(()) if path.split('?').next() == Some("/mcp") => {
            gateway::serve(&method, &headers, &body)
        }
        Ok(()) => route(&state, &method, &path, &body),
    };
    // ACAO dynamique : on reflète UNIQUEMENT une origine autorisée (jamais *),
    // sinon aucun site tiers ne peut lire nos réponses (/api/state inclu).
    let mut resp = format!(
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\n",
        status,
        reason_phrase(status),
        ctype,
        payload.len()
    );
    if ctype == "application/json" || ctype == "text/event-stream" {
        resp.push_str("Cache-Control: no-store\r\n");
    }
    // Coquille HTML : no-cache — chaque navigation doit repartir du lien
    // ?v=VERSION à jour vers les assets.
    if ctype.starts_with("text/html") {
        resp.push_str(&format!("Content-Security-Policy: {}\r\n", UI_CSP));
        resp.push_str("Cache-Control: no-cache\r\n");
    }
    // Assets UI : no-cache aussi — les fichiers sont locaux (refetch négligeable
    // sur loopback) et un cache même versionné créerait des pages périmées en
    // dev (même version, binaire différent). Le 304/ETag reste le chantier perf.
    if ctype.starts_with("text/css") || ctype.starts_with("text/javascript") {
        resp.push_str("Cache-Control: no-cache\r\n");
    }
    if let Some((_, o)) = headers.iter().find(|(n, _)| n == "origin") {
        if allowed_origins.iter().any(|a| a == o) {
            resp.push_str(&format!("Access-Control-Allow-Origin: {}\r\n", o));
        }
    }
    resp.push_str("Connection: close\r\n\r\n");
    let _ = stream.write_all(resp.as_bytes());
    let _ = stream.write_all(&payload);
    let _ = stream.flush();
}

/// Versionne un asset UI : le placeholder de version devient la version réelle
/// (utilisé par app.js pour afficher sa version et par les query ?v=).
fn ui_asset(s: &str) -> Vec<u8> {
    s.replace("__MCC_VERSION__", env!("CARGO_PKG_VERSION"))
        .into_bytes()
}

pub fn route(
    state: &Arc<AppState>,
    method: &str,
    path: &str,
    body: &[u8],
) -> (u16, &'static str, Vec<u8>) {
    let bare_path = path.split('?').next().unwrap_or(path);
    if method == "GET" && bare_path == "/" {
        // SÉCURITÉ (audit 2026-10-24) : le code d'accès n'est PLUS injecté dans la
        // page — quiconque joint le port pouvait le lire sur / et contourner toute
        // l'auth. L'UI le demande à l'utilisateur (login), le garde en localStorage
        // et le présente sur chaque appel /api/* (bind loopback : jamais demandé).
        let html = ui::INDEX_HTML.replace("__MCC_VERSION__", env!("CARGO_PKG_VERSION"));
        return (200, "text/html; charset=utf-8", html.into_bytes());
    }
    // Assets de l'UI (split v0.7.19) : servis PRÉ-authentification comme la
    // coquille — ils ne contiennent aucun secret (le code d'accès ne quitte
    // jamais le serveur) et la page de login en a besoin pour s'afficher.
    if method == "GET" && bare_path == "/app.css" {
        return (200, "text/css; charset=utf-8", ui_asset(ui::APP_CSS));
    }
    if method == "GET" && bare_path == "/app-boot.js" {
        return (
            200,
            "text/javascript; charset=utf-8",
            ui_asset(ui::APP_BOOT_JS),
        );
    }
    if method == "GET" && bare_path == "/app.js" {
        return (200, "text/javascript; charset=utf-8", ui_asset(ui::APP_JS));
    }
    if method == "GET" && bare_path == "/api/hello" {
        // amorce de la preuve de pair : identité + nonce à usage unique, aucun secret
        let j = serde_json::json!({
            "app": "mcp-control",
            "version": env!("CARGO_PKG_VERSION"),
            "nonce": issue_hello_nonce(),
        });
        return (200, "application/json", j.to_string().into_bytes());
    }
    if (method == "GET" || method == "POST") && bare_path == "/api/state" {
        let j = manage::state_json(state);
        return (200, "application/json", j.to_string().into_bytes());
    }
    if method == "GET" && bare_path == "/api/detail" {
        if let Some(id) = query_param(path, "id") {
            let j = manage::detail_json(state, &id);
            return (200, "application/json", j.to_string().into_bytes());
        }
    }
    // Historique runtime : crashs, redémarrages, échecs de démarrage et
    // backoff — l'outil de supervision en lecture seule.
    if method == "GET" && bare_path == "/api/runtime" {
        let j = manage::runtime_json(state);
        return (200, "application/json", j.to_string().into_bytes());
    }
    if method == "GET" && bare_path == "/api/store" {
        let j = manage::store_json();
        return (200, "application/json", j.to_string().into_bytes());
    }
    if method == "GET" && bare_path == "/api/store-logos" {
        return (
            200,
            "application/json",
            logos::LOGOS_JSON.as_bytes().to_vec(),
        );
    }
    // Table des frameworks MCP connus (id, label, langage) — l'UI en dérive
    // les badges sans dupliquer la liste côté JavaScript.
    if method == "GET" && bare_path == "/api/frameworks" {
        let arr: Vec<serde_json::Value> = crate::frameworks::FRAMEWORKS
            .iter()
            .map(|f| serde_json::json!({"id": f.id, "label": f.label, "lang": f.lang}))
            .collect();
        let j = serde_json::json!({"frameworks": arr});
        return (200, "application/json", j.to_string().into_bytes());
    }
    if method == "POST" && bare_path == "/api/action" {
        let text = String::from_utf8_lossy(body).to_string();
        let j = manage::handle_action(state, &text);
        return (200, "application/json", j.to_string().into_bytes());
    }
    (
        404,
        "application/json",
        br#"{"ok":false,"error":"not found"}"#.to_vec(),
    )
}

pub fn urldecode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'%' if i + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[i + 1..i + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(b) => {
                        out.push(b);
                        i += 3;
                    }
                    Err(_) => {
                        out.push(bytes[i]);
                        i += 1;
                    }
                }
            }
            b'+' => {
                out.push(b' ');
                i += 1;
            }
            b => {
                out.push(b);
                i += 1;
            }
        }
    }
    String::from_utf8_lossy(&out).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hs(pairs: &[(&str, &str)]) -> Vec<(String, String)> {
        pairs
            .iter()
            .map(|(n, v)| (n.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn check_access_options_est_toujours_refuse() {
        let hosts = vec!["127.0.0.1".to_string()];
        let origins = vec!["http://127.0.0.1:5090".to_string()];
        let r = check_access(
            false,
            None,
            &hosts,
            &origins,
            "OPTIONS",
            "/api/action",
            &[("host".into(), "127.0.0.1:5090".into())],
            false,
        );
        assert_eq!(r, Err((403, "Forbidden")));
    }

    #[test]
    fn check_access_host_inconnu_refuse_dns_rebinding() {
        let hosts = vec![
            "127.0.0.1".to_string(),
            "192.0.2.10".to_string(),
            "::1".to_string(),
        ];
        let origins = vec![];
        let r = check_access(
            false,
            None,
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[("host", "evil.example.com:5090")]),
            false,
        );
        assert_eq!(r, Err((403, "Forbidden")));
        // host autorisé, avec ou sans port, IPv6 entre crochets
        for h in ["127.0.0.1:5090", "127.0.0.1", "[::1]:5090"] {
            let r2 = check_access(
                false,
                None,
                &hosts,
                &origins,
                "GET",
                "/api/state",
                &hs(&[("host", h)]),
                false,
            );
            assert!(r2.is_ok(), "host {} doit passer", h);
        }
    }

    #[test]
    fn check_access_origin_inconnue_refuse() {
        let hosts = vec!["127.0.0.1".to_string()];
        let origins = vec!["http://127.0.0.1:5090".to_string()];
        let r = check_access(
            false,
            None,
            &hosts,
            &origins,
            "POST",
            "/api/action",
            &hs(&[
                ("host", "127.0.0.1:5090"),
                ("origin", "https://malveille.example"),
            ]),
            false,
        );
        assert_eq!(r, Err((403, "Forbidden")));
        let ok = check_access(
            false,
            None,
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[
                ("host", "127.0.0.1:5090"),
                ("origin", "http://127.0.0.1:5090"),
            ]),
            false,
        );
        assert!(ok.is_ok());
    }

    #[test]
    fn check_access_api_exige_le_code_hors_loopback() {
        let hosts = vec!["127.0.0.1".to_string()];
        let origins = vec!["http://127.0.0.1:5090".to_string()];
        let base = &[("host", "127.0.0.1:5090")] as &[(&str, &str)];
        // sans code → 401
        let r = check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(base),
            false,
        );
        assert_eq!(r, Err((401, "Unauthorized")));
        // mauvais code → 401
        let r = check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[("host", "127.0.0.1:5090"), ("x-mcc-access", "zzz")]),
            false,
        );
        assert_eq!(r, Err((401, "Unauthorized")));
        // bon code (header) → OK ; bon code (?access=) → OK
        assert!(check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[("host", "127.0.0.1:5090"), ("x-mcc-access", "abc")]),
            false,
        )
        .is_ok());
        assert!(check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/detail?id=x&access=abc",
            &hs(base),
            false,
        )
        .is_ok());
        // loopback (require_access=false) : aucun code requis
        assert!(check_access(
            false,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(base),
            false,
        )
        .is_ok());
    }

    #[test]
    fn check_access_action_post_exige_content_type_json() {
        let hosts = vec!["127.0.0.1".to_string()];
        let origins = vec![];
        // text/plain = requête « simple » sans préflight depuis un site web → 415
        let r = check_access(
            false,
            None,
            &hosts,
            &origins,
            "POST",
            "/api/action",
            &hs(&[("host", "127.0.0.1:5090"), ("content-type", "text/plain")]),
            false,
        );
        assert_eq!(r, Err((415, "Unsupported Media Type")));
        // application/json (+paramètres) → OK ; absent → 415
        assert!(check_access(
            false,
            None,
            &hosts,
            &origins,
            "POST",
            "/api/action",
            &hs(&[
                ("host", "127.0.0.1:5090"),
                ("content-type", "application/json; charset=utf-8")
            ]),
            false,
        )
        .is_ok());
        assert_eq!(
            check_access(
                false,
                None,
                &hosts,
                &origins,
                "POST",
                "/api/action",
                &hs(&[("host", "127.0.0.1:5090")]),
                false,
            ),
            Err((415, "Unsupported Media Type"))
        );
        // GET /api/state sans content-type : pas concerné
        assert!(check_access(
            false,
            None,
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(&[("host", "127.0.0.1:5090")]),
            false,
        )
        .is_ok());
    }

    #[test]
    fn is_loopback_bind_cas_connus() {
        for s in ["127.0.0.1", "localhost", "::1", "[::1]"] {
            assert!(is_loopback_bind(s), "{} = loopback", s);
        }
        for s in ["0.0.0.0", "", "192.0.2.10"] {
            assert!(!is_loopback_bind(s), "{} = PAS loopback", s);
        }
    }

    #[test]
    fn code_acces_jamais_exige_aux_clients_locaux() {
        // bind loopback : aucun code, quel que soit le client
        assert!(!effective_require_access(true, true));
        assert!(!effective_require_access(true, false));
        // bind 0.0.0.0 : client LOCAL (desktop, navigateur de la machine) → pas de code
        assert!(
            !effective_require_access(false, true),
            "une connexion venue de la machine n'exige jamais le code"
        );
        // bind 0.0.0.0 + client du LAN → code exigé
        assert!(effective_require_access(false, false));
    }

    #[test]
    fn query_param_extrait_et_decode() {
        assert_eq!(
            query_param("/api/detail?id=a%20b&access=c+d", "access"),
            Some("c d".to_string())
        );
        assert_eq!(query_param("/api/detail?id=x", "access"), None);
        assert_eq!(query_param("/api/state", "access"), None);
    }

    #[test]
    fn generate_access_fait_32_hex_et_est_unique() {
        let a = generate_access();
        let b = generate_access();
        assert_eq!(a.len(), 32);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b, "deux tirages RandomState ne doivent pas coller");
    }

    #[test]
    fn random_hex_nonce_est_corrige_et_non_correle() {
        // CSPRNG obligatoire : les mots successifs d'un nonce (et d'un appel au
        // suivant) ne doivent montrer AUCUNE corrélation — RandomState suivait
        // un compteur thread-local, prévisible après observation.
        let a = random_hex(4);
        let b = random_hex(4);
        assert_eq!(a.len(), 64);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(a, b);
        // mots distincts à l'intérieur d'un même tirage (64 bits chacun)
        let mots: Vec<&str> = a
            .as_bytes()
            .chunks(16)
            .map(|c| std::str::from_utf8(c).unwrap())
            .collect();
        assert_eq!(mots.len(), 4);
        assert!(
            mots.iter().collect::<std::collections::HashSet<_>>().len() == 4,
            "mots identiques dans un même nonce : entropie insuffisante"
        );
        // os_random répond (source OS toujours disponible en pratique)
        let mut buf = [0u8; 8];
        assert!(os_random(&mut buf), "source d'entropie OS indisponible");
    }

    #[test]
    fn urldecode_classique() {
        assert_eq!(urldecode("seed-overmind"), "seed-overmind");
        assert_eq!(urldecode("a%20b+c"), "a b c");
        assert_eq!(urldecode("caf%C3%A9"), "caf\u{e9}");
        assert_eq!(urldecode("%2Fapi%3Fid%3D1"), "/api?id=1");
    }

    #[test]
    fn urldecode_tolerant_aux_sequences_invalides() {
        assert_eq!(urldecode("100%"), "100%");
        assert_eq!(urldecode("%ZZ"), "%ZZ");
        assert_eq!(urldecode("%A"), "%A");
    }

    #[test]
    fn find_sub_cas_limites() {
        assert_eq!(find_sub(b"\r\n\r\n", b"\r\n\r\n"), Some(0));
        assert_eq!(find_sub(&[13u8, 10u8], b"\r\n\r\n"), None);
    }

    #[test]
    fn check_access_hello_preauth_et_proof_remplace_le_code() {
        let hosts = vec!["127.0.0.1".to_string()];
        let origins = vec![];
        let base = &[("host", "127.0.0.1:5090")] as &[(&str, &str)];
        // /api/hello est pré-authentifié (nonce seulement) même hors loopback
        assert!(check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/hello",
            &hs(base),
            false
        )
        .is_ok());
        // une preuve de pair valide authentifie /api/state sans code en clair
        assert!(check_access(
            true,
            Some("abc"),
            &hosts,
            &origins,
            "GET",
            "/api/state",
            &hs(base),
            true
        )
        .is_ok());
        // sans preuve ni code → 401 inchangé
        assert_eq!(
            check_access(
                true,
                Some("abc"),
                &hosts,
                &origins,
                "GET",
                "/api/state",
                &hs(base),
                false
            ),
            Err((401, "Unauthorized"))
        );
    }

    #[test]
    fn nonce_hello_usage_unique_et_verif_proof() {
        let nonce = issue_hello_nonce();
        assert_eq!(nonce.len(), 64);
        let proof = crate::crypto::hmac_sha256_hex(
            b"code-partage",
            format!("mcc-proof-v1:{nonce}").as_bytes(),
        );
        assert!(verify_proof(&nonce, &proof, "code-partage"));
        // usage unique : la même preuve ne repasse pas
        assert!(!verify_proof(&nonce, &proof, "code-partage"));
        // mauvais code partagé → rejet (et le nonce est consommé par le 1er essai)
        let nonce2 = issue_hello_nonce();
        let proof2 = crate::crypto::hmac_sha256_hex(
            b"code-partage",
            format!("mcc-proof-v1:{nonce2}").as_bytes(),
        );
        assert!(!verify_proof(&nonce2, &proof2, "autre-code"));
        // nonce inconnu / format invalide → rejet sans panique
        assert!(!verify_proof("inconnu", "00", "code-partage"));
        assert!(!verify_proof(&"a".repeat(64), &proof, "code-partage"));
    }

    fn test_state_with_access(code: Option<&str>) -> std::sync::Arc<crate::types::AppState> {
        std::sync::Arc::new(crate::types::AppState {
            servers: Mutex::new(vec![]),
            log: Mutex::new(std::collections::VecDeque::new()),
            settings: Mutex::new(crate::types::Settings {
                api_access: code.map(String::from),
                ..crate::types::Settings::default()
            }),
            children: Mutex::new(std::collections::HashMap::new()),
            config_path: String::new(),
            peers: Mutex::new(std::collections::HashMap::new()),
            allowed_origins: Mutex::new(vec![]),
            allowed_hosts: Mutex::new(vec!["127.0.0.1".to_string()]),
            spawn_fails: Mutex::new(std::collections::HashMap::new()),
            runtime: Mutex::new(crate::runtime::RuntimeLog::new()),
            spawning: Mutex::new(std::collections::HashSet::new()),
            update_latest: Mutex::new(None),
            lan_ok: Mutex::new(None),
        })
    }

    #[test]
    fn route_racine_ne_revele_jamais_le_code_acces() {
        // Régression P0 (audit 2026-10-24) : la page / embarquait le code d'accès,
        // quiconque joint le port le lisait et contournait toute l'auth API.
        let state = test_state_with_access(Some("code-secret-123"));
        let (status, ctype, body) = route(&state, "GET", "/", b"");
        assert_eq!(status, 200);
        assert!(ctype.starts_with("text/html"));
        let html = String::from_utf8_lossy(&body).to_string();
        assert!(
            !html.contains("code-secret-123"),
            "la page / ne doit PAS contenir le code d'accès"
        );
        assert!(
            html.contains("loginDlg"),
            "la page doit proposer le déverrouillage (loginDlg)"
        );
    }

    #[test]
    fn les_assets_ui_sont_servis_avec_type_et_version() {
        let state = test_state_with_access(None);
        let version = env!("CARGO_PKG_VERSION");
        for (path, ctype_attendu, contenu) in [
            ("/app.css", "text/css", "--accent"),
            ("/app-boot.js", "text/javascript", "dataset.theme"),
            ("/app.js", "text/javascript", "const MCC_VERSION"),
        ] {
            let (status, ctype, body) = route(&state, "GET", path, b"");
            assert_eq!(status, 200, "{path} doit être servi");
            assert_eq!(ctype, format!("{ctype_attendu}; charset=utf-8"), "{path}");
            let text = String::from_utf8_lossy(&body).to_string();
            assert!(text.contains(contenu), "{path} : contenu attendu absent");
            assert!(
                !text.contains("__MCC_VERSION__"),
                "{path} : placeholder de version NON substitué"
            );
        }
        // la coquille pointe vers les assets VERSIONNÉS (substitution faite)
        let (_, _, shell) = route(&state, "GET", "/", b"");
        let html = String::from_utf8_lossy(&shell).to_string();
        assert!(
            html.contains(&format!("/app.css?v={version}")),
            "lien css non versionné"
        );
        assert!(
            html.contains(&format!("/app.js?v={version}")),
            "lien js non versionné"
        );
        // asset inconnu → pas de 200 travesti (le 404 du routeur prend le relais)
        let (status, _, _) = route(&state, "GET", "/app.inexistant", b"");
        assert_ne!(status, 200);
    }

    #[test]
    fn route_api_hello_renvoie_un_nonce_sans_secret() {
        let state = test_state_with_access(Some("code-secret-123"));
        let (status, ctype, body) = route(&state, "GET", "/api/hello", b"");
        assert_eq!(status, 200);
        assert!(ctype.starts_with("application/json"));
        let v: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(v["app"], "mcp-control");
        assert_eq!(v["nonce"].as_str().unwrap().len(), 64);
        let text = String::from_utf8_lossy(&body).to_string();
        assert!(!text.contains("code-secret-123"), "aucun secret dans hello");
    }

    #[test]
    fn route_accepte_query_access_sur_state_et_detail() {
        let state = test_state_with_access(Some("code-secret-123"));
        let (st1, _, body1) = route(&state, "GET", "/api/state?access=code-secret-123", b"");
        assert_eq!(st1, 200);
        let v1: serde_json::Value = serde_json::from_slice(&body1).unwrap();
        assert!(v1.get("servers").is_some());

        // /api/detail avec ?id=...&access=... ne doit pas polluer l'id avec "&access=..."
        state.servers.lock().unwrap().push(crate::types::Server {
            id: "seed-overmind".into(),
            label: "Overmind".into(),
            transport: crate::types::Transport::Http {
                host: None,
                port: 3333,
                path: "/mcp".into(),
            },
            origin: crate::types::Origin::Seed,
            enabled: true,
            auto_restart: true,
            favorite: false,
            state: crate::types::SState::Up,
            pid: None,
            bound_host: None,
            server_name: None,
            version: None,
            protocol: None,
            sse: false,
            sse_endpoint: false,
            ws: false,
            lang: None,
            framework: None,
            via_ssh: false,
            tools: vec![],
            latency_ms: None,
            last_seen_ts: None,
            last_error: None,
            start_cmd: None,
            tools_disabled: vec![],
        });
        let expected_id = "seed-overmind";
        let path = format!("/api/detail?id={}&access=code-secret-123", expected_id);
        let (st2, _, body2) = route(&state, "GET", &path, b"");
        assert_eq!(st2, 200);
        let v2: serde_json::Value = serde_json::from_slice(&body2).unwrap();
        assert_eq!(v2["id"], serde_json::json!(expected_id));
    }
}
