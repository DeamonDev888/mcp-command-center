pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>MCP Control</title>
<style>
/* ===== Jetons de design ====================================================
   Une seule échelle pour tout : rayons, hauteurs de contrôle, filets.
   Toute valeur « à la main » dans un composant est une dette — elle est ici.
   Chrome #0d0d0e · Sidebar #0a0a0b · Card #161618 · Accent #4a84fe
   Textes 94/74/54/36 % · hover accent 4 % · actif 8 % · filet 9/12/16 %      */
:root{
  --chrome:#0d0d0e;--sidebar:#0a0a0b;--card:#161618;--elevated:#1c1c1f;
  --accent:#4a84fe;
  --t1:rgba(255,255,255,.94);--t2:rgba(255,255,255,.74);--t3:rgba(255,255,255,.54);--t4:rgba(255,255,255,.36);
  --border:rgba(255,255,255,.12);--border-soft:rgba(255,255,255,.07);--border-strong:rgba(255,255,255,.2);
  --hover:rgba(74,132,254,.04);--active:rgba(74,132,254,.08);--active2:rgba(74,132,254,.14);
  --wash:rgba(255,255,255,.035);
  --ok:#55a583;--bad:#e75e78;--warn:#c08532;--orange:#db704b;--cyan:#6f9ba6;--purple:#9e94d5;
  --mono:'JetBrains Mono',ui-monospace,'Cascadia Code',Consolas,monospace;
  --sans:-apple-system,'Segoe UI',system-ui,sans-serif;
  /* échelle de rayons */
  --r1:4px;--r2:6px;--r3:8px;--r4:10px;--r5:12px;--rpill:999px;
  /* hauteurs de contrôle : une seule par rang */
  --h-ctl:26px;--h-sm:22px;--h-xs:18px;
  /* ombres : deux niveaux, pas plus */
  --sh-pop:0 14px 34px rgba(0,0,0,.55),0 2px 6px rgba(0,0,0,.35);
  --sh-dlg:0 24px 64px rgba(0,0,0,.6),0 2px 8px rgba(0,0,0,.4);
  /* rythme vertical */
  --gap:8px;--pad-sec:16px;
}
/* Thème clair — bascule <html data-theme="light"> (préférence ⚙ Réglages).
   Hiérarchie de surfaces : chrome (fond de la zone de contenu) plus soutenu,
   sidebar légèrement plus claire, cartes BLANCHES détachées (fond + ombre),
   filets nettement visibles. Les rayons/hauteurs/ombres viennent des jetons
   du design system — rien n'est « à la main » ici. */
:root[data-theme="light"]{
  --chrome:#e4e7ed;--sidebar:#f1f3f7;--card:#ffffff;--elevated:#fafbfd;
  --t1:rgba(13,17,24,.94);--t2:rgba(13,17,24,.72);--t3:rgba(13,17,24,.54);--t4:rgba(13,17,24,.40);
  --border:rgba(13,17,24,.30);--border-soft:rgba(13,17,24,.15);--border-strong:rgba(13,17,24,.44);
  --hover:rgba(74,132,254,.09);--active:rgba(74,132,254,.16);--active2:rgba(74,132,254,.30);
  --wash:rgba(13,17,24,.04);
  --ok:#2e7a5c;--bad:#c23f5c;--warn:#8f5c10;--orange:#a94f2c;--cyan:#3f7180;--purple:#6d64c0;
  --sh-pop:0 10px 26px rgba(16,22,34,.14),0 2px 6px rgba(16,22,34,.10);
  --sh-dlg:0 24px 60px rgba(16,22,34,.24),0 3px 10px rgba(16,22,34,.14);
}
/* Correctifs clairs pour les rares couleurs fixes du thème sombre */
:root[data-theme="light"] .netbanner{color:#a93250}
:root[data-theme="light"] .netbanner b{color:#c9405e}
:root[data-theme="light"] .tag.http,
:root[data-theme="light"] .tag.https,
:root[data-theme="light"] .tag.stdio,
:root[data-theme="light"] .tag.ws{color:var(--t2);border-color:rgba(13,17,24,.34);background:rgba(13,17,24,.05)}
:root[data-theme="light"] .dot.DOWN{background:#b6bcc7}
/* Relief du thème clair : les cartes deviennent BLANCHES sur le fond gris et
   portent une ombre douce — sans elles, cartes et fond se confondent. */
:root[data-theme="light"] .tile{background:var(--card);border:1px solid var(--border-soft);
  box-shadow:0 1px 2px rgba(16,22,34,.10),0 4px 12px rgba(16,22,34,.07)}
:root[data-theme="light"] .conf-block{background:var(--card);border:1px solid var(--border-soft);
  box-shadow:0 1px 2px rgba(16,22,34,.08),0 3px 10px rgba(16,22,34,.05)}
:root[data-theme="light"] .sec-label{color:var(--t3);border-bottom:1px solid var(--border-soft);padding-bottom:8px}
:root[data-theme="light"] .srow{background:var(--card);border:1px solid var(--border-soft);
  box-shadow:0 1px 1px rgba(16,22,34,.06)}
:root[data-theme="light"] .srow:hover{background:var(--elevated);border-color:var(--border);
  box-shadow:0 2px 7px rgba(16,22,34,.12)}
:root[data-theme="light"] .srow.sel{background:var(--active);border-color:var(--accent);
  box-shadow:0 2px 8px rgba(74,132,254,.18)}
:root[data-theme="light"] .grphead{background:var(--card);border:1px solid var(--border-soft);
  box-shadow:0 1px 1px rgba(16,22,34,.05)}
:root[data-theme="light"] .grphead:hover{background:var(--elevated);border-color:var(--border)}
:root[data-theme="light"] .machine-row,
:root[data-theme="light"] .peer-row,
:root[data-theme="light"] .scard{background:var(--card);border:1px solid var(--border-soft);
  box-shadow:0 1px 2px rgba(16,22,34,.08)}
:root[data-theme="light"] .machine-row:hover,
:root[data-theme="light"] .scard:hover{border-color:var(--border);box-shadow:0 2px 7px rgba(16,22,34,.12)}
:root[data-theme="light"] dialog{background:var(--card);border:1px solid var(--border);
  box-shadow:var(--sh-dlg)}
:root[data-theme="light"] dialog::backdrop{background:rgba(16,22,34,.46);backdrop-filter:blur(3px)}
:root[data-theme="light"] input,
:root[data-theme="light"] select{background:var(--elevated);border:1px solid var(--border);
  box-shadow:inset 0 1px 2px rgba(16,22,34,.05)}
:root[data-theme="light"] input:hover,
:root[data-theme="light"] select:hover{border-color:var(--border-strong)}
:root[data-theme="light"] .sw.on{border-color:rgba(46,122,92,.55);background:rgba(46,122,92,.10)}
:root[data-theme="light"] .sw.off{border-color:var(--border);color:var(--t4)}
:root[data-theme="light"] .tgl.on{border-color:rgba(46,122,92,.55);background:rgba(46,122,92,.10)}
:root[data-theme="light"] .legacy{border-color:var(--border)}
:root[data-theme="light"] .crumb{color:var(--t3)}
:root[data-theme="light"] .logline{color:var(--t3)}
:root[data-theme="light"] #logpanel{background:var(--sidebar);border-top:1px solid var(--border)}
:root[data-theme="light"] .tb-badge{background:var(--card);border-color:var(--border-soft)}
/* Séparateurs de zone : un filet + un fond très légèrement teinté sous les
   titres de section, pour que l'explorateur soit lisible d'un coup d'œil. */
:root[data-theme="light"] #explorer{background:var(--sidebar);border-right:1px solid var(--border);
  box-shadow:1px 0 0 rgba(16,22,34,.03)}
:root[data-theme="light"] .sec-label{background:linear-gradient(var(--sidebar),var(--sidebar))}
:root[data-theme="light"] details.fold,
:root[data-theme="light"] .tabs{background:var(--card);border:1px solid var(--border-soft);
  border-radius:var(--r3);box-shadow:0 1px 2px rgba(16,22,34,.07)}
:root[data-theme="light"] .tool{background:var(--card);border-color:var(--border-soft)}
:root[data-theme="light"] .pill{background:var(--elevated)}

*{box-sizing:border-box;margin:0;padding:0}
/* L'attribut hidden doit gagner contre les display:flex/inline-flex des
   classes : sans cette règle, un élément masqué reste visible. */
[hidden]{display:none !important}
html,body{height:100%}
body{background:var(--chrome);color:var(--t1);font:13px/1.5 var(--sans);display:flex;flex-direction:column;
  overflow:hidden;-webkit-font-smoothing:antialiased;text-rendering:optimizeLegibility}

/* Barres de défilement fines : le défaut du système casse la densité de l'UI. */
*{scrollbar-width:thin;scrollbar-color:rgba(255,255,255,.16) transparent}
*::-webkit-scrollbar{width:11px;height:11px}
*::-webkit-scrollbar-track{background:transparent}
*::-webkit-scrollbar-thumb{background:rgba(255,255,255,.14);border-radius:var(--rpill);
  border:3px solid transparent;background-clip:padding-box}
*::-webkit-scrollbar-thumb:hover{background:rgba(255,255,255,.26);background-clip:padding-box}
*::-webkit-scrollbar-corner{background:transparent}
:root[data-theme="light"] *{scrollbar-color:rgba(15,18,25,.25) transparent}
:root[data-theme="light"] *::-webkit-scrollbar-thumb{background:rgba(15,18,25,.22);background-clip:padding-box}

/* Focus clavier : visible partout, jamais au clic souris. */
:focus{outline:none}
:focus-visible{outline:2px solid var(--accent);outline-offset:1px;border-radius:var(--r2)}
::selection{background:var(--active2)}

/* ===== Barre de titre ===== */
#titlebar{height:44px;flex-shrink:0;display:flex;align-items:center;gap:var(--gap);padding:0 10px 0 var(--pad-sec);
  background:var(--sidebar);border-bottom:1px solid var(--border);user-select:none;position:relative;z-index:20}
#titlebar::after{content:'';position:absolute;inset:0 0 auto;height:1px;
  background:linear-gradient(90deg,transparent,rgba(255,255,255,.06),transparent);pointer-events:none}
