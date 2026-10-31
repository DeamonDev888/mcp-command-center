/* Code d'accès API : demandé à l'utilisateur (dialogue de déverrouillage) puis
   gardé en localStorage — JAMAIS injecté par le serveur dans la page (audit
   2026-10-24 : quiconque joint le port lisait le code sur / et contournait
   toute l'auth). En bind loopback aucun code n'est requis : le dialogue
   n'apparaît jamais. Chaque requête /api/ présente le code (X-MCC-Access) ;
   un 401 rouvre le dialogue (code tourné ou faux). */
const accessCode = () => { try { return localStorage.getItem('mcc.access') || ''; } catch (e) { return ''; } };
const MCC_VERSION = '__MCC_VERSION__';

/* ===== Thème sombre/clair (⚙ Réglages) =====
   Préférence locale (mcc.theme) appliquée AVANT le premier rendu par le
   script inline anti-flash du <body> — ici on bascule et on marque l'actif. */
// Applique le thème SANS toucher à la préférence enregistrée.
// applyTheme est aussi appelé à l'ouverture des Réglages (pour aligner les
// boutons) : écrire ici écrasait le choix de l'utilisateur — le thème clair
// ne tenait jamais (il revenait à « sombre » à la réouverture des Réglages).
function applyTheme(t) {
  document.documentElement.dataset.theme = t === 'light' ? 'light' : 'dark';
  const light = document.documentElement.dataset.theme === 'light';
  const dk = document.getElementById('tDark');
  const lt = document.getElementById('tLight');
  if (dk) dk.classList.toggle('sel', !light);
  if (lt) lt.classList.toggle('sel', light);
}
// Choix EXPLICITE de l'utilisateur : applique ET enregistre.
function applyThemeAndSave(t) {
  applyTheme(t);
  try { localStorage.setItem('mcc.theme', document.documentElement.dataset.theme); } catch (e) {}
}
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
    updateLogoutBtn();
    load();
  } else {
    msg.textContent = '\u2717 code refus\u00e9 — voir settings.api_access du config.json du serveur';
  }
  return false;
}

// « Oublier le code » : rien ne doit rester dans le navigateur après un poste
// partagé ou une rotation de code côté serveur.
function forgetCode() {
  try { localStorage.removeItem('mcc.access'); } catch (e) {}
  const msg = document.getElementById('loginMsg');
  if (msg) msg.textContent = '\u2713 code effac\u00e9 de ce navigateur';
  updateLogoutBtn();
}

// Le bouton de déconnexion n'apparaît que si un code est stocké.
function updateLogoutBtn() {
  const b = document.getElementById('tbLogout');
  if (b) b.hidden = !accessCode();
}
function logout() {
  forgetCode();
  const d = document.getElementById('loginDlg');
  if (d) { if (!d.open) d.showModal(); }
  load();
}

