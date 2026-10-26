pub const INDEX_HTML: &str = r#"<!doctype html>
<html lang="fr">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Console MCP</title>
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
.tb-badge.upd{color:var(--warn);border-color:rgba(192,133,50,.5);background:rgba(192,133,50,.1);cursor:pointer}
.tb-badge.upd:hover{border-color:var(--warn);color:var(--warn)}
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
button.danger:hover{border-color:var(--bad);color:var(--bad);background:rgba(231,94,120,.06)}
button:disabled{opacity:.35;cursor:not-allowed}

/* ===== Layout principal : explorateur + éditeur ===== */
#main{flex:1;display:flex;min-height:0}

/* --- Explorateur (gauche) --- */
#explorer{width:280px;flex-shrink:0;background:var(--sidebar);border-right:1px solid var(--border);
  display:flex;flex-direction:column;overflow-y:auto;overflow-x:hidden;padding-bottom:2px}
.sec-label{font:10px/1 var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.09em;
  color:var(--t4);padding:14px var(--pad-sec) 7px;user-select:none;
  display:flex;align-items:center;justify-content:space-between;gap:6px}
.sec-label select{width:auto;flex-shrink:0;height:var(--h-xs);background:var(--chrome);border:1px solid var(--border-soft);
  border-radius:var(--r1);color:var(--t3);font:10px/1 var(--mono);padding:0 2px;cursor:pointer;
  text-transform:none;letter-spacing:0}
.sec-label select:hover{color:var(--accent);border-color:var(--accent)}
/* Lignes de serveur : 34 px de haut, filet de sélection aligné sur le texte. */
.srow{padding:6px 10px 6px 13px;margin:1px 8px;border-radius:var(--r3);cursor:pointer;position:relative;
  border:1px solid transparent;transition:background .1s,border-color .1s}
.srow:hover{background:var(--wash)}
.srow.sel{background:var(--active);border-color:var(--active2)}
.srow.sel::before{content:'';position:absolute;left:0;top:6px;bottom:6px;width:2px;
  border-radius:var(--rpill);background:var(--accent)}
.srow.off{opacity:.45}
.srow .l1{display:flex;align-items:center;gap:7px;min-width:0;height:16px}
.srow .l1 .nm{flex:1;font-size:12.5px;font-weight:500;color:var(--t1);white-space:nowrap;
  overflow:hidden;text-overflow:ellipsis;letter-spacing:.1px}