/* Marque : monogramme « console » (fenêtre + smile) + wordmark discret.
   L'ancien point bleu + « MCP Command Center » était trop bruyant. */
#titlebar .logo{white-space:nowrap;display:flex;align-items:center;gap:7px;margin-right:6px}
#titlebar .logo .mark{width:17px;height:17px;color:var(--t3);flex-shrink:0}
#titlebar .logo:hover .mark{color:var(--accent)}
#titlebar .logo .wordmark{font-weight:600;font-size:12.5px;letter-spacing:.24px;color:var(--t1);
  text-transform:uppercase}
#titlebar .logo .wm-dim{color:var(--t4);font-weight:500;margin-left:6px;letter-spacing:.18px}
.tb-logout{background:none;border:1px solid var(--border);border-radius:var(--r2);color:var(--t3);
  font-size:10px;line-height:16px;padding:0 6px}
.tb-logout:hover{color:var(--bad);border-color:var(--bad)}
.tb-logout[hidden]{display:none}
/* Pastilles d'état : une seule hauteur, un seul rayon, une seule typo. */
.tb-badge{display:inline-flex;align-items:center;gap:4px;height:var(--h-sm);padding:0 8px;
  font:11px/1 var(--mono);color:var(--t3);background:var(--wash);border:1px solid var(--border-soft);
  border-radius:var(--r2);white-space:nowrap}
.tb-badge b{color:var(--t1);font-weight:600;font-variant-numeric:tabular-nums}
.tb-badge.ok b{color:var(--ok)}
#stAuto{height:var(--h-sm);padding:0 9px;font:11px/1 var(--sans);border-radius:var(--r2);
  border:1px solid var(--border);color:var(--t3);background:transparent}
#stAuto:hover{border-color:var(--accent);color:var(--accent);background:var(--hover)}
#stAuto.auto-off{color:var(--t4);border-style:dashed}
#stAuto.auto-on{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08)}
/* Version + mise à jour : à droite de la ligne Journal (le pied de fenêtre
   reste une seule ligne, même journal replié). Le bouton coupe la propagation
   du clic : cliquer « Mettre à jour » ne doit pas replier le journal. */
.sb-ver{font:11px/1 var(--mono);color:var(--t3);letter-spacing:.02em;flex-shrink:0}
.sb-upd{height:22px;padding:0 12px;font:11px/1 var(--sans);font-weight:700;gap:5px;flex-shrink:0;
  color:#07293d;border-color:#8fd3ff;background:#8fd3ff;border-radius:var(--rpill);
  box-shadow:0 1px 6px rgba(143,211,255,.35)}
