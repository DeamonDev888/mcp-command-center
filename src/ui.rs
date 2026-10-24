pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>MCP Command Center</title>
<style>
/* ===== Thème Hermes Desktop (skin "nous", mode sombre) =====
   Chrome #0d0d0e · Sidebar #0a0a0b · Card #161618 · Accent Nous #4a84fe
   Textes 94/74/54/36% · hover accent 4% · actif accent 8% · bordure 12% */
:root{
  --chrome:#0d0d0e;--sidebar:#0a0a0b;--card:#161618;--elevated:#1c1c1f;
  --accent:#4a84fe;
  --t1:rgba(255,255,255,.94);--t2:rgba(255,255,255,.74);--t3:rgba(255,255,255,.54);--t4:rgba(255,255,255,.36);
  --border:rgba(255,255,255,.12);--border-soft:rgba(255,255,255,.07);
  --hover:rgba(74,132,254,.04);--active:rgba(74,132,254,.08);--active2:rgba(74,132,254,.14);
  --ok:#55a583;--bad:#e75e78;--warn:#c08532;--orange:#db704b;--cyan:#6f9ba6;--purple:#9e94d5;
  --mono:'JetBrains Mono',ui-monospace,'Cascadia Code',Consolas,monospace;
  --sans:-apple-system,'Segoe UI',system-ui,sans-serif;
}
*{box-sizing:border-box;margin:0;padding:0}
html,body{height:100%}
body{background:var(--chrome);color:var(--t1);font:13px/1.5 var(--sans);display:flex;flex-direction:column;overflow:hidden}

/* ===== Titlebar ===== */
#titlebar{height:42px;flex-shrink:0;display:flex;align-items:center;gap:10px;padding:0 12px 0 16px;
  background:var(--sidebar);border-bottom:1px solid var(--border);user-select:none}
#titlebar .logo{font-weight:600;font-size:13px;letter-spacing:.2px;white-space:nowrap}
#titlebar .logo em{color:var(--accent);font-style:normal}
.tb-badge{font:11px var(--mono);color:var(--t3);background:var(--hover);border:1px solid var(--border-soft);
  border-radius:6px;padding:2px 8px;white-space:nowrap}
.tb-badge b{color:var(--t1);font-weight:600}
.tb-badge.ok b{color:var(--ok)}
#stAuto{padding:3px 9px;font-size:11px;border-radius:6px}
#stAuto.auto-off{color:var(--t4);border-style:dashed}
.tb-badge.upd{color:var(--warn);border-color:rgba(192,133,50,.5);background:rgba(192,133,50,.1);cursor:pointer}
.tb-badge.upd:hover{border-color:var(--warn);color:var(--warn)}
.tb-spacer{flex:1}
.tb-actions{display:flex;gap:6px}
/* Contrôles de fenêtre (mode desktop Tauri, fenêtre sans décorations) */
#winCtrls{display:none;gap:2px;margin-left:8px}
.wbtn{width:34px;height:28px;padding:0;display:flex;align-items:center;justify-content:center;
  border:none;border-radius:6px;background:transparent;color:var(--t3);font-size:12px;cursor:pointer}
.wbtn:hover{background:var(--hover);color:var(--t1)}
.wbtn.wclose:hover{background:rgba(231,94,120,.16);color:var(--bad)}

/* ===== Boutons (style Hermes : discret, accent au survol) ===== */
button{background:transparent;color:var(--t2);border:1px solid var(--border);border-radius:7px;
  padding:4px 11px;cursor:pointer;font:12px var(--sans);transition:background .12s,border-color .12s,color .12s;white-space:nowrap}
button:hover{background:var(--hover);border-color:var(--accent);color:var(--accent)}
button.primary{background:var(--active);border-color:var(--active2);color:var(--accent);font-weight:500}
button.primary:hover{background:var(--active2)}
button.danger:hover{border-color:var(--bad);color:var(--bad);background:rgba(231,94,120,.06)}
button:disabled{opacity:.35;cursor:not-allowed}

/* ===== Layout principal : explorateur + éditeur ===== */
#main{flex:1;display:flex;min-height:0}

/* --- Explorateur (gauche) --- */
#explorer{width:280px;flex-shrink:0;background:var(--sidebar);border-right:1px solid var(--border);
  display:flex;flex-direction:column;overflow-y:auto;overflow-x:hidden}
.sec-label{font:10px var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.9px;
  color:var(--t4);padding:14px 16px 6px;user-select:none;display:flex;align-items:center;justify-content:space-between;gap:6px}
.sec-label select{width:auto;flex-shrink:0;background:var(--chrome);border:1px solid var(--border);border-radius:6px;
  color:var(--t3);font:10px var(--mono);padding:2px 3px;cursor:pointer;text-transform:none;letter-spacing:0}
.sec-label select:hover{color:var(--accent);border-color:var(--accent)}
.sec-label select:focus{outline:none}
.srow{padding:7px 10px 7px 14px;margin:1px 8px;border-radius:8px;cursor:pointer;position:relative;
  border:1px solid transparent;transition:background .1s}