let state = null;
let selectedId = null;
let detailCache = null;
let detailTab = 'outils';
const esc = s => String(s ?? '').replace(/[&<>"']/g, c => ({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const js = s => esc(JSON.stringify(String(s ?? '')));
let overviewFilter = null; // filtre de tuile d'accueil : 'up' = ne montrer que les UP

/* ===== Icônes =============================================================
   Un seul jeu SVG inline (trait 1.5, couleur courante, zéro emoji) : rendu
   identique quelle que soit la version d'Windows et alignement typographique
   propre dans une interface monochrome. Chaque icône est inlinée par ic() —
   pas de référence à un symbole (guillemet-double suivi de dièse interdit
   dans la raw string Rust qui embarque cette page), attributs en simples
   quotes comme le reste des SVG de la page. */
const ICONS = {
  search: "<circle cx='11' cy='11' r='7'/><path d='m21 21-4.3-4.3'/>",
  globe: "<circle cx='12' cy='12' r='8.5'/><path d='M3.5 12h17M12 3.5c2.4 2.4 3.8 5.3 3.8 8.5s-1.4 6.1-3.8 8.5c-2.4-2.4-3.8-5.3-3.8-8.5s1.4-6.1 3.8-8.5Z'/>",
  monitor: "<rect x='2.5' y='4' width='19' height='13' rx='2'/><path d='M8 21h8M12 17v4'/>",
  cart: "<circle cx='9' cy='20' r='1.4'/><circle cx='17' cy='20' r='1.4'/><path d='M2.5 3.5h2.2l2.5 11.5h10.3l2-8H6'/>",
  gear: "<circle cx='12' cy='12' r='3.1'/><path d='M12 2.8v2.6M12 18.6v2.6M2.8 12h2.6M18.6 12h2.6M5.5 5.5l1.9 1.9M16.6 16.6l1.9 1.9M18.5 5.5l-1.9 1.9M7.4 16.6l-1.9 1.9'/>",
  refresh: "<path d='M21 12a9 9 0 1 1-2.6-6.4L21 8'/><path d='M21 3v5h-5'/>",
  power: "<path d='M18.4 6.6a9 9 0 1 1-12.8 0'/><path d='M12 2.5V12'/>",
  lock: "<rect x='4.5' y='10.5' width='15' height='10' rx='2'/><path d='M8 10.5V7a4 4 0 0 1 8 0v3.5'/>",
  zap: "<path d='M13 2.5 4.5 13.5H11l-1 8 8.5-11H12l1-8Z'/>",
  star: "<path d='m12 3.6 2.5 5.1 5.7.8-4.1 4 1 5.6L12 16.4l-5.1 2.7 1-5.6-4.1-4 5.7-.8Z'/>",
  eye: "<path d='M2.5 12S6 5.5 12 5.5 21.5 12 21.5 12 18 18.5 12 18.5 2.5 12 2.5 12Z'/><circle cx='12' cy='12' r='3'/>",
  eyeOff: "<path d='M4 4l16 16'/><path d='M10.3 5.7a9.8 9.8 0 0 1 1.7-.2c6 0 9.5 6.5 9.5 6.5a17.6 17.6 0 0 1-3 3.8M6.7 6.7C4.1 8.4 2.5 12 2.5 12S6 18.5 12 18.5c1.3 0 2.5-.3 3.6-.8'/><path d='M9.9 9.9a3 3 0 0 0 4.2 4.2'/>",
  play: "<path d='M7 4.5 19 12 7 19.5Z'/>",
  stop: "<rect x='6' y='6' width='12' height='12' rx='1.5'/>",
  pause: "<path d='M9 5v14M15 5v14'/>",
  trash: "<path d='M3.5 6h17M8.5 6V4.5A1.5 1.5 0 0 1 10 3h4a1.5 1.5 0 0 1 1.5 1.5V6M19 6l-.8 13a2 2 0 0 1-2 1.9H7.8a2 2 0 0 1-2-1.9L5 6'/><path d='M10 10.5v6M14 10.5v6'/>",
  copy: "<rect x='9' y='9' width='12' height='12' rx='2'/><path d='M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1'/>",
  tag: "<path d='m3 11 8-8h9v9l-8.5 8.5a2 2 0 0 1-2.8 0L3 13.8a2 2 0 0 1 0-2.8Z'/><circle cx='16.5' cy='7.5' r='1.3'/>",
  link: "<path d='M10 13a5 5 0 0 0 7.5.5l3-3a5 5 0 0 0-7-7l-1.7 1.7'/><path d='M14 11a5 5 0 0 0-7.5-.5l-3 3a5 5 0 0 0 7 7l1.7-1.7'/>",
  terminal: "<path d='m4 17 6-5-6-5'/><path d='M12 19h8'/>",
  folder: "<path d='M3.5 7A1.5 1.5 0 0 1 5 5.5h4.2l1.8 2h9A1.5 1.5 0 0 1 21.5 9v8a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 17Z'/>",
  external: "<path d='M14 4h6v6M20 4l-9 9'/><path d='M19 14v5a1.5 1.5 0 0 1-1.5 1.5h-12A1.5 1.5 0 0 1 4 19V7.5A1.5 1.5 0 0 1 5.5 6H10'/>",
  activity: "<path d='M22 12h-4l-3 8L9 4l-3 8H2'/>",
  check: "<path d='m4.5 12.5 5 5L19.5 7'/>",
  x: "<path d='m6 6 12 12M18 6 6 18'/>",
  bell: "<path d='M6 9a6 6 0 0 1 12 0c0 5 2 6.5 2 6.5H4S6 14 6 9Z'/><path d='M10 19.5a2.2 2.2 0 0 0 4 0'/>",
  upload: "<path d='M12 15.5V4m0 0L7.5 8.5M12 4l4.5 4.5'/><path d='M4 16.5V19a1.5 1.5 0 0 0 1.5 1.5h13A1.5 1.5 0 0 0 20 19v-2.5'/>",
  download: "<path d='M12 4v11.5m0 0L7.5 11M12 15.5l4.5-4.5'/><path d='M4 16.5V19a1.5 1.5 0 0 0 1.5 1.5h13A1.5 1.5 0 0 0 20 19v-2.5'/>",
  warn: "<path d='M12 3.5 2.5 20h19L12 3.5Z'/><path d='M12 10v4.5M12 17.1v.1'/>",
  save: "<path d='M5 3.5h11L20.5 8v11a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 19V5A1.5 1.5 0 0 1 5 3.5Z'/><path d='M8 3.5V8h7V3.5M7.5 20.5v-6h9v6'/>",
};
const ic = n => `<svg class='ic' viewBox='0 0 24 24' aria-hidden='true'>${ICONS[n] || ''}</svg>`;

async function load() {
  if (document.hidden) return; // fenêtre masquée (tray) : polling suspendu — CPU/mémoire au repos
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
  // Alerte réseau : la passerelle LAN ne répond pas → les serveurs distants
  // sont injoignables (VPN le plus souvent). Une pastille rouge dans la barre
  // du Journal — visible même le journal replié — remplace l'ancien bandeau
  // pleine largeur qui poussait toute l'interface ; tooltip + clic portent
  // l'explication complète au lieu de l'imposer en permanence.
  const nw = document.getElementById('netWarn');
  if (nw) {
    const off = state.lan_ok === false;
    nw.hidden = !off;
    if (off && !nw.innerHTML) nw.innerHTML = ic('warn') + '<span>R&eacute;seau local injoignable</span>';
  }
  document.getElementById('stUp').textContent = s.up;
  document.getElementById('stTotal').textContent = s.total;
  document.getElementById('stTools').textContent = s.tools;
  document.getElementById('stRefresh').textContent = state.settings.refresh_secs;
  armPolling();
  const au = document.getElementById('stAuto');
  if (au) {
    au.innerHTML = ic('power') + '<span>Auto-boot ' + (state.autostart ? 'ON' : 'OFF') + '</span>';
    au.classList.toggle('auto-off', !state.autostart);
  }
  const upd = document.getElementById('stUpdate');
  if (state.update_available && state.latest_version) {
    upd.hidden = false;
    // libellé épuré : « Mettre à jour » — les versions vivent en survol
    upd.innerHTML = ic('download') + ' Mettre &agrave; jour';
    upd.title = 'Mettre à jour vers v' + state.latest_version + ' (installée : v' + MCC_VERSION
      + ') — clic : téléchargement direct pour cette machine';
  } else upd.hidden = true;
  renderServers();
  renderPeers();
  renderLog();
  if (!selectedId) renderOverview();
}

/* ===== Explorateur ===== */
/* Clé de groupement : 'local' (loopback de cette machine) ou host/IP de la
   machine distante (LAN/VPS). L'explorateur affiche les locaux à plat puis
   UN GROUPE REPLIABLE PAR MACHINE distante. */
function hostOf(s) {
  if (s.transport !== 'http') return 'local';
  return s.transport_info.host || 'local';
}
/* Helper global : serveur par id (applySort a son propre byId local, ombre légale). */
function byId(id) {
  return state && state.servers ? state.servers.find(s => s.id === id) : null;
}
function groupKeys() {
  // hosts des serveurs distants (ordre d'affichage) + machines enregistrées
  // (ssh_hosts) même sans serveur importé — le groupe vide montre où ranger.
  const keys = [];
  state.servers.forEach(s => { const k = hostOf(s); if (k !== 'local' && !keys.includes(k)) keys.push(k); });
  (state.settings.ssh_hosts || []).forEach(h => { if (!keys.includes(h.ip)) keys.push(h.ip); });
  return keys;
}
function srvRow(s) {
  const ti = s.transport === 'http'
    ? (s.transport_info.host ? `${esc(s.transport_info.host)}:` : '') + s.transport_info.port
    : 'stdio &middot; ' + esc(s.transport_info.program);
  const off = s.tools_disabled.length ? ` &middot; ${s.tools_disabled.length} off` : '';
  /* Ligne 1 : état + nom + actions (le nom ne se bat plus avec les badges).
     Ligne 2 : état textuel, transport, badges langage/transport/framework,
     compteur d'outils. Les badges sont non rétractables : à très faible
     largeur la fin de la ligne 2 se coupe, jamais le nom ni les actions. */
  return `<div class="srow ${selectedId===s.id?'sel':''} ${s.enabled?'':'off'}" draggable="true" data-id="${esc(s.id)}"
    onclick="select(${js(s.id)})"
    ondragstart="rowDragStart(event,${js(s.id)})" ondragover="rowDragOver(event)"
    ondragleave="rowDragLeave(event)" ondrop="rowDrop(event)" ondragend="rowDragEnd(event)">
    <div class="l1"><span class="dot ${s.state}"></span><span class="nm" title="${esc(s.label)}">${esc(s.label)}</span>${rowFav(s)}${rowAuto(s)}${rowCtl(s)}${rowToggle(s)}</div>
    <div class="l2"><span class="st ${esc(s.state)}">${esc(s.state)}</span> ${ti}${off}${langChip(s)}${typeBadge(s)}${frameworkBadge(s)}${s.transport==='stdio'?'<span class="legacy" title="Legacy : serveur stdio uniquement (pas de transport HTTP)">legacy</span>':''}<span class="cnt">${s.tools_count}</span></div>
  </div>`;
}
/* Replis des groupes mémorisés (localStorage) : 'local' + une clé par machine. */
function groupClosed(key) {
  try { return localStorage.getItem('mcc.group.' + key) === '1'; } catch (e) { return false; }
}
function toggleGroup(key) {
  try { localStorage.setItem('mcc.group.' + key, groupClosed(key) ? '0' : '1'); } catch (e) {}
  renderServers();
}
function groupHeadHtml(key, label, up, total, machine) {
  const tip = machine
    ? `Machine distante ${esc(label)} &mdash; cliquer pour replier/d&eacute;plier`
    : 'Serveurs MCP locaux &agrave; cette machine (loopback) &mdash; cliquer pour replier/d&eacute;plier';
  const act = machine
    ? `<button class="gact" title="Oublier cette machine (les serveurs d&eacute;j&agrave; import&eacute;s restent)" onclick="event.stopPropagation();sshForget(${js(label)})">${ic('trash')}</button>` : '';
  return `<div class="grphead ${groupClosed(key)?'closed':''}" onclick="toggleGroup(${js(key)})"
    ondragover="groupDragOver(event)" ondragleave="groupDragLeave(event)" ondrop="groupDrop(event,${js(key)})"
    title="${tip}"><span class="gchev">&#9662;</span><span class="gname">${esc(label)}</span>${act}<span class="gcount">${up}/${total}</span></div>`;
}
/* Filtre courant de l'explorateur : texte (champ #srvFilter) + filtre de tuile
   d'accueil (overviewFilter). Renvoie true si le serveur doit être affiché. */
/* Serveurs masqués de l'affichage — VOLONTAIREMENT en mémoire seulement :
   ni config.json ni localStorage. Fermer l'app (ou recharger la page) les fait
   réapparaître, comme demandé. La supervision moteur continue de les sonder. */
const hiddenServers = new Set();

function srvVisible(s, q) {
  if (hiddenServers.has(s.id)) return false;
  if (overviewFilter === 'up' && s.state !== 'UP') return false;
  if (q) {
    const hay = (s.label + ' ' + s.id + ' ' + (s.framework || '') + ' '
      + (s.transport === 'http' ? (s.transport_info.port ?? '') : '') + ' ' + (s.state || '')).toLowerCase();
    if (!hay.includes(q)) return false;
  }
  return true;
}
function renderServers() {
  const localBox = document.getElementById('srvList');
  const groupsBox = document.getElementById('remoteGroups');
  const label = document.getElementById('remoteLabel');
  const count = document.getElementById('localCount');
  const localHead = document.getElementById('localHead');
  const clr = document.getElementById('srvFilterClear');
  const qRaw = (document.getElementById('srvFilter')?.value || '').trim();
  const q = qRaw.toLowerCase();
  if (clr) clr.hidden = !qRaw;
  if (!state.servers.length) {
    localHead.hidden = true; count.textContent = '';
    localBox.innerHTML = '<div class="empty-sec">aucun serveur — lance un scan</div>';
    label.hidden = true; groupsBox.innerHTML = '';
    const hb = document.getElementById('hiddenBar');
    if (hb) hb.hidden = true;
    return;
  }
  // Les compteurs de groupe comptent TOUS les serveurs affichables du groupe
  // (les masqués ne comptent ni dans les lignes ni dans les comptes) ; seules
  // les lignes visibles passent ensuite par le filtre texte.
  const shown = state.servers.filter(s => !hiddenServers.has(s.id));
  const localAll = shown.filter(s => hostOf(s) === 'local');
  const local = localAll.filter(s => srvVisible(s, q));
  const remotes = shown.filter(s => hostOf(s) !== 'local');
  const byKey = {};
  remotes.forEach(s => { const k = hostOf(s); (byKey[k] = byKey[k] || []).push(s); });
  // barre « masqués — afficher » : le seul chemin pour les réafficher
  const hb = document.getElementById('hiddenBar');
  if (hb) {
    const n = hiddenServers.size;
    hb.hidden = !n;
    if (n) hb.innerHTML = ic('eyeOff') + ' ' + n + ' serveur' + (n > 1 ? 's' : '')
      + ' masqu\u00e9' + (n > 1 ? 's' : '') + ' (reviennent \u00e0 la r\u00e9ouverture) <b>afficher</b>';
  }
  const upOf = rows => rows.filter(s => s.state === 'UP').length;
  /* Favoris (⭐) épinglés en tête de chaque section, quel que soit le tri :
     tri stable — les épinglées gardent leur ordre relatif entre elles. */
  const favFirst = rows => rows.slice().sort((a, b) => ((b.favorite === true) - (a.favorite === true)));
  const noneMatch = '<div class="empty-sec">aucun serveur ne correspond au filtre</div>';
  localHead.hidden = false;
  count.textContent = upOf(localAll) + '/' + localAll.length;
  localBox.innerHTML = local.length ? favFirst(local).map(srvRow).join('')
    : (localAll.length ? noneMatch : '<div class="empty-sec">aucun serveur — lance un scan</div>');
  const keys = groupKeys();
  label.hidden = !keys.length;
  groupsBox.innerHTML = keys.map(k => {
    const all = byKey[k] || [];
    const rows = all.filter(s => srvVisible(s, q));
    const body = rows.length ? favFirst(rows).map(srvRow).join('')
      : (all.length ? noneMatch
        : '<div class="empty-sec empty-indent">aucun serveur import&eacute; de cette machine</div>');
    return groupHeadHtml(k, k, upOf(all), all.length, true)
      + `<div class="grpbody mbody ${groupClosed(k)?'closed':''}">${body}</div>`;
  }).join('');
}

// Chip du langage MCP (inféré : commande stdio ou en-têtes HTTP) — monogramme coloré façon logo
const LANG_META = {
  node:   { t: 'JS',  c: '#dcc94f', n: 'Node.js / TypeScript' },
  python: { t: 'PY',  c: '#3fb950', n: 'Python' },
  rust:   { t: 'RS',  c: '#f97316', n: 'Rust' },
  go:     { t: 'GO',  c: '#00add8', n: 'Go' },
  java:   { t: 'JV',  c: '#e76f00', n: 'Java' },
  dotnet: { t: 'C#',  c: '#512bd4', n: '.NET' },
  ruby:   { t: 'RB',  c: '#cc342d', n: 'Ruby' },
  php:    { t: 'PHP', c: '#777bb4', n: 'PHP' },
};
// Glyphe Rust : « R » dans un engrenage (lecture de l'icône .rs des éditeurs).
// NB : attributs en simples quotes — guillemet-double suivi de dièse interdit
// dans ce bloc (cette séquence fermerait la raw string Rust d'INDEX_HTML).
const RUST_LOGO = "<svg class='rustlogo' viewBox='0 0 24 24' aria-hidden='true'><g stroke='#f97316' stroke-width='1.4' stroke-linecap='round' fill='none'><circle cx='12' cy='12' r='7.9'/><path d='M19.9 12h2M17.6 17.6l1.4 1.4M12 19.9v2M6.4 17.6 5 19M4.1 12h-2M6.4 6.4 5 5M12 4.1v-2M17.6 6.4 19 5'/></g><text x='12' y='16.2' text-anchor='middle' font-size='12' font-weight='800' font-family='inherit' fill='#f97316'>R</text></svg>";
/* Logos langages embarqués (marques officielles en SVG minimal) :
   JS = carré jaune officiel ECMAScript ; Python = les deux serpents (la forme
   bleue est répétée pivotée de 180°, comme le vrai logo) ; Go = tuile cyan
   officielle ; .NET = carré violet officiel ; Java = la tasse fumante ;
   Ruby = le gem facetté ; PHP = l'ellipse violette. NB : attributs en simples
   quotes — guillemet-double suivi de dièse interdit dans ce bloc (fermerait la
   raw string Rust d'INDEX_HTML). */
const PY_PATH = "M11.8 2.1c-2.6 0-4.7 1.1-4.7 3.2v2.6h4.9v.9H5.1c-2.1 0-3.8 1.6-3.8 4.4 0 2.9 1.5 4.5 3.6 4.5h2v-2.8c0-1.9 1.6-3.5 3.5-3.5h4.4c1.6 0 2.9-1.3 2.9-2.9V5.3c0-2.1-2.1-3.2-4.7-3.2h-1.2z";
const LANG_LOGOS = {
  rust: RUST_LOGO,
  node: "<svg class='langlogo' viewBox='0 0 24 24' aria-hidden='true'><rect x='1.5' y='1.5' width='21' height='21' rx='3.5' fill='#dcc94f'/><text x='12' y='16.6' text-anchor='middle' font-size='10.5' font-weight='800' font-family='inherit' fill='#221e08'>JS</text></svg>",
  python: "<svg class='langlogo' viewBox='0 0 24 24' aria-hidden='true'><g><path fill='#3776ab' d='" + PY_PATH + "'/><circle cx='9.7' cy='5' r='.95' fill='#fff'/><g transform='rotate(180 12 12)'><path fill='#ffd43b' d='" + PY_PATH + "'/><circle cx='9.7' cy='5' r='.95' fill='#fff'/></g></g></svg>",
  go: "<svg class='langlogo' viewBox='0 0 24 24' aria-hidden='true'><rect x='1.5' y='1.5' width='21' height='21' rx='3.5' fill='#00add8'/><text x='12' y='16.2' text-anchor='middle' font-size='10' font-weight='800' font-family='inherit' fill='#fff'>GO</text></svg>",
  java: "<svg class='langlogo' viewBox='0 0 24 24' aria-hidden='true'><path fill='#f89820' d='M12.4 2.4c-.6 1.4.1 2.4 1.2 3.1 1.1.8 1.7 1.8 1.3 3 1.2-.7 1.6-2.1.8-3.3-.6-1-2-1.7-3.3-2.8z'/><path fill='#f89820' d='M9.4 7c-.5 1.1.1 2 1.1 2.5.9.6 1.4 1.4 1 2.4 1.1-.6 1.5-1.8.8-2.8-.6-.9-1.8-1.4-2.9-2.1z'/><path fill='#5382a1' d='M7.2 12.1h9.3v4.7c0 2.3-1.9 4.2-4.2 4.2h-.9c-2.3 0-4.2-1.9-4.2-4.2z'/><path fill='none' stroke='#5382a1' stroke-width='1.3' stroke-linecap='round' d='M16.9 13.4h1.2c1.1 0 2 .9 2 2s-.9 2-2 2h-1.3'/></svg>",
  dotnet: "<svg class='langlogo' viewBox='0 0 24 24' aria-hidden='true'><rect x='1.5' y='1.5' width='21' height='21' rx='3.5' fill='#512bd4'/><text x='12' y='15.4' text-anchor='middle' font-size='6.4' font-weight='700' font-family='inherit' fill='#fff'>.NET</text></svg>",
  ruby: "<svg class='langlogo' viewBox='0 0 24 24' aria-hidden='true'><path fill='#cc342d' d='M7.2 4.5h9.6l3.7 4.6L12 20.5 3.5 9.1z'/><path fill='#e9655c' d='M7.2 4.5h9.6L12 9.5z'/><path fill='#a72b24' d='M3.5 9.1h17L12 20.5z' opacity='.4'/><path fill='none' stroke='#fff' stroke-opacity='.4' stroke-width='.7' d='M7.2 4.5 12 9.5l4.8-5M12 9.5v11M3.5 9.1h17'/></svg>",
  php: "<svg class='langlogo' viewBox='0 0 24 24' aria-hidden='true'><ellipse cx='12' cy='12' rx='10.6' ry='6.2' fill='#777bb4'/><text x='12' y='14.2' text-anchor='middle' font-size='6.2' font-weight='700' font-style='italic' font-family='inherit' fill='#fff'>php</text></svg>",
};
function langChip(s) {
  const logo = LANG_LOGOS[s.lang];
  if (!logo) return '<span class="langchip unk" title="Langage non d\u00e9tect\u00e9 (ni commande ni en-t\u00eates ni process)">?</span>';
  const m = LANG_META[s.lang];
  const name = m ? m.n : s.lang;
  return `<span class="rustwrap" title="Langage inféré : ${esc(name)}">${logo}</span>`;
}

// Badge du type de MCP : STDIO / HTTP / WS — trois modes, point.
// HTTP couvre le streamable JSON et les réponses encodées SSE (FastMCP).
function typeBadge(s) {
  // Trois modes affichés, point : STDIO / HTTP / WS. Le streamable JSON et les
  // réponses encodées SSE (FastMCP) sont tous les deux « HTTP » — pas de badge
  // séparé. Le framework (LiteMCP/FastMCP) a son propre badge.
  if (s.transport === 'stdio') return '<span class="tag stdio" title="Transport stdio (process local)">STDIO</span>';
  const lan = s.transport_info.host ? '\u00b7LAN' : '';
  const via = s.via_ssh ? '\u00b7SSH' : '';
  const viaTip = s.via_ssh ? ' — sondé via SSH depuis la machine (interne)' : '';
  const suffix = (lan ? ' — serveur distant (LAN)' : '') + viaTip;
  if (s.ws) return `<span class="tag ws" title="WebSocket — le serveur accepte une upgrade WS (101)${suffix}">WS${lan}${via}</span>`;
  return `<span class="tag http" title="HTTP streamable${suffix}">HTTP${lan}${via}</span>`;
}

// Badge du framework détecté par fingerprint :
// - litemcp : initialize répondu 2026-07-28 SANS négocier + sonde /health « ok » ;
// - fastmcp : réponses initialize encodées SSE + endpoint /sse legacy co-servi
//   (signature httpStream de mcp-proxy/FastMCP). La spec MCP utilisée par le
//   serveur (s.protocol) est exposée dans le tooltip et le détail.
// Table des frameworks MCP (id → {label, lang}) chargée depuis /api/frameworks :
// une seule source de vérité côté Rust, pas de liste dupliquée en JS.
let FRAMEWORKS = {};
async function loadFrameworks() {
  try {
    const r = await fetch('/api/frameworks');
    if (!r.ok) return;
    const j = await r.json();
    (j.frameworks || []).forEach(f => { FRAMEWORKS[f.id] = f; });
    if (state) { render(); if (selectedId && detailCache) renderDetail(); }
  } catch (e) { /* hors ligne : les badges afficheront l'id brut */ }
}
function frameworkBadge(s) {
  if (!s.framework) return '';
  const f = FRAMEWORKS[s.framework] || {label: s.framework, lang: s.lang || '?'};
  const lan = s.transport_info && s.transport_info.host ? ' — serveur distant (LAN)' : '';
  const tip = `Framework ${esc(f.label)} · ${esc(f.lang)}${lan}`;
  // LiteMCP est le seul badge qui garde un halo (classe .litemcp, animation glow).
  if (s.framework === 'litemcp') {
    return `<span class="tag litemcp" title="${tip}">${esc(f.label)}</span>`;
  }
  // Les autres badges prennent la couleur du LANGAGE du framework, comme les
  // puces de code : Rust orange, Python vert, JS jaune, Go cyan…
  const m = LANG_META[f.lang] || LANG_META[s.lang];
  if (!m) return `<span class="tag fw" title="${tip}">${esc(f.label)}</span>`;
  return `<span class="tag fw" style="color:${m.c};border-color:${m.c}66;background:${m.c}14" title="${tip}">${esc(f.label)}</span>`;
}

function rowFav(s) {
  const on = !!s.favorite;
  return `<button class="rowbtn fav ${on?'on':'off'}" title="${on
    ? 'Retirer des favoris (l\u2019\u00e9pingle maintient la ligne en haut de sa section)'
    : '\u00c9pingler en favori : la ligne passe en haut de sa section, quel que soit le tri'}"
    onclick="event.stopPropagation();setFavorite(${js(s.id)},${on?'false':'true'})">${ic('star')}</button>`;
}

function rowToggle(s) {
  return `<button class="sw ${s.enabled?'on':'off'}" title="${s.enabled?'D\u00e9sactiver ce serveur (arr\u00eate son process)':'Activer ce serveur (le d\u00e9marre si commande connue)'}" onclick="event.stopPropagation();act('toggle',${js(s.id)})">${s.enabled?'ON':'OFF'}</button>`;
}

function rowCtl(s) {
  if (!s.can_start) return '';
  const up = s.state === 'UP';
  return `<button class="rowbtn" title="${up?'Arr\u00eater':'D\u00e9marrer'} ce serveur" onclick="event.stopPropagation();act('${up?'kill':'start'}',${js(s.id)})">${up?ic('stop'):ic('play')}</button>`;
}

// Autoboot PAR SERVEUR : relance automatique de CE serveur au démarrage
// de MCP Control et après un crash. Bouton ⚡ visible dans la ligne et le détail.
function rowAuto(s) {
  if (!s.can_start) return '';
  const on = s.auto_restart;
  return `<button class="rowbtn auto ${on?'on':'off'}" title="${on
    ? 'Autoboot ACTIV\u00c9 : ce serveur est relanc\u00e9 au d\u00e9marrage et apr\u00e8s un crash \u2014 cliquer pour d\u00e9sactiver'
    : 'Autoboot D\u00c9SACTIV\u00c9 : ce serveur ne sera pas relanc\u00e9 automatiquement \u2014 cliquer pour activer'}"
    onclick="event.stopPropagation();setAuto(${js(s.id)},${on?'false':'true'})">${ic('zap')}</button>`;
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
  // un serveur distant ne se déplace pas dans la zone plate locale
  if (hostOf(byId(dragId)) !== 'local') return;
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
  // un serveur distant ne se déplace pas dans la zone plate locale
  if (hostOf(byId(moved)) !== 'local') return;
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
/* Drop d'une ligne de serveur sur l'EN-TÊTE d'un groupe machine : re-host du
   serveur (sa clé de groupe = le host du groupe). Cas d'usage : une machine a
   changé d'IP, ou une importation a posé le mauvais host — on déplace la ligne
   vers la bonne machine sans réimporter. */
function groupDragOver(ev) {
  if (!dragId) return;
  ev.preventDefault();
  ev.dataTransfer.dropEffect = 'move';
  ev.currentTarget.classList.add('drag-over-group');
}
function groupDragLeave(ev) {
  ev.currentTarget.classList.remove('drag-over-group');
}
async function groupDrop(ev, key) {
  ev.preventDefault();
  ev.currentTarget.classList.remove('drag-over-group');
  const moved = dragId;
  dragId = null;
  if (!moved) return;
  const s = byId(moved);
  if (!s || s.transport !== 'http') { alert('Seuls les serveurs HTTP peuvent \u00eatre h\u00e9berg\u00e9s par une machine distante.'); return; }
  if (!confirm('H\u00e9berger « ' + (s.label || moved) + ' » sur ' + key + ' ? (le host du transport devient ' + key + ')')) return;
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'set_host', id: moved, host: key})});
  const j = await r.json().catch(() => ({}));
  if (!j.ok) alert('set_host : ' + (j.error || '\u00e9chec'));
  await load();
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
  // Tri à plat sur l'ordre persisté — chaque groupe (locaux, puis machines)
  // affiche ses serveurs dans cet ordre commun, jamais un tri global mélangé.
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
  } else if (mode === 'framework') {
    // Regroupe par framework MCP (label humain), les serveurs sans framework
    // identifiable en fin de liste. À l'intérieur d'un framework : A→Z.
    const fwKey = s => {
      const f = FRAMEWORKS[s.framework];
      return f ? f.label : (s.framework || '\uffff');
    };
    ids.sort((a, b) => frCmp(fwKey(byId[a]), fwKey(byId[b]))
      || frCmp(byId[a].label, byId[b].label));
  } else if (mode === 'transport') {
    // stdio → HTTP → WS (WS est un HTTP avec upgrade), puis A→Z.
    const tRank = s => (s.transport === 'stdio' ? 0 : (s.ws ? 2 : 1));
    ids.sort((a, b) => (tRank(byId[a]) - tRank(byId[b]))
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
      <div class="l2">UI :${esc(p.ui_port)} &mdash; ${esc(st.up ?? '?')} UP / ${esc(st.total ?? '?')} serveurs</div>
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
/* Tuiles cliquables : « serveurs UP » filtre l'explorateur sur les UP,
   « serveurs suivis » remet l'affichage complet, « tools actifs » applique le
   tri par nombre d'outils, « machines distantes » ouvre le dialog SSH. */
function tileFilter(mode) {
  overviewFilter = (mode === 'up' && overviewFilter !== 'up') ? 'up' : null;
  renderServers();
  if (!selectedId) renderOverview();
}
function tileSortTools() {
  const sel = document.getElementById('sortSel');
  if (sel) sel.value = 'tools';
  applySort('tools');
}
function clearFilter() {
  const inp = document.getElementById('srvFilter');
  if (inp) inp.value = '';
  renderServers();
}
function renderOverview() {
  const s = state.stats;
  const nbMachines = (state.settings.ssh_hosts || []).length;
  document.getElementById('content').innerHTML = `
    <div class="tiles">
      <button class="tile${overviewFilter === 'up' ? ' sel' : ''}" onclick="tileFilter('up')"
        title="Cliquer : ne montrer que les serveurs UP dans l'explorateur (re-cliquer pour tout afficher)">
        <div class="n ok">${s.up}</div><div class="lbl">serveurs UP</div></button>
      <button class="tile${overviewFilter ? '' : ' sel'}" onclick="tileFilter('all')"
        title="Cliquer : réafficher tous les serveurs dans l'explorateur">
        <div class="n">${s.total}</div><div class="lbl">serveurs suivis</div></button>
      <button class="tile" onclick="tileSortTools()"
        title="Cliquer : trier l'explorateur par nombre d'outils">
        <div class="n">${s.tools}</div><div class="lbl">tools actifs</div></button>
      <button class="tile" onclick="openMachineDlg()"
        title="Cliquer : gérer les machines distantes (SSH)">
        <div class="n">${nbMachines}</div><div class="lbl">machines distantes</div></button>
    </div>
    <div class="welcome">
      <div class="big">Centre de contr&ocirc;le de tes serveurs MCP</div>
      <div class="hint">S&eacute;lectionne un serveur dans l'explorateur pour voir ses tools, ses infos et les basculer ON/OFF un par un.</div>
      <div class="qa">
        <button onclick="doScan()">${ic('search')} Scanner les ports locaux</button>
        <button onclick="doScanNet()">${ic('globe')} Scanner le r&eacute;seau</button>
        <button onclick="openMachineDlg()">${ic('monitor')} Choisir une machine</button>
      </div>
      <div class="hint hint-gap">Le scan utilise la plage configur&eacute;e. Pour une machine du LAN : connexion SSH par cl&eacute;,
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
  // Le rendu tourne toutes les 5 s : si l'utilisateur tape dans le formulaire
  // Démarrage (set_start_cmd), on ne détruit pas le DOM sous ses doigts —
  // le prochain cycle réaffichera l'état à jour.
  const aEl = document.activeElement;
  if (aEl && (aEl.id === 'scProgram' || aEl.id === 'scArgs' || aEl.id === 'scWorkdir')) return;
  // Hors saisie active : conserver ce qui a déjà été tapé dans ces champs.
  const typed = {};
  ['scProgram', 'scArgs', 'scWorkdir'].forEach(idOld => {
    const el = document.getElementById(idOld);
    if (el && el.value) typed[idOld] = el.value;
  });
  const ti = s.transport === 'http'
    ? `HTTP ${s.transport_info.host ? s.transport_info.host + ':' : ''}${s.transport_info.port}${s.transport_info.path}`
    : `stdio \u00b7 ${s.transport_info.program} ${(s.transport_info.args || []).join(' ')}`;
  /* Ligne clé/valeur : toute valeur informative se copie d'un clic (pid,
     commande, url…). Tiret cadratin pour « pas de valeur » — cohérent partout. */
  const kvRow = (k, v) => {
    const cpy = v && v !== '—';
    return `<div class="k">${k}</div><div class="v${cpy ? ' cpy' : ''}"${cpy
      ? ' title="Cliquer pour copier dans le presse-papiers" onclick="copyText(this.textContent)"' : ''}>${esc(v)}</div>`;
  };
  const kv = [
    ['id', s.id], ['transport', ti],
    ['origine', s.origin],
    ['bind', s.bound_host ?? '—'], ['serveur MCP', s.server_name ?? '—'],
    ['version', s.version ?? '—'], ['protocole', s.protocol ?? '—'],
    ['framework', s.framework
      ? ((FRAMEWORKS[s.framework] || {label: s.framework, lang: '?'}).label
         + ' (' + (FRAMEWORKS[s.framework] || {lang: s.lang || '?'}).lang
         + ' · spec ' + (s.protocol || '?') + ')')
      : '—'],
    ['sonde', s.via_ssh ? 'via SSH (interne à la machine)' : 'directe'],
    ['latence', s.latency_ms != null ? s.latency_ms + ' ms' : '—'],
    ['dernier vu', s.last_seen ? new Date(s.last_seen * 1000).toLocaleString('fr-FR') : '—'],
    ['erreur', s.last_error ?? '—'],
  ].map(([k, v]) => kvRow(k, v)).join('');
  /* Panneau Démarrage : contrôle du process par MCP Control.
     « au démarrage » = process lancé ici et vivant, mais MCP pas encore répondu. */
  const starting = s.enabled && s.managed && s.state !== 'UP';
  const dispState = starting ? 'STARTING' : s.state;
  const etatProcess = s.state === 'UP' ? 'en marche'
    : starting ? 'au d\u00e9marrage\u2026'
    : s.state === 'PORT-ONLY' ? 'port occup\u00e9 (aucune r\u00e9ponse MCP)'
    : s.state === 'DISABLED' ? 'd\u00e9sactiv\u00e9' : 'arr\u00eat\u00e9';
  const cmd = s.start_cmd
    ? s.start_cmd.program + ' ' + (s.start_cmd.args || []).join(' ') + (s.start_cmd.workdir ? '  \u00b7  ' + s.start_cmd.workdir : '')
    : null;
  const kvd = [
    ['contr&ocirc;le', s.managed ? 'process lanc\u00e9 et maintenu par MCP Control'
      : (cmd ? 'commande pr\u00eate \u2014 process externe non d\u00e9marr\u00e9 par ici' : 'aucune \u2014 process externe')],
    ['&eacute;tat du process', etatProcess],
    ['pid', s.pid ?? '—'],
    ['auto-restart', s.auto_restart ? 'ON \u2014 relance automatique au boot/crash' : 'OFF'],
    ['commande', cmd ?? '—'],
  ].map(([k, v]) => kvRow(k, v)).join('');
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
  /* Rendu « markdown inline » minimal pour les descriptions d'outils : les
     serveurs MCP renvoient souvent **gras** et `code` — affichés bruts, les
     astérisques cassent la lecture. Le texte est ÉCHAPPÉ d'abord (aucun < >
     ne peut donc passer dans le HTML généré) : injection impossible. */
  const mdInline = t => esc(t)
    .replace(/\*\*([^*]+)\*\*/g, '<b>$1</b>')
    .replace(/(^|[^*])\*([^*\s][^*]*)\*/g, '$1<i>$2</i>')
    .replace(/`([^`]+)`/g, '<code>$1</code>');
  const tools = (s.tools_list || []).map(t =>
    `<div class="tool ${t.disabled?'off':''}" data-tool="${esc(t.name)}">
       <code>${esc(t.name)}</code>
       <span class="desc" title="${esc(t.desc)}">${t.desc ? mdInline(t.desc) : '&mdash;'}</span>
       <button class="tgl ${t.disabled?'off':'on'}" title="${t.disabled?'R&eacute;activer ce tool':'D&eacute;sactiver ce tool'}"
               onclick="toggleTool(${js(s.id)},${js(t.name)})">
         ${t.disabled ? 'OFF' : 'ON'}
       </button>
     </div>`).join('');
  const nOn = (s.tools_list || []).filter(t => !t.disabled).length;
  const nTot = (s.tools_list || []).length;
  const toolsHtml = `<h4 class="sec">Outils &mdash; ${nOn} actif${nOn > 1 ? 's' : ''} sur ${nTot}${nTot - nOn ? ` (${nTot - nOn} d&eacute;sactiv&eacute;${nTot - nOn > 1 ? 's' : ''})` : ''}</h4>
    ${tools || '<div class="empty-note">aucun tool list&eacute;</div>'}`;
  document.getElementById('content').innerHTML = `
    <div class="crumb">serveurs / <b>${esc(s.label)}</b></div>
    <div class="d-head">
      <span class="dot dot-lg ${esc(dispState)}"></span>
      <h1>${esc(s.label)}</h1>
      <span class="pill ${esc(dispState)}" ${starting ? 'title="Process lanc&eacute;, MCP pas encore r&eacute;pondu"' : ''}>${starting ? 'AU D&Eacute;MARRAGE' : esc(s.state)}</span>
      ${langChip(s)}
      ${typeBadge(s)}
      ${frameworkBadge(s)}
      ${s.transport === 'stdio' ? '<span class="legacy" title="Legacy : serveur stdio uniquement (pas de transport HTTP)">legacy</span>' : ''}
      <span class="sub">${s.server_name ? esc(s.server_name) + ' v' + esc(s.version || '?') : ''}${s.latency_ms != null ? ' &middot; ' + s.latency_ms + ' ms' : ''}</span>
      <div class="d-actions">
        ${s.can_start && s.state !== 'UP' ? `<button class="primary" onclick="act('start',${js(s.id)})">${ic('play')} Start</button>` : ''}
        ${s.can_start && s.state === 'UP' ? `<button class="danger" onclick="act('kill',${js(s.id)})">${ic('stop')} Stop</button>` : ''}
        <button onclick="act('refresh',${js(s.id)})">${ic('activity')} Sonder</button>
        ${s.can_start ? `<button class="autobtn ${s.auto_restart?'on':''}" onclick="setAuto(${js(s.id)},${s.auto_restart?'false':'true'})"
          title="Autoboot : relance automatique de ce serveur au d&eacute;marrage de MCP Control et apr&egrave;s un crash">${ic('zap')} Autoboot ${s.auto_restart?'ON':'OFF'}</button>` : ''}
        <button class="favbtn ${s.favorite?'on':''}" onclick="setFavorite(${js(s.id)},${s.favorite?'false':'true'})"
          title="Favori : &eacute;pingle ce serveur en haut de sa section dans l'explorateur (persist&eacute;)">${ic('star')} Favori ${s.favorite?'ON':'OFF'}</button>
        <button onclick="hideServer(${js(s.id)})" title="Masquer ce serveur de l'affichage (session seulement : il revient &agrave; la r&eacute;ouverture de l'app)">${ic('eyeOff')} Masquer</button>
        <button onclick="act('toggle',${js(s.id)})" title="${s.enabled
          ? 'Mettre en pause : le process est arr&ecirc;t&eacute; et le serveur n&#39;est plus sond&eacute; (il reste dans la liste, r&eacute;activable d&#39;un clic)'
          : 'Reprendre la supervision : le serveur sera sond&eacute; et d&eacute;marr&eacute; si une commande est connue'}">${s.enabled ? ic('pause') + ' Mettre en pause' : ic('play') + ' Reprendre'}</button>
        ${s.origin !== 'seed'
          ? `<button class="danger" onclick="removeSrv(${js(s.id)})">${ic('trash')} Supprimer</button>`
          : `<button class="danger" onclick="removeSeedForever(${js(s.id)})" title="Supprime ce seed de la config : il ne sera pas r&eacute;inject&eacute; au red&eacute;marrage">${ic('trash')} Supprimer d&eacute;finitivement</button>`}
      </div>
    </div>
    ${s.origin === 'seed' ? '<div class="seed-note">Serveur seed : <b>Retirer durablement</b> l&rsquo;efface m&ecirc;me apr&egrave;s red&eacute;marrage (consign&eacute; dans config.json) ; <b>D&eacute;sactiver</b> l&rsquo;&eacute;teint sans le supprimer.</div>' : ''}
    ${s.last_error ? `<div class="d-err">${ic('warn')} ${esc(s.last_error)}</div>` : ''}
    ${fold('demarrage', 'D&eacute;marrage', `<div class="kv">${kvd}</div>${startCmdForm(s)}`)}
    ${fold('infos', 'Informations', `<div class="kv">${kv}</div>`)}
    <div class="tabs">
      <button class="tab ${detailTab === 'outils' ? 'sel' : ''}" onclick="detailTab='outils';renderDetail()">Outils (${nTot})</button>
      <button class="tab ${detailTab === 'conf' ? 'sel' : ''}" onclick="detailTab='conf';renderDetail()">Conf agent</button>
    </div>
    ${detailTab === 'conf' ? confAgent(s) : toolsHtml}`;
  // Restaure la saisie en cours du formulaire Démarrage (set_start_cmd) :
  // le re-rendu périodique régénère ces champs, on ne perd pas ce qui est tapé.
  for (const idOld in typed) {
    const el = document.getElementById(idOld);
    if (el && !el.value) el.value = typed[idOld];
  }
}

function select(id) { selectedId = id; detailCache = null; detailTab = 'outils'; render(); load(); }

/* Formulaire « commande de lancement » : pour un serveur local découvert par
   scan (port connu, commande inconnue) — poser la commande fait apparaître les
   boutons ▶/■ et l'autoboot, comme un serveur ajouté à la main. */
function startCmdForm(s) {
  if (s.can_start || s.transport === 'stdio') return '';
  const host = s.transport_info && s.transport_info.host;
  if (host) return '<div class="conf-note note-gap">Serveur distant : la commande de lancement appartient &agrave; sa machine (' + esc(host) + '), pas à MCP Control.</div>';
  const port = (s.transport_info || {}).port || 0;
  return `<div class="startcmd-form note-gap">
    <div class="conf-note">Aucune commande de lancement connue : c'est pourquoi les boutons <b>&#9654;/&#9632;</b> et <b>&#9889;</b> sont absents. Indique comment lancer ce serveur :</div>
    <div class="machine-form">
      <input id="scProgram" placeholder="programme (node, python, C:\\chemin\\serveur.exe)">
      <input id="scArgs" placeholder="arguments (ex : server.js --port ${esc(String(port))})">
      <input id="scWorkdir" placeholder="dossier de travail (optionnel)">
      <button class="primary" onclick="setStartCmd(${js(s.id)})">Enregistrer</button>
    </div>
  </div>`;
}
async function setStartCmd(id) {
  const program = (document.getElementById('scProgram').value || '').trim();
  const args = (document.getElementById('scArgs').value || '').split(/\s+/).filter(Boolean);
  const workdir = (document.getElementById('scWorkdir').value || '').trim();
  if (!program) { alert('Programme requis (ex : node, python, chemin .exe)'); return; }
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'set_start_cmd', id, program, args, workdir: workdir || null})});
  const j = await r.json();
  if (!j.ok) alert(j.error || 'erreur');
  else { await load(); detailCache = null; if (selectedId) renderDetail(); }
}
function removeSrv(id) {
  // La suppression d'un serveur importé ne demandait aucune confirmation :
  // un clic loupé coûtait la ligne. Les seeds ont déjà leur confirmation
  // dédiée (removeSeedForever) car l'effacement est durable.
  const s = byId(id);
  if (!confirm('Supprimer « ' + (s ? (s.label || id) : id) + ' » de la liste ? '
    + 'Un serveur re-détecté par un scan réapparaîtra.')) return;
  selectedId = null; detailCache = null; act('remove', id);
}
function removeSeedForever(id) {
  if (!confirm('Retirer ce seed DURABLEMENT ? Il ne sera pas réinjecté au redémarrage de MCP Control.')) return;
  selectedId = null; detailCache = null;
  fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'remove', id, durable: true})}).then(load);
}

