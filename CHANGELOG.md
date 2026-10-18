## [0.6.2] — Langage des serveurs MCP + logos

### Frontend
- Chip de langage (monogramme coloré façon logo) à côté du badge de type : JS (Node/TS), PY, RS, GO, JV, C#, RB, PHP. Survol = libellé complet. Badge absent si le langage est inconnu (aucune invention).
- Transport conservé : STDIO / HTTP / SSE / HTTPS (+ ·LAN).

### Backend
- Détection du langage — la spec MCP n'expose PAS le langage (serverInfo = name/title/version uniquement) :
  - stdio : commande de lancement (node/npx/tsx → Node, python/uv/uvx → Python, cargo → Rust, go/java/dotnet/ruby/php), extension du script en dernier recours.
  - HTTP : en-têtes Server / X-Powered-By (uvicorn/gunicorn/werkzeug → Python, express → Node, hyper/axum → Rust, Kestrel → .NET, Jetty/Tomcat → Java, net/http → Go), sinon indice dans le nom.
- RespHead (content-type/server/x-powered-by) remplace le simple content-type ; `lang` exposé dans /api/state.
- Fixture `examples/test-py-header-server.js` (se présente comme uvicorn) pour tester l'inférence.

Preuves : py-header → python (header uvicorn) ; Filesystem/demo-stdio → node (commande) ; serveurs réels sans en-tête → lang inconnu.

## [0.6.1] — Badges du type de MCP (STDIO / HTTP / SSE / HTTPS)

### UI
- Chaque serveur affiche son type de transport avant le badge « désuet » : STDIO (cyan), HTTP (violet), SSE (vert, détecté au probe), HTTPS (port 443), suffixe ·LAN pour les serveurs distants.
- Même badge dans la vue détail à côté du pill d'état.

### Backend
- Détection SSE : Content-Type `text/event-stream` capturé sur la réponse initialize → champ `sse` (McpProbe/Server) exposé dans /api/state.
- Fixture `examples/test-sse-server.js` (serveur MCP SSE zéro dépendance) pour tester.

Preuve réelle : Overmind, discord, postgres, X, NEWS détectés SSE ; demo-http (JSON chunked) HTTP ; stdio → STDIO.

### Lancement des serveurs locaux
- Seeds : chaque serveur embarque désormais sa commande de lancement réelle (`node <workdir>/bin/launch.cjs` + workdir du launcher MCP MONITOR v7.3) → les boutons ▶/■ fonctionnent même sans `config.json` (exe distribué, premier démarrage). Corrige les boutons absents (`can_start: false`) après réécriture de la config.
- Start : re-probe borné (30 s max, pas de 1 s) après le spawn — les launchers font `npm run build` avant d'écouter.
- Stop : tue aussi le process qui écoute réellement le port (netstat → taskkill, local uniquement) car le launcher détache le serveur ; état DOWN immédiat (boucle de vérification bornée).
- Spawn détaché (`CREATE_NEW_PROCESS_GROUP | DETACHED_PROCESS`) : les serveurs survivent au shell et à un Ctrl+C dans la console de l'app.

Preuve réelle : dossier vierge sans config → `can_start: true` sur les 7 seeds ; Start Overmind → UP 16 tools ; Stop → DOWN immédiat, port 3099 fermé (3,7 s).

## [0.6.0] — App NATIVE (Tauri) + CLI headless

### Architecture
- Workspace cargo : `mcp-command-center` (core headless, lib + bin) + `mcp-command-center-desktop` (Tauri 2).
- Le core devient une LIBRAIRIE : tout le moteur (config, probes, polling, HTTP, boutique) est dans `src/lib.rs` ; `run_headless()` démarre tout et retourne l'URL, `park_forever()` bloque.
- `mcp-command-center.exe` = CLI **100 % headless** : plus AUCUNE ouverture de navigateur/fenêtre. Sortie console : URL + endpoints. (Le mode --app Edge/Chrome est supprimé.)
- `mcp-command-center-desktop.exe` = **vraie fenêtre native OS** : Tauri 2 + WebView2 système (déjà fourni par Windows 11), exe autonome 9 MB, `windows_subsystem = "windows"` (aucune console). Le moteur tourne dans le process, la fenêtre navigue vers l'URL locale.
- Lanceurs : `start-command-center.bat` → fenêtre native ; `start-headless.bat` → CLI.

