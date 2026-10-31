// Boutique MCP — catalogue statique + parsing des noms (porté de l'app Hermes desktop : mcp-doc.ts, mcp-import.ts).
// (split v0.7.20 : manage.rs reste la racine — API stable via réexport)
use super::*;

// ---------- boutique MCP (catalogue + import, porté de l'app Hermes desktop :
// mcp-doc.ts pour le parsing des configs, mcp-import.ts pour « collez n'importe quoi ») ----------

pub struct StoreItem {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub cat: &'static str,
    pub kind: &'static str, // npm | python
    pub program: &'static str,
    pub args: &'static [&'static str],
}

pub const STORE: &[StoreItem] = &[
    StoreItem {
        id: "filesystem",
        name: "Filesystem",
        desc: "Lecture/écriture de fichiers dans un dossier autorisé (dossier demandé à l'installation)",
        cat: "Fichiers",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-filesystem"],
    },
    StoreItem {
        id: "gdrive",
        name: "Google Drive",
        desc: "Recherche et lecture des fichiers Google Drive",
        cat: "Fichiers",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-gdrive"],
    },
    StoreItem {
        id: "git",
        name: "Git",
        desc: "Lecture et manipulation de dépôts git (status, diff, log, commit)",
        cat: "Dev",
        kind: "python",
        program: "uvx",
        args: &["mcp-server-git"],
    },
    StoreItem {
        id: "github",
        name: "GitHub",
        desc: "API GitHub : repos, issues, PR (GITHUB_TOKEN requis en env)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-github"],
    },
    StoreItem {
        id: "memory",
        name: "Memory",
        desc: "Graph de connaissances persistant (mémoire longue)",
        cat: "Utilitaires",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-memory"],
    },
    StoreItem {
        id: "seqthink",
        name: "Sequential Thinking",
        desc: "Raisonnement structuré étape par étape",
        cat: "Utilitaires",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-sequential-thinking"],
    },
    StoreItem {
        id: "everything",
        name: "Everything",
        desc: "Serveur de démo/test qui expose tous les comportements MCP",
        cat: "Utilitaires",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-everything"],
    },
    StoreItem {
        id: "time",
        name: "Time",
        desc: "Heure et conversions de fuseaux horaires",
        cat: "Utilitaires",
        kind: "python",
        program: "uvx",
        args: &["mcp-server-time"],
    },
    StoreItem {
        id: "fetch",
        name: "Fetch",
        desc: "Récupération de pages web et conversion en markdown",
        cat: "Web",
        kind: "python",
        program: "uvx",
        args: &["mcp-server-fetch"],
    },
    StoreItem {
        id: "puppeteer",
        name: "Puppeteer",
        desc: "Automatisation de navigateur : screenshots, clics, scraping",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-puppeteer"],
    },
    StoreItem {
        id: "brave",
        name: "Brave Search",
        desc: "Recherche web Brave (BRAVE_API_KEY requis en env)",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-brave-search"],
    },
    StoreItem {
        id: "postgres",
        name: "PostgreSQL",
        desc: "Accès lecture seule à une base PostgreSQL (URL en args)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-postgres"],
    },
    StoreItem {
        id: "sqlite",
        name: "SQLite",
        desc: "Requêtes et analyse d'une base SQLite locale",
        cat: "Données",
        kind: "python",
        program: "uvx",
        args: &["mcp-server-sqlite", "--db-path", "test.db"],
    },
    StoreItem {
        id: "slack",
        name: "Slack",
        desc: "Lecture/écriture de canaux Slack (tokens requis en env)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@modelcontextprotocol/server-slack"],
    },
    StoreItem {
        id: "playwright",
        name: "Playwright",
        desc: "Automatisation de navigateur moderne (Microsoft) : navigation, clics, screenshots, snapshots d'accessibilité",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "@playwright/mcp@latest"],
    },
    StoreItem {
        id: "context7",
        name: "Context7",
        desc: "Documentation à jour des libs et frameworks injectée dans le contexte de l'agent (Upstash)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "@upstash/context7-mcp"],
    },
    StoreItem {
        id: "duckduckgo",
        name: "DuckDuckGo Search",
        desc: "Recherche web DuckDuckGo sans clé API + extraction de contenu de pages",
        cat: "Web",
        kind: "python",
        program: "uvx",
        args: &["duckduckgo-mcp-server"],
    },
    StoreItem {
        id: "kubernetes",
        name: "Kubernetes",
        desc: "Gestion d'un cluster k8s via kubectl (kubeconfig local requis)",
        cat: "Dev",
        kind: "npm",
        program: "npx",
        args: &["-y", "mcp-server-kubernetes"],
    },
    StoreItem {
        id: "supabase",
        name: "Supabase",
        desc: "Bases, tables et edge functions Supabase (OAuth navigateur au premier probe, ou SUPABASE_ACCESS_TOKEN en env)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@supabase/mcp-server-supabase@latest"],
    },
    StoreItem {
        id: "mongodb",
        name: "MongoDB",
        desc: "Requêtes et analyse MongoDB (MDB_MCP_CONNECTION_STRING requis en env)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "mongodb-mcp-server"],
    },
    StoreItem {
        id: "stripe",
        name: "Stripe",
        desc: "Paiements et clients Stripe (STRIPE_SECRET_KEY en env, ou --api-key en args)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@stripe/mcp", "--tools=all"],
    },
    StoreItem {
        id: "notion",
        name: "Notion",
        desc: "API Notion officielle (env OPENAPI_MCP_HEADERS = JSON avec Authorization Bearer + Notion-Version)",
        cat: "Données",
        kind: "npm",
        program: "npx",
        args: &["-y", "@notionhq/mcp-server"],
    },
    StoreItem {
        id: "firecrawl",
        name: "Firecrawl",
        desc: "Scraping et extraction web (FIRECRAWL_API_KEY en env ; mode sans clé disponible côté Firecrawl)",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "firecrawl-mcp"],
    },
    StoreItem {
        id: "exa",
        name: "Exa Search",
        desc: "Recherche web sémantique (EXA_API_KEY requis en env)",
        cat: "Web",
        kind: "npm",
        program: "npx",
        args: &["-y", "exa-mcp-server"],
    },
    StoreItem {
        id: "blender",
        name: "Blender",
        desc: "Pilote une session Blender vivante — addon BlenderMCP requis dans Blender (repo ahujasid/blender-mcp, pin du catalogue Hermes)",
        cat: "Création",
        kind: "python",
        program: "uvx",
        args: &["blender-mcp==1.6.4"],
    },
    StoreItem {
        id: "figma",
        name: "Figma (Framelink)",
        desc: "Contexte design Figma pour générer du code (FIGMA_API_KEY requis en env) — l'officiel distant est OAuth/HTTPS, pas encore sondable ici",
        cat: "Design",
        kind: "npm",
        program: "npx",
        args: &["-y", "figma-developer-mcp", "--stdio"],
    },
    StoreItem {
        id: "n8n",
        name: "n8n",
        desc: "Docs des nodes n8n hors de la boîte ; gestion d'instance via N8N_API_URL + N8N_API_KEY en env",
        cat: "Automatisation",
        kind: "npm",
        program: "npx",
        args: &["-y", "n8n-mcp"],
    },
    StoreItem {
        id: "todoist",
        name: "Todoist",
        desc: "Tâches Todoist officiel (Doist) : projets, tâches, échéances (TODOIST_API_KEY requis en env)",
        cat: "Productivité",
        kind: "npm",
        program: "npx",
        args: &["-y", "@doist/todoist-mcp"],
    },
    StoreItem {
        id: "taskmaster",
        name: "Task Master",
        desc: "Décompose une spec en tâches, gère les dépendances et délègue aux agents (task-master-ai ; ANTHROPIC_API_KEY requis en env)",
        cat: "Productivité",
        kind: "npm",
        program: "npx",
        args: &["-y", "--package=task-master-ai", "task-master-ai"],
    },
];