/* ===== Actions ===== */
/* ---- Menu contextuel (clic droit sur une ligne de serveur) ---------------- */
let ctxId = null;
function ctxClose() {
  const m = document.getElementById('ctxmenu');
  if (m) { m.hidden = true; m.innerHTML = ''; }
  ctxId = null;
}
/* Toast sémantique : toast(msg) neutre, toast(msg,'ok') succès,
   toast(msg,'err') erreur (affichée plus longtemps — un échec se lit) ;
   `ms` surcharge la durée quand le message est long à lire. */
function toast(msg, kind, ms) {
  const t = document.getElementById('toast');
  if (!t) return;
  t.textContent = msg;
  t.classList.remove('ok', 'err');
  if (kind) t.classList.add(kind);
  t.classList.add('on');
  clearTimeout(toast._t);
  toast._t = setTimeout(() => t.classList.remove('on'), ms || (kind === 'err' ? 3500 : 1600));
}
/* Détail de l'alerte réseau (clic sur la pastille de la barre du Journal) :
   le message complet n'encombre pas la barre, il s'affiche à la demande. */
const NET_WARN_DETAIL = 'Réseau local injoignable : les serveurs distants ne peuvent pas être sondés'
  + ' — VPN actif ? (Proton VPN : activer «\u00a0autoriser les connexions LAN\u00a0»)';