.srow:hover{background:var(--hover)}
.srow.sel{background:var(--active);border-color:var(--active2)}
.srow.sel::before{content:'';position:absolute;left:0;top:7px;bottom:7px;width:2px;border-radius:2px;background:var(--accent)}
.srow.off{opacity:.45}
.srow .l1{display:flex;align-items:center;gap:8px;min-width:0}
.srow .l1 .nm{flex:1;font-size:12.5px;font-weight:500;color:var(--t1);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.srow .l1 .cnt{font:10px var(--mono);color:var(--t4)}
.legacy{font:9px var(--mono);text-transform:uppercase;letter-spacing:.4px;color:var(--t4);
  border:1px dashed var(--border);border-radius:4px;padding:0 5px;white-space:nowrap;flex-shrink:0}
.rowbtn{background:none;border:1px solid var(--border);border-radius:5px;color:var(--t3);
  padding:0 6px;font-size:9px;line-height:15px;min-width:20px;flex-shrink:0}
.rowbtn:hover{color:var(--accent);border-color:var(--accent);background:var(--hover)}
.srow.off .rowbtn{opacity:.5}
/* Drag & drop de réordonnancement (préférence / priorité de sondage) */
.srow.dragging{opacity:.35}
.srow.drop-above{box-shadow:0 -2px 0 var(--accent)}
.srow.drop-below{box-shadow:0 2px 0 var(--accent)}
.sw{min-width:34px;text-align:center;font:9px var(--mono);font-weight:600;border-radius:9px;padding:1px 6px;flex-shrink:0}
.sw.on{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08)}
.sw.on:hover{border-color:var(--bad);color:var(--bad);background:rgba(231,94,120,.06)}
.sw.off{color:var(--t4);border-color:var(--border);text-decoration:line-through}
.sw.off:hover{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08);text-decoration:none}
.srow .l2{display:flex;align-items:center;gap:6px;margin-top:2px;padding-left:18px;
  font:10.5px var(--mono);color:var(--t3);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.srow .l2 .st{font-weight:600}
.srow .l2 .st.UP{color:var(--ok)}.srow .l2 .st.PORT-ONLY{color:var(--orange)}
.srow .l2 .st.DOWN{color:var(--t4)}.srow .l2 .st.DISABLED{color:var(--t4)}
.srow .l2 .st.PROBING,.srow .l2 .st.STARTING{color:var(--accent)}
.dot{width:8px;height:8px;border-radius:50%;flex-shrink:0}
.dot.UP{background:var(--ok);box-shadow:0 0 6px var(--ok)}
.dot.PORT-ONLY{background:var(--orange);box-shadow:0 0 6px var(--orange)}
.dot.DOWN{background:#3d3d40}
.dot.DISABLED{background:transparent;border:1px solid var(--border)}
.dot.PROBING,.dot.STARTING{background:var(--accent);animation:pulse 1s infinite}
@keyframes pulse{50%{opacity:.25}}
.peer-row{padding:6px 10px 6px 14px;margin:1px 8px;border-radius:8px}
.peer-row .l1{display:flex;align-items:center;gap:8px}
.peer-row .l1 .nm{flex:1;font:12px var(--mono);color:var(--t1);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.peer-row .l2{font:10.5px var(--mono);color:var(--t3);padding-left:20px;margin-top:1px}
.peer-row button{margin-top:5px;margin-left:20px;padding:2px 9px;font-size:11px}
#addWrap{margin-top:auto;padding:10px 8px 12px;border-top:1px solid var(--border-soft)}
#addWrap button{width:100%;display:flex;gap:6px;align-items:center;justify-content:center;padding:7px}
.empty-sec{font:11px var(--sans);color:var(--t4);padding:4px 16px 10px;font-style:italic}

/* --- Zone détail (droite) --- */
#content{flex:1;min-width:0;overflow-y:auto;padding:22px 26px}
.crumb{font:11px var(--mono);color:var(--t4);margin-bottom:10px}
.crumb b{color:var(--t3);font-weight:400}
.d-head{display:flex;align-items:center;gap:12px;margin-bottom:4px;flex-wrap:wrap}
.d-head h1{font-size:19px;font-weight:600;letter-spacing:.2px}
.d-head .sub{font:12px var(--mono);color:var(--t3)}
.d-actions{display:flex;gap:7px;margin-left:auto;flex-wrap:wrap}
.pill{font:10.5px var(--mono);font-weight:600;padding:3px 10px;border-radius:20px;border:1px solid}
.pill.UP{color:var(--ok);border-color:rgba(85,165,131,.4);background:rgba(85,165,131,.08)}
.pill.PORT-ONLY{color:var(--orange);border-color:rgba(219,112,75,.4);background:rgba(219,112,75,.08)}
.pill.DOWN{color:var(--t3);border-color:var(--border);background:var(--hover)}
.pill.DISABLED{color:var(--t4);border-color:var(--border);background:transparent;border-style:dashed}
.pill.PROBING,.pill.STARTING{color:var(--accent);border-color:var(--active2);background:var(--active)}
.d-err{color:var(--bad);font:11.5px var(--mono);background:rgba(231,94,120,.06);border:1px solid rgba(231,94,120,.25);
  border-radius:8px;padding:8px 12px;margin:12px 0 0;word-break:break-word}
.seed-note{font-size:11.5px;color:var(--t3);background:var(--hover);border:1px solid var(--border-soft);
  border-radius:8px;padding:7px 11px;margin:12px 0 0}
.seed-note b{color:var(--t2)}
/* Onglets du détail + blocs de conf agent */
.tabs{display:flex;gap:4px;margin:20px 0 0;border-bottom:1px solid var(--border-soft)}
.tab{border:1px solid transparent;background:none;border-radius:7px 7px 0 0;padding:5px 12px;color:var(--t3);font:12px var(--sans)}
.tab:hover{color:var(--accent);background:var(--hover)}
.tab.sel{color:var(--accent);background:var(--active2);font-weight:600;border-bottom:2px solid var(--accent)}
.conf-note{font-size:11.5px;color:var(--t4);margin:0 0 12px}
.conf-note b{color:var(--t3)}
.conf-block{background:var(--sidebar);border:1px solid var(--border-soft);border-radius:10px;margin-bottom:10px;overflow:hidden}
.conf-head{display:flex;align-items:center;gap:8px;padding:7px 12px;font:11px var(--sans);color:var(--t3);border-bottom:1px solid var(--border-soft)}
.conf-head button{margin-left:auto;padding:2px 10px;font-size:11px}
.conf-block pre{margin:0;padding:12px 14px;font:11.5px var(--mono);color:var(--t2);overflow-x:auto;white-space:pre}
/* Barre latérale ajustable */
#grip{width:5px;flex-shrink:0;cursor:col-resize;background:transparent;transition:background .12s}
#grip:hover,#grip.drag{background:var(--active2)}
h4.sec{font:10px var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.9px;color:var(--t4);
  margin:24px 0 10px;display:flex;align-items:center;gap:10px}
h4.sec::after{content:'';flex:1;height:1px;background:var(--border-soft)}
/* Replis du détail (Informations / Démarrage) — même style que h4.sec */
details.fold{margin:24px 0 0}
details.fold summary{list-style:none;cursor:pointer;user-select:none;
  font:10px var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.9px;color:var(--t4);
  display:flex;align-items:center;gap:10px;transition:color .12s}
details.fold summary::-webkit-details-marker{display:none}
details.fold summary::before{content:'\25B8';font-size:9px;flex-shrink:0;transition:color .12s}
details.fold[open] summary::before{content:'\25BE'}
details.fold summary::after{content:'';flex:1;height:1px;background:var(--border-soft)}
details.fold summary:hover{color:var(--accent)}
details.fold .kv{margin-top:12px}
.kv{display:grid;grid-template-columns:170px 1fr;gap:7px 14px;font-size:12.5px}
.kv .k{color:var(--t4)}
.kv .v{font:12px var(--mono);color:var(--t2);word-break:break-all}

/* Tools (inspiration Antigravity : lignes nettes, toggle à droite) */
.tool{display:flex;align-items:center;gap:12px;background:var(--card);border:1px solid var(--border-soft);
  border-radius:9px;padding:9px 12px;margin-bottom:6px;transition:border-color .1s,background .1s}
.tool:hover{border-color:var(--border);background:var(--elevated)}
.tool.off{opacity:.55}
.tool code{color:var(--accent);font:12px var(--mono);flex-shrink:0}
.tool .desc{flex:1;font-size:12px;color:var(--t3);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.tgl{min-width:52px;text-align:center;font:10.5px var(--mono);font-weight:600;border-radius:20px;padding:3px 12px}
.tgl.on{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08)}
.tgl.on:hover{border-color:var(--bad);color:var(--bad);background:rgba(231,94,120,.06)}
.tgl.off{color:var(--t4);border-color:var(--border);text-decoration:line-through}
.tgl.off:hover{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08);text-decoration:none}

/* Vue d'ensemble */
.welcome{max-width:560px;margin:60px auto 0;text-align:center}
.welcome .big{font-size:15px;color:var(--t2);margin-bottom:6px}
.welcome .hint{font-size:12.5px;color:var(--t4);margin-bottom:22px}
.welcome .qa{display:flex;gap:8px;justify-content:center;flex-wrap:wrap}
.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:10px;margin-bottom:6px}
.tile{background:var(--card);border:1px solid var(--border-soft);border-radius:10px;padding:12px 14px}
.tile .n{font:22px var(--mono);font-weight:600;color:var(--t1)}
.tile .n.ok{color:var(--ok)}
.tile .lbl{font-size:11px;color:var(--t4);margin-top:2px}

/* --- Journal (bas, style console IDE) --- */
#logpanel{flex-shrink:0;height:170px;background:var(--sidebar);border-top:1px solid var(--border);
  display:flex;flex-direction:column}
#logpanel.min{height:32px}
#loghead{height:32px;flex-shrink:0;display:flex;align-items:center;gap:8px;padding:0 14px;cursor:pointer;user-select:none}
#loghead .lbl{font:10px var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.9px;color:var(--t4)}
#loghead .arrow{color:var(--t4);font-size:10px;transition:transform .15s}
#logpanel.min #loghead .arrow{transform:rotate(-90deg)}
#logbody{flex:1;overflow-y:auto;padding:2px 14px 10px;font:11px var(--mono)}
#logpanel.min #logbody{display:none}
.logline{padding:1px 0;color:var(--t3);white-space:pre-wrap}
.logline .t{color:var(--t4)}
.logline.warn{color:var(--warn)}
.logline.error{color:var(--bad)}

