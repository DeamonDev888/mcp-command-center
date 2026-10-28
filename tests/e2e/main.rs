//! Tests e2e de MCP Control : le VRAI binaire est lancé dans un répertoire
//! temporaire isolé (cf. harness.rs) et interrogé en HTTP réel.
//!
//! Exécution recommandée (chaque scénario démarre son propre moteur, ports
//! attribués dynamiquement) :
//! ```text
//! cargo test --test e2e -- --test-threads=1
//! ```
//! - scenarios_boot      : démarrage, auth loopback/LAN, Host/Origin, preuve de pair
//! - scenarios_security  : fuzz HTTP brut + label hostile (jamais en parallèle)
//! - scenarios_lifecycle : cycle de vie des serveurs (add/toggle/kill…)
//! - scenarios_scan      : découverte locale (scan, fixtures MCP factices)
//! - scenarios_actions   : chaque bouton, chaque action (contrat dispatcher +
//!   scénarios réels par action)
//! - scenarios_gateway   : la console comme serveur MCP (POST /mcp : tools de
//!   supervision, proxy call_server_tool, ressources, transport, auth)
//!
//! Les scénarios sont SÉRIALISÉS par le verrou global harness::E2E_LOCK
//! (serial_guard en première ligne de chaque test) : `cargo test --test e2e`
//! est sûr en mode parallèle aussi.

mod harness;
mod scenarios_actions;
mod scenarios_boot;
mod scenarios_gateway;
mod scenarios_lifecycle;
mod scenarios_scan;
mod scenarios_security;