function showNetDetail() {
  toast(NET_WARN_DETAIL, 'err', 6000);
}
function copyText(txt) {
  const done = () => toast('copié', 'ok');
  if (navigator.clipboard && navigator.clipboard.writeText) {
    navigator.clipboard.writeText(txt).then(done).catch(() => { copyFallback(txt); done(); });
  } else { copyFallback(txt); done(); }
}
function copyFallback(txt) {
  const ta = document.createElement('textarea');
  ta.value = txt; ta.style.position = 'fixed'; ta.style.opacity = '0';
  document.body.appendChild(ta); ta.select();
  try { document.execCommand('copy'); } catch (e) {}
  document.body.removeChild(ta);
}
function normPath(p) { return String(p || '').replace(/\\/g, '/').replace(/\/+$/, ''); }
// Chemin absolu + relatif (au workdir) : programme du start_cmd, ou fichier de
// script passé en argument quand le programme est un lanceur nu (node, npx,
// python…). Aucun chemin inventé : si rien n'est résolvable, la valeur est vide.
function srvPaths(s) {
  const ti = s.transport_info || {}, sc = s.start_cmd || {};
  const wd = normPath(ti.workdir || sc.workdir || '');
  const prog = ti.program || sc.program || '';
  const args = ti.args || sc.args || [];
  let abs = '';
  if (s.transport === 'stdio') {
    if (prog && /[\\/]/.test(prog)) abs = prog;
    else {
      const fileArg = args.find(a => /\.(js|mjs|cjs|jsx|ts|py|exe|jar|rb|php|sh|bat|cmd)$/i.test(a));
      // à défaut, un chemin absolu passé en argument (ex. server-filesystem C:/Users/X)
      const dirArg = args.find(a => /^[A-Za-z]:[\\/]/.test(a) || /^\/(?![-])/.test(a));
      if (fileArg) abs = (wd ? wd + '/' : '') + String(fileArg).replace(/\\/g, '/').replace(/^\/+/, '');
      else if (dirArg) abs = dirArg;
    }
  } else if (prog) abs = prog;
  const a = normPath(abs);
  const rel = (a && wd && a.toLowerCase().startsWith(wd.toLowerCase() + '/')) ? a.slice(wd.length + 1) : '';
  return { abs: a, rel: rel, workdir: wd };
}
// URL d'un serveur HTTP — variante gardée (la constante srvUrl plus haut ne
// vérifie pas le transport et sert au bloc « Conf agent »).
function ctxUrl(s) {
  if (s.transport !== 'http') return '';
  const ti = s.transport_info || {};
  return 'http://' + (ti.host || '127.0.0.1') + ':' + ti.port + (ti.path || '/mcp');
}
function srvCmd(s) {
  const sc = s.start_cmd || {};
  if (!sc.program) return '';
  return [sc.program].concat(sc.args || []).join(' ');
}
function srvInfoText(s) {
  const p = srvPaths(s);
  const fw = (typeof FRAMEWORKS === 'object' && FRAMEWORKS[s.framework]) ? FRAMEWORKS[s.framework].label : (s.framework || '—');
  const L = [
    'serveur      : ' + s.label,
    'id           : ' + s.id,
    'état         : ' + s.state + (s.enabled ? '' : ' (désactivé)') + (s.auto_restart ? ' · autoboot' : ''),
    'transport    : ' + s.transport + (s.ws ? ' (WS)' : ''),
    'url          : ' + (ctxUrl(s) || '—'),
    'commande     : ' + (srvCmd(s) || '—'),
    'framework    : ' + fw,
    'langage      : ' + (s.lang || '—'),
    'version      : ' + (s.version || '—') + ' · protocole ' + (s.protocol || '—'),
    'nom annoncé  : ' + (s.server_name || '—'),
    'outils       : ' + s.tools_count + '/' + s.tools_total + ((s.tools_disabled || []).length ? ' (' + s.tools_disabled.length + ' off)' : ''),
    'latence      : ' + (s.latency_ms != null ? s.latency_ms + ' ms' : '—'),
    'pid          : ' + (s.pid != null ? s.pid : '—')
  ];
  if (p.abs) L.push('chemin       : ' + p.abs);
  if (p.rel) L.push('chemin rel.  : ' + p.rel);
  if (p.workdir) L.push('workdir      : ' + p.workdir);
  if (s.last_error) L.push('dernière err : ' + s.last_error);
  return L.join('\n');
}
async function openInExplorer(path) {
  if (!path) return;
  try {
    const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({action: 'open_path', path: path})});
    const j = await r.json();
    toast(j.ok ? 'explorateur ouvert' : ('échec : ' + (j.error || 'inconnu')), j.ok ? 'ok' : 'err');
  } catch (e) { toast('échec : ' + e, 'err'); }
}
function rowCtx(ev, id) {
  ev.preventDefault();
  ev.stopPropagation();
  const s = (state && state.servers ? state.servers : []).find(x => x.id === id);
  if (!s) return;
  if (selectedId !== id) select(id);
  const p = srvPaths(s);
  const items = [];
  const add = (lbl, fn, dis) => items.push({lbl: lbl, fn: fn, dis: !!dis});
  add(ic('copy') + ' Copier toutes les infos', () => copyText(srvInfoText(s)));
  add(ic('tag') + ' Copier le nom', () => copyText(s.label));
  if (ctxUrl(s)) add(ic('link') + " Copier l'URL", () => copyText(ctxUrl(s)));
  if (srvCmd(s)) add(ic('terminal') + ' Copier la commande', () => copyText(srvCmd(s)));
  add(ic('folder') + ' Copier le chemin', () => copyText(p.abs), !p.abs);
  add(ic('folder') + ' Copier le chemin relatif', () => copyText(p.rel), !p.rel);
  add(ic('external') + " Ouvrir dans l'explorateur", () => openInExplorer(p.abs || p.workdir), !(p.abs || p.workdir));
  if (s.can_start) {
    add('—', null, false);
    add(s.state === 'UP' ? ic('stop') + ' Arrêter ce serveur' : ic('play') + ' Démarrer ce serveur',
        () => act(s.state === 'UP' ? 'kill' : 'start', s.id));
  }
  add(ic('refresh') + ' Sonder maintenant', () => act('refresh', s.id));
  add('—', null, false);
  add(ic('eyeOff') + ' Masquer', () => hideServer(s.id));
  showMenu(ev, items);
}
// Affiche un menu d'entrées {lbl, fn, dis} à la position du curseur.
// '—' insère un séparateur. Une seule mécanique pour les lignes serveur et outil.
function showMenu(ev, items) {
  const m = document.getElementById('ctxmenu');
  m.innerHTML = items.map((it, i) => it.lbl === '—'
    ? '<div class="ctxsep"></div>'
    : '<div class="ctxitem' + (it.dis ? ' dis' : '') + '" data-i="' + i + '">' + it.lbl + '</div>').join('');
  m.querySelectorAll('.ctxitem').forEach(el => {
    el.onclick = (e) => {
      e.stopPropagation();
      const it = items[+el.dataset.i];
      if (!it || it.dis) return;
      ctxClose();
      it.fn();
    };
  });
  m.hidden = false;
  const w = m.offsetWidth, h = m.offsetHeight;
  m.style.left = Math.max(6, Math.min(ev.clientX, window.innerWidth - w - 8)) + 'px';
  m.style.top = Math.max(6, Math.min(ev.clientY, window.innerHeight - h - 8)) + 'px';
}
// Clic droit sur un OUTIL du panneau de détail.
function toolCtx(ev, id, name) {
  ev.preventDefault();
  ev.stopPropagation();
  // state.servers ne porte PAS tools_list : seul detailCache l'expose.
  const s = (state && state.servers ? state.servers : []).find(x => x.id === id) || {};
  const d = (detailCache && detailCache.id === id) ? detailCache : {};
  const t = (d.tools_list || []).find(x => x.name === name) || {name: name, desc: ''};
  const items = [];
  const add = (lbl, fn, dis) => items.push({lbl: lbl, fn: fn, dis: !!dis});
  add(ic('copy') + " Copier le nom de l'outil", () => copyText(name));
  add(ic('copy') + ' Copier « serveur.outil »', () => copyText((d.label || s.label || id) + '.' + name));
  if (t.desc) add(ic('copy') + ' Copier la description', () => copyText(t.desc));
  add(ic('terminal') + " Copier l'appel JSON-RPC", () => copyText(JSON.stringify({
    jsonrpc: '2.0', id: 1, method: 'tools/call',
    params: {name: name, arguments: {}}
  }, null, 2)));
  if (ctxUrl(s)) add(ic('link') + " Copier l'URL + outil", () => copyText(ctxUrl(s) + '   # outil : ' + name));
  add('—', null, false);
  add(t.disabled ? ic('check') + ' Réactiver cet outil' : ic('x') + ' Désactiver cet outil',
      () => toggleTool(id, name));
  showMenu(ev, items);
}
// Clic droit : délégation (plus fiable que des attributs inline sur du HTML
// régénéré à chaque rendu). Ligne serveur → menu serveur, ligne outil → menu outil.
document.addEventListener('contextmenu', (ev) => {
  const t = ev.target && ev.target.closest ? ev.target : null;
  if (!t) return;
  const row = t.closest('.srow');
  if (row && row.dataset.id) { ev.preventDefault(); ev.stopPropagation(); rowCtx(ev, row.dataset.id); return; }
  const tool = t.closest('.tool');
  if (tool && tool.dataset.tool && selectedId) {
    ev.preventDefault(); ev.stopPropagation(); toolCtx(ev, selectedId, tool.dataset.tool);
  }
});
// Diagnostic : une exception JS non capturée s'écrit dans le TITRE de la
// fenêtre (lisible depuis l'extérieur, ex. PowerShell MainWindowTitle) — le
// webview n'a pas de console accessible sur la build packagée.
window.addEventListener('error', (e) => {
  try { document.title = 'JSERR ' + (e.message || e.error || 'inconnu'); } catch (_) {}
});
window.addEventListener('unhandledrejection', (e) => {
  try { document.title = 'JSERR promesse ' + ((e.reason && e.reason.message) || e.reason || 'inconnu'); } catch (_) {}
});
document.addEventListener('click', ctxClose);
/* Échap ferme le menu contextuel PUIS efface le filtre de l'explorateur ;
   « / » et Ctrl+K posent le focus dans le champ de filtre (raccourci
   standard : recherche instantanée depuis n'importe où). */