/* --- Boutique MCP (porté de l'app Hermes : directory à cartes + import) --- */
.tag{font:10px var(--mono);padding:1px 7px;border-radius:3px;border:1px solid var(--border);color:var(--t3)}
.tag.npm{color:#bc8cff;border-color:#bc8cff44}
.tag.py{color:#39c5cf;border-color:#39c5cf44}
.tag.stdio{color:#39c5cf;border-color:#39c5cf44}
.tag.http{color:#bc8cff;border-color:#bc8cff44}
.tag.sse{color:var(--ok);border-color:rgba(85,165,131,.45)}
.tag.https{color:#d2a8ff;border-color:#d2a8ff44;background:#d2a8ff11;box-shadow:0 0 8px rgba(210,168,255,.35)}
/* Framework détecté par fingerprint (litemcp) : badge violet « glow HTTPS » — halo pulsant */
.tag.litemcp{color:#e9d5ff;border-color:#d2a8ff88;background:#d2a8ff1f;box-shadow:0 0 10px rgba(210,168,255,.55);animation:glowPulse 2.4s ease-in-out infinite}
@keyframes glowPulse{50%{box-shadow:0 0 18px rgba(210,168,255,.9)}}
.srow .l1 .tag{font-size:8.5px;padding:0 5px;line-height:14px;flex-shrink:0}
.langchip{font:8.5px var(--mono);font-weight:700;border:1px solid;border-radius:4px;padding:0 4px;line-height:14px;flex-shrink:0}
.langchip.unk{color:var(--t4);border-color:var(--border-soft);background:transparent;font-weight:400}
#storeDlg{width:min(1180px,96vw)}
.store-bar{display:flex;gap:10px;align-items:center;margin-bottom:12px;flex-wrap:wrap}
.store-count{color:var(--t3);font-weight:400;font-size:12.5px;margin-left:2px}
/* Machines distantes (dialog SSH) */
.machine-form{display:flex;gap:8px;flex-wrap:wrap;margin-bottom:10px}
.machine-form input{flex:1;min-width:170px}
.machine-msg{font:11px var(--mono);color:var(--t2);min-height:16px;margin-bottom:8px}
.machine-row{background:var(--sidebar);border:1px solid var(--border-soft);border-radius:9px;padding:8px 10px;margin-bottom:8px}
.machine-row .l1{display:flex;align-items:center;gap:8px}
.machine-row .nm{font:12.5px var(--mono);color:var(--t1)}
.machine-row .l2{font:10.5px var(--mono);color:var(--t3);margin:3px 0 6px}
.machine-row .macts{display:flex;gap:6px;flex-wrap:wrap}
.store-bar input{flex:1;min-width:180px}
.store-cats{display:flex;gap:6px;flex-wrap:wrap}
.scat{padding:3px 12px;border-radius:14px;font-size:11px}
.scat.sel{background:var(--active2);border-color:var(--accent);color:var(--accent)}
.store-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(184px,1fr));gap:7px}
.scard{background:var(--sidebar);border:1px solid var(--border-soft);border-radius:9px;padding:8px 10px;display:flex;flex-direction:column;gap:4px}
.scard:hover{border-color:var(--border)}
.scard-top{display:flex;align-items:center;gap:6px}
.scard-top b{flex:1;font-size:12px}
.slogo{width:18px;height:18px;flex-shrink:0}
.scard-desc{font-size:10.5px;color:var(--t3);flex:1;display:-webkit-box;-webkit-line-clamp:2;-webkit-box-orient:vertical;overflow:hidden}
.scard-cmd{font:9.5px var(--mono);color:var(--t4);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.scard button{align-self:flex-start;padding:2px 9px;font-size:11px}

/* --- Formulaire d'ajout (modal) --- */
dialog{background:var(--card);color:var(--t1);border:1px solid var(--border);border-radius:14px;padding:0;
  width:min(680px,92vw);max-height:85vh;box-shadow:0 18px 50px rgba(0,0,0,.5)}
dialog::backdrop{background:rgba(0,0,0,.6)}
.dlg-head{display:flex;align-items:center;gap:10px;padding:15px 20px;border-bottom:1px solid var(--border-soft)}
.dlg-head h2{font-size:15px;font-weight:600;flex:1}
.dlg-head .close{background:none;border:none;font-size:18px;color:var(--t4);cursor:pointer;padding:2px 6px}
.dlg-head .close:hover{color:var(--t1);background:transparent;border:none}
.dlg-body{padding:16px 20px 20px}
form{display:grid;gap:12px}
form label{font-size:11px;color:var(--t4);display:block;margin-bottom:4px}
input,select{width:100%;background:var(--chrome);border:1px solid var(--border);border-radius:8px;
  padding:7px 10px;color:var(--t1);font:12.5px var(--mono)}
input:focus,select:focus{outline:none;border-color:var(--accent);box-shadow:0 0 0 2px var(--active)}
.row{display:flex;gap:10px}
.row>div{flex:1}
</style>
</head>
<body>

<div id="titlebar" data-tauri-drag-region>
  <span class="logo" data-tauri-drag-region><em>&#9679;</em> MCP <em>Command Center</em></span>
  <span class="tb-badge" title="Version installée">v__MCC_VERSION__</span>
  <button id="stAuto" onclick="toggleAutostart()" title="Lancer l'application au démarrage de l'OS (tray silencieux)"></button>
  <span class="tb-badge upd" id="stUpdate" style="display:none"></span>
  <span class="tb-badge ok">UP <b id="stUp">0</b>/<b id="stTotal">0</b></span>
  <span class="tb-badge">tools <b id="stTools">0</b></span>
  <span class="tb-badge">refresh <b id="stRefresh">15</b>s</span>
  <span class="tb-spacer" data-tauri-drag-region></span>
  <div class="tb-actions">
    <button class="primary" onclick="doScan()">&#128269; Scan ports</button>
    <button onclick="doScanNet()">&#127760; Scan r&eacute;seau</button>
    <button onclick="openMachineDlg()">🖥️ Machines</button>
    <button onclick="openStore()">&#128722; Boutique</button>
    <button onclick="refreshAll()" title="Re-sonder tous les serveurs connus et d&eacute;couvrir les nouveaux (tous ports en &eacute;coute)">&#8635;</button>
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
      </select>
    </div>
    <div id="srvList"></div>
    <div class="sec-label">Machines &amp; pairs</div>
    <div id="peerList"></div>
    <div id="addWrap"><button onclick="toggleAdd()">+ Ajouter un serveur</button></div>
  </aside>
  <div id="grip" title="Glisser pour ajuster la largeur"></div>
  <section id="content"></section>
</div>

<div id="logpanel">
  <div id="loghead" onclick="document.getElementById('logpanel').classList.toggle('min')">
    <span class="arrow">&#9662;</span><span class="lbl">Journal</span>
    <span class="tb-badge" style="margin-left:auto">auto-d&eacute;tection : HTTP streamable &middot; stdio JSON-RPC</span>
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
        <div style="margin-top:12px">
          <label>Commande de lancement (option — permet Start/Stop par le command center)</label>
          <div class="row" style="margin-bottom:8px">
            <div><input id="fHttpProgram" placeholder="ex. node ou uvx mcp-server-x"></div>
            <div><input id="fHttpWorkdir" placeholder="workdir (option)"></div>
          </div>
          <input id="fHttpArgs" placeholder="args (ex. dist/server.js --port 3099)">
        </div>
      </div>
      <div id="stdioFields" style="display:none">
        <div class="row">
          <div><label>Programme</label><input id="fProgram" placeholder="node"></div>
          <div><label>Workdir (option)</label><input id="fWorkdir" placeholder="C:\chemin\serveur"></div>
        </div>
        <div style="margin-top:12px"><label>Args (s&eacute;par&eacute;s par espace)</label><input id="fArgs" placeholder="dist/index.js --port 3000"></div>
      </div>
      <div style="display:flex;gap:8px;justify-content:flex-end;margin-top:6px">
        <button type="button" onclick="dlg.close()">Annuler</button>
        <button type="submit" class="primary">Ajouter &amp; sonder</button>
      </div>
    </form>
  </div>
</dialog>

<dialog id="storeDlg">
  <div class="dlg-head">
    <h2>&#128722; Boutique &mdash; serveurs MCP <span id="storeCount" class="store-count"></span></h2>
    <button class="close" onclick="storeDlg.close()">&times;</button>
  </div>
  <div class="dlg-body" style="padding-top:12px">
    <div class="store-bar">
      <input id="storeSearch" placeholder="Rechercher dans la boutique&hellip;" oninput="renderStoreItems()">
      <div id="storeCats" class="store-cats"></div>
    </div>
    <div id="storeGrid" class="store-grid"></div>
  </div>
</dialog>

<dialog id="machineDlg">
  <div class="dlg-head">
    <h2>🖥️ Machines distantes</h2>
    <button class="close" onclick="machineDlg.close()">&times;</button>
  </div>
  <div class="dlg-body">
    <div class="conf-note">Connexion SSH par cl&eacute; (BatchMode — aucun mot de passe possible) : la machine
    doit accepter ta cl&eacute; publique. L'app liste ses ports en &eacute;coute via <b>netstat</b> distant,
    y sonde les serveurs MCP et les importe ici automatiquement.</div>
    <div class="machine-form">
      <input id="mIp" placeholder="IP ou h&ocirc;te (ex. 192.168.1.42)">
      <input id="mUser" placeholder="Utilisateur SSH">
      <button class="primary" onclick="sshDiscover()">🔗 SSH &amp; d&eacute;couvrir</button>
      <button onclick="sshSave()">💾 Enregistrer</button>
    </div>
    <div id="machineMsg" class="machine-msg"></div>
    <div id="machineList"></div>
    <div style="margin-top:12px"><button onclick="doPeers();setTimeout(renderMachines,3000)">🔄 D&eacute;tecter les command centers sur le LAN</button></div>
  </div>
</dialog>

<dialog id="loginDlg">
  <div class="dlg-head">
    <h2>&#128274; Acc&egrave;s prot&eacute;g&eacute;</h2>
  </div>
  <div class="dlg-body">
    <div class="conf-note">Cette UI est expos&eacute;e au r&eacute;seau : saisis le <b>code d'acc&egrave;s API</b>
    du serveur (<b>settings.api_access</b> de son config.json, ou variable <b>MCC_API_ACCESS</b>).
    Il reste dans ce navigateur uniquement — le serveur ne l'envoie jamais dans la page.</div>
    <form onsubmit="return loginSubmit(event)">
      <label>Code d'acc&egrave;s</label>
      <input id="loginCode" type="password" autocomplete="off" placeholder="code d'acc&egrave;s">
      <div id="loginMsg" class="machine-msg"></div>
      <div style="display:flex;justify-content:flex-end;margin-top:6px">
        <button type="submit" class="primary">D&eacute;verrouiller</button>
      </div>
    </form>
  </div>
</dialog>

<script>
/* Code d'accès API : demandé à l'utilisateur (dialogue de déverrouillage) puis
   gardé en localStorage — JAMAIS injecté par le serveur dans la page (audit
   2026-10-24 : quiconque joint le port lisait le code sur / et contournait
   toute l'auth). En bind loopback aucun code n'est requis : le dialogue
   n'apparaît jamais. Chaque requête /api/ présente le code (X-MCC-Access) ;
   un 401 rouvre le dialogue (code tourné ou faux). */
const accessCode = () => { try { return localStorage.getItem('mcc.access') || ''; } catch (e) { return ''; } };
const MCC_VERSION = '__MCC_VERSION__';
const _mccFetch = window.fetch.bind(window);
window.fetch = async (url, opts = {}) => {
  const isApi = typeof url === 'string' && url.startsWith('/api/');
  const code = accessCode();
  if (isApi && code) {
    opts.headers = Object.assign({}, opts.headers || {}, {'X-MCC-Access': code});
  }
  const r = await _mccFetch(url, opts);
  if (isApi && r.status === 401) showLogin();
  return r;
};
function showLogin() {
  const d = document.getElementById('loginDlg');
  if (d && !d.open) { d.showModal(); document.getElementById('loginCode').focus(); }
}
async function loginSubmit(ev) {
  ev.preventDefault();
  const code = document.getElementById('loginCode').value.trim();
  const msg = document.getElementById('loginMsg');
  // fetch brut (pas l'intercepteur) : pas de boucle 401 pendant la vérif
  const r = await _mccFetch('/api/state', {headers: code ? {'X-MCC-Access': code} : {}});
  if (r.ok) {
    try { localStorage.setItem('mcc.access', code); } catch (e) {}
    msg.textContent = '';
    document.getElementById('loginCode').value = '';
    document.getElementById('loginDlg').close();
    load();
  } else {
    msg.textContent = '\u2717 code refus\u00e9 — voir settings.api_access du config.json du serveur';
  }
  return false;
}

let state = null;
let selectedId = null;
let detailCache = null;
let detailTab = 'outils';
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const js = s => esc(JSON.stringify(String(s ?? '')));

async function load() {
  try {
    const r = await fetch('/api/state');
    state = await r.json();
    if (selectedId && !state.servers.find(s => s.id === selectedId)) { selectedId = null; detailCache = null; }
    render();
    if (selectedId) {
      const r2 = await fetch('/api/detail?id=' + encodeURIComponent(selectedId));
      if (r2.ok) { detailCache = await r2.json(); renderDetail(); }
    }
  } catch(e) { /* serveur redémarre */ }
}

function render() {
  if (!state) return;
  const s = state.stats;
  document.getElementById('stUp').textContent = s.up;
  document.getElementById('stTotal').textContent = s.total;
  document.getElementById('stTools').textContent = s.tools;
  document.getElementById('stRefresh').textContent = state.settings.refresh_secs;
  const au = document.getElementById('stAuto');
  if (au) {
    au.textContent = (state.autostart ? '⏭ Auto-boot ON' : '⏭ Auto-boot OFF');
    au.classList.toggle('auto-off', !state.autostart);
  }
  const upd = document.getElementById('stUpdate');
  if (state.update_available && state.latest_version) {
    upd.style.display = '';
    upd.textContent = '⬆ v' + state.latest_version + ' dispo';
    upd.title = 'Nouvelle version ' + state.latest_version + ' publiée sur GitHub (installée : v' + MCC_VERSION + ') — github.com/DeamonDev888/mcp-command-center/releases';
    upd.onclick = () => window.open('https://github.com/DeamonDev888/mcp-command-center/releases/latest', '_blank');
  } else upd.style.display = 'none';
  renderServers();
  renderPeers();
  renderLog();
  if (!selectedId) renderOverview();
}

/* ===== Explorateur ===== */
function renderServers() {
  const box = document.getElementById('srvList');
  if (!state.servers.length) { box.innerHTML = '<div class="empty-sec">aucun serveur — lance un scan</div>'; return; }
  box.innerHTML = state.servers.map(s => {
    const ti = s.transport === 'http'
      ? (s.transport_info.host ? `${esc(s.transport_info.host)}:` : '') + s.transport_info.port
      : 'stdio &middot; ' + esc(s.transport_info.program);
    const off = s.tools_disabled.length ? ` &middot; ${s.tools_disabled.length} off` : '';
    return `<div class="srow ${selectedId===s.id?'sel':''} ${s.enabled?'':'off'}" draggable="true" data-id="${esc(s.id)}"
      onclick="select(${js(s.id)})"
      ondragstart="rowDragStart(event,${js(s.id)})" ondragover="rowDragOver(event)"
      ondragleave="rowDragLeave(event)" ondrop="rowDrop(event)" ondragend="rowDragEnd(event)">
      <div class="l1"><span class="dot ${s.state}"></span><span class="nm" title="${esc(s.label)}">${esc(s.label)}</span>${langChip(s)}${typeBadge(s)}${frameworkBadge(s)}${s.transport==='stdio'?'<span class="legacy" title="Legacy : serveur stdio uniquement (pas de transport HTTP)">d&eacute;suet</span>':''}<span class="cnt">${s.tools_count}</span>${rowToggle(s)}${rowCtl(s)}</div>
      <div class="l2"><span class="st ${esc(s.state)}">${esc(s.state)}</span> ${ti}${off}</div>
    </div>`;
  }).join('');
}

// Chip du langage MCP (inféré : commande stdio ou en-têtes HTTP) — monogramme coloré façon logo
const LANG_META = {
  node:   { t: 'JS',  c: '#f7df1e', n: 'Node.js / TypeScript' },
  python: { t: 'PY',  c: '#4b8bbe', n: 'Python' },
  rust:   { t: 'RS',  c: '#dea584', n: 'Rust' },
  go:     { t: 'GO',  c: '#00add8', n: 'Go' },
  java:   { t: 'JV',  c: '#e76f00', n: 'Java' },
  dotnet: { t: 'C#',  c: '#512bd4', n: '.NET' },
  ruby:   { t: 'RB',  c: '#cc342d', n: 'Ruby' },
  php:    { t: 'PHP', c: '#777bb4', n: 'PHP' },
};
function langChip(s) {
  const m = LANG_META[s.lang];
  if (!m) return '<span class="langchip unk" title="Langage non d\u00e9tect\u00e9 (ni commande ni en-t\u00eates ni process)">?</span>';
  return `<span class="langchip" style="color:${m.c};border-color:${m.c}66;background:${m.c}14" title="Langage inféré : ${m.n}">${m.t}</span>`;
}

// Badge du type de MCP : STDIO / HTTPS / HTTP+SSE (streamable + endpoint legacy
// co-servi, typique FastMCP) / SSE (réponses event-stream sans POST prouvé) /
// HTTP (streamable), + ·LAN si distant. Le POST initialize répondu = streamable
// HTTP quel que soit l'encodage des réponses (le probe n'atteint le badge que là).
function typeBadge(s) {
  if (s.transport === 'stdio') return '<span class="tag stdio" title="Transport stdio (process local)">STDIO</span>';
  const lan = s.transport_info.host ? '\u00b7LAN' : '';
  const via = s.via_ssh ? '\u00b7SSH' : '';
  const viaTip = s.via_ssh ? ' — sondé via SSH depuis la machine (interne)' : '';
  const suffix = (lan ? ' — serveur distant (LAN)' : '') + viaTip;
  if (s.transport_info.port === 443) return `<span class="tag https" title="HTTPS (TLS, port 443)${suffix}">HTTPS${lan}${via}</span>`;
  if (s.sse && s.sse_endpoint) return `<span class="tag sse" title="HTTP streamable + endpoint SSE legacy co-servé : réponses initialize en text/event-stream et GET /sse ouvert (comportement FastMCP/mcp-proxy)${suffix}">HTTP+SSE${lan}${via}</span>`;
  const t = s.sse ? 'SSE' : 'HTTP';
  const cls = t === 'SSE' ? 'sse' : 'http';
  const tip = t === 'SSE' ? 'Réponses initialize en text/event-stream' : 'HTTP streamable (JSON)';
  return `<span class="tag ${cls}" title="${tip}${suffix}">${t}${lan}${via}</span>`;
}

// Badge du framework détecté par fingerprint (seul litemcp aujourd'hui) :
// initialize répondu 2026-07-28 SANS négocier la version demandée + sonde
// GET /health → 200 « ok » — les deux signatures documentées de litemcp.
function frameworkBadge(s) {
  if (s.framework !== 'litemcp') return '';
  const lan = s.transport_info.host ? ' — serveur distant (LAN)' : '';
  return `<span class="tag litemcp" title="Framework LiteMCP : lib Rust MCP stateless, HTTP streamable (spec 2026-07-28) — fingerprint : version non négociée + sonde /health${lan}">LiteMCP</span>`;
}

function rowToggle(s) {
  return `<button class="sw ${s.enabled?'on':'off'}" title="${s.enabled?'D\u00e9sactiver ce serveur (arr\u00eate son process)':'Activer ce serveur (le d\u00e9marre si commande connue)'}" onclick="event.stopPropagation();act('toggle',${js(s.id)})">${s.enabled?'ON':'OFF'}</button>`;
}

function rowCtl(s) {
  if (!s.can_start) return '';
  const up = s.state === 'UP';
  return `<button class="rowbtn" title="${up?'Arr\u00eater':'D\u00e9marrer'} ce serveur" onclick="event.stopPropagation();act('${up?'kill':'start'}',${js(s.id)})">${up?'\u25a0':'\u25b6'}</button>`;
}

/* État des replis du détail (mémorisé entre les cycles et les sessions) */
function saveFold(el) {
  try { localStorage.setItem('mcc.fold.' + el.dataset.fold, el.open); } catch (e) {}
}

async function toggleAutostart() {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'set_autostart', enabled: !state.autostart})});
  await load();
}

/* ===== Drag & drop : réordonner les serveurs (préférence / priorité) ===== */
let dragId = null;
function rowDragStart(ev, id) {
  dragId = id;
  ev.dataTransfer.effectAllowed = 'move';
  try { ev.dataTransfer.setData('text/plain', id); } catch (e) {}
  ev.currentTarget.classList.add('dragging');
}
function rowDragOver(ev) {
  if (!dragId) return;
  ev.preventDefault();
  ev.dataTransfer.dropEffect = 'move';
  const row = ev.currentTarget;
  if (row.dataset.id === dragId) return;
  const rect = row.getBoundingClientRect();
  const above = ev.clientY < rect.top + rect.height / 2;
  row.classList.toggle('drop-above', above);
  row.classList.toggle('drop-below', !above);
  row._dropAbove = above;
}
function rowDragLeave(ev) {
  ev.currentTarget.classList.remove('drop-above', 'drop-below');
}
function rowDrop(ev) {
  ev.preventDefault();
  const row = ev.currentTarget;
  row.classList.remove('drop-above', 'drop-below');
  const moved = dragId;
  dragId = null;
  if (!moved || row.dataset.id === moved) return;
  // le glisser-déposer reprend la main : le menu de tri revient sur « manuel »
  manualOrder = null;
  const sel = document.getElementById('sortSel');
  if (sel) sel.value = 'manuel';
  // nouvel ordre = ids visibles, la ligne déplacée réinsérée au-dessus/en dessous
  const ids = [...document.querySelectorAll('#srvList .srow')]
    .map(r => r.dataset.id).filter(x => x !== moved);
  const at = ids.indexOf(row.dataset.id);
  ids.splice(row._dropAbove ? at : at + 1, 0, moved);
  postReorder(ids);
}

/* ===== Menu de reclassement (⇅) =====
   Réutilise l'action API « reorder » : l'ordre appliqué est persisté
   (settings.server_order) et gouverne l'affichage ET la priorité de sondage.
   « manuel » rend l'ordre posé au glisser-déposer AVANT le tri (mémorisé côté
   client au moment où un tri est appliqué — un drag le réactualise). */
let manualOrder = null;
const SORT_STATE_RANK = {UP: 0, STARTING: 1, 'PORT-ONLY': 1, PROBING: 1, DOWN: 2, DISABLED: 3};
const frCmp = (a, b) => String(a).localeCompare(String(b), 'fr', {sensitivity: 'base'});
function sortByPort(s) {
  return s.transport === 'http' ? (s.transport_info.port || 65536) : 65536;
}
function applySort(mode) {
  if (!state || !state.servers.length) return;
  const byId = Object.fromEntries(state.servers.map(s => [s.id, s]));
  if (mode === 'manuel') {
    if (manualOrder && manualOrder.length) postReorder(manualOrder.filter(id => byId[id]));
    return;
  }
  if (manualOrder === null) manualOrder = state.servers.map(s => s.id);
  let ids = state.servers.map(s => s.id);
  if (mode === 'az') {
    ids.sort((a, b) => frCmp(byId[a].label, byId[b].label));
  } else if (mode === 'etat') {
    ids.sort((a, b) => ((SORT_STATE_RANK[byId[a].state] ?? 9) - (SORT_STATE_RANK[byId[b].state] ?? 9))
      || frCmp(byId[a].label, byId[b].label));
  } else if (mode === 'port') {
    ids.sort((a, b) => (sortByPort(byId[a]) - sortByPort(byId[b])) || frCmp(byId[a].label, byId[b].label));
  } else if (mode === 'latence') {
    // non sondés (null) en fin, puis du plus rapide au plus lent
    ids.sort((a, b) => ((byId[a].latency_ms ?? 1e12) - (byId[b].latency_ms ?? 1e12)));
  } else if (mode === 'tools') {
    ids.sort((a, b) => ((byId[b].tools_count || 0) - (byId[a].tools_count || 0))
      || frCmp(byId[a].label, byId[b].label));
  }
  postReorder(ids);
}
async function postReorder(ids) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'reorder', ids})}).catch(() => {});
  await load();
}
function rowDragEnd(ev) {
  ev.currentTarget.classList.remove('dragging');
  document.querySelectorAll('.srow').forEach(r => r.classList.remove('drop-above', 'drop-below'));
  dragId = null;
}

