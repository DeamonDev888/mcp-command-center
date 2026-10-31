//! Page UI : coquille HTML — les styles et scripts vivent dans des ASSETS
//! servis (src/ui/app.css, app-boot.js, app.js) : fin du monolithe inline,
//! CSP serrée possible, diffs d'UI lisibles, plus de contrainte de raw string.
//! Service : GET / (coquille), /app.css, /app-boot.js, /app.js (lib.rs) —
//! query ?v=VERSION pour le cache immuable des assets.
pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>MCP Control</title>
<link rel="stylesheet" href="/app.css?v=__MCC_VERSION__">
<script src="/app-boot.js?v=__MCC_VERSION__"></script>
<script src="/app.js?v=__MCC_VERSION__" defer></script>
</head>
<body>

<div id="titlebar" data-tauri-drag-region>
  <span class="logo" data-tauri-drag-region>
    <svg class="mark" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="2.5" y="4.5" width="19" height="15" rx="3.2" fill="none" stroke="currentColor" stroke-width="1.5"/>
      <circle cx="8" cy="10" r="1.7" fill="currentColor"/>
      <path d="M5.5 16.2c1.6-1.5 3.1-2.2 4.6-2.2s3 .7 4.6 2.2" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
    </svg>
    <span class="wordmark">MCP<span class="wm-dim"> Control</span></span>
  </span>
  <span class="tb-badge ok" title="Serveurs MCP qui répondent, sur le total suivi">UP <b id="stUp">0</b>/<b id="stTotal">0</b></span>
  <span class="tb-badge" title="Outils MCP exposés par les serveurs UP">outils <b id="stTools">0</b></span>
  <button id="tbLogout" class="tb-logout" hidden onclick="logout()"
          title="Oublier le code d'accès enregistré dans ce navigateur"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><rect x='4.5' y='10.5' width='15' height='10' rx='2'/><path d='M8 10.5V7a4 4 0 0 1 8 0v3.5'/></svg></button>
  <span class="tb-spacer" data-tauri-drag-region></span>
  <div class="tb-actions">
    <button class="primary" onclick="doScan()" title="Balayer les ports locaux pour d&eacute;couvrir les serveurs MCP"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><circle cx='11' cy='11' r='7'/><path d='m21 21-4.3-4.3'/></svg>Scanner les ports</button>
    <button onclick="doScanNet()" title="Balayer le r&eacute;seau local pour d&eacute;couvrir les serveurs MCP distants"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><circle cx='12' cy='12' r='8.5'/><path d='M3.5 12h17M12 3.5c2.4 2.4 3.8 5.3 3.8 8.5s-1.4 6.1-3.8 8.5c-2.4-2.4-3.8-5.3-3.8-8.5s1.4-6.1 3.8-8.5Z'/></svg>Scanner le r&eacute;seau</button>
    <button onclick="openMachineDlg()" title="Machines distantes (SSH) et d&eacute;tection des moteurs MCP Control du LAN"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><rect x='2.5' y='4' width='19' height='13' rx='2'/><path d='M8 21h8M12 17v4'/></svg>Machines</button>
    <button onclick="openStore()" title="Catalogue de serveurs MCP pr&ecirc;ts &agrave; installer"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><circle cx='9' cy='20' r='1.4'/><circle cx='17' cy='20' r='1.4'/><path d='M2.5 3.5h2.2l2.5 11.5h10.3l2-8H6'/></svg>Boutique</button>
    <button onclick="openSettingsDlg()" title="R&eacute;glages du client"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><circle cx='12' cy='12' r='3.1'/><path d='M12 2.8v2.6M12 18.6v2.6M2.8 12h2.6M18.6 12h2.6M5.5 5.5l1.9 1.9M16.6 16.6l1.9 1.9M18.5 5.5l-1.9 1.9M7.4 16.6l-1.9 1.9'/></svg>R&eacute;glages</button>
    <button onclick="refreshAll()" title="Re-sonder tous les serveurs connus et d&eacute;couvrir les nouveaux (tous ports en &eacute;coute)"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M21 12a9 9 0 1 1-2.6-6.4L21 8'/><path d='M21 3v5h-5'/></svg></button>
  </div>
  <div id="winCtrls">
    <button class="wbtn" id="wMin" title="R&eacute;duire">&#x2500;</button>
    <button class="wbtn" id="wMax" title="Agrandir / restaurer">&#x25A1;</button>
    <button class="wbtn wclose" id="wClose" title="Fermer">&#x2715;</button>
  </div>