document.addEventListener('keydown', (e) => {
  if (e.key === 'Escape') {
    ctxClose();
    const f = document.getElementById('srvFilter');
    if (f && (f.value || document.activeElement === f)) {
      f.value = '';
      renderServers();
      f.blur();
    }
    return;
  }
  const el = document.activeElement;
  const typing = el && (el.tagName === 'INPUT' || el.tagName === 'TEXTAREA'
    || el.tagName === 'SELECT' || el.isContentEditable);
  if (!typing && (e.key === '/' || ((e.ctrlKey || e.metaKey) && e.key.toLowerCase() === 'k'))) {
    e.preventDefault();
    // la boutique ouverte a la priorité : c'est SA recherche qu'on vise
    const d = document.getElementById('storeDlg');
    if (d && d.open) {
      const ss = document.getElementById('storeSearch');
      if (ss) { ss.focus(); ss.select(); }
      return;
    }
    const f = document.getElementById('srvFilter');
    if (f) { f.focus(); f.select(); }
  }
});
window.addEventListener('resize', ctxClose);

async function act(action, id) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action, id})});
  await load();
}

// Autoboot par serveur (⚡) : relance auto au démarrage de MCP Control
// et après un crash. Persisté dans config.json par le moteur.
async function setAuto(id, enabled) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'set_auto_restart', id, enabled})});
  await load();
  if (selectedId === id) { detailCache = null; await load(); }
}

