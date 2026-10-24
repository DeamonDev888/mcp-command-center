//! Catalogue Glama — les serveurs MCP les plus populaires du classement
//! glama.ai (tri GitHub stars) absents du catalogue principal. Paquets
//! vérifiés sur npm / PyPI ; ids préfixés « gl- ».

use crate::manage::StoreItem;

pub const STORE_GLAMA: &[StoreItem] = &[
    StoreItem {
        id: "gl-gitlab",
        name: "GitLab",
        desc: "API GitLab : gestion de projets, opérations fichiers, merge requests, pipelines (env : GITLAB_PERSONAL_ACCESS_TOKEN, GITLAB_API_URL optionnel)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-gitlab"],
    },
    StoreItem {
        id: "gl-sentry",
        name: "Sentry",
        desc: "Inspection des rapports d'erreurs, stacktraces et informations de debug de ton compte Sentry (env : SENTRY_ACCESS_TOKEN)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "@sentry/mcp-server"],
    },
    StoreItem {
        id: "gl-googlemaps",
        name: "Google Maps",
        desc: "API Google Maps : géocodage, itinéraires, distances, lieux et élévation (env : GOOGLE_MAPS_API_KEY)",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-google-maps"],
    },
    StoreItem {
        id: "gl-paddleocr",
        name: "PaddleOCR",
        desc: "OCR multi-usage : reconnaissance de texte imprimé et manuscrit, parsing de documents complexes (tableaux, formulaires)",
        cat: "Utilitaires",
        kind: "python",
        program: "uvx",
        args: &["paddleocr-mcp"],
    },
    StoreItem {
        id: "gl-codegraph",
        name: "CodeGraph",
        desc: "Graphe de code sémantique pré-indexé pour agents de code : relations entre symboles, analyse d'impact, contexte instantané",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "@colbymchenry/codegraph"],
    },
    StoreItem {
        id: "gl-worldmonitor",
        name: "World Monitor",
        desc: "Intelligence globale en temps réel : agrégation d'actualités par IA, suivi géopolitique et d'infrastructures",
        cat: "Recherche",
        kind: "npm",
        program: "npx",
        args: &["-y", "worldmonitor"],
    },
];