</div>

<div id="main">
  <aside id="explorer">
    <div class="sec-label" title="Glisser-déposer une ligne pour r&eacute;ordonner (pr&eacute;f&eacute;rence d'affichage et priorit&eacute; de sondage) — ou reclasser via le menu : l'ordre appliqué est persisté">
      <span>Serveurs MCP</span>
      <select id="sortSel" onchange="applySort(this.value)" title="Reclasser les serveurs — « manuel » rend l'ordre pos&eacute; au glisser-déposer avant le tri">
        <option value="manuel">⇅ manuel</option>
        <option value="az">A→Z</option>
        <option value="etat">état</option>
        <option value="port">port</option>
        <option value="latence">latence</option>
        <option value="tools">tools</option>
        <option value="framework">framework</option>
        <option value="transport">transport</option>
      </select>
    </div>
    <div id="srvFilterWrap">
      <input id="srvFilter" placeholder="Filtrer les serveurs&hellip;"
        title="Filtre par nom, port, framework ou &eacute;tat &mdash; raccourcis : / ou Ctrl+K, &Eacute;chap efface"
        oninput="renderServers()">
      <button id="srvFilterClear" hidden onclick="clearFilter()" title="Effacer le filtre">&times;</button>
    </div>
    <div id="hiddenBar" hidden onclick="showHiddenServers()"
      title="Les serveurs masqu&eacute;s ne le restent que pour cette session : ils reviennent &agrave; la r&eacute;ouverture de l'application (ou d'un rechargement de la page)"></div>
    <div id="localSec">
      <div class="grphead" id="localHead" onclick="toggleGroup('local')"
        title="Serveurs MCP locaux &agrave; cette machine (loopback) — cliquer pour replier/d&eacute;plier">
        <span class="gchev">&#9662;</span><span class="gname">Serveurs locaux</span><span class="gcount" id="localCount"></span>
      </div>
      <div class="grpbody" id="srvList"></div>
    </div>
    <div class="sec-label" id="remoteLabel" hidden>Machines distantes</div>
    <div id="remoteGroups"></div>
    <div class="sec-label">Machines &amp; pairs</div>
    <div id="peerList"></div>
    <div id="addWrap"><button onclick="toggleAdd()">+ Ajouter un serveur</button></div>
  </aside>
  <div id="grip" title="Glisser pour ajuster la largeur"></div>
  <section id="content"></section>
</div>

<div id="logpanel">
  <div id="loghead" onclick="toggleLog()">
    <span class="arrow">&#9662;</span><span class="lbl">Journal</span>
    <button id="netWarn" class="tb-badge netwarn" hidden onclick="event.stopPropagation();showNetDetail()"
      title="Les serveurs distants ne peuvent pas &ecirc;tre sond&eacute;s &mdash; VPN actif ? (Proton VPN : activer &laquo;&nbsp;autoriser les connexions LAN&nbsp;&raquo;)"></button>
    <button id="stAuto" class="sb-auto tb-push" onclick="event.stopPropagation();toggleAutostart()"
      title="Lancer l'application au d&eacute;marrage de l'OS (tray silencieux)"></button>
    <span class="tb-badge">auto-d&eacute;tection : HTTP streamable &middot; stdio JSON-RPC</span>
    <span class="sb-ver" title="Version install&eacute;e">v__MCC_VERSION__</span>
    <button id="stUpdate" class="sb-upd" hidden onclick="event.stopPropagation();downloadUpdate()"
      title="T&eacute;l&eacute;charge directement la mise &agrave; jour pour cette machine"></button>
  </div>
  <div id="logbody"></div>
</div>