// Favori (⭐) : épingle la ligne en haut de sa section (persisté). L'épingle
// s'applique À L'AFFICHAGE — l'ordre posé au glisser-déposer reste en base.
async function setFavorite(id, enabled) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'set_favorite', id, enabled})});
  await load();
  if (selectedId === id) { detailCache = null; await load(); }
}

// Masquer de l'affichage (session seulement : la supervision moteur continue,
// mais la ligne ne s'affiche plus ni dans la liste ni dans les compteurs).
// Rien n'est persisté : la réouverture de l'app les fait revenir.
function hideServer(id) {
  hiddenServers.add(id);
  if (selectedId === id) { selectedId = null; detailCache = null; renderOverview(); }
  renderServers();
  toast(hiddenServers.size + ' serveur' + (hiddenServers.size > 1 ? 's' : '')
    + ' masqu\u00e9' + (hiddenServers.size > 1 ? 's' : '')
    + ' — il revient \u00e0 la r\u00e9ouverture de l\u2019app');
}
function showHiddenServers() {
  hiddenServers.clear();
  renderServers();
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

/* ===== Réglages du client (dialog ⚙ à volets) ===== */
/* Dirty state : tout input/change dans le dialog le marque ; fermer (Esc,
   ×, Annuler) avec des saisies pendantes demande confirmation au lieu de les
   jeter en silence. Le thème, lui, s'applique et se sauve au clic. */
let settingsDirty = false;
/* Instantané des valeurs notifications à l'ouverture du dialog : un
   Enregistrer ne POSTe set_notify que si la section a réellement bougé. */
let notifyOriginal = '';
function setDirty(v) {
  settingsDirty = v;
  const b = document.getElementById('settingsDirty');
  if (b) b.hidden = !v;
}
function settingsConfirmDiscard() {
  return !settingsDirty || confirm('Modifications non enregistr\u00e9es \u2014 fermer quand m\u00eame ?');
}
function closeSettings() {
  if (!settingsConfirmDiscard()) return;
  setDirty(false);
  document.getElementById('settingsDlg').close();
}
function setPane(name) {
  document.querySelectorAll('#settingsDlg .set-nav-btn').forEach(b => b.classList.toggle('sel', b.dataset.pane === name));
  document.querySelectorAll('#settingsDlg .set-pane').forEach(p => p.classList.toggle('sel', p.dataset.pane === name));
  try { localStorage.setItem('mcc.setpane', name); } catch (e) {}
}
/* Labels humains des clés renvoyées par set_settings : le message parle
   utilisateur (« plage de scan »), pas clés internes (« scan »). */
const SETTINGS_LABELS = {
  refresh: 'intervalle de sondage',
  scan: 'plage de scan',
  ui_bind: '\u00e9coute',
  network: 'r\u00e9seau',
  network_ports: 'ports UI des pairs',
};
function setFieldErr(id, msg) {
  const el = document.getElementById('err_' + id);
  if (!el) return;
  if (msg) { el.textContent = msg; el.hidden = false; }
  else { el.textContent = ''; el.hidden = true; }
}
function clearFieldErrs() {
  ['refresh', 'scan', 'ports', 'webhook', 'states', 'min'].forEach(id => setFieldErr(id, ''));
}
function showMsg(rowId, msgId, text) {
  const row = document.getElementById(rowId);
  const el = document.getElementById(msgId);
  if (row) row.hidden = false;
  if (el) el.textContent = text;
}

function openSettingsDlg() {
  const s = state.settings || {};
  const n = s.notify || {};
  document.getElementById('sRefresh').value = s.refresh_secs || 15;
  document.getElementById('sScanFrom').value = s.scan_from || 3000;
  document.getElementById('sScanTo').value = s.scan_to || 3200;
  document.getElementById('sNetEnabled').checked = !!s.network_enabled;
  document.getElementById('sNetPorts').value = (s.network_ports || []).join(', ');
  document.getElementById('sUiBind').value = s.ui_bind || '127.0.0.1';
  document.getElementById('sUiPort').value = s.ui_port || 5090;
  // Notifications (moteur : action set_notify, section notify de config.json)
  document.getElementById('sNotifDesktop').checked = !!n.desktop;
  document.getElementById('sWebhookUrl').value = n.webhook_url || '';
  document.getElementById('sWebhookMethod').value = n.method === 'PUT' ? 'PUT' : 'POST';
  document.getElementById('sWebhookHeaders').value = n.headers || '';
  const states = n.states || ['DOWN', 'PORT-ONLY'];
  const hasState = v => states.some(x => String(x).toUpperCase() === v);
  document.getElementById('sStDown').checked = hasState('DOWN');
  document.getElementById('sStPortOnly').checked = hasState('PORT-ONLY');
  document.getElementById('sNotifMin').value = (n.min_interval_secs ?? 300);
  notifyOriginal = JSON.stringify(notifyFormValues());
  // sVersion/sLatest sont des <span> (affichage), pas des champs éditables
  document.getElementById('sVersion').textContent = 'v' + MCC_VERSION;
  document.getElementById('sLatest').textContent = state.latest_version ? 'v' + state.latest_version : '\u2014';
  const upd = document.getElementById('updMsg');
  upd.textContent = state.update_available ? '\u2b06 v' + state.latest_version + ' disponible' : 'Aucune mise \u00e0 jour connue (auto-v\u00e9rification au boot puis chaque heure).';
  upd.style.color = state.update_available ? 'var(--warn)' : '';
  const row = document.getElementById('resetMsgRow');
  if (row) row.hidden = true;
  document.getElementById('resetMsg').textContent = '';
  ['notifyMsgRow', 'runtimeMsgRow'].forEach(id => { const el = document.getElementById(id); if (el) el.hidden = true; });
  ['notifyMsg', 'runtimeMsg'].forEach(id => { const el = document.getElementById(id); if (el) el.textContent = ''; });
  document.getElementById('settingsMsg').textContent = '';
  clearFieldErrs();
  setDirty(false);
  // dernier volet consulté (préférence locale)
  let pane = 'general';
  try {
    const p = localStorage.getItem('mcc.setpane');
    if (p && document.querySelector('#settingsDlg .set-pane[data-pane="' + p + '"]')) pane = p;
  } catch (e) {}
  setPane(pane);
  const foot = document.getElementById('settingsFootNote');
  if (foot) foot.textContent = 'Les r\u00e9glages sont enregistr\u00e9s dans config.json.';
  applyTheme(document.documentElement.dataset.theme === 'light' ? 'light' : 'dark');
  document.getElementById('settingsDlg').showModal();
  // focus dans le dialog dès l'ouverture : Esc fonctionne immédiatement (le
  // navigateur ne déclenche le cancel que si le focus est dedans) et la
  // navigation clavier part du volet actif.
  const nav = document.querySelector('#settingsDlg .set-nav-btn.sel');
  if (nav) nav.focus();
}
async function checkUpdateNow() {
  const msg = document.getElementById('updMsg');
  msg.style.color = '';
  msg.textContent = '⏳ vérification de la release GitHub…';
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'check_update'})});
  const j = await r.json();
  if (j.ok && j.checked && j.update_available) {
    document.getElementById('sLatest').textContent = 'v' + j.latest;
    msg.textContent = '\u2b06 v' + j.latest + ' disponible (installée : v' + j.current + ') — bouton « Mettre à jour » en bas à droite';
    msg.style.color = 'var(--warn)';
  } else if (j.ok && j.checked) {
    document.getElementById('sLatest').textContent = j.latest ? 'v' + j.latest : '—';
    msg.textContent = '✓ à jour — vérifié à l\u2019instant : v' + j.current + ' est la dernière release';
    msg.style.color = 'var(--ok)';
  } else {
    msg.textContent = j.latest
      ? '✗ vérification impossible (réseau ?) — dernière info connue : v' + j.latest + ' disponible'
      : '✗ vérification impossible (réseau ?) — nouvel essai auto au boot puis chaque heure';
  }
}
async function openReleasePage() {
  // window.open est neutralisé dans la webview Tauri (desktop) : le clic mort
  // « rien ne se passe ». On passe par le moteur, qui lance le navigateur par
  // défaut de la machine (action open_release_page, URL fixe sans argument).
  // Repli : si le moteur ne peut pas (headless sans navigateur), on tente
  // window.open — utile quand l'UI tourne dans un vrai navigateur.
  try {
    const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify({action: 'open_release_page'})});
    const j = await r.json();
    if (j.ok) return;
  } catch (e) { /* moteur injoignable : on tente le repli */ }
  window.open('https://github.com/DeamonDev888/mcp-control/releases/latest', '_blank', 'noopener,noreferrer');
}
/* OS du navigateur — la machine où atterrit le téléchargement quand l'UI est
   servie par un moteur distant (VPS). Le moteur valide et retombe sur le sien. */
