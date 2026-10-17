pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>MCP Command Center</title>
<style>
:root{
  --bg:#0d1117;--card:#161b22;--card2:#1c2330;--border:#2d3748;
  --fg:#e6edf3;--dim:#8b949e;--accent:#58a6ff;--ok:#3fb950;--warn:#d29922;--bad:#f85149;
  --mono:ui-monospace,'Cascadia Code',Consolas,monospace;
}
*{box-sizing:border-box;margin:0;padding:0}
body{background:var(--bg);color:var(--fg);font:14px/1.5 -apple-system,'Segoe UI',system-ui,sans-serif;padding:20px}
header{display:flex;align-items:center;gap:14px;margin-bottom:18px;flex-wrap:wrap}
header h1{font-size:20px;letter-spacing:.5px}
header h1 span{color:var(--accent)}
.badge{background:var(--card);border:1px solid var(--border);border-radius:20px;padding:3px 12px;font-size:12px;color:var(--dim)}
.badge b{color:var(--fg)}
.badge.ok b{color:var(--ok)}
button{background:var(--card);color:var(--fg);border:1px solid var(--border);border-radius:8px;padding:6px 14px;cursor:pointer;font-size:13px;transition:.15s}
button:hover{border-color:var(--accent);color:var(--accent)}
button.primary{background:#1f6feb22;border-color:#1f6feb;color:#79b8ff}
button.danger:hover{border-color:var(--bad);color:var(--bad)}
button:disabled{opacity:.4;cursor:not-allowed}
.toolbar{display:flex;gap:8px;margin-bottom:14px;flex-wrap:wrap;align-items:center}
.spacer{flex:1}
.grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(340px,1fr));gap:12px}
.card{background:var(--card);border:1px solid var(--border);border-radius:12px;padding:14px;cursor:pointer;transition:.15s}
.card:hover{border-color:#4a5a72;transform:translateY(-1px)}
.card.disabled{opacity:.45}
.card-top{display:flex;align-items:center;gap:10px;margin-bottom:8px}
.dot{width:10px;height:10px;border-radius:50%;flex-shrink:0}
.dot.UP{background:var(--ok);box-shadow:0 0 8px var(--ok)}
.dot.PORT-ONLY{background:var(--warn);box-shadow:0 0 8px var(--warn)}
.dot.DOWN{background:#4a5568}
.dot.DISABLED{background:#30363d;border:1px solid var(--border)}
.dot.PROBING,.dot.STARTING{background:var(--accent);animation:pulse 1s infinite}
@keyframes pulse{50%{opacity:.3}}
.card-top h3{font-size:15px;flex:1;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.state{font:11px var(--mono);padding:2px 8px;border-radius:4px;background:var(--card2);border:1px solid var(--border)}
.state.UP{color:var(--ok);border-color:#2ea04366}
.state.PORT-ONLY{color:var(--warn);border-color:#d2992266}
.meta{font:12px var(--mono);color:var(--dim);display:flex;flex-wrap:wrap;gap:4px 14px}
.meta .v{color:var(--fg)}
.tag{display:inline-block;font:10px var(--mono);padding:1px 7px;border-radius:3px;background:var(--card2);border:1px solid var(--border);color:var(--dim)}
.tag.http{color:#bc8cff;border-color:#bc8cff44}
.tag.stdio{color:#39c5cf;border-color:#39c5cf44}
.tag.disc{color:var(--warn);border-color:#d2992244}
.err{color:var(--bad);font:11px var(--mono);margin-top:6px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.actions{display:flex;gap:6px;margin-top:10px}
.actions button{padding:4px 10px;font-size:12px}
dialog{background:var(--card);color:var(--fg);border:1px solid var(--border);border-radius:14px;padding:0;width:min(720px,92vw);max-height:85vh}
dialog::backdrop{background:#000a}
.d-head{display:flex;align-items:center;gap:10px;padding:16px 20px;border-bottom:1px solid var(--border)}
.d-head h2{font-size:17px;flex:1}
.d-head .close{background:none;border:none;font-size:20px;color:var(--dim);cursor:pointer}
.d-body{padding:16px 20px;overflow-y:auto;max-height:calc(85vh - 70px)}
.kv{display:grid;grid-template-columns:150px 1fr;gap:6px 12px;font-size:13px;margin-bottom:16px}
.kv .k{color:var(--dim)}
.kv .v{font:12px var(--mono);word-break:break-all}
.tools h4{margin-bottom:8px;color:var(--dim);font-size:12px;text-transform:uppercase;letter-spacing:1px}
.tool{display:flex;gap:10px;padding:7px 10px;border:1px solid var(--border);border-radius:8px;margin-bottom:6px;background:var(--card2)}
.tool code{color:#79b8ff;font:12px var(--mono);flex-shrink:0}
.tool span{font-size:12px;color:var(--dim);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
.logbox{background:#0a0d12;border:1px solid var(--border);border-radius:10px;padding:12px;font:11px var(--mono);max-height:200px;overflow-y:auto;margin-top:16px}
.logline{padding:1px 0;color:var(--dim)}
.logline .t{color:#6272a4}
.logline.warn{color:var(--warn)}
.logline.error{color:var(--bad)}
.empty{color:var(--dim);text-align:center;padding:40px;border:1px dashed var(--border);border-radius:12px}
.peers{margin-top:14px}
.peers h3{font-size:13px;color:var(--accent);margin-bottom:8px}
.peer{display:flex;align-items:center;gap:10px;background:var(--card);border:1px solid var(--border);border-radius:10px;padding:10px 14px;margin-bottom:6px;flex-wrap:wrap}
.peer b{color:var(--fg)}
.peer .pv{color:var(--dim);font:12px var(--mono)}
.toolbtn{padding:1px 8px !important;font-size:11px !important;border-radius:4px !important}
.toolbtn.off{opacity:.45;text-decoration:line-through;border-color:var(--bad)}
form{display:grid;gap:10px;margin-top:8px}
form label{font-size:12px;color:var(--dim);display:block;margin-bottom:3px}
input{width:100%;background:var(--card2);border:1px solid var(--border);border-radius:8px;padding:8px 10px;color:var(--fg);font:13px var(--mono)}
input:focus{outline:none;border-color:var(--accent)}
.row{display:flex;gap:10px}
.row>div{flex:1}
#addFormWrap{margin-top:16px;background:var(--card2);border:1px solid var(--border);border-radius:12px;padding:16px;display:none}
#addFormWrap.open{display:block}
h4.sec{color:var(--dim);font-size:12px;text-transform:uppercase;letter-spacing:1px;margin:18px 0 8px}
</style>
</head>
<body>
<header>
  <h1>&#9679; MCP <span>Command Center</span></h1>
  <span class="badge ok">UP <b id="stUp">0</b>/<b id="stTotal">0</b></span>
  <span class="badge">tools <b id="stTools">0</b></span>
  <span class="badge">refresh <b id="stRefresh">15</b>s</span>
  <span class="badge">v0.2.1 &mdash; LAN &amp; tools ON/OFF</span>
</header>

<div class="toolbar">
  <button class="primary" onclick="doScan()">&#128269; Scan ports</button>
  <button onclick="doScanNet()">&#127760; Scan r&eacute;seau</button>
  <button onclick="doPeers()">&#129309; D&eacute;tecter pairs</button>
  <button onclick="toggleAdd()">+ Ajouter serveur</button>
  <button onclick="refreshAll()">&#8635; Tout rafra&icirc;chir</button>
  <span class="spacer"></span>
  <span class="badge" id="lastScan">d&eacute;tection auto : HTTP streamable, stdio JSON-RPC</span>
</div>

<div id="addFormWrap">
  <form onsubmit="return addServer(event)">
    <div class="row">
      <div><label>Transport</label>
        <select id="fTransport" onchange="transportChanged()" style="width:100%;background:var(--card2);border:1px solid var(--border);border-radius:8px;padding:8px;color:var(--fg)">
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
      <div><label>Args (s&eacute;par&eacute;s par espace)</label><input id="fArgs" placeholder="dist/index.js --port 3000"></div>
    </div>
    <div style="display:flex;gap:8px;justify-content:flex-end">
      <button type="button" onclick="toggleAdd()">Annuler</button>
      <button type="submit" class="primary">Ajouter &amp; sonder</button>
    </div>
  </form>
</div>

<div id="grid" class="grid"></div>

<div id="peersBox" class="peers"></div>

<dialog id="dlg">
  <div class="d-head">
    <h2 id="dlgTitle">D&eacute;tail</h2>
    <button class="close" onclick="dlg.close()">&times;</button>
  </div>
  <div class="d-body" id="dlgBody"></div>
</dialog>

<div class="logbox" id="logbox"></div>

<script>
let state = null;
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));

async function load() {
  try {
    const r = await fetch('/api/state');
    state = await r.json();
    render();
  } catch(e) { /* UI serveur redemarre */ }
}

function render() {
  if (!state) return;
  const s = state.stats;
  document.getElementById('stUp').textContent = s.up;
  document.getElementById('stTotal').textContent = s.total;
  document.getElementById('stTools').textContent = s.tools;
  document.getElementById('stRefresh').textContent = state.settings.refresh_secs;
  const g = document.getElementById('grid');
  if (!state.servers.length) { g.innerHTML = '<div class="empty">Aucun serveur. Lance un scan ou ajoute-en un.</div>'; }
  else g.innerHTML = state.servers.map(card).join('');
  renderPeers();
  const lb = document.getElementById('logbox');
  lb.innerHTML = state.log.map(l => {
    const d = new Date(l.ts * 1000).toLocaleTimeString('fr-FR');
    return `<div class="logline ${l.level}"><span class="t">${d}</span> ${esc(l.msg)}</div>`;
  }).join('');
  lb.scrollTop = lb.scrollHeight;
}

function card(s) {
  const ti = s.transport === 'http' ? `port ${s.transport_info.port}` : `stdio: ${esc(s.transport_info.program)}`;
  const name = s.server_name ? `${esc(s.server_name)} <span style="color:var(--dim)">v${esc(s.version || '?')}</span>` : '<span style="color:var(--dim)">&mdash;</span>';
  const originTag = s.origin === 'discovered' ? '<span class="tag disc">d&eacute;couvert</span>' :
                    s.origin === 'manual' ? '<span class="tag">manuel</span>' : '<span class="tag">seed</span>';
  return `<div class="card ${s.enabled ? '' : 'disabled'}" onclick="detail('${esc(s.id)}')">
    <div class="card-top">
      <span class="dot ${s.state}"></span>
      <h3>${esc(s.label)}</h3>
      <span class="state ${s.state}">${s.state}</span>
    </div>
    <div class="meta">
      <span class="tag ${s.transport}">${s.transport}</span>
      <span>${ti}</span>
      ${s.latency_ms != null ? `<span>${s.latency_ms} ms</span>` : ''}
      ${s.transport_info.host ? `<span title="serveur distant">&#127760; ${esc(s.transport_info.host)}:${s.transport_info.port}</span>` : ''}
      ${!s.transport_info.host && s.bound_host ? `<span>[${esc(s.bound_host)}]</span>` : ''}
      ${s.tools_disabled.length ? `<span style="color:var(--bad)">${s.tools_disabled.length} tool(s) OFF</span>` : ''}
    </div>
    <div class="meta" style="margin-top:6px">
      ${name}
      <span class="v">${s.tools_count} tools</span>
      ${originTag}
    </div>
    ${s.last_error ? `<div class="err" title="${esc(s.last_error)}">&#9888; ${esc(s.last_error)}</div>` : ''}
    <div class="actions" onclick="event.stopPropagation()">
      ${s.transport === 'stdio' && s.state !== 'UP'
        ? `<button onclick="act('start_stdio','${esc(s.id)}')">Start</button>` : ''}
      ${s.transport === 'stdio' && s.state === 'UP'
        ? `<button class="danger" onclick="act('kill_stdio','${esc(s.id)}')">Stop</button>` : ''}
      <button onclick="detail('${esc(s.id)}')">Tools</button>
      <button onclick="act('refresh','${esc(s.id)}')">Sonder</button>
      <button onclick="act('toggle','${esc(s.id)}')">${s.enabled ? 'D&eacute;sactiver' : 'Activer'}</button>
      <button class="danger" onclick="act('remove','${esc(s.id)}')">Retirer</button>
    </div>
  </div>`;
}

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
  alert('Scan r\u00e9seau lanc\u00e9 (254 h\u00f4tes, 1-2 min). Le journal affichera les d\u00e9tections.');
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
  await load();
  detail(id);
}

function renderPeers() {
  const box = document.getElementById('peersBox');
  if (!state.peers || !state.peers.length) { box.innerHTML = ''; return; }
  const rows = state.peers.map(p => {
    const st = p.stats || {};
    return `<div class="peer">
      <b>&#129309; ${esc(p.ip)}</b>
      <span class="pv">UI :${p.ui_port}</span>
      <span class="pv">${st.up ?? '?'} UP / ${st.total ?? '?'} serveurs</span>
      <button onclick="importPeer('${esc(p.ip)}')">Importer ses serveurs</button>
    </div>`;
  }).join('');
  box.innerHTML = `<h3>Command centers d&eacute;tect&eacute;s sur le r&eacute;seau</h3>${rows}`;
}

function toggleAdd() { document.getElementById('addFormWrap').classList.toggle('open'); }
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
  else { toggleAdd(); await act('refresh', j.id); }
  return false;
}

async function detail(id) {
  const r = await fetch('/api/detail?id=' + encodeURIComponent(id));
  const s = await r.json();
  document.getElementById('dlgTitle').textContent = s.label || id;
  const kv = [
    ['id', s.id], ['transport', s.transport === 'http' ? `HTTP ${s.transport_info.port}${s.transport_info.path}` : `stdio ${s.transport_info.program} ${(s.transport_info.args||[]).join(' ')}`],
    ['origine', s.origin], ['état', s.state], ['pid', s.pid ?? '-'],
    ['bind', s.bound_host ?? '-'], ['serveur MCP', s.server_name ?? '-'],
    ['version', s.version ?? '-'], ['protocole', s.protocol ?? '-'],
    ['latence', s.latency_ms != null ? s.latency_ms + ' ms' : '-'],
    ['dernier vu', s.last_seen ? new Date(s.last_seen * 1000).toLocaleString('fr-FR') : '-'],
    ['erreur', s.last_error ?? '-'],
  ].map(([k, v]) => `<div class="k">${k}</div><div class="v">${esc(v)}</div>`).join('');
  const tools = (s.tools_list || []).map(t =>
    `<div class="tool">
       <button class="toolbtn ${t.disabled ? 'off' : ''}" title="${t.disabled ? 'r\u00e9activer' : 'd\u00e9sactiver'} ce tool"
               onclick="toggleTool('${esc(s.id)}','${esc(t.name).replace(/'/g, "\\'")}')">
         ${t.disabled ? 'OFF' : 'ON'}
       </button>
       <code>${esc(t.name)}</code>
       <span title="${esc(t.desc)}">${esc(t.desc)}</span>
     </div>`).join('');
  const nOn = (s.tools_list || []).filter(t => !t.disabled).length;
  document.getElementById('dlgBody').innerHTML =
    `<div class="kv">${kv}</div><div class="tools"><h4>Tools — ${nOn} actifs / ${(s.tools_list || []).length} (${(s.tools_list || []).length - nOn} OFF)</h4>${tools || '<span style="color:var(--dim)">aucun</span>'}</div>`;
  dlg.showModal();
}

load();
setInterval(load, 5000);
</script>
</body>
</html>
"#;