<dialog id="dlg">
  <div class="dlg-head">
    <h2>Ajouter un serveur MCP</h2>
    <button class="close" onclick="dlg.close()">&times;</button>
  </div>
  <div class="dlg-body">
    <form onsubmit="return addServer(event)">
      <div class="row">
        <div><label>Transport</label>
          <select id="fTransport" onchange="transportChanged()">
            <option value="http">HTTP (streamable)</option>
            <option value="stdio">STDIO (process)</option>
          </select></div>
        <div><label>Nom affich&eacute;</label><input id="fLabel" placeholder="mon-serveur"></div>
      </div>
      <div id="httpFields">
        <div class="row">
          <div><label>Port</label><input id="fPort" type="number" placeholder="3099"></div>
          <div><label>Chemin MCP</label><input id="fPath" value="/mcp"></div>
        </div>
        <div class="stack">
          <label>Commande de lancement (option — permet Start/Stop par MCP Control)</label>
          <div class="row row-gap">
            <div><input id="fHttpProgram" placeholder="ex. node ou uvx mcp-server-x"></div>
            <div><input id="fHttpWorkdir" placeholder="workdir (option)"></div>
          </div>
          <input id="fHttpArgs" placeholder="args (ex. dist/server.js --port 3099)">
        </div>
      </div>
      <div id="stdioFields" hidden>
        <div class="row">
          <div><label>Programme</label><input id="fProgram" placeholder="node"></div>
          <div><label>Workdir (option)</label><input id="fWorkdir" placeholder="C:\chemin\serveur"></div>
        </div>
        <div class="stack"><label>Args (s&eacute;par&eacute;s par espace)</label><input id="fArgs" placeholder="dist/index.js --port 3000"></div>
      </div>
      <div class="row row-end">
        <button type="button" onclick="dlg.close()">Annuler</button>
        <button type="submit" class="primary">Ajouter &amp; sonder</button>
      </div>
    </form>
  </div>
</dialog>

<dialog id="storeDlg">
  <div class="dlg-head">
    <h2><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><circle cx='9' cy='20' r='1.4'/><circle cx='17' cy='20' r='1.4'/><path d='M2.5 3.5h2.2l2.5 11.5h10.3l2-8H6'/></svg> Boutique &mdash; serveurs MCP <span id="storeCount" class="store-count"></span></h2>
    <button class="close" onclick="storeDlg.close()">&times;</button>
  </div>
  <div class="dlg-body dlg-body-tight">
    <div class="store-head">
      <div class="store-bar">
        <input id="storeSearch" placeholder="Rechercher dans la boutique&hellip;" oninput="renderStoreItems()">
        <select id="storeSort" onchange="renderStoreItems()" title="Trier les r&eacute;sultats">
          <option value="cat">Ordre du catalogue</option>
          <option value="az">A→Z</option>
        </select>
        <button id="storeHideInst" class="scat" onclick="toggleHideInst()"
          title="Masquer les serveurs d&eacute;j&agrave; pr&eacute;sents dans ta liste">Masquer les install&eacute;s</button>
      </div>
      <div id="storeCats" class="store-cats"></div>
      <div id="storeResults" class="store-results"></div>
    </div>
    <div id="storeGrid" class="store-grid"></div>
  </div>
</dialog>

<dialog id="machineDlg">
  <div class="dlg-head">
    <h2><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><rect x='2.5' y='4' width='19' height='13' rx='2'/><path d='M8 21h8M12 17v4'/></svg> Machines distantes</h2>
    <button class="close" onclick="machineDlg.close()">&times;</button>
  </div>
  <div class="dlg-body">
    <div class="conf-note">Connexion SSH par cl&eacute; (BatchMode — aucun mot de passe possible) : la machine
    doit accepter ta cl&eacute; publique. L'app liste ses ports en &eacute;coute via <b>netstat</b> distant,
    y sonde les serveurs MCP et les importe ici automatiquement.</div>
    <div class="machine-form">
      <input id="mIp" placeholder="IP ou h&ocirc;te (ex. 192.168.1.42)">
      <input id="mUser" placeholder="Utilisateur SSH">
      <button class="primary" onclick="sshDiscover()"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7'/><path d='M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7'/></svg> SSH &amp; d&eacute;couvrir</button>
      <button onclick="sshSave()"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M5 3.5h11L20.5 8v11a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 19V5A1.5 1.5 0 0 1 5 3.5Z'/><path d='M8 3.5V8h7V3.5M7.5 20.5v-6h9v6'/></svg> Enregistrer</button>
    </div>
    <div id="machineMsg" class="machine-msg"></div>
    <div id="machineList"></div>
    <div class="stack"><button onclick="doPeers();setTimeout(renderMachines,3000)"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M21 12a9 9 0 1 1-2.6-6.4L21 8'/><path d='M21 3v5h-5'/></svg> D&eacute;tecter les moteurs MCP Control sur le LAN</button></div>
  </div>
</dialog>