.sb-upd:hover{background:#a9dcff;border-color:#a9dcff;color:#07293d}
.sb-upd:active{background:#8fd3ff;border-color:#8fd3ff}
:root[data-theme="light"] .sb-upd{border-color:#5eb8ef}
.tb-spacer{flex:1;height:100%}
/* Groupe d'actions : un seul bloc, filets internes alignés. */
.tb-actions{display:flex;gap:6px;align-items:center}
.tb-actions button{height:var(--h-ctl);padding:0 10px;font-size:12px}
/* Contrôles de fenêtre (mode desktop Tauri, fenêtre sans décorations) */
#winCtrls{display:none;gap:2px;margin-left:6px;align-items:center}
.wbtn{width:34px;height:var(--h-ctl);padding:0;display:flex;align-items:center;justify-content:center;
  border:none;border-radius:var(--r2);background:transparent;color:var(--t3);font-size:12px;cursor:pointer}
.wbtn:hover{background:var(--wash);color:var(--t1)}
.wbtn.wclose:hover{background:rgba(231,94,120,.16);color:var(--bad)}

/* ===== Boutons ===========================================================
   Une hauteur, un rayon, une transition. Le survol teinte, l'accent marque. */
button{display:inline-flex;align-items:center;justify-content:center;gap:6px;
  height:var(--h-ctl);padding:0 11px;background:transparent;color:var(--t2);
  border:1px solid var(--border);border-radius:var(--r2);cursor:pointer;
  font:12px/1 var(--sans);white-space:nowrap;
  transition:background .12s,border-color .12s,color .12s,box-shadow .12s}
button:hover{background:var(--hover);border-color:var(--border-strong);color:var(--t1)}
button:active{background:var(--active)}
button.primary{background:var(--active);border-color:var(--active2);color:var(--accent);font-weight:500}
button.primary:hover{background:var(--active2);border-color:var(--accent);color:var(--accent)}
#tDark.sel,#tLight.sel{background:var(--active);border-color:var(--accent);color:var(--accent)}
/* Destructif : bordure rouge PERMANENTE, pas seulement au survol — une action
   irréversible ne doit pas se fondre visuellement dans les actions neutres. */
button.danger{border-color:rgba(231,94,120,.38);color:var(--bad)}
button.danger:hover{border-color:var(--bad);color:var(--bad);background:rgba(231,94,120,.08)}
/* La suppression est séparée du groupe d'actions courantes du détail. */
.d-actions button.danger{margin-left:12px}
button:disabled{opacity:.35;cursor:not-allowed}

/* ===== Layout principal : explorateur + éditeur ===== */
#main{flex:1;display:flex;min-height:0}

/* --- Explorateur (gauche) --- */
#explorer{width:320px;flex-shrink:0;background:var(--sidebar);border-right:1px solid var(--border);
  display:flex;flex-direction:column;overflow-y:auto;overflow-x:hidden;padding-bottom:2px}
.sec-label{font:10px/1 var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.09em;
  color:var(--t4);padding:14px var(--pad-sec) 7px;user-select:none;
  display:flex;align-items:center;justify-content:space-between;gap:6px}
.sec-label select{width:auto;flex-shrink:0;height:var(--h-xs);background:var(--chrome);border:1px solid var(--border-soft);
  border-radius:var(--r1);color:var(--t3);font:10px/1 var(--mono);padding:0 2px;cursor:pointer;
  text-transform:none;letter-spacing:0}
.sec-label select:hover{color:var(--accent);border-color:var(--accent)}
/* Filtre de l'explorateur (raccourcis : / ou Ctrl+K, Échap pour effacer). */
#srvFilterWrap{padding:7px 8px 3px;display:flex;gap:6px;align-items:center}
#srvFilter{height:24px;font-size:11.5px;flex:1;min-width:0;padding:0 8px}
#srvFilterClear{width:22px;height:22px;padding:0;font-size:13px;color:var(--t4);border-color:transparent;flex-shrink:0}
#srvFilterClear:hover{color:var(--t1);background:var(--wash);border-color:transparent}
/* Barre « serveurs masqués » (session seulement — l'app fermée les rend). */
#hiddenBar{display:flex;align-items:center;gap:7px;margin:3px 8px 2px;padding:5px 10px;
  font:11px/1.3 var(--sans);color:var(--t3);background:var(--wash);
  border:1px dashed var(--border);border-radius:var(--r2);cursor:pointer;user-select:none}
#hiddenBar[hidden]{display:none}
#hiddenBar:hover{color:var(--t1);border-color:var(--border-strong)}
#hiddenBar .ic{width:13px;height:13px;flex-shrink:0}
#hiddenBar b{margin-left:auto;color:var(--accent);font-weight:600;white-space:nowrap}
/* Lignes de serveur : 34 px de haut, filet de sélection aligné sur le texte. */
.srow{padding:6px 10px 6px 13px;margin:1px 8px;border-radius:var(--r3);cursor:pointer;position:relative;
  border:1px solid transparent;transition:background .1s,border-color .1s}
.srow:hover{background:var(--wash)}
.srow.sel{background:var(--active);border-color:var(--active2)}
.srow.sel::before{content:'';position:absolute;left:0;top:6px;bottom:6px;width:2px;
  border-radius:var(--rpill);background:var(--accent)}
.srow.off{opacity:.45}
.srow .l1{display:flex;align-items:center;gap:7px;min-width:0;height:18px}
/* Le nom est SEUL en ligne 1 avec les boutons d'action ; les badges descendent
   en ligne 2. Sur une ligne commune, les badges (non rétractables) écrasaient
   le nom (flex:1) jusqu'à 0 px : plusieurs serveurs n'affichaient plus leur
   nom dans l'explorateur. */
.srow .l1 .nm{flex:1;min-width:0;font-size:12.5px;font-weight:500;color:var(--t1);white-space:nowrap;
  overflow:hidden;text-overflow:ellipsis;letter-spacing:.1px}
.srow .cnt{font:10px/1 var(--mono);color:var(--t4);font-variant-numeric:tabular-nums;flex-shrink:0}
.legacy{font:9px/1 var(--mono);text-transform:uppercase;letter-spacing:.04em;color:var(--t4);
  border:1px dashed var(--border);border-radius:var(--r1);padding:1px 5px;white-space:nowrap;flex-shrink:0}
.ctxmenu{position:fixed;z-index:9999;min-width:270px;background:var(--elevated);border:1px solid var(--border);
  border-radius:var(--r3);padding:5px;box-shadow:var(--sh-pop);font-size:12px}
.ctxmenu[hidden]{display:none}
.ctxitem{padding:6px 9px;border-radius:var(--r2);cursor:pointer;color:var(--t2);white-space:nowrap;
  display:flex;gap:8px;align-items:center;line-height:1.35}
.ctxitem:hover{background:var(--active);color:var(--t1)}
.ctxitem.dis{opacity:.35;cursor:default;pointer-events:none}
.ctxsep{height:1px;background:var(--border-soft);margin:5px 7px}
#toast{position:fixed;left:50%;bottom:28px;transform:translateX(-50%) translateY(8px);
  background:var(--elevated);color:var(--t1);border:1px solid var(--border);border-radius:var(--r3);
  padding:7px 14px;font-size:12px;box-shadow:var(--sh-pop);opacity:0;pointer-events:none;
  transition:opacity .15s,transform .15s;z-index:10000}