/// Item de boutique par id — principal, puis aitmpl, puis glama.
pub(super) fn find_store_item(sid: &str) -> Option<&'static StoreItem> {
    STORE
        .iter()
        .find(|s| s.id == sid)
        .or_else(|| {
            crate::store_aitmpl::STORE_AITMPL
                .iter()
                .find(|s| s.id == sid)
        })
        .or_else(|| crate::store_glama::STORE_GLAMA.iter().find(|s| s.id == sid))
        .or_else(|| {
            crate::store_github::STORE_GITHUB
                .iter()
                .find(|s| s.id == sid)
        })
}

/// Tous les items (principal + aitmpl + glama).
pub(super) fn all_store_items() -> impl Iterator<Item = &'static StoreItem> {
    STORE
        .iter()
        .chain(crate::store_aitmpl::STORE_AITMPL.iter())
        .chain(crate::store_glama::STORE_GLAMA.iter())
        .chain(crate::store_github::STORE_GITHUB.iter())
}

pub fn store_json() -> Value {
    json!(all_store_items()
        .map(|s| json!({
            "id": s.id, "name": s.name, "desc": s.desc,
            "cat": s.cat, "kind": s.kind,
            "program": s.program, "args": s.args,
        }))
        .collect::<Vec<_>>())
}

/// Porté de mcp-import.ts : "@modelcontextprotocol/server-filesystem" → "filesystem".
pub(super) fn cleanup_name(base: &str) -> String {
    let mut b = base.to_string();
    if let Some(slash) = b.rfind('/') {
        b = b[slash + 1..].to_string();
    }
    b = regex_like_strip(&b, &["cjs", "js", "mjs", "py", "ts"]);
    for p in ["mcp-server-", "server-", "mcp-"] {
        if b.starts_with(p) {
            b = b[p.len()..].to_string();
        }
    }
    for s in ["-mcp-server", "-mcp", "-server"] {
        if b.ends_with(s) {
            b = b[..b.len() - s.len()].to_string();
        }
    }
    if b.is_empty() {
        base.to_string()
    } else {
        b
    }
}

pub(super) fn regex_like_strip(name: &str, exts: &[&str]) -> String {
    for e in exts {
        if let Some(stripped) = name.strip_suffix(&format!(".{}", e)) {
            return stripped.to_string();
        }
    }
    name.to_string()
}