### CI
- Jobs core scopés sur `-p mcp-command-center` (le desktop ne compile pas sur ubuntu sans webkit2gtk).
- Nouveau job `desktop` (windows-latest) : construit la fenêtre native et l'ajoute à la release (`mcp-command-center-desktop-windows-x86_64.exe`).
- Prérequis MSVC : toolchain `stable-x86_64-pc-windows-msvc` + Build Tools 2022 + WebView2 Runtime (tous présents ici).

## [0.5.1] — Audit complet + formatage

### Gates
- cargo fmt appliqué (tout le repo formaté, vérifié par fmt --check) + fmt --check ajouté au rituel de release.
- Corrigé : refresh_secs non clampé (une config avec 0 = busy-loop CPU) → clamp 5..3600.
- Nettoyé : variable batch morte dans scan_ports.

### Audit — feuille de route
Voir README (section Feuille de route) pour la liste complète construire/optimiser/sécuriser.

## [0.5.0] — Fenêtre desktop + Boutique MCP

### Desktop
- L'app s'ouvre maintenant dans une FENÊTRE dédiée (Edge/Chrome --app, style Electron sans la payload) au lieu d'un onglet navigateur. Fallback navigateur. MCC_NO_BROWSER=1 pour désactiver.

### Boutique MCP (portée de l'app Hermes desktop)
- Bouton « Boutique » : catalogue de 14 serveurs MCP officiels (Filesystem, Git, GitHub, Memory, Fetch, Puppeteer, Brave, PostgreSQL, SQLite, Slack…), catégories + recherche, install en un clic.
- Import « collez n'importe quoi » (porté de mcp-import.ts) : snippet mcp.json (mcpServers ou mcp_servers ou map brute), commande npx/uvx/docker/claude mcp add, ou simple URL → serveurs créés.
- Backend : /api/store, actions install_store + import_mcp, parsing tolerant (mcp-doc.ts), nettoyage de noms (@modelcontextprotocol/server-filesystem → filesystem).

### Corrigé (Windows)
- Rust Command::new("npx") échouait : CreateProcess ne résout que les .exe — résolution .cmd dans le PATH ajoutée (npx/uvx fonctionnent).
- Timeout de handshake stdio 6 s → 15 s (npx télécharge le package au premier lancement).

## [0.4.3] — L'app s'ouvre (navigateur auto au démarrage)

### Corrigé (UX bloquant)
- Double-cliquer l'exe ne montrait rien : serveur console silencieux, rien n'ouvrait la UI.
- Au démarrage, le navigateur par défaut s'ouvre automatiquement sur http://127.0.0.1:<port> (le port réel après fallback). Désactivable avec MCC_NO_BROWSER=1.
- L'URL ouverte pointe sur 127.0.0.1 même en bind 0.0.0.0.

## [0.4.2] — Persistance réparée sur la release distribuée

### Corrigé (bloquant)
- La release téléchargée ne persistait RIEN : config.json était écrit vers le chemin compile-time du runner CI (CARGO_MANIFEST_DIR), inexistant sur la machine cliente → tout serveur ajouté disparaissait au restart.
- Résolution au runtime : MCC_CONFIG (override) → CARGO_MANIFEST_DIR/config.json si présent (dev) → dossier de l'exe (release distribuée). Validé par simulation machine cliente : add → restart → serveur conservé, config.json écrit à côté de l'exe.

## [0.4.1] — ON/OFF = vrai interrupteur

### Backend
- toggle OFF tue le process lancé par le command center (stdio ou HTTP) et libère le pid — avant, désactiver un serveur UP laissait son process tourner.
- toggle ON démarre/sonde immédiatement (poll_one) au lieu d'attendre le cycle suivant.

### UI
- Switch ON/OFF sur chaque ligne de l'explorateur (en plus du bouton Activer/Désactiver du détail).

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