function browserOs() {
  const ua = navigator.userAgent || '';
  if (ua.includes('Windows')) return 'windows';
  if (ua.includes('Mac OS') || ua.includes('Macintosh')) return 'macos';
  if (ua.includes('Linux') && !ua.includes('Android')) return 'linux';
  return null;
}
let lastUpdateUrl = null;
/* ⬆ Mettre à jour : téléchargement DIRECT de l'asset de la dernière release
   pour CETTE machine (OS du navigateur + type du binaire qui sert l'UI :
   desktop → installateur, sinon CLI) — plus de détour par la page des
   releases. L'URL est fabriquée par le moteur depuis une table blanche, jamais
   par la requête. Repli : si le moteur n'a pas pu lancer de navigateur
   (headless), l'URL rendue est ouverte ici — utile quand l'UI tourne dans un
   vrai navigateur. */
async function downloadUpdate() {
  const body = {action: 'download_update'};
  const os = browserOs();
  if (os) body.os = os;
  try {
    const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify(body)});
    const j = await r.json();
    if (j.url) lastUpdateUrl = j.url;
    if (j.ok) { toast('téléchargement lancé : ' + (j.asset || 'mise à jour'), 'ok'); return; }
    if (j.url) { window.open(j.url, '_blank', 'noopener,noreferrer'); return; }
    toast(j.error || 'échec du téléchargement', 'err');
  } catch (e) {
    // moteur injoignable : repli sur la dernière URL connue, sinon message
    if (lastUpdateUrl) window.open(lastUpdateUrl, '_blank', 'noopener,noreferrer');
    else toast('moteur injoignable — réessaie plus tard', 'err');
  }
}
async function resetState(what) {
  const sel = document.getElementById('sResetWhat');
  const label = sel.options[sel.selectedIndex].text;
  if (!confirm('Réinitialiser « ' + label + ' » ? Action irréversible.')) return;
  const msg = document.getElementById('resetMsg');
  const row = document.getElementById('resetMsgRow');
  if (row) row.hidden = false;
  msg.textContent = '⏳ réinitialisation…';
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'reset_state', what})});
  const j = await r.json();
  if (!j.ok) msg.textContent = '✗ ' + (j.error || 'échec');
  else {
    const det = (j.reinjected ? ' · ' + j.reinjected + ' seed(s) réinjecté(s)' : '') + (j.removed ? ' · ' + j.removed + ' serveur(s) retiré(s)' : '');
    msg.textContent = '✓ Réinitialisé (' + (j.cleared || []).join(', ') + ')' + det;
    await load();
  }
}
/* Valeurs du formulaire notifications, telles quelles (sans le champ action) :
   sérialisées pour détecter si la section a réellement changé (évite un
   « configuration enregistrée » dans le journal à chaque Enregistrer). */
function notifyFormValues() {
  return {
    desktop: document.getElementById('sNotifDesktop').checked,
    webhook_url: document.getElementById('sWebhookUrl').value.trim(),
    method: document.getElementById('sWebhookMethod').value,
    headers: document.getElementById('sWebhookHeaders').value,
    states: [
      ...(document.getElementById('sStDown').checked ? ['DOWN'] : []),
      ...(document.getElementById('sStPortOnly').checked ? ['PORT-ONLY'] : []),
    ],
    min_interval_secs: +document.getElementById('sNotifMin').value || 0,
  };
}
/* Lecture + validation de TOUS les champs du dialog : chaque erreur est
   affichée sous sa ligne (set-err), jamais une seule ligne globalisée.
   Renvoie null si invalide, sinon le corps d'action set_settings et, si la
   section Notifications a changé, le corps d'action set_notify. */
function collectSettingsForm() {
  let ok = true;
  const intOrNull = id => { const raw = document.getElementById(id).value.trim(); return raw === '' ? null : +raw; };
  // Intervalle de sondage : entier 5..3600
  const refresh = intOrNull('sRefresh');
  if (refresh === null || !Number.isInteger(refresh) || refresh < 5 || refresh > 3600) {
    setFieldErr('refresh', 'entier entre 5 et 3600 secondes'); ok = false;
  } else setFieldErr('refresh', '');
  // Plage de scan : entiers 1..65535, bornes échangées si inversées
  let sf = intOrNull('sScanFrom'), st = intOrNull('sScanTo');
  let swapped = false;
  if (sf === null || st === null || !Number.isInteger(sf) || !Number.isInteger(st)
      || sf < 1 || sf > 65535 || st < 1 || st > 65535) {
    setFieldErr('scan', 'ports entiers entre 1 et 65535'); ok = false;
    sf = null; st = null;
  } else {
    if (sf > st) { const t = sf; sf = st; st = t; swapped = true; }
    setFieldErr('scan', '');
  }
  // Ports UI des pairs : liste d'entiers 1..65535
  const rawPorts = document.getElementById('sNetPorts').value.trim();
  const tokens = rawPorts ? rawPorts.split(/[\s,]+/).filter(Boolean) : [];
  const bad = tokens.filter(t => !/^\d+$/.test(t) || +t < 1 || +t > 65535);
  if (bad.length) { setFieldErr('ports', 'ports invalides : ' + bad.slice(0, 3).join(', ')); ok = false; }
  else setFieldErr('ports', '');
  const netPorts = tokens.filter(t => /^\d+$/.test(t)).map(Number).filter(p => p >= 1 && p <= 65535);
  // Webhook : vide autorisé (canal off), sinon URL http(s)
  const url = document.getElementById('sWebhookUrl').value.trim();
  if (url && !/^https?:\/\//i.test(url)) { setFieldErr('webhook', 'URL http:// ou https:// attendue'); ok = false; }
  else setFieldErr('webhook', '');
  // États notifiés : au moins un (sinon aucun incident ne déclencherait rien)
  const notify = notifyFormValues();
  notify.webhook_url = url;
  if (!notify.states.length) { setFieldErr('states', 'coche au moins un état (DOWN ou PORT-ONLY)'); ok = false; }
  else setFieldErr('states', '');
  // Délai anti-spam : borné 0..86400 (le moteur clampe déjà, on affiche juste)
  notify.min_interval_secs = Math.min(86400, Math.max(0, notify.min_interval_secs));
  if (!ok) return null;
  return {
    swapped,
    settings: {
      action: 'set_settings',
      refresh_secs: refresh,
      scan_from: sf,
      scan_to: st,
      ui_bind: document.getElementById('sUiBind').value,
      network_enabled: document.getElementById('sNetEnabled').checked,
      network_ports: netPorts,
    },
    notifyChanged: JSON.stringify(notify) !== notifyOriginal,
    notify,
  };
}
async function saveSettings() {
  const form = collectSettingsForm();
  if (!form) return false;
  const msg = document.getElementById('settingsMsg');
  const parts = [];
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify(form.settings)});
  const j = await r.json();
  if (!j.ok) { msg.textContent = '\u2717 ' + (j.error || '\u00e9chec'); return false; }
  parts.push((j.changed || []).length
    ? '\u2713 enregistr\u00e9 : ' + j.changed.map(k => SETTINGS_LABELS[k] || k).join(', ')
    : 'r\u00e9glages inchang\u00e9s');
  if ((j.changed || []).includes('ui_bind')) parts.push('l\u2019\u00e9coute s\u2019applique au prochain d\u00e9marrage');
  if (form.swapped) parts.push('plage de scan r\u00e9ordonn\u00e9e (du plus petit au plus grand port)');
  if (form.notifyChanged) {
    const r2 = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
      body: JSON.stringify(Object.assign({action: 'set_notify'}, form.notify))});
    const j2 = await r2.json();
    parts.push(j2.ok ? 'notifications enregistr\u00e9es' : '\u2717 notifications : ' + (j2.error || '\u00e9chec'));
  }
  msg.textContent = parts.join(' \u00b7 ');
  setDirty(false);
  await load();
  armPolling();
  return true;
}
/* Test des notifications : enregistre d'abord les saisies pendantes (le
   moteur teste la config PERSISTÉE), puis émet sur chaque canal activé. */