function renderPeers() {
  const box = document.getElementById('peerList');
  if (!state.peers || !state.peers.length) { box.innerHTML = '<div class="empty-sec">aucun pair d&eacute;tect&eacute;</div>'; return; }
  box.innerHTML = state.peers.map(p => {
    const st = p.stats || {};
    return `<div class="peer-row">
      <div class="l1"><span class="dot UP"></span><span class="nm">${esc(p.ip)}</span></div>
      <div class="l2">UI :${p.ui_port} &mdash; ${st.up ?? '?'} UP / ${st.total ?? '?'} serveurs</div>
      <button onclick="importPeer(${js(p.ip)})">Importer ses serveurs</button>
    </div>`;
  }).join('');
}

function renderLog() {
  const lb = document.getElementById('logbody');
  lb.innerHTML = state.log.map(l => {
    const ts = Number.isFinite(l.ts) ? l.ts : 0;
    const d = new Date(ts * 1000).toLocaleTimeString('fr-FR');
    return `<div class="logline ${esc(l.level)}"><span class="t">${esc(d)}</span>  ${esc(l.msg)}</div>`;
  }).join('');
  lb.scrollTop = lb.scrollHeight;
}

/* ===== Zone de détail ===== */
function renderOverview() {
  const s = state.stats;
  const nbMachines = (state.settings.ssh_hosts || []).length;
  document.getElementById('content').innerHTML = `
    <div class="tiles">
      <div class="tile"><div class="n ok">${s.up}</div><div class="lbl">serveurs UP</div></div>
      <div class="tile"><div class="n">${s.total}</div><div class="lbl">serveurs suivis</div></div>
      <div class="tile"><div class="n">${s.tools}</div><div class="lbl">tools actifs</div></div>
      <div class="tile"><div class="n">${nbMachines}</div><div class="lbl">machines distantes</div></div>
    </div>
    <div class="welcome">
      <div class="big">Centre de contr&ocirc;le de tes serveurs MCP</div>
      <div class="hint">S&eacute;lectionne un serveur dans l'explorateur pour voir ses tools, ses infos et les basculer ON/OFF un par un.</div>
      <div class="qa">
        <button onclick="doScan()">&#128269; Scanner les ports locaux</button>
        <button onclick="doScanNet()">&#127760; Scanner le r&eacute;seau</button>
        <button onclick="openMachineDlg()">🖥️ Choisir une machine</button>
      </div>
      <div class="hint" style="margin-top:16px">Le scan utilise la plage configur&eacute;e. Pour une machine du LAN : connexion SSH par cl&eacute;,
      l'app liste ses ports en &eacute;coute, y sonde les serveurs MCP et les importe ici.</div>
    </div>`;
}