#toast.on{opacity:1;transform:translateX(-50%) translateY(0)}
/* Variantes sémantiques : succès et erreur ne passent plus inaperçus. */
#toast.ok{border-color:rgba(85,165,131,.5)}
#toast.err{border-color:rgba(231,94,120,.55);color:#f2b0bd}
.netbanner{background:rgba(231,94,120,.12);border-bottom:1px solid rgba(231,94,120,.45);
  color:#f0a8b6;font-size:11.5px;padding:6px 12px;display:flex;gap:8px;align-items:center}
.netbanner b{color:#ffd7de}
.netbanner[hidden]{display:none}
/* Boutons de ligne : 18 px, rayon 4, jamais plus hauts que le texte. */
.rowbtn{display:inline-flex;align-items:center;justify-content:center;height:var(--h-xs);min-width:20px;
  padding:0 5px;background:none;border:1px solid var(--border-soft);border-radius:var(--r1);color:var(--t3);
  font:9px/1 var(--mono);flex-shrink:0}
.rowbtn:hover{color:var(--accent);border-color:var(--accent);background:var(--hover)}
/* Autoboot par serveur (⚡) : allumé = relance auto au démarrage/crash */
.rowbtn.auto{color:var(--t3);opacity:.45}
.rowbtn.auto.on{color:#f0a35e;border-color:#f0a35e66;background:#f0a35e14;opacity:1}
.rowbtn.auto.on:hover{color:var(--bad);border-color:var(--bad);background:rgba(231,94,120,.08)}
/* Favori (⭐) : épinglé = la ligne passe en haut de sa section (persisté). */
.rowbtn.fav{color:var(--t4);opacity:.45}
.rowbtn.fav:hover{color:#f0c05e;border-color:#f0c05e;background:#f0c05e14;opacity:1}
.rowbtn.fav.on{color:#f0c05e;border-color:#f0c05e66;background:#f0c05e14;opacity:1}
.rowbtn.fav.on .ic{fill:currentColor}
.rowbtn.fav.on:hover{color:var(--bad);border-color:var(--bad);background:rgba(231,94,120,.08)}
/* Bouton Favori du panneau de détail */
.favbtn.on{color:#f0c05e;border-color:#f0c05e66;background:#f0c05e14}
.favbtn.on:hover{color:var(--bad);border-color:var(--bad);background:rgba(231,94,120,.08)}
.srow.off .rowbtn{opacity:.5}
/* Drag & drop de réordonnancement (préférence / priorité de sondage) */
.srow.dragging{opacity:.35}
.srow.drop-above{box-shadow:0 -2px 0 var(--accent)}
.srow.drop-below{box-shadow:0 2px 0 var(--accent)}
/* ===== Sections repliables de l'explorateur =====
   Serveurs locaux à plat, puis UN GROUPE REPLIABLE PAR MACHINE DISTANTE
   (VPS/LAN) : les serveurs importés d'une machine sont rangés sous son IP. */
.sec-label[hidden]{display:none}
.grphead{display:flex;align-items:center;gap:6px;margin:1px 8px;padding:5px 8px;border-radius:var(--r2);
  cursor:pointer;user-select:none;color:var(--t3);border:1px solid transparent;transition:background .1s}
.grphead:hover{background:var(--wash);color:var(--t2)}
.grphead .gchev{font-size:9px;width:10px;flex-shrink:0;color:var(--t4);transition:transform .12s}
.grphead.closed .gchev{transform:rotate(-90deg)}
.grphead .gname{flex:1;font-size:11px;font-weight:600;letter-spacing:.03em;
  white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.grphead .gcount{font:10px/1 var(--mono);color:var(--t4);flex-shrink:0;font-variant-numeric:tabular-nums}
.grphead .gact{background:none;border:none;padding:0 3px;font-size:11px;line-height:16px;color:var(--t4);flex-shrink:0}
.grphead .gact:hover{color:var(--bad);background:none;border:none}
.grphead.closed .gact{display:none}
.grphead.drag-over-group{border-color:var(--accent);background:var(--active)}
.grpbody.closed{display:none}
/* Rangées distantes : légère indentation sous leur groupe machine */
.mbody .srow{padding-left:23px}
.mbody .srow.sel::before{left:10px}
.sw{display:inline-flex;align-items:center;justify-content:center;height:var(--h-xs);min-width:34px;
  font:9px/1 var(--mono);font-weight:600;border-radius:var(--rpill);padding:0 6px;flex-shrink:0;
  border:1px solid var(--border-soft);background:none;color:var(--t3)}
.sw.on{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08)}
.sw.on:hover{border-color:var(--bad);color:var(--bad);background:rgba(231,94,120,.06)}
.sw.off{color:var(--t4);border-color:var(--border);text-decoration:line-through}
.sw.off:hover{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08);text-decoration:none}
.srow .l2{display:flex;align-items:center;gap:6px;margin-top:2px;padding-left:20px;min-width:0;
  font:10.5px/1.35 var(--mono);color:var(--t3);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.srow .l2 .langchip{height:14px}
/* Les logos de langage sont plus petits dans l'explorateur que dans le détail. */
.srow .rustlogo{width:14px;height:14px}
.srow .l2 .st{font-weight:600}
.srow .l2 .st.UP{color:var(--ok)}.srow .l2 .st.PORT-ONLY{color:var(--orange)}
.srow .l2 .st.DOWN{color:var(--t4)}.srow .l2 .st.DISABLED{color:var(--t4)}
.srow .l2 .st.PROBING,.srow .l2 .st.STARTING{color:var(--accent)}
.dot{width:7px;height:7px;border-radius:50%;flex-shrink:0}
.dot.UP{background:var(--ok);box-shadow:0 0 6px var(--ok)}
.dot.PORT-ONLY{background:var(--orange);box-shadow:0 0 6px var(--orange)}
.dot.DOWN{background:#3d3d40}
.dot.DISABLED{background:transparent;border:1px solid var(--border)}
.dot.PROBING,.dot.STARTING{background:var(--accent);animation:pulse 1s infinite}
@keyframes pulse{50%{opacity:.25}}
.peer-row{padding:6px 10px 6px 13px;margin:1px 8px;border-radius:var(--r3)}
.peer-row .l1{display:flex;align-items:center;gap:8px}
.peer-row .l1 .nm{flex:1;font:12px/1.35 var(--mono);color:var(--t1);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.peer-row .l2{font:10.5px/1.35 var(--mono);color:var(--t3);padding-left:20px;margin-top:1px}
.peer-row button{margin-top:5px;margin-left:20px;height:var(--h-sm);padding:0 9px;font-size:11px}
#addWrap{margin-top:auto;padding:10px 8px 12px;border-top:1px solid var(--border-soft)}
#addWrap button{width:100%;height:30px}
.empty-sec{font:11px/1.4 var(--sans);color:var(--t4);padding:4px var(--pad-sec) 10px;font-style:italic}

/* --- Zone détail (droite) --- */
#content{flex:1;min-width:0;overflow-y:auto;padding:24px 28px 32px}
.crumb{font:11px/1 var(--mono);color:var(--t4);margin-bottom:12px;letter-spacing:.02em}
.crumb b{color:var(--t3);font-weight:400}
.d-head{display:flex;align-items:center;gap:12px;margin-bottom:6px;flex-wrap:wrap}
.d-head h1{font-size:19px;font-weight:600;letter-spacing:.1px;line-height:1.2}
.d-head .sub{font:12px/1.35 var(--mono);color:var(--t3)}
.d-actions{display:flex;gap:6px;margin-left:auto;flex-wrap:wrap}
.pill{display:inline-flex;align-items:center;height:var(--h-sm);font:10.5px/1 var(--mono);font-weight:600;
  padding:0 10px;border-radius:var(--rpill);border:1px solid}
.pill.UP{color:var(--ok);border-color:rgba(85,165,131,.4);background:rgba(85,165,131,.08)}
.pill.PORT-ONLY{color:var(--orange);border-color:rgba(219,112,75,.4);background:rgba(219,112,75,.08)}
.pill.DOWN{color:var(--t3);border-color:var(--border);background:var(--wash)}
.pill.DISABLED{color:var(--t4);border-color:var(--border);background:transparent;border-style:dashed}
.pill.PROBING,.pill.STARTING{color:var(--accent);border-color:var(--active2);background:var(--active)}
.d-err{color:var(--bad);font:11.5px/1.5 var(--mono);background:rgba(231,94,120,.06);
  border:1px solid rgba(231,94,120,.25);border-radius:var(--r3);padding:9px 12px;margin:14px 0 0;word-break:break-word}
.seed-note{font-size:11.5px/1.5 var(--sans);color:var(--t3);background:var(--wash);border:1px solid var(--border-soft);
  border-radius:var(--r3);padding:8px 12px;margin:14px 0 0}
.seed-note b{color:var(--t2)}

/* ===== Onglets ===========================================================
   Bandeau plein, filet continu, soulignement 2 px qui RECOUVRE le filet :
   l'onglet actif est une marque, pas une pastille. */
.tabs{display:flex;gap:2px;margin:22px 0 0;border-bottom:1px solid var(--border)}
.tab{height:30px;padding:0 13px;border:1px solid transparent;border-bottom:2px solid transparent;
  background:none;border-radius:var(--r2) var(--r2) 0 0;color:var(--t3);font:12px/1 var(--sans);
  margin-bottom:-1px;transition:color .12s,background .12s,border-color .12s}
.tab:hover{color:var(--t1);background:var(--wash);border-bottom-color:transparent}
.tab.sel{color:var(--t1);font-weight:600;background:transparent;border-bottom:2px solid var(--accent)}
.tab.sel:hover{background:var(--hover)}
.conf-note{font:11.5px/1.5 var(--sans);color:var(--t4);margin:14px 0 12px}
.conf-note b{color:var(--t3)}
.conf-block{background:var(--sidebar);border:1px solid var(--border-soft);border-radius:var(--r3);
  margin-bottom:10px;overflow:hidden}
.conf-head{display:flex;align-items:center;gap:8px;padding:0 12px;height:32px;font:11px/1 var(--sans);
  color:var(--t3);border-bottom:1px solid var(--border-soft);background:var(--wash)}
.conf-head button{margin-left:auto;height:var(--h-sm);padding:0 10px;font-size:11px}
.conf-block pre{margin:0;padding:12px 14px;font:11.5px/1.6 var(--mono);color:var(--t2);overflow-x:auto;white-space:pre}
/* Barre latérale ajustable */
#grip{width:5px;flex-shrink:0;cursor:col-resize;background:transparent;transition:background .12s}
#grip:hover,#grip.drag{background:var(--active2)}
h4.sec{font:10px/1 var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.09em;color:var(--t4);
  margin:26px 0 11px;display:flex;align-items:center;gap:10px}
h4.sec::after{content:'';flex:1;height:1px;background:var(--border-soft)}
/* Replis du détail (Informations / Démarrage) — même style que h4.sec */
details.fold{margin:26px 0 0}
details.fold summary{list-style:none;cursor:pointer;user-select:none;
  font:10px/1 var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.09em;color:var(--t4);
  display:flex;align-items:center;gap:10px;transition:color .12s}
details.fold summary::-webkit-details-marker{display:none}
details.fold summary::before{content:'\25B8';font-size:9px;flex-shrink:0;transition:color .12s}
details.fold[open] summary::before{content:'\25BE'}
details.fold summary::after{content:'';flex:1;height:1px;background:var(--border-soft)}
details.fold summary:hover{color:var(--accent)}
details.fold .kv{margin-top:12px}
.kv{display:grid;grid-template-columns:168px 1fr;gap:7px 16px;font-size:12.5px;align-items:baseline}
.kv .k{color:var(--t4);font-size:12px}
.kv .v{font:12px/1.45 var(--mono);color:var(--t2);word-break:break-all}
/* Valeur copiable d'un clic (pid, commande, url…) — l'affordance est au
   survol, le menu contextuel reste disponible. */
.kv .v.cpy{cursor:pointer}
.kv .v.cpy:hover{color:var(--accent)}

/* ===== Outils ============================================================
   Lignes nettes : nom en mono accent, description en gris, bascule à droite. */
.tool{display:flex;align-items:center;gap:12px;background:var(--card);border:1px solid var(--border-soft);
  border-radius:var(--r3);padding:8px 12px;margin-bottom:6px;transition:border-color .1s,background .1s}
.tool:hover{border-color:var(--border);background:var(--elevated)}
.tool.off{opacity:.55}
.tool code{color:var(--accent);font:12px/1.35 var(--mono);flex-shrink:0}
.tool .desc{flex:1;font-size:12px;color:var(--t3);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
/* Rendu markdown inline des descriptions : le gras ressort légèrement, le
   code s'affiche comme du code — au lieu d'astérisques bruts. */
.tool .desc b{color:var(--t2);font-weight:600}
.tool .desc code{font:11px/1.3 var(--mono);color:var(--t2);background:var(--wash);
  border:1px solid var(--border-soft);border-radius:3px;padding:0 4px}
.tgl{display:inline-flex;align-items:center;justify-content:center;height:var(--h-sm);min-width:52px;
  font:10.5px/1 var(--mono);font-weight:600;border-radius:var(--rpill);padding:0 12px;
  border:1px solid var(--border-soft);background:transparent;color:var(--t3)}
.tgl.on{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08)}
.tgl.on:hover{border-color:var(--bad);color:var(--bad);background:rgba(231,94,120,.06)}
.tgl.off{color:var(--t4);border-color:var(--border);text-decoration:line-through}
.tgl.off:hover{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08);text-decoration:none}

/* Vue d'ensemble */
.welcome{max-width:560px;margin:64px auto 0;text-align:center}
.welcome .big{font-size:15px;color:var(--t2);margin-bottom:6px}
.welcome .hint{font-size:12.5px;color:var(--t4);margin-bottom:22px}
.welcome .qa{display:flex;gap:8px;justify-content:center;flex-wrap:wrap}
.tiles{display:grid;grid-template-columns:repeat(auto-fit,minmax(130px,1fr));gap:10px;margin-bottom:6px}
.tile{background:var(--card);border:1px solid var(--border-soft);border-radius:var(--r4);padding:13px 14px}
.tile .n{font:22px/1.1 var(--mono);font-weight:600;color:var(--t1);font-variant-numeric:tabular-nums}
.tile .n.ok{color:var(--ok)}
.tile .lbl{font-size:11px;color:var(--t4);margin-top:3px}
/* Tuiles cliquables (filtre UP, tri par tools, machines) : une tuile est un
   bouton sans l'apparence bouton par défaut. */
button.tile{height:auto;text-align:left;font:inherit;color:inherit;cursor:pointer}
button.tile:hover{border-color:var(--border-strong)}
.tile.sel{border-color:var(--active2);background:var(--active)}

/* --- Journal (bas, style console IDE) --- */
#logpanel{flex-shrink:0;height:170px;background:var(--sidebar);border-top:1px solid var(--border);
  display:flex;flex-direction:column}
#logpanel.min{height:32px}
#loghead{height:32px;flex-shrink:0;display:flex;align-items:center;gap:8px;padding:0 14px;cursor:pointer;
  user-select:none;border-bottom:1px solid var(--border-soft)}
#logpanel.min #loghead{border-bottom:none}
#loghead .lbl{font:10px/1 var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.09em;color:var(--t4)}
#loghead .arrow{color:var(--t4);font-size:10px;transition:transform .15s}
#logpanel.min #loghead .arrow{transform:rotate(-90deg)}
#logbody{flex:1;overflow-y:auto;padding:4px 14px 10px;font:11px/1.6 var(--mono)}
#logpanel.min #logbody{display:none}
.logline{padding:1px 0;color:var(--t3);white-space:pre-wrap}
.logline .t{color:var(--t4);font-variant-numeric:tabular-nums}
.logline.warn{color:var(--warn)}
.logline.error{color:var(--bad)}

/* --- Boutique MCP (directory à cartes + import) --- */
.tag{display:inline-flex;align-items:center;height:var(--h-xs);font:10px/1 var(--mono);padding:0 7px;
  border-radius:var(--r1);border:1px solid var(--border);color:var(--t3);white-space:nowrap}
.tag.npm{color:#bc8cff;border-color:#bc8cff44}
.tag.py{color:#39c5cf;border-color:#39c5cf44}
/* Transports : MONOCHROME, trois niveaux de gris distincts (aucune teinte de
   langage). http = le plus clair, stdio = médian, ws = le plus sourd. */
.tag.stdio{color:#b9bdc7;border-color:rgba(185,189,199,.42);background:rgba(185,189,199,.06)}
.tag.http{color:#e8e8ec;border-color:rgba(232,232,236,.45);background:rgba(232,232,236,.07)}
.tag.sse{color:var(--ok);border-color:rgba(85,165,131,.45)}
.tag.ws{color:#8b909b;border-color:rgba(139,144,155,.42);background:rgba(139,144,155,.06)}
.tag.https{color:#e8e8ec;border-color:rgba(232,232,236,.45);background:rgba(232,232,236,.07)}
/* LiteMCP (fingerprint) : le SEUL badge à halo — orange, la couleur de Rust.
   Halo STATIQUE : un pulse permanent sur une liste attire l'œil sans fin ;
   l'effet reste lisible sans animation. */
.tag.litemcp{color:#fdba74;border-color:#f9731699;background:#f973161f;box-shadow:0 0 8px rgba(249,115,22,.4)}
/* FastMCP (mcp-proxy httpStream) : repli neutre — le JS inline impose la couleur du langage */
.tag.fastmcp{color:#8fd3dE;border-color:#6f9ba688;background:#6f9ba61f;box-shadow:0 0 8px rgba(111,155,166,.45)}
/* autres frameworks MCP détectés par nom (rmcp, mcp-go, Spring AI, xmcp…) */
.tag.fw{color:#a5b4fc;border-color:#a5b4fc55;background:#a5b4fc11}
/* Icônes SVG (jeu unique, trait 1.5, couleur courante — aucun emoji) :
   rendu identique quelle que soit la version d'Windows, alignement
   typographique propre, cohérence avec l'interface monochrome. */
.ic{width:14px;height:14px;fill:none;stroke:currentColor;stroke-width:1.5;
  stroke-linecap:round;stroke-linejoin:round;display:inline-block;vertical-align:-2px;flex-shrink:0}
.rowbtn .ic{width:10px;height:10px}
.ctxitem .ic{width:13px;height:13px}
.tb-badge .ic{width:12px;height:12px}
.scard .ic,.set-nav-btn .ic{width:13px;height:13px}
.netbanner .ic,.d-err .ic{width:13px;height:13px}
.srow .l2 .tag{font-size:8.5px;padding:0 5px;height:14px}
.langchip{display:inline-flex;align-items:center;height:15px;font:8.5px/1 var(--mono);font-weight:700;
  border:1px solid;border-radius:var(--r1);padding:0 4px;flex-shrink:0}
.langchip.unk{color:var(--t4);border-color:var(--border-soft);background:transparent;font-weight:400}
/* Logo Rust (langage inféré) : « R » cerclé d'un engrenage, comme l'icône .rs des éditeurs */
.rustwrap{display:inline-flex;align-items:center;flex-shrink:0;margin:0 1px}
.rustlogo{width:18px;height:18px}
/* Logos langages (SVG officiels embarqués : JS, Python, Go, Java, .NET, Ruby, PHP) */
.langlogo{width:18px;height:18px;display:block}
/* Dans l'explorateur, tous les logos de langage (Rust comme les autres)
   descendent à 14 px pour ne pas gonfler la hauteur des lignes. */
.srow .langlogo{width:14px;height:14px}
#storeDlg{width:min(1180px,96vw)}
/* Tête de boutique COLLANTE : recherche, tri et catégories restent visibles
   pendant le défilement de la grille (le dialog est le conteneur défilant). */
.store-head{position:sticky;top:0;z-index:3;background:var(--card);
  margin:-14px -16px 0;padding:12px 16px 10px;border-bottom:1px solid var(--border-soft)}
.store-bar{display:flex;gap:10px;align-items:center;margin-bottom:8px;flex-wrap:wrap}
.store-bar input{flex:1;min-width:180px}
.store-bar select{width:auto;flex:0 0 auto;height:26px;font-size:11px}
/* Bascule « Masquer les installés » : une pilule cohérente avec les
   catégories (état .sel), pas une case à cocher minuscule. */
#storeHideInst{flex:0 0 auto}
#storeHideInst.sel{background:var(--active2);border-color:var(--accent);color:var(--accent)}
.store-results{font:10.5px/1 var(--mono);color:var(--t4);margin-bottom:8px}
.store-count{color:var(--t3);font-weight:400;font-size:12.5px;margin-left:2px}
/* Machines distantes (dialog SSH) */
.machine-form{display:flex;gap:8px;flex-wrap:wrap;margin-bottom:10px}
.machine-form input{flex:1;min-width:170px}
.machine-msg{font:11px/1.5 var(--mono);color:var(--t2);min-height:16px;margin-bottom:8px}
.machine-row{background:var(--sidebar);border:1px solid var(--border-soft);border-radius:var(--r3);padding:9px 11px;margin-bottom:8px}
.machine-row .l1{display:flex;align-items:center;gap:8px}
.machine-row .nm{font:12.5px/1.35 var(--mono);color:var(--t1)}
.machine-row .l2{font:10.5px/1.4 var(--mono);color:var(--t3);margin:3px 0 7px}
.machine-row .macts{display:flex;gap:6px;flex-wrap:wrap}
.store-cats{display:flex;gap:6px;flex-wrap:wrap}
.scat{height:var(--h-sm);padding:0 12px;border-radius:var(--rpill);font-size:11px}
.scat.sel{background:var(--active2);border-color:var(--accent);color:var(--accent)}
.store-grid{display:grid;grid-template-columns:repeat(auto-fill,minmax(184px,1fr));gap:7px}
.scard{background:var(--sidebar);border:1px solid var(--border-soft);border-radius:var(--r3);padding:9px 10px;
  display:flex;flex-direction:column;gap:4px;transition:border-color .1s}
.scard:hover{border-color:var(--border)}
.scard-top{display:flex;align-items:center;gap:6px}
.scard-top b{flex:1;font-size:12px}
.slogo{width:18px;height:18px;flex-shrink:0}
.scard-desc{font-size:10.5px/1.45 var(--sans);color:var(--t3);flex:1;display:-webkit-box;-webkit-line-clamp:2;
  -webkit-box-orient:vertical;overflow:hidden}
.scard button{align-self:flex-start;height:var(--h-sm);padding:0 9px;font-size:11px}
/* Déjà installé : plus de bouton Installer — état vert non cliquable. */
.scard button.btn-inst{color:var(--ok);border-color:rgba(85,165,131,.45);background:rgba(85,165,131,.08);cursor:default}
.scard button.btn-inst:disabled{opacity:1}

/* --- Dialogues (modal) --- */
dialog{background:var(--card);color:var(--t1);border:1px solid var(--border);border-radius:var(--r5);padding:0;
  width:min(680px,92vw);max-height:85vh;box-shadow:var(--sh-dlg);
  /* le reset global `*{margin:0}` supprime le `margin:auto` du navigateur :
     sans cette ligne les modales s'ouvrent collées en haut à gauche */
  margin:auto;max-width:92vw}
dialog::backdrop{background:rgba(0,0,0,.62);backdrop-filter:blur(2px)}
.dlg-head{display:flex;align-items:center;gap:10px;padding:0 20px;height:52px;
  border-bottom:1px solid var(--border-soft)}
.dlg-head h2{font-size:15px;font-weight:600;flex:1;letter-spacing:.1px}
.dlg-head .close{background:none;border:none;height:var(--h-ctl);width:var(--h-ctl);padding:0;
  font-size:16px;color:var(--t4);cursor:pointer;border-radius:var(--r2)}
.dlg-head .close:hover{color:var(--t1);background:var(--wash);border:none}
/* ===== Dialog « Réglages » : mise en page réglages système =====
   Navigation par sections à gauche (volets), lignes libellé/description à
   gauche et contrôle à droite, segmented control pour le thème, switch pour
   les booleans. L'état « non enregistré » est affiché dans le pied. */
.settings-dlg{width:min(820px,95vw)}
.dlg-title{display:flex;align-items:center;gap:11px;flex:1;min-width:0}
.dlg-title .mark{width:19px;height:19px;color:var(--t3);flex-shrink:0}
.dlg-title h2{margin:0;font-size:14.5px;font-weight:600;letter-spacing:.1px}
.dlg-title p{margin:2px 0 0;font:11.5px/1.3 var(--sans);color:var(--t4)}
/* Volets : nav étroite + panneau courant, hauteur bornée avec défilement. */
.set-shell{display:flex;min-height:0}
.set-nav{width:162px;flex-shrink:0;padding:12px 8px;border-right:1px solid var(--border-soft);
  background:var(--sidebar);display:flex;flex-direction:column;gap:2px}
.set-nav-btn{display:flex;align-items:center;gap:8px;height:30px;padding:0 10px;justify-content:flex-start;
  border:none;background:none;color:var(--t3);font:12px/1 var(--sans);text-align:left;border-radius:var(--r2);
  white-space:nowrap;transition:background .1s,color .1s}
.set-nav-btn:hover{background:var(--wash);color:var(--t1)}
.set-nav-btn.sel{background:var(--active);color:var(--accent);font-weight:600}
.set-nav-btn.danger:hover{color:var(--bad);background:rgba(231,94,120,.08)}
.set-nav-btn.danger.sel{background:rgba(231,94,120,.12);color:var(--bad)}
.set-nav-sep{height:1px;background:var(--border-soft);margin:8px 6px;flex-shrink:0}
.set-panes{flex:1;min-width:0;max-height:min(62vh,560px);overflow-y:auto}
.set-pane{display:none}
.set-pane.sel{display:block}
/* Ligne de réglage */
.set-list{background:var(--card);border:1px solid var(--border-soft);border-radius:var(--r4);overflow:hidden}
.set-row{display:flex;align-items:center;gap:18px;padding:12px 15px;min-height:52px}
.set-row+.set-row{border-top:1px solid var(--border-soft)}
/* Variante empilée : libellé au-dessus, contrôle pleine largeur dessous
   (webhook, en-têtes — un contrôle large ne doit jamais écraser le libellé). */
.set-row.col{flex-direction:column;align-items:stretch;gap:9px}
.set-row.col .set-ctrl{max-width:none}
.set-label{flex:1;min-width:0;display:flex;flex-direction:column;gap:3px}
.set-label span{font:12.5px/1.3 var(--sans);color:var(--t1)}
.set-label em{font:11.5px/1.45 var(--sans);color:var(--t4);font-style:normal;max-width:46ch}
.set-ctrl{display:flex;align-items:center;gap:8px;flex-shrink:0;justify-content:flex-end;max-width:52%}
.set-ctrl-wide{justify-content:flex-start;width:100%;max-width:none}
.set-ctrl-wide input,.set-ctrl-wide textarea{width:100%}
.set-num{width:86px !important;height:30px !important;text-align:right}
.set-txt{width:150px !important;height:30px !important}
.set-unit,.set-dash{font:11.5px var(--mono);color:var(--t4);flex-shrink:0}
.set-static{font:12px var(--mono);color:var(--t2);min-width:64px;text-align:right}
.set-note{font:11.5px var(--sans);color:var(--t2)}
.set-msgline{padding:0 20px}
/* Erreur de champ (validation par ligne, sous la description) */
.set-err{font:11px/1.4 var(--sans);color:var(--bad)}
.set-err[hidden]{display:none}
/* Pied : indicateur de modifications non enregistrées */
.set-dirty{font:11.5px/1.3 var(--sans);color:var(--warn);white-space:nowrap}
/* Cases à cocher simples (états notifiés) */
.set-inline{flex-wrap:wrap}
.set-chk{display:inline-flex;align-items:center;gap:6px;font:12px var(--sans);color:var(--t2);cursor:pointer}
.set-chk input{width:14px;height:14px;accent-color:var(--accent);cursor:pointer}
/* Zone d'en-têtes webhook (même style que les inputs) */
textarea{width:100%;min-height:56px;background:var(--chrome);border:1px solid var(--border);border-radius:var(--r2);
  padding:8px 10px;color:var(--t1);font:12px/1.5 var(--mono);resize:vertical;
  transition:border-color .12s,box-shadow .12s}
textarea:hover{border-color:var(--border-strong)}
textarea:focus{outline:none;border-color:var(--accent);box-shadow:0 0 0 3px var(--active)}
/* Segmented control (thème) */
.seg{background:var(--chrome);border:1px solid var(--border);border-radius:var(--r2);padding:2px;gap:2px}
.seg-btn{border:none !important;background:transparent !important;height:24px;padding:0 13px;
  font-size:11.5px !important;color:var(--t3) !important;border-radius:var(--r1) !important}
.seg-btn:hover{color:var(--t1) !important;background:var(--hover) !important}
.seg-btn.sel{background:var(--card) !important;color:var(--accent) !important;font-weight:600;
  box-shadow:0 1px 2px rgba(0,0,0,.18)}
/* Switch (booleens) */
.switch{position:relative;display:inline-flex;align-items:center;cursor:pointer}
.switch input{position:absolute;opacity:0;width:0;height:0}
.switch-track{width:34px;height:20px;border-radius:var(--rpill);background:var(--chrome);
  border:1px solid var(--border);position:relative;transition:background .14s,border-color .14s}
.switch-track::after{content:'';position:absolute;top:2px;left:2px;width:14px;height:14px;border-radius:50%;
  background:var(--t3);transition:transform .14s,background .14s}
.switch input:checked+.switch-track{background:var(--accent);border-color:var(--accent)}
.switch input:checked+.switch-track::after{transform:translateX(14px);background:#fff}
.switch input:focus-visible+.switch-track{box-shadow:0 0 0 3px var(--active)}
.dlg-foot{display:flex;align-items:center;gap:12px;padding:13px 20px;border-top:1px solid var(--border-soft);
  background:var(--sidebar)}
.dlg-foot .row{margin-left:auto}
.dlg-foot-note{font:11.5px var(--sans);color:var(--t4)}
/* Volets clairs : la nav garde son fond, la carte suit le thème */
:root[data-theme="light"] .set-nav{background:var(--sidebar)}
:root[data-theme="light"] .set-pane .set-list{background:var(--card)}

.dlg-body{padding:18px 20px 20px}
form{display:grid;gap:12px}
form label{font-size:11px;color:var(--t4);display:block;margin-bottom:5px;letter-spacing:.02em}
input,select{width:100%;height:32px;background:var(--chrome);border:1px solid var(--border);border-radius:var(--r2);
  padding:0 10px;color:var(--t1);font:12.5px var(--mono);transition:border-color .12s,box-shadow .12s}
input:hover,select:hover{border-color:var(--border-strong)}
input:focus,select:focus{outline:none;border-color:var(--accent);box-shadow:0 0 0 3px var(--active)}
.row{display:flex;gap:10px}
.row>div{flex:1}


/* ===== Utilitaires de mise en page =======================================
   Ces classes remplacent les anciens style="" dispersés dans le HTML :
   la mise en page vit dans la feuille de style, pas dans les balises. */
.tb-push{margin-left:auto}
/* Utilitaires référencés par le HTML/JS (refonte : les style="" dispersés
   sont devenus des classes — les règles ci-dessous les réalisent). */
.stack{display:grid;gap:6px}
.row.row-gap{gap:8px}
.row.row-end{justify-content:flex-end}
.row.row-between{justify-content:space-between}
.note-gap,.hint-gap{margin-top:14px}
.dlg-body-tight{padding:14px 16px 16px}
.empty-note{font:11px/1.4 var(--sans);color:var(--t4);padding:10px 0;font-style:italic}
.empty-indent{padding-left:23px}
.dot.dot-lg{width:10px;height:10px}
</style>
</head>
<body>
<script>/* thème AVANT le premier rendu (anti-flash) : préférence ⚙ Réglages */
try{document.documentElement.dataset.theme=localStorage.getItem('mcc.theme')==='light'?'light':'dark'}catch(e){}
</script>

<div id="netBanner" class="netbanner" hidden></div>

<div id="titlebar" data-tauri-drag-region>
  <span class="logo" data-tauri-drag-region>
    <svg class="mark" viewBox="0 0 24 24" aria-hidden="true">
      <rect x="2.5" y="4.5" width="19" height="15" rx="3.2" fill="none" stroke="currentColor" stroke-width="1.5"/>
      <circle cx="8" cy="10" r="1.7" fill="currentColor"/>
      <path d="M5.5 16.2c1.6-1.5 3.1-2.2 4.6-2.2s3 .7 4.6 2.2" fill="none" stroke="currentColor" stroke-width="1.5" stroke-linecap="round"/>
    </svg>
    <span class="wordmark">MCP<span class="wm-dim"> Control</span></span>
  </span>
  <button id="stAuto" onclick="toggleAutostart()" title="Lancer l'application au démarrage de l'OS (tray silencieux)"></button>
  <span class="tb-badge ok" title="Serveurs MCP qui répondent, sur le total suivi">UP <b id="stUp">0</b>/<b id="stTotal">0</b></span>
  <span class="tb-badge" title="Outils MCP exposés par les serveurs UP">outils <b id="stTools">0</b></span>
  <span class="tb-badge" title="Rafraîchissement automatique de l'état (réglable dans Réglages)">refresh <b id="stRefresh">15</b>&nbsp;s</span>
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
    <span class="tb-badge tb-push">auto-d&eacute;tection : HTTP streamable &middot; stdio JSON-RPC</span>
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

<script>
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
  // Bandeau réseau : la passerelle LAN ne répond pas → les serveurs distants
  // sont injoignables (VPN le plus souvent). On l'explique au lieu de laisser
  // croire à une panne de détection.
  const nb = document.getElementById('netBanner');
  if (nb) {
    const off = state.lan_ok === false;
    nb.hidden = !off;
    if (off) nb.innerHTML = '<b>' + ic('warn') + ' R&eacute;seau local injoignable</b>'
      + '<span>les serveurs distants ne peuvent pas &ecirc;tre sond&eacute;s &mdash; VPN actif ? '
      + '(Proton VPN : activer &laquo;&nbsp;autoriser les connexions LAN&nbsp;&raquo;)</span>';
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
   toast(msg,'err') erreur (affichée plus longtemps — un échec se lit). */
function toast(msg, kind) {
  const t = document.getElementById('toast');
  if (!t) return;
  t.textContent = msg;
  t.classList.remove('ok', 'err');
  if (kind) t.classList.add(kind);
  t.classList.add('on');
  clearTimeout(toast._t);
  toast._t = setTimeout(() => t.classList.remove('on'), kind === 'err' ? 3500 : 1600);
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
</script>
<div id="ctxmenu" class="ctxmenu" hidden></div>
<div id="toast"></div>
</body>
</html>
"#;

#[cfg(test)]
mod tests {
    use super::INDEX_HTML;

    // ---------- contrat « tout bouton est câblé » --------------------------
    // Chaque handler référencé par un attribut on* du HTML embarqué (y compris
    // dans les gabarits JS qui génèrent les lignes de serveur, les menus
    // contextuels, la boutique…) doit être défini dans la page. Un handler
    // référencé et absent = bouton mort : ce test le fait échouer au compile
    // test, pas devant l'utilisateur.

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
        let mut names = Vec::new();
        for attr in ON_ATTRS {
            let mut rest: &str = INDEX_HTML;
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
        INDEX_HTML.contains(&format!("function {name}("))
            || INDEX_HTML.contains(&format!("const {name} ="))
            || INDEX_HTML.contains(&format!("let {name} ="))
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
        let mut out = Vec::new();
        for needle in ["action: '", "'action': '"] {
            let mut rest: &str = INDEX_HTML;
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
}