async function testNotify() {
  if (settingsDirty) { const saved = await saveSettings(); if (!saved) return; }
  showMsg('notifyMsgRow', 'notifyMsg', '\u23f3 test en cours\u2026');
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'test_notify'})});
  const j = await r.json();
  if (!j.ok) { showMsg('notifyMsgRow', 'notifyMsg', '\u2717 ' + (j.error || '\u00e9chec')); return; }
  const parts = [];
  if (j.desktop) parts.push(j.desktop.ok ? 'desktop \u2713 (toast affich\u00e9)' : 'desktop \u2717 \u2014 ' + (j.desktop.error || '?'));
  if (j.webhook) parts.push(j.webhook.ok ? 'webhook \u2713' : 'webhook \u2717 \u2014 ' + (j.webhook.error || '?'));
  showMsg('notifyMsgRow', 'notifyMsg', parts.join(' \u00b7 ') || 'aucun canal actif');
}
/* Historique runtime : crashs, relances, backoff — reset_runtime du moteur. */
async function resetRuntime() {
  if (!confirm('Effacer l\u2019historique runtime (crashs, relances, backoff) ?')) return;
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'reset_runtime'})});
  const j = await r.json();
  showMsg('runtimeMsgRow', 'runtimeMsg',
    j.ok ? '\u2713 historique effac\u00e9 (' + (j.cleared || 0) + ' serveur(s))' : '\u2717 ' + (j.error || '\u00e9chec'));
  await load();
}
/* Câblage du dialog (une fois au boot) : dirty state sur toute saisie, et
   Esc = event cancel — les modifications pendantes ne sont pas jetées en
   silence, une confirmation les sauvegarde ou les abandonne explicitement. */
(function () {
  const d = document.getElementById('settingsDlg');
  if (!d) return;
  d.addEventListener('input', () => setDirty(true));
  d.addEventListener('change', () => setDirty(true));
  d.addEventListener('cancel', (e) => {
    if (!settingsConfirmDiscard()) e.preventDefault();
    else setDirty(false);
  });
})();
function renderMachines() {
  if (!state) return;
  const hosts = state.settings.ssh_hosts || [];
  const peers = state.peers || [];
  const box = document.getElementById('machineList');
  let html = hosts.length ? '' :
    '<div class="empty-sec">aucune machine enregistr&eacute;e — entre une IP et un utilisateur SSH ci-dessus</div>';
  html += hosts.map(h => {
    const peer = peers.find(p => p.ip === h.ip);
    const st = peer ? `moteur MCP Control d&eacute;tect&eacute; — ${esc((peer.stats||{}).up ?? '?')} UP / ${esc((peer.stats||{}).total ?? '?')} serveurs`
                    : 'machine enregistr&eacute;e';
    return `<div class="machine-row">
      <div class="l1"><span class="dot UP"></span><span class="nm">${esc(h.ip)}</span><span class="tag http">${esc(h.user)}</span>${peer ? '<span class="tag sse">pair</span>' : ''}</div>
      <div class="l2">${st}</div>
      <div class="macts">
        <button class="primary" onclick="sshDiscover(${js(h.ip)},${js(h.user)})">${ic('link')} SSH &amp; d&eacute;couvrir</button>
        ${peer ? `<button onclick="importPeer(${js(h.ip)})">Importer ses serveurs</button>` : ''}
        <button class="danger" onclick="sshForget(${js(h.ip)})">Oublier</button>
      </div>
    </div>`;
  }).join('');
  const autres = peers.filter(p => !hosts.find(h => h.ip === p.ip));
  html += autres.map(p => `<div class="machine-row">
      <div class="l1"><span class="dot UP"></span><span class="nm">${esc(p.ip)}</span><span class="tag sse">pair</span></div>
      <div class="l2">UI :${esc(p.ui_port)} &mdash; ${esc((p.stats||{}).up ?? '?')} UP / ${esc((p.stats||{}).total ?? '?')} serveurs</div>
      <div class="macts"><button onclick="importPeer(${js(p.ip)})">Importer ses serveurs</button></div>
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
  document.getElementById('stdioFields').hidden = !stdio;
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
   « paquet » neutre — TOUTE carte porte une icône (jamais de carte sans logo,
   avant comme après le chargement de /api/store-logos). NB : guillemet-double
   suivi de dièse interdit dans ce bloc — la séquence fermerait la raw string
   Rust qui embarque cette page. */
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
  if (!storeLogos) return storeLogoSvg(PKG_GLYPH);
  // glyphe officiel par id, préfixes de catalogues additionnels retirés
  const inner = storeLogos[i.id] || storeLogos[i.id.replace(/^(at|gl)-/, '')];
  if (inner) return storeLogoSvg(inner);
  // items de catalogues additionnels : marque reconnue dans la commande
  if (/^(at|gl|gh)-/.test(i.id)) {
    const cmd = (i.program + ' ' + (i.args || []).join(' ')).toLowerCase();
    for (const [needle, lid] of PKG_LOGOS) {
      if (storeLogos[lid] && cmd.includes(needle)) return storeLogoSvg(storeLogos[lid]);
    }
  }
  // glyphe « paquet » neutre pour tout ce qui n'a pas de marque reconnue
  return storeLogoSvg(PKG_GLYPH);
}
/* Id serveur d'un item boutique : install_store clé sur « store-<id> ». */
const storeSrvId = i => 'store-' + i.id;
let storeHideInst = false;
function toggleHideInst() {
  storeHideInst = !storeHideInst;
  const b = document.getElementById('storeHideInst');
  if (b) b.classList.toggle('sel', storeHideInst);
  renderStoreItems();
}
function renderStoreItems() {
  if (!storeItems) return;
  const q = (document.getElementById('storeSearch').value || '').toLowerCase();
  const sort = document.getElementById('storeSort').value;
  const installed = new Set((state && state.servers ? state.servers : []).map(s => s.id));
  let list = storeItems.filter(i => (storeCat==='Tout' || i.cat===storeCat) &&
    (!q || i.name.toLowerCase().includes(q) || i.desc.toLowerCase().includes(q)));
  if (storeHideInst) list = list.filter(i => !installed.has(storeSrvId(i)));
  if (sort === 'az') list = [...list].sort((a, b) => a.name.localeCompare(b.name, 'fr', {sensitivity: 'base'}));
  const res = document.getElementById('storeResults');
  res.textContent = list.length === storeItems.length
    ? storeItems.length + ' serveurs'
    : list.length + ' affiché' + (list.length > 1 ? 's' : '') + ' sur ' + storeItems.length;
  document.getElementById('storeGrid').innerHTML = list.map(i => {
    const isInst = installed.has(storeSrvId(i));
    /* Pas de ligne de commande sur la carte : la description suffit, la
       commande est visible dans le détail une fois installé. */
    return `<div class="scard">
      <div class="scard-top">${storeLogo(i)}<b>${esc(i.name)}</b><span class="tag ${i.kind==='npm'?'npm':'py'}">${esc(i.kind)}</span></div>
      <div class="scard-desc">${esc(i.desc)}</div>
      ${isInst
        ? `<button class="btn-inst" disabled title="Ce serveur est d&eacute;j&agrave; dans ta liste">${ic('check')} D&eacute;j&agrave; install&eacute;</button>`
        : `<button class="primary" onclick="installStore(${js(i.id)}, this)">${ic('download')} Installer</button>`}
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
    // re-rendu : la carte bascule sur l'état « Déjà installé » (source de
    // vérité = state.servers, pas un libellé collé sur le bouton)
    await load();
    renderStoreItems();
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

/* Journal replié PAR DÉFAUT (interface épurée) : l'opérateur l'ouvre d'un clic
   sur sa barre et son choix est retenu (localStorage). */
function toggleLog() {
  const lp = document.getElementById('logpanel');
  lp.classList.toggle('min');
  try { localStorage.setItem('mcc.logmin', String(lp.classList.contains('min'))); } catch (e) {}
}
(function () {
  let v = null;
  try { v = localStorage.getItem('mcc.logmin'); } catch (e) {}
  document.getElementById('logpanel').classList.toggle('min', v === null ? true : v === 'true');
})();
/* ===== Polling armé sur les réglages =====
   L'intervalle suit settings.refresh_secs (re-armé à chaque rendu quand il
   change) : fini le 5 s fixe qui interrogeait 3× plus vite que les données
   ne changeaient. Avant le premier état chargé (ou serveur momentanément
   injoignable), on retente au rythme par défaut de 5 s. */
let pollTimer = null;
let pollMs = 0;
function armPolling() {
  const secs = Math.max(5, (state && state.settings && state.settings.refresh_secs) || 5);
  const ms = secs * 1000;
  if (ms === pollMs) return;
  if (pollTimer) clearInterval(pollTimer);
  pollMs = ms;
  pollTimer = setInterval(load, ms);
}
loadFrameworks();
updateLogoutBtn();
load();
armPolling();
// retour de la fenêtre (tray → visible) : reprise immédiate du rafraîchissement
document.addEventListener('visibilitychange', () => { if (!document.hidden) load(); });
