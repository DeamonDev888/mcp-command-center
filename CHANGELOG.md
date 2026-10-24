## [0.7.0] — Support LiteMCP : fingerprint du framework + badge « glow HTTPS »

### Ajouté
- **Replis du détail à état mémorisé** : « Démarrage » et « Informations » retenaient leur état codé en dur — réimposé à chaque cycle de rafraîchissement (5 s) et à chaque réouverture d'onglet. L'état ouvert/fermé choisi est désormais persisté (localStorage, `mcc.fold.*`) et relu au rendu : le triangle reste où tu l'as laissé, y compris après fermeture de l'onglet.
- **Bouton « Auto-boot ON/OFF » dans la barre de titre** : choix de lancer (ou non) l'app au démarrage de l'OS. Préférence persistée (`settings.autostart`, défaut true) et appliquée par le core au boot et au clic — registre `HKCU…Run` sur Windows, autostart XDG sur Linux, LaunchAgent sur macOS (plus de plugin Tauri qui forçait l'activation à chaque lancement). Garde `cfg(test)` : les tests n'écrivent jamais dans l'OS. Action API `set_autostart {enabled}` ; `autostart` exposé dans `/api/state`.
- **Liens écosystème** : le README lie la lib [LiteMCP](https://github.com/DeamonDev888/litemcp) (fingerprint natif) ; le README de litemcp renvoie vers ce client.
- **README illustré** : nouvelle section « Fonctionnalités principales » avec les captures d'écran de l'explorateur (priorité par glisser-déposer, badges LiteMCP/·LAN·SSH, version) et de la boutique (64 MCP avec logos officiels) — détection sur les autres machines/VPS/Docker via SSH, multi-serveurs, drag & drop, auto-boot, version.
- **Drag & drop réparé dans la fenêtre native** : les lignes ne bougeaient qu'en navigateur — le handler de drop de fichiers de WebView2 (activé par défaut dans Tauri) avale les événements HTML5 dragstart/over/drop. `disable_drag_drop_handler()` sur la fenêtre (aucun usage du drop de fichiers dans l'app) — le glisser-déposer de l'explorateur fonctionne désormais à la souris dans la fenêtre Tauri.
- **Version affichée + vérification de mise à jour** : badge `v0.7.0` permanent dans la barre de titre (injecté au rendu). Au boot puis chaque heure, l'app interroge la dernière release GitHub via PowerShell système (aucune dépendance TLS ; échec réseau = silence) : si une version PLUS RÉCENTE existe, un badge ambre « ⬆ vX.Y.Z dispo » apparaît (clique = page releases) et le journal le note. Comparaison numérique par segments (`0.10.0 > 0.9.9`, préfixe `v` toléré, testé) ; `update_available`/`latest_version` exposés dans `/api/state`.
- **Glisser-déposer des serveurs (préférence et priorité)** : les lignes de l'explorateur sont réordonnables par drag & drop (marqueur d'insertion au-dessus/en dessous, hint « glisser = priorité »). L'ordre est persisté dans `settings.server_order` (config.json, action API `reorder` — ids inconnus filtrés, manquants réajoutés en fin) et gouverne **l'affichage ET l'ordre de sondage** (au boot le premier cycle est séquentiel : les serveurs en haut sont sondés en premier). La suppression purge l'entrée. Config de l'app installée séparée du dossier dev (`resolve_config_path` privilégie `exe_dir/config.json` existant — un exe installé ne réécrit plus la config du dossier dev).
- **Cache de démarrage (état dernier connu)** : chaque cycle de poll enregistre l'état courant dans `state-cache.json` (à côté du config.json, écriture atomique, skippée si inchangé) : nom, version, protocole, tools complets, langage, framework, via_ssh, latence, dernier vu. Au boot, AVANT la première sonde, le cache restaure les serveurs connus (par id, activés seulement) — l'UI est peuplée instantanément au lieu d'attendre 1-2 min de sondes séquentielles (stdio spawns inclus) ; les sondes confirment/corrigent en arrière-plan. Preuve mesurée : 3 s après le boot, 30 serveurs restaurés, 569 tools via SSH déjà affichés. Test roundtrip + protection des serveurs désactivés.
- **Icônes garanties sur les trois OS** : `icon.png` 512 + `icon.ico` multi-tailles 16→256 (Windows : embarquée à l'exe, barre des tâches, tray) + **`icon.icns` généré** (128/256/512, ajoute au bundle pour le Dock macOS) + fenêtre `.icon()` explicite (Linux : _NET_WM_ICON pour la barre des tâches, tray partout). `bundle.icon` référence les trois formats ; limitation macOS documentée : un binaire nu (non bundlé) montre l'icône générique du Dock — l'icône tray (menu bar) reste la nôtre.
- **Fingerprint du framework serveur** : nouveau champ de sonde `framework` — `"litemcp"` quand les DEUX signatures documentées de litemcp sont réunies : `initialize` répondu en `2026-07-28` **sans négocier** la version demandée par le client (le probe émet `2024-11-05`, litemcp stateless strict l'ignore) **et** sa sonde `GET /health` → `200` « ok ». Un seul signal ne suffit pas (specs à jour ailleurs, `/health` génériques) ; la sonde `/health` ne part que si l'initialize a répondu 2026-07-28 — aucune requête de plus pour les serveurs qui négocient normalement.
- **Badge UI « LiteMCP »** (style « glow HTTPS ») : halo violet pulsant (`glowPulse`) sur les serveurs fingerprintés, dans la liste et l'en-tête du détail ; ligne `framework` dans les informations du détail. `/api/state` et `/api/detail` exposent `framework`.
- **Langage rust par déduction** : un serveur litemcp est une lib Rust par définition — le chip de langage affiche `RS` quand ni les en-têtes ni le process écoutant le port n'ont permis d'inférer.
- **Seed `seed-litemcp`** : la démo officielle de litemcp (`examples/hello` — 3 outils : hello, timestamp, search) lancée via le binaire release (`cargo build --release --example hello` du dépôt litemcp), port 8080. Start/Stop opérationnels depuis l'UI ; binaire manquant → le mécanisme « 3 échecs → auto-restart off » s'applique.
- **Glow sur le badge HTTPS** : le badge `HTTPS` (port 443) gagne un halo violet statique — même famille visuelle que le badge framework.

### UI
- **Sonde MCP interne via SSH (le vrai fix)** : une fois la machine logée, TOUT se passe depuis la machine — plus aucune sonde externe. Un seul exec SSH boucle sur les ports avec `curl` interne sur `127.0.0.1` : passe les binds localhost ET les pare-feux (ufw). Les serveurs importés sont marqués `via_ssh` (persistés dans config.json, badge « ·SSH » dans l'UI, ligne « sonde : via SSH » au détail) et **pollés par SSH** à chaque cycle : initialize + tools/list dans la même connexion, avec gestion de l'en-tête `Mcp-Session-Id` (capture -D, extraction sanitisée, rejeu — nécessaire aux serveurs sur SDK officiel qui rejettent tools/list sans session). Pièges corrigés au passage : `${p}` jamais développé dans les quotes simples du curl distant, double-quotes interdites dans la commande distante (échappement Windows ssh), extraction du SID qui avalait les deux-points.
- **Persistance des serveurs via_ssh** : `save_config` garde les Discovered marqués via_ssh (leur re-découverte exige une connexion SSH réussie) — ils survivent au redémarrage et sont re-pollés au boot.
- **Machines distantes : découverte SSH des serveurs MCP** : « Détecter les pairs » devient **« Choisir une machine »** (bouton titlebar + écran d'accueil). Nouveau dialog Machines : IP + utilisateur SSH (enregistrables dans `settings.ssh_hosts`) → l'app se connecte avec le **client ssh système** (aucune dépendance, OpenSSH de Windows), récupère le **netstat distant**, filtre les ports système, sonde chaque port restant et **importe les serveurs MCP trouvés**. Auth par **clé obligatoire** (BatchMode) ; erreurs ssh restituées en clair dans le journal avec indication (clé manquante, machine injoignable). Actions `ssh_save` / `ssh_forget` / `ssh_discover` ; sanitize anti-injection des champs (testé).
- **Scan en un clic** : « Scanner les ports locaux » et « Scanner le réseau » ne demandent plus de plage — ils utilisent la plage configurée (`settings.scan_from/scan_to`), zéro `prompt()`.
- **Écran d'accueil refait** : « Centre de contrôle de tes serveurs MCP », tuile **machines distantes**, trois boutons d'action directs et une ligne d'aide sur la découverte SSH. Section latérale « Command centers (pairs) » renommée « Machines & pairs ».
- **Icône personnalisée de l'application** : l'ancien carré bleu uni est remplacé par un vrai logo (hub MCP lumineux connectant 6 satellites sur tuile sombre arrondie, généré procéduralement). `icon.png` 512 + `icon.ico` multi-tailles (16→256) ; icône posée explicitement sur la fenêtre (`.icon()` — barre des tâches et Alt+Tab) et sur l'icône tray ; embarquée comme ressource de l'exe (Explorer, Task Manager, releases GitHub).
- **Icônes de marque manquantes de la boutique** : +31 glyphes Simple Icons posés par id exact de carte (ElevenLabs, MySQL, Redis, Android, Chrome, Jupyter, Dynatrace, Elasticsearch, Grafana, Hugging Face, CircleCI, Postman, Pulumi, Railway, Terraform, Webflow, Trello, Bitbucket, LeetCode, arXiv, SearXNG, Box, Mermaid, Figma Dev Mode, GitHub Integration, AKS, Sentry local, Notion, MongoDB, Supabase, JFrog) — `logos.rs` passe à 64 logos ; les rares sans marque officielle gardent le glyphe « paquet » neutre.
- **Catalogue GitHub (+4 items populaires)** : les serveurs MCP installables les plus étoilés du dépôt GitHub (topic `mcp-server`) absents des autres catalogues — Chrome DevTools (54k★), Codebase Memory (47k★), Context Mode (32k★), Serena (30k★, `uvx --from serena-agent`). Nouveau module `store_github.rs` ; les reconaissances de logo par commande couvrent les quatre préfixes de catalogue (`at-`/`gl-`/`gh-`).
- **Catalogue Glama (+6 items populaires)** : les serveurs MCP les plus populaires du classement glama.ai (tri GitHub stars) absents de la boutique — GitLab, Sentry, Google Maps, PaddleOCR, CodeGraph, World Monitor — paquets vérifiés sur npm/PyPI, clés d'env documentées. Nouveau module `store_glama.rs` ; les logos GitLab, Sentry, Google Maps et PaddlePaddle rejoignent `logos.rs` (33 logos au total), les items des catalogues additionnels sans marque reconnue reçoivent un glyphe « paquet » neutre.
- **Compteur dans le titre de la boutique** : « Boutique — serveurs MCP (60 MCP) » — mis à jour automatiquement au chargement du catalogue.
- **Boutique élargie au catalogue aitmpl (+25 items)** : les serveurs MCP de aitmpl.com (dépôt `davila7/claude-code-templates`, MIT) rejoignent la boutique — 54 items au total, nouvelles catégories (Audio, Intégrations, Recherche). Dédupliqués contre le catalogue principal (même paquet npm ou même id → écartés) et limités aux installations `npx`/`uvx` (41 items à commande docker/binaire écartés). Chaque item porte les clés d'env requises dans sa description. Nouveau module `store_aitmpl.rs` (généré) ; `install_store` et `/api/store` couvrent les deux catalogues ; logos par marque reconnue dans la commande, sinon glyphe « paquet » neutre.
- **Bloc « Collez une config… » retiré de la boutique** : la zone d'import par collage (textarea + Importer) est supprimée de l'UI — l'action API `import_mcp` reste disponible pour les scripts.
- **Logos officiels dans la boutique** : chaque carte affiche le logo vectoriel officiel de sa marque (19 glyphes Simple Icons : Google Drive, Git, GitHub, Puppeteer, Brave, PostgreSQL, SQLite, Slack, Playwright, DuckDuckGo, Kubernetes, Supabase, MongoDB, Stripe, Notion, Blender, Figma, n8n, Todoist — teintés aux couleurs officielles, adaptées au fond sombre pour GitHub/Notion/SQLite) et des glyphes génériques cohérents pour les serveurs sans marque (Filesystem, Memory, Sequential Thinking, Everything, Time, Fetch, Context7, Task Master, Firecrawl, Exa). Embarqués au binaire (nouveau module `logos.rs`, +34 Ko) et servis par `GET /api/store-logos`.
- **« Retirer durablement » un seed** : nouveau bouton (vue détail d'un serveur seed, avec confirmation) qui supprime le seed ET consigne son id dans `settings.removed_seeds` (config.json) — il n'est plus réinjecté au boot, contrairement au simple « Retirer » qui ne survit pas au redémarrage. Réinitialisation possible en vidant `removed_seeds` dans config.json. L'action `POST /api/action {"action":"remove","id":…,"durable":true}` porte la fonction.
- **Vie dans la zone de notification (tray)** : fermer la fenêtre la **masque** — l'app continue de tourner et de superviser les serveurs, icône dans le tray (clic gauche = afficher/masquer, menu clic droit : « Afficher le tableau de bord » / « Quitter » ; seule vraie sortie). Deuxième lancement pendant que l'app tourne : la fenêtre existante est réaffichée (`tauri-plugin-single-instance`) au lieu d'un second moteur condamné par les ports UI.
- **Démarrage automatique avec Windows** : l'app s'inscrit dans `HKCU\...\Run` (`tauri-plugin-autostart`, entrée « MCP Command Center --hidden ») — au boot, elle vit dans le tray et lance/supervise les serveurs (`auto_restart` des seeds) sans ouvrir de fenêtre. À retirer depuis le Gestionnaire des tâches › Applications de démarrage si besoin.
- **Barre de fenêtre unique et sombre (desktop)** : décorations natives supprimées (`decorations: false`) — fini la barre de titre Windows blanche au-dessus du header. Le header de l'UI devient la barre de fenêtre : zone de drag (`data-tauri-drag-region`, double-clic = agrandir) et boutons réduire/agrandir/fermer à droite (masqués dans un navigateur classique, où `__TAURI__` n'existe pas). Nouvelle capability `main-remote-ui` (drag + contrôles de fenêtre pour l'origine http locale du moteur) et `withGlobalTauri: true`.
- **Vue détail allégée** : la grille « Informations » (12 lignes) passe sous un repli `<details>` fermé par défaut — la page ne montre plus que l'essentiel.
- **Nouveau repli « Démarrage »** (ouvert) : état du contrôle du process — `managed` (lancé et maintenu par ce command center, issu de la map children), état du process (**« au démarrage… »** quand le process est vivant mais que MCP n'a pas encore répondu, « en marche », « arrêté », « port occupé », « désactivé »), pid, auto-restart, commande de lancement complète. L'API expose désormais `managed` et `start_cmd` dans `/api/state` et `/api/detail`.
- **État « AU DÉMARRAGE »** : le pill et le point de l'en-tête passent en style `STARTING` (accent pulsant) pendant la fenêtre spawn → première réponse MCP.

### Vérifié
- Tests du fingerprint : pur (`detect_framework` exige les deux signaux), sonde `/health` (200 « ok » vs JSON sans « ok »), intégration — faux serveur litemcp (initialize 2026-07-28 + tools/list + /health) → `framework = "litemcp"` + `lang = "rust"` ; sans `/health` → pas de fingerprint.

## [0.6.6] — Sécurité : code d'accès API, anti CSRF/DNS-rebinding, kill_port exact

### Sécurité (audit)
- **Code d'accès API** : exigé sur `/api/*` dès que le bind n'est pas loopback (header `X-MCC-Access` ou `?access=`). Généré au boot si absent et persisté (`settings.api_access`) ; pour la détection de pairs, mettre la **même valeur** des deux côtés (la sonde de pairs l'embarque désormais). Source alternative : variable d'environnement `MCC_API_ACCESS`.
- **Bind par défaut `127.0.0.1`** (était `0.0.0.0`) : l'ouverture LAN est désormais un choix explicite de l'opérateur, protégé par le code d'accès.
- **Anti CSRF/DNS-rebinding** : `OPTIONS` → 403 (préflights morts), header `Host` comparé à la liste des hosts légitimes (loopback + IP locales + `settings.allowed_origins`), `Origin` hors liste → 403, `POST /api/action` exige `Content-Type: application/json` (bloque les POST « simples » text/plain exécutables sans préflight depuis un site web). `Access-Control-Allow-Origin: *` remplacé par un reflet **uniquement** des origines autorisées.
- **kill_port par colonne** : le match par sous-chaîne (`:3141` matchait `:31410`) tuait le mauvais PID via taskkill — parse exact du port, exclusion de notre propre PID (`pids_listening_on`, testé).
- **Caps HTTP** : corps de requête plafonné à 2 Mo (413 au-delà — l'ancien code suivait un Content-Length arbitraire), connexions simultanées plafonnées à 64.
- **Boutique Filesystem** : le dossier exposé est demandé explicitement à l'installation (UI prompt / `args`), le catalogue n'embarque plus de chemin personnel.

### Corrigé (robustesse)
- Seeds introuvables (machine cliente) : après 3 échecs de spawn consécutifs, l'auto-restart se désactive (fini la boucle d'essais/warnings toutes les 15 s).
- Course double-spawn : le poller et une action API ne peuvent plus démarrer le même serveur en même temps (garde par id).
- `save_config` : les erreurs d'écriture sont loguées (avant : ajout perdu silencieusement au reboot).
- `local_ips` sur Linux/macOS : `hostname -I` est réellement parsé (avant : 0 IP parsée, fallback UDP seul → sous-réseaux manqués).
- `Host:` exact envoyé aux serveurs (`host:port`, IPv6 crochété) au lieu de `localhost` en dur (vhosts sur hôte distant).
- Import `https://` refusé avec un message clair (pas de TLS dans ce client std — une URL https finissait en sonde plaintext et en DOWN permanent).

### UI
- Wrapper `fetch` global : le code d'accès est injecté dans la page servie par le moteur (même origine) et embarqué dans chaque requête `/api/`.
- Install Filesystem : prompt du dossier à exposer ; l'install sans dossier est refusée côté backend.

### Breaking
- `POST /api/action` sans `Content-Type: application/json` est refusé (415) — mettre à jour les scripts curl (les exemples du README l'avaient déjà).
- Sur un bind LAN sans `api_access`, un code est généré au premier boot : lire `config.json` pour le récupérer.

## [0.6.5] — Fini les faux badges SSE : les FastMCP httpStream sont étiquetés HTTP+SSE

### Corrigé (classification transport)
- Tous les serveurs FastMCP lancés en `transportType: httpStream` répondaient badge « SSE ». Cause : mcp-proxy encode les réponses JSON-RPC en événements SSE (`Content-Type: text/event-stream`, `enableJsonResponse=false` par défaut) tout en restant du **streamable HTTP** — et FastMCP co-sert aussi `/sse` (compat 2024-11-05). Le Content-Type de l'initialize ne qualifie PAS le transport.
- Nouveau champ de sonde `sse_endpoint` : GET /sse considéré legacy seulement si le flux contient un vrai `event:` (un POST /mcp répondu = streamable HTTP, point).
- Badge UI : « HTTP+SSE » quand le serveur répond en streamable ET co-sert /sse (signature FastMCP/mcp-proxy), « HTTP » streamable JSON, « SSE » réservé aux réponses event-stream sans endpoint streamable prouvé. Info visible au survol.
- `/api/state` expose `sse_endpoint` ; `sse` conserve sa signification (encodage SSE des réponses).

Preuve : Overmind 3099 / discord 3141 / postgres 5433 / X 3142 → HTTP+SSE ; fixtures test-sse (réponses data: sans event) et JSON chunked → HTTP ; flux `event: endpoint` → SSE. 35 tests verts, clippy -D warnings propre.

## [0.6.5] — UI honnête : « Retirer » masqué sur les seeds + refresh parallèle

### Corrigé (audit 2026-10-19, F6/F7)
- **« Retirer » masqué pour les serveurs seed** (origin: seed) : leur retrait ne survivait pas au reboot (`load_config` réinjecte les 7 seeds) — un bouton qui promettait ce qu'il ne tient pas. Une note dans la vue détail l'explique et renvoie à **Désactiver** (persisté, lui). Les serveurs manuels/boutique gardent leur « Retirer ».
- **`refreshAll()` parallélisé** : les sondes des serveurs activés partent en `Promise.all` avec un seul rechargement d'état à la fin — avant, N rafraîchissements séquentiels pouvaient prendre N × timeout.
- **`ui-dist/index.html` (frontendDist Tauri)** : la redirection figée vers le port 5090 est remplacée par une sonde 5090→5099 qui rejoint l'URL réelle quand le core a glissé de port — le fallback mort de l'audit redevient vivant.
- Versions alignées sur 0.6.5 (tauri.conf.json traînait à 0.6.4) + `cargo fmt` appliqué au préexistant `probe.rs` (rustfmt de toolchain récente).

### Vérifié
- 35/35 tests, `cargo fmt --check` OK, clippy 0 warning.
- E2E local (app lancée avec `MCC_CONFIG` de test, 13 serveurs désactivés — aucun launcher spawné) : HTML servi contient les 3 modifs, JS embarqué compile (vm), contrat `/api/detail` (`origin` seed×7 / manual×6), et rendu `renderDetail` exécuté contre les vraies réponses API — seed : sans « Retirer » + note ; manuel : avec « Retirer », sans note.

## [0.6.4] — Plus aucune fenêtre console qui s'ouvre

### Corrigé (bug bloquant)
- Cliquer Start/Stop (ou chaque poll) ouvrait des fenêtres console : les spawns utilitaires (netstat, tasklist, taskkill, arp, ipconfig, hostname) n'avaient pas CREATE_NO_WINDOW depuis une app GUI. Helper `hidden()` appliqué à TOUS les Command de l'app.
- Kill des serveurs stdio : `Child::kill()` ne tuait que le lanceur .cmd, pas node.exe détaché → processus orphelins et consoles résiduelles. `kill_tree()` (taskkill /F /T /PID) remplace les 6 sites de kill (toggle OFF, kill_stdio, remove, chemins d'erreur handshake).

### Vérifié
- 0 console visible (MainWindowHandle ≠ 0) avant/après 5 toggles + polls.
- Start → UP (pid réel), Stop → process tué côté OS, aucune fenêtre.

## [0.6.3] — Langage détecté par le process réel + chips visibles partout

### Corrigé
- Les serveurs HTTP locaux sans en-têtes n'avaient AUCUN langage → chip absent. Le probe interroge désormais le vrai process qui écoute le port (netstat -ano → PID → tasklist → image, cache 10 s) : node.exe → node, python.exe → python, java.exe → java, dotnet.exe → dotnet…
- Normalisation de l'image (`node.exe` → `node`) pour que la chip s'affiche.
- Chip « ? » explicite quand le langage est indétectable (au lieu de rien).
- La fenêtre native desktop embarquait un vieux core (v0.6.0) → rebuild aligné sur le core courant.

Preuve : Overmind/discord/postgres/X/NEWS détectés `node` via leur process node.exe ; py-header → python (header uvicorn) ; stdio → node (commande).

## [0.6.3] — Desktop multi-OS (Windows, Linux, macOS Intel + Apple Silicon)

### Distribution
- Le job CI `desktop` construit désormais la fenêtre native Tauri sur les 4 cibles : `windows-x86_64`, `linux-x86_64`, `macos-aarch64` (Apple Silicon), `macos-x86_64` (Intel).
- Linux : dépendances WebKitGTK installées dans le job (`libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `librsvg2-dev`).
- Assets publiés : `mcp-command-center-desktop-{windows-x86_64.exe, linux-x86_64, macos-aarch64, macos-x86_64}` en plus des 4 binaires CLI.

Note Linux : le binaire desktop utilise la webview système (WebKitGTK) — le paquet d'exécution `libwebkit2gtk-4.1-0` est requis (standard sur Ubuntu 24.04+).

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
