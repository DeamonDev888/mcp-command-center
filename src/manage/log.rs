// Journal applicatif (ring buffer en mémoire + fichier mcc.log).
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

const MAX_LOG: usize = 400;

pub fn add_log(state: &AppState, level: &str, msg: &str) {
    let ts = crate::types::now_ts();
    {
        let mut log = lockv(&state.log);
        log.push_back(LogEntry {
            ts,
            level: level.to_string(),
            msg: msg.to_string(),
        });
        while log.len() > MAX_LOG {
            log.pop_front();
        }
    }
    println!("[{}] {}", level, msg);
    write_log_file(&state.config_path, ts, level, msg);
}

/// Journal disque `mcc.log` (à côté du config.json) : la partie HTTP tourne
/// parfois sans AUCUNE console (autostart `--hidden`, fenêtre masquée dans le
/// tray, binaire fenêtré) — sans fichier, aucun support ni historique possible
/// (audit 2026-10-19 F7). Rotation brute à 512 Ko (une génération `.old` suffit).
fn write_log_file(config_path: &str, ts: u64, level: &str, msg: &str) {
    use std::io::Write;
    if config_path.is_empty() {
        return;
    }
    let path = std::path::Path::new(config_path)
        .with_file_name("mcc.log")
        .to_string_lossy()
        .to_string();
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > 512 * 1024 {
            let _ = std::fs::rename(&path, format!("{}.old", path));
        }
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
    {
        let _ = writeln!(f, "[{}] [{}] {}", crate::types::iso_utc(ts), level, msg);
    }
}
