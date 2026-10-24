//! Catalogue GitHub — les serveurs MCP installables (npx/uvx) les plus
//! populaires du dépôt GitHub (topic `mcp-server`, tri étoiles), absents des
//! autres catalogues. Paquets vérifiés sur npm / PyPI ; ids préfixés « gh- ».

use crate::manage::StoreItem;

pub const STORE_GITHUB: &[StoreItem] = &[
    StoreItem {
        id: "gh-chromedevtools",
        name: "Chrome DevTools",
        desc: "Chrome DevTools pour agents de code : DOM, réseau, performance, traces, JS évalué dans un vrai navigateur (54k★ sur GitHub)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "chrome-devtools-mcp@latest"],
    },
    StoreItem {
        id: "gh-codebase-memory",
        name: "Codebase Memory",
        desc: "Intelligence de code locale et rapide pour agents de code : indexation du dépôt, recherche, mémoire de codebase (47k★ sur GitHub)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "codebase-memory-mcp"],
    },
    StoreItem {
        id: "gh-context-mode",
        name: "Context Mode",
        desc: "Optimisation de la fenêtre de contexte : met les sorties d'outils en sandbox et n'injecte que l'essentiel pour l'agent (32k★ sur GitHub)",
        cat: "Utilitaires",
        kind: "npm",
        program: "npx",
        args: &["-y", "context-mode"],
    },
    StoreItem {
        id: "gh-serena",
        name: "Serena",
        desc: "Boîte à outils de codage : recherche sémantique, édition au niveau symbole et mémoire de dépôt pour tes agents (30k★ sur GitHub)",
        cat: "Dev",
        kind: "python",
        program: "uvx",
        args: &[
            "--from",
            "serena-agent",
            "serena",
            "start-mcp-server",
            "--context",
            "ide-assistant",
        ],
    },
];