<dialog id="settingsDlg" class="settings-dlg">
  <div class="dlg-head">
    <div class="dlg-title">
      <svg class="mark" viewBox="0 0 24 24" aria-hidden="true">
        <circle cx="12" cy="12" r="3.1" fill="none" stroke="currentColor" stroke-width="1.5"/>
        <path d="M12 2.6v3.2M12 18.2v3.2M2.6 12h3.2M18.2 12h3.2M5.3 5.3l2.3 2.3M16.4 16.4l2.3 2.3M18.7 5.3l-2.3 2.3M7.6 16.4l-2.3 2.3"
          fill="none" stroke="currentColor" stroke-width="1.4" stroke-linecap="round"/>
      </svg>
      <div>
        <h2>R&eacute;glages</h2>
        <p>Pr&eacute;f&eacute;rences du client MCP Control</p>
      </div>
    </div>
    <button class="close" onclick="closeSettings()" title="Fermer">&times;</button>
  </div>
  <div class="set-shell">
    <nav class="set-nav" id="setNav" aria-label="Sections des r&eacute;glages">
      <button class="set-nav-btn sel" data-pane="general" onclick="setPane('general')"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><circle cx='12' cy='12' r='3.1'/><path d='M12 2.8v2.6M12 18.6v2.6M2.8 12h2.6M18.6 12h2.6M5.5 5.5l1.9 1.9M16.6 16.6l1.9 1.9M18.5 5.5l-1.9 1.9M7.4 16.6l-1.9 1.9'/></svg>G&eacute;n&eacute;ral</button>
      <button class="set-nav-btn" data-pane="decouverte" onclick="setPane('decouverte')"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><circle cx='11' cy='11' r='7'/><path d='m21 21-4.3-4.3'/></svg>D&eacute;couverte</button>
      <button class="set-nav-btn" data-pane="interface" onclick="setPane('interface')"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><rect x='2.5' y='4' width='19' height='13' rx='2'/><path d='M8 21h8M12 17v4'/></svg>Interface</button>
      <button class="set-nav-btn" data-pane="notifications" onclick="setPane('notifications')"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M6 9a6 6 0 0 1 12 0c0 5 2 6.5 2 6.5H4S6 14 6 9Z'/><path d='M10 19.5a2.2 2.2 0 0 0 4 0'/></svg>Notifications</button>
      <button class="set-nav-btn" data-pane="updates" onclick="setPane('updates')"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M12 15.5V4m0 0L7.5 8.5M12 4l4.5 4.5'/><path d='M4 16.5V19a1.5 1.5 0 0 0 1.5 1.5h13A1.5 1.5 0 0 0 20 19v-2.5'/></svg>Mises &agrave; jour</button>
      <div class="set-nav-sep"></div>
      <button class="set-nav-btn danger" data-pane="reset" onclick="setPane('reset')"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M12 3.5 2.5 20h19L12 3.5Z'/><path d='M12 10v4.5M12 17.1v.1'/></svg>R&eacute;initialiser</button>
    </nav>
    <div class="set-panes">
      <section class="set-pane sel" data-pane="general">
        <div class="set-list">
          <div class="set-row">
            <div class="set-label"><span>Intervalle de sondage</span>
              <em>D&eacute;lai entre deux cycles de sonde. Appliqu&eacute; au cycle suivant.</em>
              <div class="set-err" id="err_refresh" hidden></div></div>
            <div class="set-ctrl"><input id="sRefresh" type="number" min="5" max="3600" step="5" class="set-num">
              <span class="set-unit">secondes</span></div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>Th&egrave;me</span>
              <em>Apparence de l'interface. Conserv&eacute; sur ce poste.</em></div>
            <div class="set-ctrl seg" role="group" aria-label="Th&egrave;me">
              <button id="tDark" class="seg-btn" onclick="applyThemeAndSave('dark')" title="Th&egrave;me sombre (d&eacute;faut)">Sombre</button>
              <button id="tLight" class="seg-btn" onclick="applyThemeAndSave('light')" title="Th&egrave;me clair">Clair</button>
            </div>
          </div>
        </div>
      </section>

      <section class="set-pane" data-pane="decouverte">
        <div class="set-list">
          <div class="set-row">
            <div class="set-label"><span>Plage de scan de ports</span>
              <em>Ports balay&eacute;s par &laquo; Scan ports &raquo;. La d&eacute;couverte native litemcp n'en d&eacute;pend pas.</em>
              <div class="set-err" id="err_scan" hidden></div></div>
            <div class="set-ctrl"><input id="sScanFrom" type="number" min="1" max="65535" class="set-num">
              <span class="set-dash">&ndash;</span>
              <input id="sScanTo" type="number" min="1" max="65535" class="set-num"></div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>Pairs &amp; r&eacute;seau</span>
              <em>D&eacute;tecte les autres moteurs MCP Control du LAN et leurs serveurs distants.</em></div>
            <div class="set-ctrl">
              <label class="switch"><input type="checkbox" id="sNetEnabled"><span class="switch-track"></span></label>
            </div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>Ports UI des pairs</span>
              <em>Ports interrog&eacute;s chez les pairs pour les rep&eacute;rer. S&eacute;par&eacute;s par des virgules.</em>
              <div class="set-err" id="err_ports" hidden></div></div>
            <div class="set-ctrl"><input id="sNetPorts" placeholder="5090, 5091" class="set-txt"></div>
          </div>
        </div>
      </section>

      <section class="set-pane" data-pane="interface">
        <div class="set-list">
          <div class="set-row">
            <div class="set-label"><span>&Eacute;coute</span>
              <em>127.0.0.1 = cette machine seule. 0.0.0.0 = accessible du LAN (code d'acc&egrave;s exig&eacute;).
              Appliqu&eacute; au prochain d&eacute;marrage.</em></div>
            <div class="set-ctrl">
              <select id="sUiBind">
                <option value="127.0.0.1">127.0.0.1</option>
                <option value="0.0.0.0">0.0.0.0</option>
              </select>
            </div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>Port</span>
              <em>Fig&eacute; au d&eacute;marrage du listener &mdash; modifiable dans config.json.</em></div>
            <div class="set-ctrl"><input id="sUiPort" type="number" disabled class="set-num"></div>
          </div>
        </div>
      </section>

      <section class="set-pane" data-pane="notifications">
        <div class="set-list">
          <div class="set-row">
            <div class="set-label"><span>Toast desktop</span>
              <em>Notification Windows quand un serveur surveill&eacute; tombe (UP &rarr; DOWN ou PORT-ONLY).</em></div>
            <div class="set-ctrl">
              <label class="switch"><input type="checkbox" id="sNotifDesktop"><span class="switch-track"></span></label>
            </div>
          </div>
          <div class="set-row col">
            <div class="set-label"><span>Webhook</span>
              <em>Requ&ecirc;te JSON &agrave; chaque incident (compatible Slack, Discord, n8n, endpoint maison).</em>
              <div class="set-err" id="err_webhook" hidden></div></div>
            <div class="set-ctrl set-ctrl-wide">
              <input id="sWebhookUrl" placeholder="https://hooks.exemple.com/services/&#8230;">
            </div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>M&eacute;thode HTTP</span>
              <em>Verbe utilis&eacute; pour le webhook.</em></div>
            <div class="set-ctrl">
              <select id="sWebhookMethod">
                <option value="POST">POST</option>
                <option value="PUT">PUT</option>
              </select>
            </div>
          </div>
          <div class="set-row col">
            <div class="set-label"><span>En-t&ecirc;tes additionnels</span>
              <em>Une paire &laquo; Cl&eacute;: valeur &raquo; par ligne (ex. Authorization: Bearer&#8230;).</em></div>
            <div class="set-ctrl set-ctrl-wide">
              <textarea id="sWebhookHeaders" rows="3" placeholder="Authorization: Bearer &#8230;"></textarea>
            </div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>&Eacute;tats notifi&eacute;s</span>
              <em>Les notifications se d&eacute;clenchent &agrave; la transition depuis UP.</em>
              <div class="set-err" id="err_states" hidden></div></div>
            <div class="set-ctrl set-inline">
              <label class="set-chk"><input type="checkbox" id="sStDown"> DOWN</label>
              <label class="set-chk"><input type="checkbox" id="sStPortOnly"> PORT-ONLY</label>
            </div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>D&eacute;lai minimum</span>
              <em>Anti-spam : pas deux notifications du m&ecirc;me serveur dans cet intervalle.</em>
              <div class="set-err" id="err_min" hidden></div></div>
            <div class="set-ctrl"><input id="sNotifMin" type="number" min="0" max="86400" step="30" class="set-num">
              <span class="set-unit">secondes</span></div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>Test</span>
              <em>Enregistre la section puis &eacute;met une notification sur chaque canal activ&eacute;.</em></div>
            <div class="set-ctrl"><button onclick="testNotify()"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M6 9a6 6 0 0 1 12 0c0 5 2 6.5 2 6.5H4S6 14 6 9Z'/><path d='M10 19.5a2.2 2.2 0 0 0 4 0'/></svg> Tester</button></div>
          </div>
          <div class="set-row" hidden id="notifyMsgRow">
            <div class="set-ctrl set-ctrl-wide"><span id="notifyMsg" class="set-note"></span></div>
          </div>
        </div>
      </section>

      <section class="set-pane" data-pane="updates">
        <div class="set-list">
          <div class="set-row">
            <div class="set-label"><span>Version install&eacute;e</span>
              <em>Release GitHub du repo DeamonDev888/mcp-control.</em></div>
            <div class="set-ctrl"><span class="set-static" id="sVersion">v&ndash;</span></div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>Derni&egrave;re release</span>
              <em id="updMsg">&nbsp;</em></div>
            <div class="set-ctrl">
              <span class="set-static" id="sLatest">&ndash;</span>
              <button onclick="checkUpdateNow()" title="Interroge l'API GitHub releases/latest maintenant"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M21 12a9 9 0 1 1-2.6-6.4L21 8'/><path d='M21 3v5h-5'/></svg> V&eacute;rifier</button>
              <button class="primary" onclick="downloadUpdate()" title="T&eacute;l&eacute;charge directement l'installateur pour cette machine"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M12 15.5V4m0 0L7.5 8.5M12 4l4.5 4.5'/><path d='M4 16.5V19a1.5 1.5 0 0 0 1.5 1.5h13A1.5 1.5 0 0 0 20 19v-2.5'/></svg> Mettre &agrave; jour</button>
              <button onclick="openReleasePage()" title="Toutes les versions et archives"><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><path d='M14 4h6v6M20 4l-9 9'/><path d='M19 14v5a1.5 1.5 0 0 1-1.5 1.5h-12A1.5 1.5 0 0 1 4 19V7.5A1.5 1.5 0 0 1 5.5 6H10'/></svg> Releases</button>
            </div>
          </div>
        </div>
      </section>

      <section class="set-pane" data-pane="reset">
        <div class="set-list">
          <div class="set-row">
            <div class="set-label"><span>Remettre &agrave; z&eacute;ro</span>
              <em>Placement des serveurs, d&eacute;tections import&eacute;es et oublis. Irr&eacute;versible.</em></div>
            <div class="set-ctrl">
              <select id="sResetWhat">
                <option value="placement">Placement des serveurs</option>
                <option value="detection">D&eacute;tections &amp; importations</option>
                <option value="oublis">Oublis (seeds + machines)</option>
                <option value="tout">Tout</option>
              </select>
              <button class="danger" onclick="resetState(document.getElementById('sResetWhat').value)">R&eacute;initialiser</button>
            </div>
          </div>
          <div class="set-row" id="resetMsgRow" hidden>
            <div class="set-ctrl set-ctrl-wide"><span id="resetMsg" class="set-note"></span></div>
          </div>
          <div class="set-row">
            <div class="set-label"><span>Historique runtime</span>
              <em>Efface crashs, relances et compteurs d'&eacute;chec enregistr&eacute;s par serveur. Ne touche ni la liste ni les r&eacute;glages.</em></div>
            <div class="set-ctrl"><button class="danger" onclick="resetRuntime()">Effacer</button></div>
          </div>
          <div class="set-row" hidden id="runtimeMsgRow">
            <div class="set-ctrl set-ctrl-wide"><span id="runtimeMsg" class="set-note"></span></div>
          </div>
        </div>
      </section>
    </div>
  </div>
  <div id="settingsMsg" class="machine-msg set-msgline"></div>
  <div class="dlg-foot">
    <span id="settingsDirty" class="set-dirty" hidden>&#9679; Modifications non enregistr&eacute;es</span>
    <span class="dlg-foot-note" id="settingsFootNote"></span>
    <div class="row">
      <button onclick="closeSettings()">Annuler</button>
      <button class="primary" onclick="saveSettings()">Enregistrer</button>
    </div>
  </div>
</dialog>

<dialog id="loginDlg">
  <div class="dlg-head">
    <h2><svg class="ic" viewBox="0 0 24 24" aria-hidden="true"><rect x='4.5' y='10.5' width='15' height='10' rx='2'/><path d='M8 10.5V7a4 4 0 0 1 8 0v3.5'/></svg> Acc&egrave;s prot&eacute;g&eacute;</h2>
  </div>
  <div class="dlg-body">
    <div class="conf-note">Cette UI est expos&eacute;e au r&eacute;seau : saisis le <b>code d'acc&egrave;s API</b>
    du serveur (<b>settings.api_access</b> de son config.json, ou variable <b>MCC_API_ACCESS</b>).
    Il reste dans ce navigateur uniquement — le serveur ne l'envoie jamais dans la page.</div>
    <form onsubmit="return loginSubmit(event)">
      <label>Code d'acc&egrave;s</label>
      <input id="loginCode" type="password" autocomplete="off" placeholder="code d'acc&egrave;s">
      <div id="loginMsg" class="machine-msg"></div>
      <div class="row row-between">
        <button type="button" onclick="forgetCode()" title="Supprime le code enregistr&eacute; dans ce navigateur">Oublier le code enregistr&eacute;</button>
        <button type="submit" class="primary">D&eacute;verrouiller</button>
      </div>
    </form>
  </div>
</dialog>

<div id="ctxmenu" class="ctxmenu" hidden></div>
<div id="toast"></div>
</body>
</html>
"#;

/// Styles de l'UI — servis par GET /app.css (versionnés ?v=, cache immuable).
pub const APP_CSS: &str = include_str!("ui/app.css");

/// Amorçage thème AVANT le premier rendu (anti-flash) — bloquant dans <head>.
pub const APP_BOOT_JS: &str = include_str!("ui/app-boot.js");

/// Script principal de l'UI — servis par GET /app.js (defer, après le DOM).
pub const APP_JS: &str = include_str!("ui/app.js");

#[cfg(test)]
mod tests {
    use super::{APP_BOOT_JS, APP_CSS, APP_JS, INDEX_HTML};

    // ---------- contrat « tout bouton est câblé » --------------------------
    // Chaque handler référencé par un attribut on* du HTML (coquille + gabarits
    // JS de app.js : lignes de serveur, menus contextuels, boutique…) doit être
    // défini dans la page. Un handler référencé et absent = bouton mort : ce
    // test le fait échouer au compile test, pas devant l'utilisateur.
    // Depuis le split en assets servis, le « code de la page » = coquille
    // HTML + app.js — les deux sont scannés partout ci-dessous.

    /// Texte complet de l'UI : coquille HTML + script principal.
    fn ui_all() -> String {
        format!("{}\n{}", INDEX_HTML, APP_JS)
    }

    const ON_ATTRS: &[&str] = &[
        "onclick=\"",
        "onsubmit=\"",
        "onchange=\"",
        "oninput=\"",
        "ontoggle=\"",
        "ondragstart=\"",
        "ondragover=\"",
        "ondrop=\"",
        "ondragleave=\"",
        "ondragend=\"",
    ];

    /// Mots-clés JS et fonctions natives : jamais des handlers applicatifs.
    const JS_NON_HANDLERS: &[&str] = &[
        "if",
        "else",
        "for",
        "while",
        "do",
        "switch",
        "case",
        "try",
        "catch",
        "finally",
        "return",
        "function",
        "typeof",
        "new",
        "delete",
        "void",
        "await",
        "async",
        "throw",
        "let",
        "const",
        "var",
        "setTimeout",
        "setInterval",
        "clearTimeout",
        "clearInterval",
        "alert",
        "confirm",
        "prompt",
        "fetch",
    ];

    /// Identificateurs suivis de « ( » dans le code d'un attribut on*.
    /// `obj.m(` (appel de méthode) et les mots-clés sont exclus — seuls les
    /// handlers globaux de la page comptent.
    fn calls_in(code: &str, out: &mut Vec<String>) {
        let b = code.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i].is_ascii_alphabetic() || b[i] == b'_' || b[i] == b'$' {
                let s = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_' || b[i] == b'$')
                {
                    i += 1;
                }
                let name = &code[s..i];
                let mut j = i;
                while j < b.len() && (b[j] == b' ' || b[j] == b'\t') {
                    j += 1;
                }
                let prev_dot = s > 0 && b[s - 1] == b'.';
                if j < b.len() && b[j] == b'(' && !prev_dot && !JS_NON_HANDLERS.contains(&name) {
                    out.push(name.to_string());
                }
            } else {
                i += 1;
            }
        }
    }

    fn handlers_referenced() -> Vec<String> {
        let page = ui_all();
        let mut names = Vec::new();
        for attr in ON_ATTRS {
            let mut rest: &str = &page;
            while let Some(i) = rest.find(attr) {
                let start = i + attr.len();
                let end = rest[start..].find('"').map(|e| start + e).unwrap_or(start);
                calls_in(&rest[start..end], &mut names);
                rest = &rest[end..];
            }
        }
        names.sort();
        names.dedup();
        names
    }

    fn defined_in_page(name: &str) -> bool {
        let page = ui_all();
        page.contains(&format!("function {name}("))
            || page.contains(&format!("const {name} ="))
            || page.contains(&format!("let {name} ="))
    }

    #[test]
    fn chaque_gestionnaire_inline_est_defini() {
        let names = handlers_referenced();
        assert!(
            names.len() > 40,
            "l'inventaire doit rester consistant (une refonte qui casse le \
             parseur doit être vue), trouvé : {names:?}"
        );
        let morts: Vec<&String> = names.iter().filter(|n| !defined_in_page(n)).collect();
        assert!(
            morts.is_empty(),
            "handlers référencés mais jamais définis (boutons morts) : {morts:?}"
        );
    }

    // ---------- contrat actions : l'UI n'appelle que des actions connues ----

    /// Actions d'API écrites en clair (`action: 'x'`) dans la page.
    fn actions_referenced() -> Vec<String> {
        let page = ui_all();
        let mut out = Vec::new();
        for needle in ["action: '", "'action': '"] {
            let mut rest: &str = &page;
            while let Some(i) = rest.find(needle) {
                let start = i + needle.len();
                let end = rest[start..]
                    .find(['\'', '"'])
                    .map(|e| start + e)
                    .unwrap_or(start);
                let name = &rest[start..end];
                if !name.is_empty() && name.chars().all(|c| c.is_ascii_lowercase() || c == '_') {
                    out.push(name.to_string());
                }
                rest = &rest[end..];
            }
        }
        out.sort();
        out.dedup();
        out
    }

    #[test]
    fn chaque_action_api_de_l_ui_est_conue_par_le_moteur() {
        let actions = actions_referenced();
        assert!(
            actions.len() >= 20,
            "l'inventaire doit rester consistant, trouvé : {actions:?}"
        );
        let inconnues: Vec<&String> = actions
            .iter()
            .filter(|a| !crate::manage::ACTIONS.contains(&a.as_str()))
            .collect();
        assert!(
            inconnues.is_empty(),
            "actions UI inconnues du dispatcher : {inconnues:?}"
        );
        // Les actions construites par variable — act(`${up?'kill':'start'}`) —
        // ne sont pas capturées ci-dessus : les littéraux de base sont couverts
        // ici, et chaque action de la surface l'est par la suite E2E.
        for a in ["start", "kill", "refresh", "toggle", "remove"] {
            assert!(
                crate::manage::ACTIONS.contains(&a),
                "action de base manquante au dispatcher : {a}"
            );
        }
    }

    #[test]
    fn les_valeurs_de_reset_state_sont_conues_par_le_moteur() {
        // les <option value> du select de réinitialisation pilotent reset_state
        for v in ["placement", "detection", "oublis", "tout"] {
            assert!(
                INDEX_HTML.contains(&format!("<option value=\"{v}\"")),
                "option reset_state manquante : {v}"
            );
        }
    }

    // ---------- contrat « assets servis » ----------------------------------

    #[test]
    fn la_coquille_reference_les_assets_versionnes() {
        for (url, cite) in [
            ("/app.css", APP_CSS),
            ("/app-boot.js", APP_BOOT_JS),
            ("/app.js", APP_JS),
        ] {
            assert!(
                INDEX_HTML.contains(&format!("{}?v=__MCC_VERSION__", url)),
                "asset non référencé (ou non versionné) dans la coquille : {url}"
            );
            assert!(!cite.trim().is_empty(), "asset vide : {url}");
        }
        assert!(
            APP_CSS.len() > 10_000 && APP_JS.len() > 50_000,
            "assets principaux anormalement petits : css={}, js={}",
            APP_CSS.len(),
            APP_JS.len()
        );
        // le placeholder de version est bien substitué côté serveur (lib.rs)
        assert!(APP_JS.contains("const MCC_VERSION = '__MCC_VERSION__'"));
        // l'amorçage thème doit rester MINUSCULE : il est chargé bloquant
        // dans <head>, chaque octet retarde le premier rendu
        assert!(
            APP_BOOT_JS.len() < 400,
            "app-boot.js gonfle le chemin critique : {} octets",
            APP_BOOT_JS.len()
        );
    }
}
