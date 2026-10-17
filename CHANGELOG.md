## [0.4.0] — Le command center lance aussi les MCP HTTP/HTTPS

### Backend
- start_cmd optionnel pour les serveurs HTTP : le command center peut les démarrer, les arrêter et les relancer (auto-restart), comme les stdio.
- Actions génériques `start` / `kill` (alias compat `start_stdio` / `kill_stdio`).
- Auto-start HTTP : un serveur HTTP local éteint avec commande de lancement est démarré au prochain cycle de poll, puis sondé (probe → spawn → re-probe). Jamais pour un serveur distant (gate host local).
- Persistance : la commande de lancement (program/args/workdir) est écrite et rechargée depuis config.json pour les HTTP.

### UI
- Form d'ajout HTTP : champs optionnels « commande de lancement ».
- Boutons Start/Stop sur chaque serveur avec commande (HTTP comme stdio) dans l'explorateur et le détail.

## [0.3.1] — Start/Stop stdio + badge « désuet »

### UI
- Bouton Start/Stop directement sur chaque ligne stdio de l'explorateur (▶/■) en plus de la vue détail.
- Badge « désuet » (legacy) sur les serveurs stdio uniquement (explorateur + détail) : pas de transport HTTP.

### Corrigé
- start_stdio ne faisait rien sur un serveur désactivé (poll_one sort sur !enabled) — il réactive désormais le serveur, reconstruit start_cmd et persiste.
- kill_stdio laissait un pid fantôme affiché — pid mis à None et état Down immédiatement.

## [0.3.0] — Refonte frontend « explorateur »

### UI
- Refonte complete : layout IDE (Antigravity) — explorateur a gauche (liste des serveurs MCP + pairs command-center), zone de detail a droite, journal style console en bas (repliable), titlebar avec badges stats et actions.
- Theme repris de l'app Hermes Desktop (skin nous, mode sombre) : chrome #0d0d0e, sidebar #0a0a0b, cards #161618, accent Nous #4a84fe, textes 94/74/54/36%, hover accent 4%.
- Vue detail : infos completes + liste de chaque tool (nom + description) avec toggle ON/OFF individuel, actions Start/Stop/Sonder/Activer/Retirer.
- Vue d'ensemble avec tuiles de stats quand rien n'est selectionne.
- Formulaire d'ajout en modal.

# Changelog

Tous les changements notables de ce projet sont documentés ici.
Format basé sur [Keep a Changelog](https://keepachangelog.com/fr/1.1.0/).

## [0.2.1] - 2026-10-16

### Fixed

- Sémantique d'états : un serveur éteint (port fermé) repasse à `DOWN` au lieu de `PORT-ONLY` — l'état `PORT-ONLY` est désormais réservé au port occupé sans MCP (zombie), comme documenté.

## [0.2.0] - 2026-10-16

### Added

- **Détection des autres command centers sur le réseau** (pairs) : cache ARP + loopback (multi-instances sur la même machine), sonde `/api/state` sur les ports UI configurables (`network_ports`), import en un clic des serveurs du pair avec réécriture `localhost` → IP du pair.
- **Détection des serveurs MCP du réseau local** : scan parallèle (254 hôtes × plage de ports, un thread par hôte) sur tous les sous-réseaux locaux (parse `ipconfig`/`hostname -I`), auto-ajout avec l'IP en host.
- **Transport HTTP distant** : champ `host` optionnel (None = localhost avec fallback `::1`/`127.0.0.1`, Some = hôte du réseau) — sonde, polling et détail fonctionnent sur un MCP d'une autre machine.
- **Toggle ON/OFF par tool** : bouton ON/OFF par tool dans le détail d'un serveur, `tools_disabled` persisté, compteurs (cartes, stats globales, journal) reflètent les tools actifs.
- UI : section « pairs » avec import, badge hôte distant `🌐 ip:port`, bouton Tools par carte, scan réseau et détection de pairs depuis la barre d'outils.
- Réglages réseau : `ui_bind` (défaut `0.0.0.0` pour être détectable), `network_enabled`, `network_ports`.
- 4 nouveaux tests (contrat `/api/state` du pair, rejet des non-command-centers, toggle tool, compteur tools actifs).

### Fixed

- Auto-détection éliminée (une instance ne se détecte plus elle-même via son IP LAN) : exclusion des IP locales + skip du port UI propre + écarts réseau/broadcast en cache ARP.
- `/api/state` accepte désormais POST (contrat de sonde des pairs).
- Scan réseau asynchrone (réponse API immédiate, détections dans le journal).
- Filtrage des interfaces : masques et passerelles d'`ipconfig` écartés (marqueur ligne « IPv4 »).

## [0.1.0] - 2026-10-16

### Added

- Détection des serveurs MCP **HTTP streamable** : handshake `initialize` + `tools/list`, sonde `::1` **et** `127.0.0.1` (les FastMCP locaux bindent en IPv6-only), parsing SSE et réponses chunked.
- Détection et pilotage des serveurs **stdio** : spawn + handshake JSON-RPC ligne-par-ligne, Start/Stop manuels, auto-restart au boot et au crash (flag `auto_restart`).
- **Découverte automatique** par scan de plage de ports (3000-3200 par défaut) : tout port répondant `initialize` avec un `serverInfo` est ajouté comme serveur découvert.
- États à 3 niveaux : `UP` / `PORT-ONLY` (port occupé par un non-MCP ou zombie) / `DOWN` + `DISABLED`.
- **UI web embarquée dans le binaire** : cartes par serveur (état, latence, host de bind, version, protocole, tools), détail cliquable, journal temps réel, scan/ajout pilotables depuis le navigateur.
- API HTTP scriptable : `GET /api/state`, `GET /api/detail?id=`, `POST /api/action` (`refresh`, `toggle`, `scan`, `add`, `remove`, `start_stdio`, `kill_stdio`).
- Persistance `config.json` avec 7 seeds pré-chargés (Overmind, discord, postgres, X, debats, NEWS_BTC, NEWS) et reconstruction `start_cmd` au rechargement.
- Fixtures de test MCP (stdio + HTTP) dans `examples/`, zéro dépendance Node.
- 18 tests unitaires/intégration (`cargo test`) ; client HTTP maison sans aucune dépendance réseau (std + serde_json uniquement).
