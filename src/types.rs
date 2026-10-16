use std::collections::{HashMap, VecDeque};
use std::sync::{Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn now_ts() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

pub fn lockv<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|p| p.into_inner())
}

#[derive(Clone, Debug)]
pub enum Transport {
    Http { port: u16, path: String },
    Stdio { program: String, args: Vec<String>, workdir: Option<String> },
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Origin { Seed, Manual, Discovered }

impl Origin {
    pub fn as_str(&self) -> &'static str {
        match self { Origin::Seed => "seed", Origin::Manual => "manual", Origin::Discovered => "discovered" }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SState { Down, PortOnly, Up, Disabled }

impl SState {
    pub fn as_str(&self) -> &'static str {
        match self {
            SState::Down => "DOWN", SState::PortOnly => "PORT-ONLY", SState::Up => "UP",
            SState::Disabled => "DISABLED",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ToolInfo { pub name: String, pub desc: String }

#[derive(Clone, Debug)]
pub struct StartCmd { pub program: String, pub args: Vec<String>, pub workdir: Option<String> }

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
    pub tools: Vec<ToolInfo>,
    pub latency_ms: Option<u64>,
    pub last_seen_ts: Option<u64>,
    pub last_error: Option<String>,
    pub start_cmd: Option<StartCmd>,
    pub auto_restart: bool,
}

#[derive(Clone, Debug)]
pub struct LogEntry { pub ts: u64, pub level: String, pub msg: String }

#[derive(Clone, Debug)]
pub struct Settings {
    pub refresh_secs: u64,
    pub scan_from: u16,
    pub scan_to: u16,
    pub ui_port: u16,
}

impl Default for Settings {
    fn default() -> Self {
        Settings { refresh_secs: 15, scan_from: 3000, scan_to: 3200, ui_port: 5090 }
    }
}

pub struct AppState {
    pub servers: Mutex<Vec<Server>>,
    pub log: Mutex<VecDeque<LogEntry>>,
    pub settings: Mutex<Settings>,
    pub children: Mutex<HashMap<String, crate::probe::StdioHandle>>,
    pub config_path: String,
}
