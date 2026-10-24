use serde_json::Value;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

pub fn lockv<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Clone, Debug)]
pub enum Transport {
    /// host: None = localhost (sonde ::1 puis 127.0.0.1) ; Some(ip) = hôte du réseau local
    Http {
        host: Option<String>,
        port: u16,
        path: String,
    },
    Stdio {
        program: String,
        args: Vec<String>,
        workdir: Option<String>,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Origin {
    Seed,
    Manual,
    Discovered,
}

impl Origin {
    pub fn as_str(&self) -> &'static str {
        match self {
            Origin::Seed => "seed",
            Origin::Manual => "manual",
            Origin::Discovered => "discovered",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SState {
    Down,
    PortOnly,
    Up,
    Disabled,
}

impl SState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SState::Down => "DOWN",
            SState::PortOnly => "PORT-ONLY",
            SState::Up => "UP",
            SState::Disabled => "DISABLED",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolInfo {
    pub name: String,
    pub desc: String,
}

#[derive(Clone, Debug)]
pub struct StartCmd {
    pub program: String,
    pub args: Vec<String>,
    pub workdir: Option<String>,
}

#[derive(Clone, Debug)]
pub struct Server {
    pub id: String,
    pub label: String,
    pub transport: Transport,
    pub origin: Origin,
    pub enabled: bool,
    pub state: SState,
    pub pid: Option<u32>,
    pub bound_host: Option<String>,
    pub server_name: Option<String>,
    pub version: Option<String>,
    pub protocol: Option<String>,
    /// Réponses initialize encodées SSE (Content-Type: text/event-stream).
    /// Ne qualifie PAS le transport : FastMCP httpStream répond ainsi par défaut.
    pub sse: bool,
    /// Endpoint SSE legacy (/sse → event:) co-servi (FastMCP httpStream le fait).
    pub sse_endpoint: bool,
    /// Langage inféré (stdio: commande ; HTTP: en-têtes Server/X-Powered-By)
    pub lang: Option<String>,
    /// Framework détecté par fingerprint ("litemcp" : réponse initialize en
    /// 2026-07-28 sans négociation + sonde GET /health « ok »). Détecté au
    /// probe, jamais persisté dans config.json.
    pub framework: Option<String>,
    /// Serveur sondé À TRAVERS la connexion SSH de sa machine (curl interne sur
    /// 127.0.0.1) — nécessaire quand le port est bindé localhost ou filtré ufw.
    /// L'utilisateur SSH vient de settings.ssh_hosts (clé = host du transport).
    pub via_ssh: bool,
    pub tools: Vec<ToolInfo>,
    pub latency_ms: Option<u64>,
    pub last_seen_ts: Option<u64>,
    pub last_error: Option<String>,
    pub start_cmd: Option<StartCmd>,
    pub auto_restart: bool,
    /// Tools désactivés individuellement pour ce serveur
    pub tools_disabled: Vec<String>,
}

#[derive(Clone, Debug)]
pub struct LogEntry {
    pub ts: u64,
    pub level: String,
    pub msg: String,
}

#[derive(Clone, Debug)]
pub struct Settings {
    pub refresh_secs: u64,
    pub scan_from: u16,
    pub scan_to: u16,
    pub ui_port: u16,
    /// "127.0.0.1" (défaut) = UI locale seulement ; "0.0.0.0" = accessible du LAN
    /// (requis pour la détection de pairs) — un code d'accès API est alors généré et exigé.
    pub ui_bind: String,
    pub network_enabled: bool,
    pub network_ports: Vec<u16>,
    /// Code d'accès API exigé sur /api/* quand le bind n'est pas loopback.
    /// Sources : env MCC_API_TOKEN, sinon config, sinon généré au boot (persisté).
    /// Pour les pairs, mettre la MÊME valeur des deux côtés.
    pub api_access: Option<String>,
    /// Origines HTTP supplémentaires autorisées (ex. "http://mon-host.local:5090").
    pub allowed_origins: Vec<String>,
    /// Seeds retirés durablement (« Retirer durablement ») : réinjectés au boot
    /// par défaut, ceux-ci sont filtrés au chargement de la config.
    pub removed_seeds: Vec<String>,
    /// Machines distantes enregistrées pour la découverte SSH (ip + utilisateur).
    pub ssh_hosts: Vec<SshHost>,
    /// Ordre d'affichage et de priorité de sondage des serveurs (ids ; les ids
    /// absents passent en dernier, ordre relatif conservé). Réordonné par
    /// glisser-déposer dans l'explorateur.
    pub server_order: Vec<String>,
    /// Lancer l'app au démarrage de l'OS (bouton de la barre de titre —
    /// appliqué au boot : registre Run sur Windows, autostart XDG / LaunchAgent
    /// ailleurs).
    pub autostart: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            refresh_secs: 15,
            scan_from: 3000,
            scan_to: 3200,
            ui_port: 5090,
            // SÉCURITÉ : loopback par défaut — l'ouverture LAN (0.0.0.0) est un choix
            // explicite de l'opérateur (config), protégé par le code d'accès API.
            ui_bind: "127.0.0.1".to_string(),
            network_enabled: true,
            network_ports: vec![5090, 5091],
            api_access: std::env::var("MCC_API_ACCESS")
                .ok()
                .filter(|v| !v.is_empty()),
            allowed_origins: vec![],
            removed_seeds: vec![],
            ssh_hosts: vec![],
            server_order: vec![],
            autostart: true,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Peer {
    pub ip: String,
    pub ui_port: u16,
    pub last_seen: u64,
    pub servers: Value,
}

/// Machine distante connue pour la découverte SSH (auth par clé, BatchMode).
#[derive(Clone, Debug)]
pub struct SshHost {
    pub ip: String,
    pub user: String,
}

pub struct AppState {
    pub servers: Mutex<Vec<Server>>,
    pub log: Mutex<VecDeque<LogEntry>>,
    pub settings: Mutex<Settings>,
    pub children: Mutex<HashMap<String, crate::probe::StdioHandle>>,
    pub config_path: String,
    /// Pairs command-center détectés sur le LAN (ip -> infos + serveurs /api/state)
    pub peers: Mutex<HashMap<String, Peer>>,
    /// Origines HTTP autorisées (reflétées en ACAO, exigées si Origin présent) — remplies au boot.
    pub allowed_origins: Mutex<Vec<String>>,
    /// Hosts autorisés dans le header Host (anti DNS-rebinding) — remplis au boot.
    pub allowed_hosts: Mutex<Vec<String>>,
    /// Échecs consécutifs de spawn auto par id — ≥ 3 → auto_restart désactivé.
    pub spawn_fails: Mutex<HashMap<String, u32>>,
    /// Ids en cours de spawn (garde : le poller et une action API ne doublonnent pas)
    pub spawning: Mutex<HashSet<String>>,
    /// Dernière version publiée sur GitHub, SI PLUS RÉCENTE que l'installée
    /// (badge « mise à jour disponible »). None = à jour ou vérification
    /// impossible (hors ligne, dépôt privé).
    pub update_latest: Mutex<Option<String>>,
}