.srow .l1 .cnt{font:10px/1 var(--mono);color:var(--t4);font-variant-numeric:tabular-nums}
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
/* Bouton Autoboot du panneau de détail */
.autobtn.on{color:#f0a35e;border-color:#f0a35e66;background:#f0a35e14}
.autobtn.on:hover{color:var(--bad);border-color:var(--bad);background:rgba(231,94,120,.08)}
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
.srow .l2{display:flex;align-items:center;gap:6px;margin-top:2px;padding-left:20px;
  font:10.5px/1.35 var(--mono);color:var(--t3);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
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

/* ===== Outils ============================================================
   Lignes nettes : nom en mono accent, description en gris, bascule à droite. */
.tool{display:flex;align-items:center;gap:12px;background:var(--card);border:1px solid var(--border-soft);
  border-radius:var(--r3);padding:8px 12px;margin-bottom:6px;transition:border-color .1s,background .1s}
.tool:hover{border-color:var(--border);background:var(--elevated)}
.tool.off{opacity:.55}
.tool code{color:var(--accent);font:12px/1.35 var(--mono);flex-shrink:0}
.tool .desc{flex:1;font-size:12px;color:var(--t3);overflow:hidden;text-overflow:ellipsis;white-space:nowrap}
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
/* LiteMCP (fingerprint) : le SEUL badge à halo — orange, la couleur de Rust. */
.tag.litemcp{color:#fdba74;border-color:#f9731699;background:#f973161f;box-shadow:0 0 10px rgba(249,115,22,.55);animation:glowLite 2.4s ease-in-out infinite}
/* FastMCP (mcp-proxy httpStream) : repli neutre — le JS inline impose la couleur du langage */
.tag.fastmcp{color:#8fd3dE;border-color:#6f9ba688;background:#6f9ba61f;box-shadow:0 0 8px rgba(111,155,166,.45)}
/* autres frameworks MCP détectés par nom (rmcp, mcp-go, Spring AI, xmcp…) */
.tag.fw{color:#a5b4fc;border-color:#a5b4fc55;background:#a5b4fc11}
@keyframes glowPulse{50%{box-shadow:0 0 18px rgba(210,168,255,.9)}}
@keyframes glowLite{50%{box-shadow:0 0 18px rgba(249,115,22,.9)}}
.srow .l1 .tag{font-size:8.5px;padding:0 5px;height:15px}
.langchip{display:inline-flex;align-items:center;height:15px;font:8.5px/1 var(--mono);font-weight:700;
  border:1px solid;border-radius:var(--r1);padding:0 4px;flex-shrink:0}
.langchip.unk{color:var(--t4);border-color:var(--border-soft);background:transparent;font-weight:400}
/* Logo Rust (langage inféré) : « R » cerclé d'un engrenage, comme l'icône .rs des éditeurs */
.rustwrap{display:inline-flex;align-items:center;flex-shrink:0;margin:0 1px}
.rustlogo{width:18px;height:18px}
#storeDlg{width:min(1180px,96vw)}
.store-bar{display:flex;gap:10px;align-items:center;margin-bottom:12px;flex-wrap:wrap}
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
.store-bar input{flex:1;min-width:180px}
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
.scard-cmd{font:9.5px/1.4 var(--mono);color:var(--t4);white-space:nowrap;overflow:hidden;text-overflow:ellipsis}
.scard button{align-self:flex-start;height:var(--h-sm);padding:0 9px;font-size:11px}

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
   Groupes en cartes, lignes libellé/description à gauche et contrôle à
   droite, segmented control pour le thème, switch pour les booleans. */
.settings-dlg{width:min(760px,95vw)}
.dlg-title{display:flex;align-items:center;gap:11px;flex:1;min-width:0}
.dlg-title .mark{width:19px;height:19px;color:var(--t3);flex-shrink:0}
.dlg-title h2{margin:0;font-size:14.5px;font-weight:600;letter-spacing:.1px}
.dlg-title p{margin:2px 0 0;font:11.5px/1.3 var(--sans);color:var(--t4)}
.set-group{margin:0 0 18px}
.set-group:last-of-type{margin-bottom:6px}
.set-group-title{font:10px var(--sans);font-weight:600;text-transform:uppercase;letter-spacing:.1em;
  color:var(--t4);margin:0 0 7px 2px}
.set-list{background:var(--card);border:1px solid var(--border-soft);border-radius:var(--r4);overflow:hidden}
.set-row{display:flex;align-items:center;gap:18px;padding:12px 15px;min-height:52px}
.set-row+.set-row{border-top:1px solid var(--border-soft)}
.set-label{flex:1;min-width:0;display:flex;flex-direction:column;gap:3px}
.set-label span{font:12.5px/1.3 var(--sans);color:var(--t1)}
.set-label em{font:11.5px/1.45 var(--sans);color:var(--t4);font-style:normal;max-width:46ch}
.set-ctrl{display:flex;align-items:center;gap:8px;flex-shrink:0;justify-content:flex-end}
.set-ctrl-wide{justify-content:flex-start;width:100%}
.set-num{width:86px !important;height:30px !important;text-align:right}
.set-txt{width:150px !important;height:30px !important}
.set-unit,.set-dash{font:11.5px var(--mono);color:var(--t4);flex-shrink:0}
.set-static{font:12px var(--mono);color:var(--t2);min-width:64px;text-align:right}
.set-note{font:11.5px var(--sans);color:var(--t2)}
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
.set-group-danger .set-group-title{color:var(--bad)}
.dlg-foot{display:flex;align-items:center;gap:12px;padding:13px 20px;border-top:1px solid var(--border-soft);
  background:var(--sidebar)}
.dlg-foot .row{margin-left:auto}
.dlg-foot-note{font:11.5px var(--sans);color:var(--t4)}
.settings-dlg .dlg-body{max-height:min(62vh,560px);overflow-y:auto}

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
/* Case à cocher maison : le carré natif est trop gros et mal aligné
   avec le texte — on le remplace par un contrôle de 16 px, centré. */
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
    <span class="wordmark">Console<span class="wm-dim"> MCP</span></span>
  </span>
  <span class="tb-badge" title="Version installée">v__MCC_VERSION__</span>
  <button id="stAuto" onclick="toggleAutostart()" title="Lancer l'application au démarrage de l'OS (tray silencieux)"></button>
  <span class="tb-badge upd" id="stUpdate" hidden></span>
  <span class="tb-badge ok">UP <b id="stUp">0</b>/<b id="stTotal">0</b></span>
  <span class="tb-badge">outils <b id="stTools">0</b></span>
  <span class="tb-badge">refresh <b id="stRefresh">15</b>s</span>
  <button id="tbLogout" class="tb-logout" hidden onclick="logout()"
          title="Oublier le code d'accès enregistré dans ce navigateur">&#128275;</button>
  <span class="tb-spacer" data-tauri-drag-region></span>
  <div class="tb-actions">
    <button class="primary" onclick="doScan()" title="Balayer les ports locaux pour d&eacute;couvrir les serveurs MCP">&#128269; Scanner les ports</button>
    <button onclick="doScanNet()" title="Balayer le r&eacute;seau local pour d&eacute;couvrir les serveurs MCP distants">&#127760; Scanner le r&eacute;seau</button>
    <button onclick="openMachineDlg()">🖥️ Machines</button>
    <button onclick="openStore()">&#128722; Boutique</button>
    <button onclick="openSettingsDlg()" title="R&eacute;glages du client">&#9881; R&eacute;glages</button>
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
        <option value="framework">framework</option>
        <option value="transport">transport</option>
      </select>
    </div>
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
          <label>Commande de lancement (option — permet Start/Stop par le command center)</label>
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
    <h2>&#128722; Boutique &mdash; serveurs MCP <span id="storeCount" class="store-count"></span></h2>
    <button class="close" onclick="storeDlg.close()">&times;</button>
  </div>
  <div class="dlg-body dlg-body-tight">
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
    <div class="stack"><button onclick="doPeers();setTimeout(renderMachines,3000)">🔄 D&eacute;tecter les command centers sur le LAN</button></div>
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
        <p>Pr&eacute;f&eacute;rences du client Console MCP</p>
      </div>
    </div>
    <button class="close" onclick="settingsDlg.close()" title="Fermer">&times;</button>
  </div>
  <div class="dlg-body">
    <section class="set-group">
      <h3 class="set-group-title">G&eacute;n&eacute;ral</h3>
      <div class="set-list">
        <div class="set-row">
          <div class="set-label"><span>Intervalle de sondage</span>
            <em>D&eacute;lai entre deux cycles de sonde. Appliqu&eacute; au cycle suivant.</em></div>
          <div class="set-ctrl"><input id="sRefresh" type="number" min="5" max="3600" step="5" class="set-num">
            <span class="set-unit">secondes</span></div>
        </div>
        <div class="set-row">
          <div class="set-label"><span>Th&egrave;me</span>
            <em>Apparence de l'interface. Conserv&eacute; sur ce poste.</em></div>
          <div class="set-ctrl seg" role="group" aria-label="Th&egrave;me">
            <button id="tDark" class="seg-btn" onclick="applyTheme('dark')" title="Th&egrave;me sombre (d&eacute;faut)">Sombre</button>
            <button id="tLight" class="seg-btn" onclick="applyTheme('light')" title="Th&egrave;me clair">Clair</button>
          </div>
        </div>
      </div>
    </section>

    <section class="set-group">
      <h3 class="set-group-title">D&eacute;couverte</h3>
      <div class="set-list">
        <div class="set-row">
          <div class="set-label"><span>Plage de scan de ports</span>
            <em>Ports balay&eacute;s par &laquo; Scan ports &raquo;. La d&eacute;couverte native litemcp n'en d&eacute;pend pas.</em></div>
          <div class="set-ctrl"><input id="sScanFrom" type="number" min="1" max="65535" class="set-num">
            <span class="set-dash">&ndash;</span>
            <input id="sScanTo" type="number" min="1" max="65535" class="set-num"></div>
        </div>
        <div class="set-row">
          <div class="set-label"><span>Pairs &amp; r&eacute;seau</span>
            <em>D&eacute;tecte les autres Command Center du LAN et leurs serveurs distants.</em></div>
          <div class="set-ctrl">
            <label class="switch"><input type="checkbox" id="sNetEnabled"><span class="switch-track"></span></label>
          </div>
        </div>
        <div class="set-row">
          <div class="set-label"><span>Ports UI des pairs</span>
            <em>Ports interrog&eacute;s chez les pairs pour les rep&eacute;rer.</em></div>
          <div class="set-ctrl"><input id="sNetPorts" placeholder="5090, 5091" class="set-txt"></div>
        </div>
      </div>
    </section>

    <section class="set-group">
      <h3 class="set-group-title">Interface</h3>
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

    <section class="set-group">
      <h3 class="set-group-title">Mises &agrave; jour</h3>
      <div class="set-list">
        <div class="set-row">
          <div class="set-label"><span>Version install&eacute;e</span>
            <em>Release GitHub du repo DeamonDev888/mcp-command-center.</em></div>
          <div class="set-ctrl"><span class="set-static" id="sVersion">v&ndash;</span></div>
        </div>
        <div class="set-row">
          <div class="set-label"><span>Derni&egrave;re release</span>
            <em id="updMsg">&nbsp;</em></div>
          <div class="set-ctrl">
            <span class="set-static" id="sLatest">&ndash;</span>
            <button onclick="checkUpdateNow()" title="Interroge l'API GitHub releases/latest maintenant">&#8635; V&eacute;rifier</button>
            <button class="primary" onclick="openUpdate()" title="Ouvre la page de la derni&egrave;re release">&#11014; Mettre &agrave; jour</button>
          </div>
        </div>
      </div>
    </section>

    <section class="set-group set-group-danger">
      <h3 class="set-group-title">R&eacute;initialiser</h3>
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
      </div>
    </section>

    <div id="settingsMsg" class="machine-msg"></div>
  </div>
  <div class="dlg-foot">
    <span class="dlg-foot-note" id="settingsFootNote"></span>
    <div class="row">
      <button onclick="settingsDlg.close()">Annuler</button>
      <button class="primary" onclick="saveSettings()">Enregistrer</button>
    </div>
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
function applyTheme(t) {
  document.documentElement.dataset.theme = t === 'light' ? 'light' : 'dark';
  try { localStorage.setItem('mcc.theme', document.documentElement.dataset.theme); } catch (e) {}
  const light = document.documentElement.dataset.theme === 'light';
  const dk = document.getElementById('tDark');
  const lt = document.getElementById('tLight');
  if (dk) dk.classList.toggle('sel', !light);
  if (lt) lt.classList.toggle('sel', light);
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
    if (off) nb.innerHTML = '<b>&#9888; R&eacute;seau local injoignable</b>'
      + '<span>les serveurs distants ne peuvent pas &ecirc;tre sond&eacute;s &mdash; VPN actif ? '
      + '(Proton VPN : activer &laquo;&nbsp;autoriser les connexions LAN&nbsp;&raquo;)</span>';
  }
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
    upd.hidden = false;
    upd.textContent = '⬆ v' + state.latest_version + ' dispo';
    upd.title = 'Nouvelle version ' + state.latest_version + ' publiée sur GitHub (installée : v' + MCC_VERSION + ') — github.com/DeamonDev888/mcp-command-center/releases';
    upd.onclick = () => window.open('https://github.com/DeamonDev888/mcp-command-center/releases/latest', '_blank', 'noopener,noreferrer');
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
  return `<div class="srow ${selectedId===s.id?'sel':''} ${s.enabled?'':'off'}" draggable="true" data-id="${esc(s.id)}"
    onclick="select(${js(s.id)})"
    ondragstart="rowDragStart(event,${js(s.id)})" ondragover="rowDragOver(event)"
    ondragleave="rowDragLeave(event)" ondrop="rowDrop(event)" ondragend="rowDragEnd(event)">
    <div class="l1"><span class="dot ${s.state}"></span><span class="nm" title="${esc(s.label)}">${esc(s.label)}</span>${langChip(s)}${typeBadge(s)}${frameworkBadge(s)}${s.transport==='stdio'?'<span class="legacy" title="Legacy : serveur stdio uniquement (pas de transport HTTP)">d&eacute;suet</span>':''}<span class="cnt">${s.tools_count}</span>${rowAuto(s)}${rowToggle(s)}${rowCtl(s)}</div>
    <div class="l2"><span class="st ${esc(s.state)}">${esc(s.state)}</span> ${ti}${off}</div>
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
    ? `<button class="gact" title="Oublier cette machine (les serveurs d&eacute;j&agrave; import&eacute;s restent)" onclick="event.stopPropagation();sshForget(${js(label)})">&#128465;</button>` : '';
  return `<div class="grphead ${groupClosed(key)?'closed':''}" onclick="toggleGroup(${js(key)})"
    ondragover="groupDragOver(event)" ondragleave="groupDragLeave(event)" ondrop="groupDrop(event,${js(key)})"
    title="${tip}"><span class="gchev">&#9662;</span><span class="gname">${esc(label)}</span>${act}<span class="gcount">${up}/${total}</span></div>`;
}
function renderServers() {
  const localBox = document.getElementById('srvList');
  const groupsBox = document.getElementById('remoteGroups');
  const label = document.getElementById('remoteLabel');
  const count = document.getElementById('localCount');
  const localHead = document.getElementById('localHead');
  if (!state.servers.length) {
    localHead.hidden = true; count.textContent = '';
    localBox.innerHTML = '<div class="empty-sec">aucun serveur — lance un scan</div>';
    label.hidden = true; groupsBox.innerHTML = '';
    return;
  }
  const local = state.servers.filter(s => hostOf(s) === 'local');
  const remotes = state.servers.filter(s => hostOf(s) !== 'local');
  const byKey = {};
  remotes.forEach(s => { const k = hostOf(s); (byKey[k] = byKey[k] || []).push(s); });
  const upOf = rows => rows.filter(s => s.state === 'UP').length;
  localHead.hidden = false;
  count.textContent = upOf(local) + '/' + local.length;
  localBox.innerHTML = local.map(srvRow).join('');
  const keys = groupKeys();
  label.hidden = !keys.length;
  groupsBox.innerHTML = keys.map(k => {
    const rows = byKey[k] || [];
    const body = rows.length ? rows.map(srvRow).join('')
      : '<div class="empty-sec empty-indent">aucun serveur import&eacute; de cette machine</div>';
    return groupHeadHtml(k, k, upOf(rows), rows.length, true)
      + `<div class="grpbody mbody ${groupClosed(k)?'closed':''}">${body}</div>`;
  }).join('');
}

// Chip du langage MCP (inféré : commande stdio ou en-têtes HTTP) — monogramme coloré façon logo
const LANG_META = {
  node:   { t: 'JS',  c: '#f7df1e', n: 'Node.js / TypeScript' },
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
function langChip(s) {
  if (s.lang === 'rust') return `<span class="rustwrap" title="Langage inféré : Rust">${RUST_LOGO}</span>`;
  const m = LANG_META[s.lang];
  if (!m) return '<span class="langchip unk" title="Langage non d\u00e9tect\u00e9 (ni commande ni en-t\u00eates ni process)">?</span>';
  return `<span class="langchip" style="color:${m.c};border-color:${m.c}66;background:${m.c}14" title="Langage inféré : ${m.n}">${m.t}</span>`;
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

function rowToggle(s) {
  return `<button class="sw ${s.enabled?'on':'off'}" title="${s.enabled?'D\u00e9sactiver ce serveur (arr\u00eate son process)':'Activer ce serveur (le d\u00e9marre si commande connue)'}" onclick="event.stopPropagation();act('toggle',${js(s.id)})">${s.enabled?'ON':'OFF'}</button>`;
}

function rowCtl(s) {
  if (!s.can_start) return '';
  const up = s.state === 'UP';
  return `<button class="rowbtn" title="${up?'Arr\u00eater':'D\u00e9marrer'} ce serveur" onclick="event.stopPropagation();act('${up?'kill':'start'}',${js(s.id)})">${up?'\u25a0':'\u25b6'}</button>`;
}

// Autoboot PAR SERVEUR : relance automatique de CE serveur au démarrage du
// command center et après un crash. Bouton ⚡ visible dans la ligne et le détail.
function rowAuto(s) {
  if (!s.can_start) return '';
  const on = s.auto_restart;
  return `<button class="rowbtn auto ${on?'on':'off'}" title="${on
    ? 'Autoboot ACTIV\u00c9 : ce serveur est relanc\u00e9 au d\u00e9marrage et apr\u00e8s un crash \u2014 cliquer pour d\u00e9sactiver'
    : 'Autoboot D\u00c9SACTIV\u00c9 : ce serveur ne sera pas relanc\u00e9 automatiquement \u2014 cliquer pour activer'}"
    onclick="event.stopPropagation();setAuto(${js(s.id)},${on?'false':'true'})">\u26a1</button>`;
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
  const ti = s.transport === 'http'
    ? `HTTP ${s.transport_info.host ? s.transport_info.host + ':' : ''}${s.transport_info.port}${s.transport_info.path}`
    : `stdio \u00b7 ${s.transport_info.program} ${(s.transport_info.args || []).join(' ')}`;
  const kv = [
    ['id', s.id], ['transport', ti],
    ['origine', s.origin],
    ['bind', s.bound_host ?? '-'], ['serveur MCP', s.server_name ?? '-'],
    ['version', s.version ?? '-'], ['protocole', s.protocol ?? '-'],
    ['framework', s.framework
      ? ((FRAMEWORKS[s.framework] || {label: s.framework, lang: '?'}).label
         + ' (' + (FRAMEWORKS[s.framework] || {lang: s.lang || '?'}).lang
         + ' · spec ' + (s.protocol || '?') + ')')
      : '-'],
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
    : starting ? 'au d\u00e9marrage\u2026'
    : s.state === 'PORT-ONLY' ? 'port occup\u00e9 (aucune r\u00e9ponse MCP)'
    : s.state === 'DISABLED' ? 'd\u00e9sactiv\u00e9' : 'arr\u00eat\u00e9';
  const cmd = s.start_cmd
    ? s.start_cmd.program + ' ' + (s.start_cmd.args || []).join(' ') + (s.start_cmd.workdir ? '  \u00b7  ' + s.start_cmd.workdir : '')
    : null;
  const kvd = [
    ['contr&ocirc;le', s.managed ? 'process lanc\u00e9 et maintenu par le command center'
      : (cmd ? 'commande pr\u00eate \u2014 process externe non d\u00e9marr\u00e9 par ici' : 'aucune \u2014 process externe')],
    ['&eacute;tat du process', etatProcess],
    ['pid', s.pid ?? '-'],
    ['auto-restart', s.auto_restart ? 'ON \u2014 relance automatique au boot/crash' : 'OFF'],
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
    `<div class="tool ${t.disabled?'off':''}" data-tool="${esc(t.name)}">
       <code>${esc(t.name)}</code>
       <span class="desc" title="${esc(t.desc)}">${esc(t.desc) || '&mdash;'}</span>
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
      ${s.transport === 'stdio' ? '<span class="legacy" title="Legacy : serveur stdio uniquement (pas de transport HTTP)">d&eacute;suet</span>' : ''}
      <span class="sub">${s.server_name ? esc(s.server_name) + ' v' + esc(s.version || '?') : ''}${s.latency_ms != null ? ' &middot; ' + s.latency_ms + ' ms' : ''}</span>
      <div class="d-actions">
        ${s.can_start && s.state !== 'UP' ? `<button class="primary" onclick="act('start',${js(s.id)})">&#9654; Start</button>` : ''}
        ${s.can_start && s.state === 'UP' ? `<button class="danger" onclick="act('kill',${js(s.id)})">&#9632; Stop</button>` : ''}
        <button onclick="act('refresh',${js(s.id)})">Sonder</button>
        ${s.can_start ? `<button class="autobtn ${s.auto_restart?'on':''}" onclick="setAuto(${js(s.id)},${s.auto_restart?'false':'true'})"
          title="Autoboot : relance automatique de ce serveur au d&eacute;marrage du command center et apr&egrave;s un crash">&#9889; Autoboot ${s.auto_restart?'ON':'OFF'}</button>` : ''}
        <button onclick="act('toggle',${js(s.id)})" title="${s.enabled
          ? 'Mettre en pause : le process est arr&ecirc;t&eacute; et le serveur n&#39;est plus sond&eacute; (il reste dans la liste, r&eacute;activable d&#39;un clic)'
          : 'Reprendre la supervision : le serveur sera sond&eacute; et d&eacute;marr&eacute; si une commande est connue'}">${s.enabled ? '&#9208; Mettre en pause' : '&#9654; Reprendre'}</button>
        ${s.origin !== 'seed'
          ? `<button class="danger" onclick="removeSrv(${js(s.id)})">&#128465; Supprimer</button>`
          : `<button class="danger" onclick="removeSeedForever(${js(s.id)})" title="Supprime ce seed de la config : il ne sera pas r&eacute;inject&eacute; au red&eacute;marrage">&#128465; Supprimer d&eacute;finitivement</button>`}
      </div>
    </div>
    ${s.origin === 'seed' ? '<div class="seed-note">Serveur seed : <b>Retirer durablement</b> l&rsquo;efface m&ecirc;me apr&egrave;s red&eacute;marrage (consign&eacute; dans config.json) ; <b>D&eacute;sactiver</b> l&rsquo;&eacute;teint sans le supprimer.</div>' : ''}
    ${s.last_error ? `<div class="d-err">&#9888; ${esc(s.last_error)}</div>` : ''}
    ${fold('demarrage', 'D&eacute;marrage', `<div class="kv">${kvd}</div>${startCmdForm(s)}`)}
    ${fold('infos', 'Informations', `<div class="kv">${kv}</div>`)}
    <div class="tabs">
      <button class="tab ${detailTab === 'outils' ? 'sel' : ''}" onclick="detailTab='outils';renderDetail()">Outils (${nTot})</button>
      <button class="tab ${detailTab === 'conf' ? 'sel' : ''}" onclick="detailTab='conf';renderDetail()">Conf agent</button>
    </div>
    ${detailTab === 'conf' ? confAgent(s) : toolsHtml}`;
}

function select(id) { selectedId = id; detailCache = null; detailTab = 'outils'; render(); load(); }

/* Formulaire « commande de lancement » : pour un serveur local découvert par
   scan (port connu, commande inconnue) — poser la commande fait apparaître les
   boutons ▶/■ et l'autoboot, comme un serveur ajouté à la main. */
function startCmdForm(s) {
  if (s.can_start || s.transport === 'stdio') return '';
  const host = s.transport_info && s.transport_info.host;
  if (host) return '<div class="conf-note note-gap">Serveur distant : la commande de lancement appartient &agrave; sa machine (' + esc(host) + '), pas au command center.</div>';
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
function removeSrv(id) { selectedId = null; detailCache = null; act('remove', id); }
function removeSeedForever(id) {
  if (!confirm('Retirer ce seed DURABLEMENT ? Il ne sera pas réinjecté au redémarrage du command center.')) return;
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
function toast(msg) {
  const t = document.getElementById('toast');
  if (!t) return;
  t.textContent = msg;
  t.classList.add('on');
  clearTimeout(toast._t);
  toast._t = setTimeout(() => t.classList.remove('on'), 1200);
}
function copyText(txt) {
  const done = () => toast('copié');
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
    toast(j.ok ? 'explorateur ouvert' : ('échec : ' + (j.error || 'inconnu')));
  } catch (e) { toast('échec : ' + e); }
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
  add('📋 Copier toutes les infos', () => copyText(srvInfoText(s)));
  add('🏷 Copier le nom', () => copyText(s.label));
  if (ctxUrl(s)) add('🔗 Copier l URL', () => copyText(ctxUrl(s)));
  if (srvCmd(s)) add('⌨ Copier la commande', () => copyText(srvCmd(s)));
  add('🗂 Copier le chemin', () => copyText(p.abs), !p.abs);
  add('🗂 Copier le chemin relatif', () => copyText(p.rel), !p.rel);
  add('📂 Ouvrir dans l explorateur', () => openInExplorer(p.abs || p.workdir), !(p.abs || p.workdir));
  if (s.can_start) {
    add('—', null, false);
    add(s.state === 'UP' ? '■ Arrêter ce serveur' : '▶ Démarrer ce serveur',
        () => act(s.state === 'UP' ? 'kill' : 'start', s.id));
  }
  add('🔄 Sonder maintenant', () => act('refresh', s.id));
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
  add('📋 Copier le nom de l outil', () => copyText(name));
  add('📋 Copier « serveur.outil »', () => copyText((d.label || s.label || id) + '.' + name));
  if (t.desc) add('📃 Copier la description', () => copyText(t.desc));
  add('🧾 Copier l appel JSON-RPC', () => copyText(JSON.stringify({
    jsonrpc: '2.0', id: 1, method: 'tools/call',
    params: {name: name, arguments: {}}
  }, null, 2)));
  if (ctxUrl(s)) add('🔗 Copier l URL + outil', () => copyText(ctxUrl(s) + '   # outil : ' + name));
  add('—', null, false);
  add(t.disabled ? '✅ Réactiver cet outil' : '⛔ Désactiver cet outil',
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
document.addEventListener('keydown', (e) => { if (e.key === 'Escape') ctxClose(); });
window.addEventListener('resize', ctxClose);

async function act(action, id) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action, id})});
  await load();
}

// Autoboot par serveur (⚡) : relance auto au démarrage du command center
// et après un crash. Persisté dans config.json par le moteur.
async function setAuto(id, enabled) {
  await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'set_auto_restart', id, enabled})});
  await load();
  if (selectedId === id) { detailCache = null; await load(); }
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

/* ===== Réglages du client (dialog ⚙) ===== */
function openSettingsDlg() {
  const s = state.settings || {};
  document.getElementById('sRefresh').value = s.refresh_secs || 15;
  document.getElementById('sScanFrom').value = s.scan_from || 3000;
  document.getElementById('sScanTo').value = s.scan_to || 3200;
  document.getElementById('sNetEnabled').checked = !!s.network_enabled;
  document.getElementById('sNetPorts').value = (s.network_ports || []).join(', ');
  document.getElementById('sUiBind').value = s.ui_bind || '127.0.0.1';
  document.getElementById('sUiPort').value = s.ui_port || 5090;
  // sVersion/sLatest sont des <span> (affichage), pas des champs éditables
  document.getElementById('sVersion').textContent = 'v' + MCC_VERSION;
  document.getElementById('sLatest').textContent = state.latest_version ? 'v' + state.latest_version : '—';
  const upd = document.getElementById('updMsg');
  upd.textContent = state.update_available ? '⬆ v' + state.latest_version + ' disponible' : 'Aucune mise à jour détectée.';
  upd.style.color = state.update_available ? 'var(--warn)' : '';
  document.getElementById('resetMsg').textContent = '';
  const row = document.getElementById('resetMsgRow');
  if (row) row.hidden = true;
  document.getElementById('settingsMsg').textContent = '';
  const foot = document.getElementById('settingsFootNote');
  if (foot) foot.textContent = 'Les réglages sont enregistrés dans config.json.';
  applyTheme(document.documentElement.dataset.theme === 'light' ? 'light' : 'dark');
  document.getElementById('settingsDlg').showModal();
}
async function checkUpdateNow() {
  const msg = document.getElementById('updMsg');
  msg.style.color = '';
  msg.textContent = '⏳ vérification de la release GitHub…';
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify({action: 'check_update'})});
  const j = await r.json();
  if (j.ok && j.update_available) {
    document.getElementById('sLatest').textContent = 'v' + j.latest;
    msg.textContent = '⬆ v' + j.latest + ' disponible (installée : v' + j.current + ')';
    msg.style.color = 'var(--warn)';
  } else if (j.ok) {
    document.getElementById('sLatest').textContent = j.latest ? 'v' + j.latest : '—';
    msg.textContent = '✓ à jour — dernière release : v' + (j.latest || j.current);
    msg.style.color = 'var(--ok)';
  } else {
    msg.textContent = '✗ vérification impossible (réseau ?) — le check auto tourne au boot puis chaque heure';
  }
}
function openUpdate() {
  window.open('https://github.com/DeamonDev888/mcp-command-center/releases/latest', '_blank', 'noopener,noreferrer');
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
async function saveSettings() {
  const ports = document.getElementById('sNetPorts').value
    .split(/[\s,]+/).filter(Boolean).map(Number)
    .filter(n => Number.isInteger(n) && n >= 1 && n <= 65535);
  const body = {
    action: 'set_settings',
    refresh_secs: +document.getElementById('sRefresh').value || 15,
    scan_from: +document.getElementById('sScanFrom').value || 3000,
    scan_to: +document.getElementById('sScanTo').value || 3200,
    ui_bind: document.getElementById('sUiBind').value,
    network_enabled: document.getElementById('sNetEnabled').checked,
    network_ports: ports,
  };
  const msg = document.getElementById('settingsMsg');
  const r = await fetch('/api/action', {method: 'POST', headers: {'Content-Type': 'application/json'},
    body: JSON.stringify(body)});
  const j = await r.json();
  if (!j.ok) msg.textContent = '✗ ' + (j.error || 'échec');
  else {
    const n = (j.changed || []).length;
    msg.textContent = n ? '✓ Enregistré (' + j.changed.join(', ') + ')' : '✓ Aucun changement';
    await load();
  }
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
    const st = peer ? `command center d&eacute;tect&eacute; — ${esc((peer.stats||{}).up ?? '?')} UP / ${esc((peer.stats||{}).total ?? '?')} serveurs`
                    : 'machine enregistr&eacute;e';
    return `<div class="machine-row">
      <div class="l1"><span class="dot UP"></span><span class="nm">${esc(h.ip)}</span><span class="tag http">${esc(h.user)}</span>${peer ? '<span class="tag sse">pair</span>' : ''}</div>
      <div class="l2">${st}</div>
      <div class="macts">
        <button class="primary" onclick="sshDiscover(${js(h.ip)},${js(h.user)})">🔗 SSH &amp; d&eacute;couvrir</button>
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
loadFrameworks();
updateLogoutBtn();
load();
setInterval(load, 5000);
// retour de la fenêtre (tray → visible) : reprise immédiate du rafraîchissement
document.addEventListener('visibilitychange', () => { if (!document.hidden) load(); });
</script>
<div id="ctxmenu" class="ctxmenu" hidden></div>
<div id="toast"></div>
</body>
</html>
"#;
