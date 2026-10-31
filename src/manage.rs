use crate::probe;
use crate::types::{lockv, AppState, LogEntry, Origin, SState, Server, StartCmd, Transport};
use serde_json::{json, Map, Value};
#[cfg_attr(not(windows), allow(unused_imports))] // Stdio : chemins cfg(windows)
use std::process::{Command, Stdio};
use std::sync::Arc;

// log : déplacé vers manage/log.rs (v0.7.20) — réexport pour API stable.
mod log;
pub(crate) use log::*;

// ---------- config ----------

// config : déplacé vers manage/config.rs (v0.7.20) — réexport pour API stable.
mod config;
pub(crate) use config::*;

// store : déplacé vers manage/store.rs (v0.7.20) — réexport pour API stable.
mod store;
pub(crate) use store::*;

/// Porté de mcp-doc.ts : accepte {"mcpServers": {...}}, {"mcp_servers": {...}}
/// ou une map brute name → entry. Retourne (nom, entry).
// import : déplacé vers manage/import.rs (v0.7.20) — réexport pour API stable.
mod import;
pub(crate) use import::*;

/// Nombre de tools actifs (tools_disabled écartés).
pub fn tools_actifs(s: &Server) -> usize {
    s.tools
        .iter()
        .filter(|t| !s.tools_disabled.contains(&t.name))
        .count()
}

/// Nombre de sondes menées en parallèle par cycle. Les serveurs lents (SSH,
/// process MCP muet) ne doivent pas retarder tout le monde : avec 30 serveurs
/// dont 15 en SSH, un cycle séquentiel dépassait largement le refresh de 15 s.
// litemcp : déplacé vers manage/litemcp.rs (v0.7.20) — réexport pour API stable.
mod litemcp;
pub(crate) use litemcp::*;

// autostart : déplacé vers manage/autostart.rs (v0.7.20) — réexport pour API stable.
mod autostart;
pub(crate) use autostart::*;

/// Filesystem : le dossier exposé doit être présent — il faut au moins les
/// options + le package + UN chemin (le nom du package contient un « / », la
/// longueur est donc le critère fiable), et le dernier arg doit ressembler à un chemin.
fn filesystem_dossier_manquant(args: &[String]) -> bool {
    args.len() < 3
        || args
            .last()
            .map(|a| !a.contains('/') && !a.contains('\\'))
            .unwrap_or(true)
}

/// Compte un échec de spawn auto pour l'id ; à partir de 3 échecs consécutifs,
/// désactive l'auto-restart (sinon boucle infinie d'essais + warnings sur une
/// machine cliente où les chemins des seeds n'existent pas).
/// Enregistre un événement runtime (crash, redémarrage, succès…) et le
/// persiste — l'historique survit au redémarrage de l'application.
/// NOTIFICATION d'incident : déclenchée sur une transition UP → DOWN/PORT-ONLY.
/// Deux canaux : toast desktop (Windows) et webhook HTTP. Le délai minimum
/// (`min_interval_secs`) évite le spam quand un serveur oscille.
// events : déplacé vers manage/events.rs (v0.7.20) — réexport pour API stable.
mod events;
pub(crate) use events::*;
// scans : déplacé vers manage/scans.rs (v0.7.20) — réexport pour API stable.
mod scans;
pub(crate) use scans::*;

// peers : déplacé vers manage/peers.rs (v0.7.20) — réexport pour API stable.
mod peers;
pub(crate) use peers::*;

// actions : déplacé vers manage/actions.rs (v0.7.20) — réexport pour API stable.
mod actions;
pub(crate) use actions::*;

// update : déplacé vers manage/update.rs (v0.7.20) — réexport pour API stable.
mod update;
pub(crate) use update::*;

// cache : déplacé vers manage/cache.rs (v0.7.20) — réexport pour API stable.
mod cache;
pub(crate) use cache::*;

// json : déplacé vers manage/json.rs (v0.7.20) — réexport pour API stable.
mod json;
pub(crate) use json::*;

#[cfg(test)]
mod tests;
