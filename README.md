# MCP Command Center

Ton tableau de bord pour les serveurs **MCP** : l'app les **trouve toute seule**, les **surveille** en continu, et te laisse les **démarrer / arrêter** d'un clic. Application native en Rust — légère (~40 Mo en mémoire), aucune dépendance à installer.

![Aperçu](assets/readme-accueil.png)
*L'app au démarrage : tes serveurs à gauche (état, badges, langage), les chiffres clés au centre, le journal en bas. Tout est découvert automatiquement.*

## En 30 secondes

1. Lance `start-command-center.bat` (ou l'exe installé).
2. L'app s'ouvre et **scanne tous les ports de la machine** — chaque serveur MCP apparaît tout seul, quel que soit son port.
3. C'est tout. Clique sur un serveur pour voir ses tools, ses infos, et le piloter.

> Astuce : un serveur ne s'affiche pas ? Clique sur **⟳** (en haut à droite) — il re-sonde tout et cherche les nouveaux.

## Ce que tu vois

![Détail d'un serveur](assets/readme-detail.png)
*Un serveur sélectionné : sa liste de tools (chacun activable/désactivable), son état, sa latence, sa version, et le **framework** avec lequel il est construit (badge `FastMCP`, `LiteMCP`…) avec la spec MCP utilisée.*

![Serveur Rust](assets/readme-rust.png)
*Un serveur écrit en Rust : logo **R** cerclé d'un engrenage + badge du framework détecté automatiquement (ici LiteMCP, spec 2026-07-28).*

![Boutique](assets/readme-boutique.png)
*La **boutique** : 64 serveurs MCP installables en un clic (Filesystem, GitHub, Playwright, Puppeteer…), avec leurs logos officiels.*

## Les boutons

| Bouton | Ce qu'il fait |
|---|---|
| **⟳** | Rafraîchit tout **et** découvre les nouveaux serveurs |
| **🔍 Scan ports** | Scanne une plage de ports précise (réglable dans config.json) |
| **🌐 Scan réseau** | Cherche des serveurs MCP sur les autres machines du réseau local |
| **🖥️ Machines** | Connecte une autre machine (VPS, Docker…) en SSH et importe ses serveurs — même ceux bindés `localhost` |
| **🛒 Boutique** | 64 serveurs MCP à installer en un clic |
| **+ Ajouter** | Ajoute un serveur à la main (port HTTP, ou programme stdio) |
| **ON/OFF** (par ligne) | Allume / éteint un serveur |
| **▶ / ■** | Démarre / arrête le process du serveur |
| **⇅** (au-dessus de la liste) | Reclasse les serveurs : A→Z, état, port, latence, tools — ou glisse une ligne à la main |
| **⏭ Auto-boot** | Lance (ou non) l'app au démarrage de Windows ; fermer la fenêtre = réduit dans le tray |

## Questions fréquentes

**Mon serveur MCP n'apparaît pas ?**
Au démarrage, l'app lit tous les ports en écoute de la machine et sonde chacun (`/mcp`, `/`, `/api/mcp`) — il est trouvé automatiquement. Sinon : bouton **⟳**, ou **+ Ajouter** avec son port.

**C'est quoi les badges `FastMCP`, `LiteMCP`, `HTTP+SSE` ?**
Le framework détecté automatiquement par fingerprint (avec la spec MCP utilisée), et le type de transport. Tu n'as rien à configurer.

**On me demande un « code d'accès » ?**
Seulement si l'UI est ouverte aux autres machines du LAN. En usage local (fenêtre de l'app, navigateur de la machine) : **jamais aucun code**. Le code vit dans `settings.api_access` du config.json.

**Où sont les logs ?**
Dans `mcc.log`, à côté du config.json (et dans le panneau Journal de l'app).

**Comment ça démarre avec Windows ?**
Bouton **⏭ Auto-boot ON** dans la barre de titre. L'app vit alors dans le tray et surveille tes serveurs sans fenêtre.

## Pour les scripts (API)

Tout est pilotable en HTTP local (aucun code requis en local) :

```bash
# état complet + journal
curl http://127.0.0.1:5090/api/state

# détail d'un serveur (liste des tools)
curl "http://127.0.0.1:5090/api/detail?id=seed-overmind"

# une action : refresh | toggle | scan | discover | add | remove | start | kill | reorder | install_store | import_mcp …
curl -X POST -H "Content-Type: application/json" \
  -d '{"action":"add","transport":"stdio","label":"mon-mcp","program":"node","args":["dist/index.js"]}' \
  http://127.0.0.1:5090/api/action
```

Si l'UI est ouverte au LAN (`"ui_bind": "0.0.0.0"`), chaque appel `/api/*` exige le code (`X-MCC-Access`). Les pairs command-center s'authentifient par preuve HMAC — le code ne circule jamais en clair.

## Sécurité, en une minute

- Par défaut, l'UI n'écoute que la machine locale (`127.0.0.1`) — rien d'exposé.
- Ouverture LAN : code d'accès généré automatiquement, jamais envoyé dans la page.
- Protections anti-CSRF/DNS-rebinding, CSP stricte, corps de requêtes plafonnés.

## Compiler / installer

```bash
cargo build --release -p mcp-command-center -p mcp-command-center-desktop
```

- `mcp-command-center` : le moteur (headless, console) — `start-headless.bat`
- `mcp-command-center-desktop` : la fenêtre native (Tauri) — `start-command-center.bat`
- Tests : `cargo test` (75) · Lint : `cargo clippy --all-targets -- -D warnings`

> **Écosystème** : les serveurs écrits avec [**LiteMCP**](https://github.com/DeamonDev888/litemcp) (lib MCP Rust stateless, [crates.io](https://crates.io/crates/litemcp)) sont détectés et badgés nativement.

## Limites connues

- La détection HTTP ne fait pas de TLS (un serveur `https://` n'est pas sondable — passe par un tunnel local).
- `config.json` vit à côté de l'exe (chemin figé, surchargeable via `MCC_CONFIG`).