/* ===== Onglet Conf agent : prêt à coller dans Hermes / Claude / etc. ===== */
const srvName = s => (s.label || s.id || 'serveur').toLowerCase().replace(/[^a-z0-9_-]+/g, '-').replace(/^-+|-+$/g, '') || 'serveur';
const srvUrl = s => {
  const h = s.transport_info.host || '127.0.0.1';
  return 'http://' + h + ':' + s.transport_info.port + (s.transport_info.path || '/mcp');
};
function hermesYaml(s) {
  const n = srvName(s);
  if (s.transport === 'http')
    return 'mcp_servers:\n  ' + n + ':\n    url: ' + srvUrl(s) + '\n    enabled: true';
  const args = (s.transport_info.args || []).map(a => JSON.stringify(a)).join(', ');
  return 'mcp_servers:\n  ' + n + ':\n    command: ' + (s.transport_info.program || 'npx') + '\n    args: [' + args + ']\n    enabled: true';
}
function mcpJson(s) {
  const n = srvName(s);
  const blk = s.transport === 'http'
    ? {type: 'http', url: srvUrl(s)}
    : {command: s.transport_info.program || 'npx', args: s.transport_info.args || []};
  return JSON.stringify({mcpServers: {[n]: blk}}, null, 2);
}
function claudeCmd(s) {
  const n = srvName(s);
  return s.transport === 'http'
    ? 'claude mcp add --transport http ' + n + ' ' + srvUrl(s)
    : 'claude mcp add ' + n + ' -- ' + (s.transport_info.program || 'npx') + ' ' + (s.transport_info.args || []).join(' ');
}
function copyConf(btn) {
  const pre = btn.closest('.conf-block').querySelector('pre');
  navigator.clipboard.writeText(pre.textContent).then(() => {
    btn.textContent = 'copi\u00e9 \u2713';
    setTimeout(() => btn.textContent = 'copier', 1500);
  }).catch(() => {});
}
function confAgent(s) {
  const blk = (t, code) => '<div class="conf-block"><div class="conf-head">' + t + '<button onclick="copyConf(this)">copier</button></div><pre>' + esc(code) + '</pre></div>';
  return '<h4 class="sec">Connecter ce serveur &agrave; un agent</h4>' +
    '<div class="conf-note">Copie le bloc de ton agent &mdash; Hermes : colle sous <b>mcp_servers:</b> dans <b>~/.hermes/config.yaml</b>. Cl&eacute;s API : bloc <b>env:</b> (Hermes) ou <b>"env"</b> (JSON) &mdash; voir la desc du serveur.</div>' +
    blk('Hermes &mdash; config.yaml', hermesYaml(s)) +
    blk('JSON mcpServers (Claude, Cursor, VS Code&hellip;)', mcpJson(s)) +
    blk('claude mcp add', claudeCmd(s));
}
function renderDetail() {
  const s = detailCache;
  if (!s || s.id !== selectedId) return;
  const ti = s.transport === 'http'
    ? `HTTP ${s.transport_info.host ? esc(s.transport_info.host)+':' : ''}${s.transport_info.port}${esc(s.transport_info.path)}`
    : `stdio &middot; ${esc(s.transport_info.program)} ${(s.transport_info.args||[]).join(' ')}`;
  const kv = [
    ['id', s.id], ['transport', ti],
    ['origine', s.origin],
    ['bind', s.bound_host ?? '-'], ['serveur MCP', s.server_name ?? '-'],
    ['version', s.version ?? '-'], ['protocole', s.protocol ?? '-'],
    ['framework', s.framework === 'litemcp' ? 'LiteMCP (Rust · HTTP stateless 2026-07-28)' : '-'],
    ['sonde', s.via_ssh ? 'via SSH (interne à la machine)' : 'directe'],
    ['latence', s.latency_ms != null ? s.latency_ms + ' ms' : '-'],
    ['dernier vu', s.last_seen ? new Date(s.last_seen * 1000).toLocaleString('fr-FR') : '-'],
    ['erreur', s.last_error ?? '-'],
  ].map(([k, v]) => `<div class="k">${k}</div><div class="v">${esc(v)}</div>`).join('');
  /* Panneau Démarrage : contrôle du process par le command center.
     « au démarrage » = process lancé ici et vivant, mais MCP pas encore répondu. */
  const starting = s.enabled && s.managed && s.state !== 'UP';
  const dispState = starting ? 'STARTING' : s.state;
  const etatProcess = s.state === 'UP' ? 'en marche'
    : starting ? 'au d&eacute;marrage&hellip;'
    : s.state === 'PORT-ONLY' ? 'port occup&eacute; (aucune r&eacute;ponse MCP)'
    : s.state === 'DISABLED' ? 'd&eacute;sactiv&eacute;' : 'arr&ecirc;t&eacute;';
  const cmd = s.start_cmd
    ? s.start_cmd.program + ' ' + (s.start_cmd.args || []).join(' ') + (s.start_cmd.workdir ? '  &middot;  ' + s.start_cmd.workdir : '')
    : null;
  const kvd = [
    ['contr&ocirc;le', s.managed ? 'process lanc&eacute; et maintenu par le command center'
      : (cmd ? 'commande pr&ecirc;te &mdash; process externe non d&eacute;marr&eacute; par ici' : 'aucune &mdash; process externe')],
    ['&eacute;tat du process', etatProcess],
    ['pid', s.pid ?? '-'],
    ['auto-restart', s.auto_restart ? 'ON &mdash; relance automatique au boot/crash' : 'OFF'],
    ['commande', cmd ?? '-'],
  ].map(([k, v]) => `<div class="k">${k}</div><div class="v">${esc(v)}</div>`).join('');
  /* Replis à état mémorisé : le rendu tourne à chaque cycle de rafraîchissement,
     l'état ouvert/fermé choisi par l'utilisateur est donc persisté (localStorage)
     et relu ici — sinon le triangle revenait à son état d'origine toutes les 5 s
     et à chaque réouverture d'onglet. */
  const foldOpen = name => {
    try {
      const v = localStorage.getItem('mcc.fold.' + name);
      if (v !== null) return v === 'true';
    } catch (e) {}
    return name === 'demarrage'; // défauts : Démarrage ouvert, Informations fermé
  };
  const fold = (name, title, inner) =>
    `<details class="fold" data-fold="${name}"${foldOpen(name) ? ' open' : ''} ontoggle="saveFold(this)"><summary>${title}</summary>${inner}</details>`;
  const tools = (s.tools_list || []).map(t =>
    `<div class="tool ${t.disabled?'off':''}">
       <code>${esc(t.name)}</code>
       <span class="desc" title="${esc(t.desc)}">${esc(t.desc) || '&mdash;'}</span>
       <button class="tgl ${t.disabled?'off':'on'}" title="${t.disabled?'R&eacute;activer ce tool':'D&eacute;sactiver ce tool'}"
               onclick="toggleTool(${js(s.id)},${js(t.name)})">
         ${t.disabled ? 'OFF' : 'ON'}
       </button>
     </div>`).join('');
  const nOn = (s.tools_list || []).filter(t => !t.disabled).length;
  const nTot = (s.tools_list || []).length;
  const toolsHtml = `<h4 class="sec">Tools &mdash; ${nOn} actif(s) / ${nTot}${nTot - nOn ? ` (${nTot - nOn} OFF)` : ''}</h4>
    ${tools || '<div style="color:var(--t4);font-size:12px">aucun tool list&eacute;</div>'}`;
  document.getElementById('content').innerHTML = `
    <div class="crumb">serveurs / <b>${esc(s.label)}</b></div>
    <div class="d-head">
      <span class="dot ${esc(dispState)}" style="width:10px;height:10px"></span>
      <h1>${esc(s.label)}</h1>
      <span class="pill ${esc(dispState)}" ${starting ? 'title="Process lanc&eacute;, MCP pas encore r&eacute;pondu"' : ''}>${starting ? 'AU D&Eacute;MARRAGE' : esc(s.state)}</span>
      ${langChip(s)}
      ${typeBadge(s)}
      ${frameworkBadge(s)}
      ${s.transport === 'stdio' ? '<span class="legacy" title="Legacy : serveur stdio uniquement (pas de transport HTTP)">d&eacute;suet</span>' : ''}
      <span class="sub">${s.server_name ? esc(s.server_name) + ' v' + esc(s.version || '?') : ''}${s.latency_ms != null ? ' &middot; ' + s.latency_ms + ' ms' : ''}</span>
      <div class="d-actions">
        ${s.can_start && s.state !== 'UP' ? `<button class="primary" onclick="act('start',${js(s.id)})">&#9654; Start</button>` : ''}
        ${s.can_start && s.state === 'UP' ? `<button class="danger" onclick="act('kill',${js(s.id)})">&#9632; Stop</button>` : ''}
        <button onclick="act('refresh',${js(s.id)})">Sonder</button>
        <button onclick="act('toggle',${js(s.id)})">${s.enabled ? 'D&eacute;sactiver' : 'Activer'}</button>
        ${s.origin !== 'seed'
          ? `<button class="danger" onclick="removeSrv(${js(s.id)})">Retirer</button>`
          : `<button class="danger" onclick="removeSeedForever(${js(s.id)})" title="Supprime ce seed de la config : il ne sera pas r&eacute;inject&eacute; au red&eacute;marrage">Retirer durablement</button>`}
      </div>
    </div>
    ${s.origin === 'seed' ? '<div class="seed-note">Serveur seed : <b>Retirer durablement</b> l&rsquo;efface m&ecirc;me apr&egrave;s red&eacute;marrage (consign&eacute; dans config.json) ; <b>D&eacute;sactiver</b> l&rsquo;&eacute;teint sans le supprimer.</div>' : ''}
    ${s.last_error ? `<div class="d-err">&#9888; ${esc(s.last_error)}</div>` : ''}
    ${fold('demarrage', 'D&eacute;marrage', `<div class="kv">${kvd}</div>`)}
    ${fold('infos', 'Informations', `<div class="kv">${kv}</div>`)}
    <div class="tabs">
      <button class="tab ${detailTab === 'outils' ? 'sel' : ''}" onclick="detailTab='outils';renderDetail()">Outils (${nTot})</button>
      <button class="tab ${detailTab === 'conf' ? 'sel' : ''}" onclick="detailTab='conf';renderDetail()">Conf agent</button>
    </div>
    ${detailTab === 'conf' ? confAgent(s) : toolsHtml}`;
}

function select(id) { selectedId = id; detailCache = null; detailTab = 'outils'; render(); load(); }
function removeSrv(id) { selectedId = null; detailCache = null; act('remove', id); }
function removeSeedForever(id) {
  if (!confirm('Retirer ce seed DURABLEMENT ? Il ne sera pas réinjecté au redémarrage du command center.')) return;
  selectedId = null; detailCache = null;
  fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'remove', id, durable: true})}).then(load);
}

/* ===== Actions ===== */
async function act(action, id) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action, id})});
  await load();
}

/* Scan en un clic : la plage vient des réglages (settings), aucune saisie demandée */
async function doScan() {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'scan'})});
  await load();
}

async function doScanNet() {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'scan_network'})});
  await load();
}

async function refreshAll() {
  // Re-sonde les serveurs connus ET découvre les nouveaux (tous ports en
  // écoute — action discover) en parallèle, un seul rechargement d'état à la fin.
  const ids = state.servers.filter(s => s.enabled).map(s => s.id);
  const jobs = ids.map(id =>
    fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({action: 'refresh', id})}).catch(() => {}));
  jobs.push(fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'discover'})}).catch(() => {}));
  await Promise.all(jobs);
  await load();
}

async function doPeers() {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'discover_peers'})});
  await load();
}

async function importPeer(ip) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'import_peer', ip})});
  await load();
}

/* ===== Machines distantes (SSH) ===== */
function openMachineDlg() {
  document.getElementById('machineDlg').showModal();
  renderMachines();
}
function renderMachines() {
  if (!state) return;
  const hosts = state.settings.ssh_hosts || [];
  const peers = state.peers || [];
  const box = document.getElementById('machineList');
  let html = hosts.length ? '' :
    '<div class="empty-sec">aucune machine enregistr&eacute;e — entre une IP et un utilisateur SSH ci-dessus</div>';
  html += hosts.map(h => {
    const peer = peers.find(p => p.ip === h.ip);
    const st = peer ? `command center d&eacute;tect&eacute; — ${(peer.stats||{}).up ?? '?'} UP / ${(peer.stats||{}).total ?? '?'} serveurs`
                    : 'machine enregistr&eacute;e';
    return `<div class="machine-row">
      <div class="l1"><span class="dot UP"></span><span class="nm">${esc(h.ip)}</span><span class="tag http">${esc(h.user)}</span>${peer ? '<span class="tag sse">pair</span>' : ''}</div>
      <div class="l2">${st}</div>
      <div class="macts">
        <button class="primary" onclick="sshDiscover('${esc(h.ip)}','${esc(h.user)}')">🔗 SSH &amp; d&eacute;couvrir</button>
        ${peer ? `<button onclick="importPeer('${esc(h.ip)}')">Importer ses serveurs</button>` : ''}
        <button class="danger" onclick="sshForget('${esc(h.ip)}')">Oublier</button>
      </div>
    </div>`;
  }).join('');
  const autres = peers.filter(p => !hosts.find(h => h.ip === p.ip));
  html += autres.map(p => `<div class="machine-row">
      <div class="l1"><span class="dot UP"></span><span class="nm">${esc(p.ip)}</span><span class="tag sse">pair</span></div>
      <div class="l2">UI :${p.ui_port} &mdash; ${(p.stats||{}).up ?? '?'} UP / ${(p.stats||{}).total ?? '?'} serveurs</div>
      <div class="macts"><button onclick="importPeer('${esc(p.ip)}')">Importer ses serveurs</button></div>
    </div>`).join('');
  box.innerHTML = html;
}
function sshForm() {
  const ip = (document.getElementById('mIp').value || '').trim();
  const user = (document.getElementById('mUser').value || '').trim();
  return {ip, user};
}
async function sshDiscover(ip, user) {
  const f = sshForm();
  ip = ip || f.ip; user = user || f.user;
  if (!ip || !user) { alert('IP et utilisateur SSH requis'); return; }
  const msg = document.getElementById('machineMsg');
  msg.textContent = '⏳ connexion SSH + netstat distant + sonde des ports… (jusqu\u2019à ~30 s, vois le journal)';
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'ssh_discover', ip, user})});
  const j = await r.json();
  if (j.ok && j.started) {
    msg.textContent = '⏳ d\u00e9couverte lanc\u00e9e sur ' + ip + ' — le journal et la liste se remplissent pendant la sonde.';
    let t = 0;
    const iv = setInterval(async () => {
      await load(); renderMachines();
      if (++t >= 10) { clearInterval(iv); }
    }, 4000);
  } else if (!j.ok) {
    msg.textContent = '✗ ' + (j.error || 'échec');
  }
}
async function sshSave() {
  const f = sshForm();
  if (!f.ip || !f.user) { alert('IP et utilisateur SSH requis'); return; }
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'ssh_save', ip: f.ip, user: f.user})});
  const j = await r.json();
  document.getElementById('machineMsg').textContent = j.ok ? '✓ machine enregistr\u00e9e' : '✗ ' + (j.error || 'erreur');
  await load(); renderMachines();
}
async function sshForget(ip) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'ssh_forget', ip})});
  await load(); renderMachines();
}

async function toggleTool(id, name) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'toggle_tool', id, tool: name})});
  const r = await fetch('/api/detail?id=' + encodeURIComponent(id));
  if (r.ok) { detailCache = await r.json(); renderDetail(); }
  await load();
}

function toggleAdd() { document.getElementById('dlg').showModal(); }
function transportChanged() {
  const stdio = document.getElementById('fTransport').value === 'stdio';
  document.getElementById('httpFields').style.display = stdio ? 'none' : '';
  document.getElementById('stdioFields').style.display = stdio ? '' : 'none';
}

async function addServer(ev) {
  ev.preventDefault();
  const t = document.getElementById('fTransport').value;
  const body = {action: 'add', transport: t, label: document.getElementById('fLabel').value};
  if (t === 'http') {
    body.port = +document.getElementById('fPort').value;
    body.path = document.getElementById('fPath').value || '/mcp';
    const prog = document.getElementById('fHttpProgram').value.trim();
    if (prog) {
      body.program = prog;
      const wd = document.getElementById('fHttpWorkdir').value.trim();
      if (wd) body.workdir = wd;
      body.args = document.getElementById('fHttpArgs').value.split(/\s+/).filter(Boolean);
    }
  }
  else { body.program = document.getElementById('fProgram').value;
         body.workdir = document.getElementById('fWorkdir').value || null;
         body.args = document.getElementById('fArgs').value.split(/\s+/).filter(Boolean); }
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(body)});
  const j = await r.json();
  if (!j.ok) alert(j.error || 'erreur');
  else { document.getElementById('dlg').close(); selectedId = j.id; await act('refresh', j.id); }
  return false;
}

/* ===== Boutique MCP (porté de Hermes mcp-import.ts / connectors-directory) ===== */
let storeItems = null;
let storeLogos = null;
let storeCat = 'Tout';
async function openStore() {
  document.getElementById('storeDlg').showModal();
  if (!storeItems) {
    const r = await fetch('/api/store');
    storeItems = await r.json();
    document.getElementById('storeCount').textContent = '(' + storeItems.length + ' MCP)';
    // logos officiels (glyphes embarqués au binaire, servis en JSON)
    fetch('/api/store-logos').then(r2 => r2.json()).then(j => {
      storeLogos = j;
      renderStoreItems(); // re-rendu quand les logos arrivent
    }).catch(() => {});
    const cats = ['Tout', ...new Set(storeItems.map(i => i.cat))];
    document.getElementById('storeCats').innerHTML = cats.map(c =>
      `<button class="scat ${c===storeCat?'sel':''}" onclick="storeCat=${js(c)};renderStoreItems()">${esc(c)}</button>`).join('');
  }
  renderStoreItems();
}
/* Logo d'une carte : glyphe officiel par id (STORE_LOGOS), sinon pour les items
   aitmpl une marque reconnue dans la commande d'installation, sinon un glyphe
   « paquet » neutre. NB : guillemet-double suivi de dièse interdit dans ce
   bloc — la séquence fermerait la raw string Rust qui embarque cette page. */
const PKG_GLYPH = "<path fill='none' stroke='#8a93a5' stroke-width='2' stroke-linejoin='round' d='m12 2 9 5v10l-9 5-9-5V7Zm0 10 9-5m-9 5-9-5m9 5v10'/>";
const PKG_LOGOS = [
  ['context7', 'context7'], ['mongodb', 'mongodb'], ['supabase', 'supabase'],
  ['playwright', 'playwright'], ['figma', 'figma'], ['firecrawl', 'firecrawl'],
  ['notionhq', 'notion'], ['stripe', 'stripe'], ['kubernetes', 'kubernetes'],
  ['duckduckgo', 'duckduckgo'], ['todoist', 'todoist'], ['sqlite', 'sqlite'],
  ['postgres', 'postgresql'], ['server-gdrive', 'googledrive'], ['slack', 'slack'],
  ['brave', 'brave'], ['n8n', 'n8n'], ['gitlab', 'gitlab'], ['sentry', 'sentry'],
  ['server-google-maps', 'googlemaps'], ['paddleocr', 'paddlepaddle'],
  ['chrome-devtools', 'googlechrome'],
];
function storeLogoSvg(inner) {
  return `<svg class="slogo" viewBox="0 0 24 24" aria-hidden="true">${inner}</svg>`;
}
function storeLogo(i) {
  if (!storeLogos) return '';
  // glyphe officiel par id, préfixes de catalogues additionnels retirés
  const inner = storeLogos[i.id] || storeLogos[i.id.replace(/^(at|gl)-/, '')];
  if (inner) return storeLogoSvg(inner);
  // items de catalogues additionnels : marque reconnue dans la commande,
  // sinon glyphe « paquet » neutre
  if (/^(at|gl|gh)-/.test(i.id)) {
    const cmd = (i.program + ' ' + (i.args || []).join(' ')).toLowerCase();
    for (const [needle, lid] of PKG_LOGOS) {
      if (storeLogos[lid] && cmd.includes(needle)) return storeLogoSvg(storeLogos[lid]);
    }
    return storeLogoSvg(PKG_GLYPH);
  }
  return '';
}
function renderStoreItems() {
  if (!storeItems) return;
  const q = (document.getElementById('storeSearch').value || '').toLowerCase();
  const list = storeItems.filter(i => (storeCat==='Tout' || i.cat===storeCat) &&
    (!q || i.name.toLowerCase().includes(q) || i.desc.toLowerCase().includes(q)));
  document.getElementById('storeGrid').innerHTML = list.map(i => {
    const cmd = (i.program + ' ' + i.args.join(' ')).replace(/C:\/Users\/Deamon/g, '~');
    return `<div class="scard">
      <div class="scard-top">${storeLogo(i)}<b>${esc(i.name)}</b><span class="tag ${i.kind==='npm'?'npm':'py'}">${i.kind}</span></div>
      <div class="scard-desc">${esc(i.desc)}</div>
      <div class="scard-cmd" title="${esc(i.program + ' ' + i.args.join(' '))}">${esc(cmd)}</div>
      <button class="primary" onclick="installStore(${js(i.id)}, this)">&#10515; Installer</button>
    </div>`;
  }).join('') || '<div class="empty-sec">aucun r&eacute;sultat</div>';
}
async function installStore(id, btn) {
  btn.disabled = true; btn.textContent = '\u23f3 Installation\u2026';
  const body = {action: 'install_store', id};
  if (id === 'filesystem') {
    // le dossier exposé est un choix EXPLICITE (jamais de chemin personnel par défaut)
    const dir = prompt('Dossier à exposer au serveur Filesystem (chemin complet) :', '');
    if (!dir || !dir.trim()) {
      btn.disabled = false; btn.textContent = '\u23f3 Installer';
      return;
    }
    body.args = ['-y', '@modelcontextprotocol/server-filesystem', dir.trim()];
  }
  const r = await fetch('/api/action', {method:'POST', headers:{'Content-Type':'application/json'},
    body: JSON.stringify(body)});
  const j = await r.json();
  if (j.ok) {
    btn.textContent = j.added ? (j.state === 'UP' ? '\u2713 Install\u00e9 et UP' : '\u2713 Install\u00e9 (d\u00e9marrage\u2026)') : 'D\u00e9j\u00e0 install\u00e9';
    await load();
  } else { btn.disabled = false; btn.textContent = 'R\u00e9essayer'; alert(j.error || 'erreur'); }
}

/* ===== Barre latérale ajustable (largeur mémorisée) ===== */
(function () {
  const grip = document.getElementById('grip'), expl = document.getElementById('explorer');
  const clamp = w => Math.min(520, Math.max(180, w));
  try { const saved = +localStorage.getItem('mcc.explorer.w'); if (saved) expl.style.width = clamp(saved) + 'px'; } catch (e) {}
  grip.addEventListener('mousedown', e => {
    e.preventDefault();
    grip.classList.add('drag');
    const x0 = e.clientX, w0 = expl.getBoundingClientRect().width;
    const mv = ev => { expl.style.width = clamp(w0 + ev.clientX - x0) + 'px'; };
    const up = () => {
      try { localStorage.setItem('mcc.explorer.w', String(Math.round(expl.getBoundingClientRect().width))); } catch (e) {}
      grip.classList.remove('drag');
      document.removeEventListener('mousemove', mv);
      document.removeEventListener('mouseup', up);
    };
    document.addEventListener('mousemove', mv);
    document.addEventListener('mouseup', up);
  });
})();

/* ===== Contrôles de fenêtre (desktop Tauri uniquement : ni data-tauri-drag-region
   ni __TAURI__ n'ont d'effet dans un navigateur classique, les boutons y restent masqués) ===== */
(function () {
  try {
    const W = window.__TAURI__ && window.__TAURI__.window;
    if (!W) return;
    const tw = W.getCurrentWindow ? W.getCurrentWindow() : (W.getCurrent ? W.getCurrent() : null);
    if (!tw) return;
    document.getElementById('winCtrls').style.display = 'flex';
    document.getElementById('wMin').onclick = () => tw.minimize();
    document.getElementById('wMax').onclick = () => tw.toggleMaximize();
    document.getElementById('wClose').onclick = () => tw.close();
  } catch (e) { /* navigateur classique : rien à câbler */ }
})();

load();
setInterval(load, 5000);
</script>
</body>
</html>
"#;
