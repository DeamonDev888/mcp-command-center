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
.tb-spacer{flex:1}
.tb-actions{display:flex;gap:6px}

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
  color:var(--t4);padding:14px 16px 6px;user-select:none}
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
h4.sec{font:10px var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.9px;color:var(--t4);
  margin:24px 0 10px;display:flex;align-items:center;gap:10px}
h4.sec::after{content:'';flex:1;height:1px;background:var(--border-soft)}
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

<div id="titlebar">
  <span class="logo"><em>&#9679;</em> MCP <em>Command Center</em></span>
  <span class="tb-badge ok">UP <b id="stUp">0</b>/<b id="stTotal">0</b></span>
  <span class="tb-badge">tools <b id="stTools">0</b></span>
  <span class="tb-badge">refresh <b id="stRefresh">15</b>s</span>
  <span class="tb-spacer"></span>
  <div class="tb-actions">
    <button class="primary" onclick="doScan()">&#128269; Scan ports</button>
    <button onclick="doScanNet()">&#127760; Scan r&eacute;seau</button>
    <button onclick="doPeers()">&#129309; Pairs</button>
    <button onclick="refreshAll()">&#8635;</button>
  </div>
</div>

<div id="main">
  <aside id="explorer">
    <div class="sec-label">Serveurs MCP</div>
    <div id="srvList"></div>
    <div class="sec-label">Command centers (pairs)</div>
    <div id="peerList"></div>
    <div id="addWrap"><button onclick="toggleAdd()">+ Ajouter un serveur</button></div>
  </aside>
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

<script>
let state = null;
let selectedId = null;
let detailCache = null;
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));

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
    return `<div class="srow ${selectedId===s.id?'sel':''} ${s.enabled?'':'off'}" onclick="select('${esc(s.id)}')">
      <div class="l1"><span class="dot ${s.state}"></span><span class="nm">${esc(s.label)}</span>${s.transport==='stdio'?'<span class="legacy" title="Legacy : serveur stdio uniquement (pas de transport HTTP)">d&eacute;suet</span>':''}<span class="cnt">${s.tools_count}</span>${rowCtl(s)}</div>
      <div class="l2"><span class="st ${s.state}">${s.state}</span> ${ti}${off}</div>
    </div>`;
  }).join('');
}

function rowCtl(s) {
  if (s.transport !== 'stdio') return '';
  const up = s.state === 'UP';
  return `<button class="rowbtn" title="${up?'Arr\u00eater':'D\u00e9marrer'} ce serveur" onclick="event.stopPropagation();act('${up?'kill_stdio':'start_stdio'}','${esc(s.id)}')">${up?'\u25a0':'\u25b6'}</button>`;
}

function renderPeers() {
  const box = document.getElementById('peerList');
  if (!state.peers || !state.peers.length) { box.innerHTML = '<div class="empty-sec">aucun pair d&eacute;tect&eacute;</div>'; return; }
  box.innerHTML = state.peers.map(p => {
    const st = p.stats || {};
    return `<div class="peer-row">
      <div class="l1"><span class="dot UP"></span><span class="nm">${esc(p.ip)}</span></div>
      <div class="l2">UI :${p.ui_port} &mdash; ${st.up ?? '?'} UP / ${st.total ?? '?'} serveurs</div>
      <button onclick="importPeer('${esc(p.ip)}')">Importer ses serveurs</button>
    </div>`;
  }).join('');
}

function renderLog() {
  const lb = document.getElementById('logbody');
  lb.innerHTML = state.log.map(l => {
    const d = new Date(l.ts * 1000).toLocaleTimeString('fr-FR');
    return `<div class="logline ${l.level}"><span class="t">${d}</span>  ${esc(l.msg)}</div>`;
  }).join('');
  lb.scrollTop = lb.scrollHeight;
}

/* ===== Zone de détail ===== */
function renderOverview() {
  const s = state.stats;
  document.getElementById('content').innerHTML = `
    <div class="tiles">
      <div class="tile"><div class="n ok">${s.up}</div><div class="lbl">serveurs UP</div></div>
      <div class="tile"><div class="n">${s.total}</div><div class="lbl">serveurs suivis</div></div>
      <div class="tile"><div class="n">${s.tools}</div><div class="lbl">tools actifs</div></div>
      <div class="tile"><div class="n">${state.settings.refresh_secs}s</div><div class="lbl">rafra&icirc;chissement</div></div>
    </div>
    <div class="welcome">
      <div class="big">S&eacute;lectionne un serveur dans l'explorateur</div>
      <div class="hint">Pour voir ses tools, ses infos et les basculer ON/OFF un par un.</div>
      <div class="qa">
        <button onclick="doScan()">&#128269; Scanner les ports locaux</button>
        <button onclick="doScanNet()">&#127760; Scanner le r&eacute;seau</button>
        <button onclick="doPeers()">&#129309; D&eacute;tecter les pairs</button>
      </div>
    </div>`;
}

function renderDetail() {
  const s = detailCache;
  if (!s || s.id !== selectedId) return;
  const ti = s.transport === 'http'
    ? `HTTP ${s.transport_info.host ? esc(s.transport_info.host)+':' : ''}${s.transport_info.port}${esc(s.transport_info.path)}`
    : `stdio &middot; ${esc(s.transport_info.program)} ${(s.transport_info.args||[]).join(' ')}`;
  const kv = [
    ['id', s.id], ['transport', ti],
    ['origine', s.origin], ['pid', s.pid ?? '-'],
    ['bind', s.bound_host ?? '-'], ['serveur MCP', s.server_name ?? '-'],
    ['version', s.version ?? '-'], ['protocole', s.protocol ?? '-'],
    ['latence', s.latency_ms != null ? s.latency_ms + ' ms' : '-'],
    ['dernier vu', s.last_seen ? new Date(s.last_seen * 1000).toLocaleString('fr-FR') : '-'],
    ['erreur', s.last_error ?? '-'],
  ].map(([k, v]) => `<div class="k">${k}</div><div class="v">${esc(v)}</div>`).join('');
  const tools = (s.tools_list || []).map(t =>
    `<div class="tool ${t.disabled?'off':''}">
       <code>${esc(t.name)}</code>
       <span class="desc" title="${esc(t.desc)}">${esc(t.desc) || '&mdash;'}</span>
       <button class="tgl ${t.disabled?'off':'on'}" title="${t.disabled?'R&eacute;activer ce tool':'D&eacute;sactiver ce tool'}"
               onclick="toggleTool('${esc(s.id)}','${esc(t.name).replace(/'/g, "\\'")}')">
         ${t.disabled ? 'OFF' : 'ON'}
       </button>
     </div>`).join('');
  const nOn = (s.tools_list || []).filter(t => !t.disabled).length;
  const nTot = (s.tools_list || []).length;
  document.getElementById('content').innerHTML = `
    <div class="crumb">serveurs / <b>${esc(s.label)}</b></div>
    <div class="d-head">
      <span class="dot ${s.state}" style="width:10px;height:10px"></span>
      <h1>${esc(s.label)}</h1>
      <span class="pill ${s.state}">${s.state}</span>
      ${s.transport === 'stdio' ? '<span class="legacy" title="Legacy : serveur stdio uniquement (pas de transport HTTP)">d&eacute;suet</span>' : ''}
      <span class="sub">${s.server_name ? esc(s.server_name) + ' v' + esc(s.version || '?') : ''}${s.latency_ms != null ? ' &middot; ' + s.latency_ms + ' ms' : ''}</span>
      <div class="d-actions">
        ${s.transport === 'stdio' && s.state !== 'UP' ? `<button class="primary" onclick="act('start_stdio','${esc(s.id)}')">&#9654; Start</button>` : ''}
        ${s.transport === 'stdio' && s.state === 'UP' ? `<button class="danger" onclick="act('kill_stdio','${esc(s.id)}')">&#9632; Stop</button>` : ''}
        <button onclick="act('refresh','${esc(s.id)}')">Sonder</button>
        <button onclick="act('toggle','${esc(s.id)}')">${s.enabled ? 'D&eacute;sactiver' : 'Activer'}</button>
        <button class="danger" onclick="removeSrv('${esc(s.id)}')">Retirer</button>
      </div>
    </div>
    ${s.last_error ? `<div class="d-err">&#9888; ${esc(s.last_error)}</div>` : ''}
    <h4 class="sec">Informations</h4>
    <div class="kv">${kv}</div>
    <h4 class="sec">Tools &mdash; ${nOn} actif(s) / ${nTot}${nTot - nOn ? ` (${nTot - nOn} OFF)` : ''}</h4>
    ${tools || '<div style="color:var(--t4);font-size:12px">aucun tool list&eacute;</div>'}`;
}

function select(id) { selectedId = id; detailCache = null; render(); load(); }
function removeSrv(id) { selectedId = null; detailCache = null; act('remove', id); }

/* ===== Actions ===== */
async function act(action, id) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action, id})});
  await load();
}

async function doScan() {
  const from = prompt('Scan ports de :', state.settings.scan_from ?? 3000);
  if (from === null) return;
  const to = prompt('jusqu\u2019\u00e0 :', state.settings.scan_to ?? 3200);
  if (to === null) return;
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'scan', from: +from, to: +to})});
  await load();
}

async function refreshAll() {
  const ids = state.servers.filter(s => s.enabled).map(s => s.id);
  for (const id of ids) await act('refresh', id);
}

async function doScanNet() {
  const from = prompt('Scan r\u00e9seau ports de :', state.settings.scan_from ?? 3000);
  if (from === null) return;
  const to = prompt('jusqu\u2019\u00e0 :', state.settings.scan_to ?? 3200);
  if (to === null) return;
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'scan_network', from: +from, to: +to})});
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
  if (t === 'http') { body.port = +document.getElementById('fPort').value; body.path = document.getElementById('fPath').value || '/mcp'; }
  else { body.program = document.getElementById('fProgram').value;
         body.workdir = document.getElementById('fWorkdir').value || null;
         body.args = document.getElementById('fArgs').value.split(/\s+/).filter(Boolean); }
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(body)});
  const j = await r.json();
  if (!j.ok) alert(j.error || 'erreur');
  else { document.getElementById('dlg').close(); selectedId = j.id; await act('refresh', j.id); }
  return false;
}

load();
setInterval(load, 5000);
</script>
</body>
</html>
"#;
