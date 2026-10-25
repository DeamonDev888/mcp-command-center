//! Frameworks MCP populaires par langage — détection du framework ET du langage.
//!
//! La spec MCP n'expose PAS le framework ni le langage dans `serverInfo`
//! (name/title/version seulement). Ce module combine donc trois familles de
//! signaux, du plus fiable au plus indicatif :
//!
//! 1. **signature comportementale** (forte) : `litemcp` (version de spec
//!    2026-07-28 non négociée + sonde `/health` « ok ») et `fastmcp`
//!    (réponses initialize encodées SSE + endpoint `/sse` legacy co-servi) ;
//! 2. **nom annoncé** par le serveur (`serverInfo.name`), quand le développeur
//!    y a laissé le nom du framework (`fastmcp`, `rmcp`, `mcp-go`, `xmcp`…) ;
//! 3. **langage** : en-têtes HTTP (`Server`, `X-Powered-By`), process
//!    propriétaire du port, ou commande de lancement pour les serveurs stdio.
//!
//! ~5 frameworks maximum par langage, les plus utilisés de l'écosystème.
//! Aucune invention : un serveur dont le framework n'est pas identifiable
//! n'affiche PAS de badge (le langage seul reste affiché).

/// Un framework MCP identifiable.
#[derive(Debug, PartialEq)]
pub struct Framework {
    /// Identifiant stable (persisté dans config/cache, exposé par l'API).
    pub id: &'static str,
    /// Libellé court affiché dans le badge.
    pub label: &'static str,
    /// Langage du framework (`python`, `node`, `rust`, `go`, `java`, `dotnet`,
    /// `ruby`, `php`).
    pub lang: &'static str,
    /// Sous-chaînes (minuscules) recherchées dans le nom annoncé par le serveur.
    pub keywords: &'static [&'static str],
}

/// Table des frameworks populaires — 5 maximum par langage.
pub const FRAMEWORKS: &[Framework] = &[
    // ---------- Python ----------
    Framework {
        id: "fastmcp-py",
        label: "FastMCP",
        lang: "python",
        keywords: &["fastmcp-python", "fastmcp python"],
    },
    Framework {
        id: "fastapi-mcp",
        label: "FastAPI-MCP",
        lang: "python",
        keywords: &["fastapi-mcp", "fastapi mcp"],
    },
    Framework {
        id: "mcpo",
        label: "mcpo",
        lang: "python",
        keywords: &["mcpo"],
    },
    Framework {
        id: "mcp-proxy",
        label: "mcp-proxy",
        lang: "python",
        keywords: &["mcp-proxy", "mcp_proxy"],
    },
    Framework {
        id: "mcp-py-sdk",
        label: "MCP SDK",
        lang: "python",
        keywords: &["mcp-python-sdk", "python-sdk", "mcp python sdk"],
    },
    // ---------- TypeScript / Node ----------
    Framework {
        id: "fastmcp-ts",
        label: "FastMCP",
        lang: "node",
        keywords: &["fastmcp-typescript", "fastmcp typescript"],
    },
    Framework {
        id: "xmcp",
        label: "xmcp",
        lang: "node",
        keywords: &["xmcp"],
    },
    Framework {
        id: "mcp-framework",
        label: "mcp-framework",
        lang: "node",
        keywords: &["mcp-framework"],
    },
    Framework {
        id: "mcp-ts-sdk",
        label: "MCP SDK",
        lang: "node",
        keywords: &[
            "@modelcontextprotocol/sdk",
            "modelcontextprotocol/sdk",
            "typescript-sdk",
        ],
    },
    Framework {
        id: "fastify-mcp",
        label: "Fastify MCP",
        lang: "node",
        keywords: &["fastify-mcp", "fastify mcp"],
    },
    // ---------- Rust ----------
    Framework {
        id: "litemcp",
        label: "LiteMCP",
        lang: "rust",
        keywords: &["litemcp", "lite-mcp"],
    },
    Framework {
        id: "rmcp",
        label: "rmcp",
        lang: "rust",
        keywords: &["rmcp", "rust-sdk", "modelcontextprotocol/rust"],
    },
    Framework {
        id: "rust-mcp-sdk",
        label: "rust-mcp-sdk",
        lang: "rust",
        keywords: &["rust-mcp-sdk", "rust_mcp_sdk"],
    },
    Framework {
        id: "tower-mcp",
        label: "tower-mcp",
        lang: "rust",
        keywords: &["tower-mcp", "tower_mcp"],
    },
    Framework {
        id: "mcp-rs",
        label: "mcp-rs",
        lang: "rust",
        keywords: &["mcp-rs", "mcp_rs"],
    },
    // ---------- Go ----------
    Framework {
        id: "mcp-go",
        label: "mcp-go",
        lang: "go",
        keywords: &["mcp-go", "mark3labs"],
    },
    Framework {
        id: "go-sdk",
        label: "Go SDK",
        lang: "go",
        keywords: &["modelcontextprotocol/go", "go-sdk", "golang-sdk"],
    },
    Framework {
        id: "mcp-golang",
        label: "mcp-golang",
        lang: "go",
        keywords: &["mcp-golang"],
    },
    Framework {
        id: "gomcp",
        label: "gomcp",
        lang: "go",
        keywords: &["gomcp"],
    },
    Framework {
        id: "go-mcp",
        label: "go-mcp",
        lang: "go",
        keywords: &["go-mcp", "go_mcp"],
    },
    // ---------- Java ----------
    Framework {
        id: "spring-ai",
        label: "Spring AI",
        lang: "java",
        keywords: &["spring-ai", "spring ai", "springaialibaba"],
    },
    Framework {
        id: "mcp-java-sdk",
        label: "MCP Java SDK",
        lang: "java",
        keywords: &["mcp-java-sdk", "java-sdk", "io.modelcontextprotocol"],
    },
    Framework {
        id: "quarkus-mcp",
        label: "Quarkus MCP",
        lang: "java",
        keywords: &["quarkus-mcp", "quarkus mcp", "quarkus"],
    },
    Framework {
        id: "langchain4j",
        label: "LangChain4j",
        lang: "java",
        keywords: &["langchain4j", "langchain4j-mcp"],
    },
    Framework {
        id: "mcp-java",
        label: "mcp-java",
        lang: "java",
        keywords: &["mcp-java", "mcp4j"],
    },
    // ---------- C# / .NET ----------
    Framework {
        id: "mcp-csharp-sdk",
        label: "MCP C# SDK",
        lang: "dotnet",
        // PAS le mot « modelcontextprotocol » seul : tous les serveurs npm
        // officiels s'appellent « @modelcontextprotocol/server-* » et seraient
        // pris pour du C#. On exige un marqueur propre au SDK .NET.
        keywords: &[
            "modelcontextprotocol/csharp",
            "csharp-sdk",
            "mcp-csharp",
            "dotnet-sdk",
        ],
    },
    Framework {
        id: "mcp-dotnet",
        label: "mcp-dotnet",
        lang: "dotnet",
        keywords: &["mcp-dotnet", "dotnet-mcp"],
    },
    Framework {
        id: "mcpdotnet",
        label: "mcpdotnet",
        lang: "dotnet",
        keywords: &["mcpdotnet", "mcp.net"],
    },
    Framework {
        id: "csharp-mcp",
        label: "csharp-mcp",
        lang: "dotnet",
        keywords: &["csharp-mcp", "mcp-server-dotnet"],
    },
    // ---------- Ruby ----------
    Framework {
        id: "mcp-ruby",
        label: "MCP Ruby",
        lang: "ruby",
        keywords: &["mcp-ruby", "ruby-sdk", "modelcontextprotocol/ruby"],
    },
    Framework {
        id: "fast-mcp",
        label: "fast-mcp",
        lang: "ruby",
        keywords: &["fast-mcp", "fast_mcp"],
    },
    Framework {
        id: "ruby-mcp",
        label: "ruby-mcp",
        lang: "ruby",
        keywords: &["ruby-mcp", "ruby_mcp"],
    },
    Framework {
        id: "mcp-rb",
        label: "mcp-rb",
        lang: "ruby",
        keywords: &["mcp-rb"],
    },
    // ---------- PHP ----------
    Framework {
        id: "php-mcp-sdk",
        label: "PHP MCP SDK",
        lang: "php",
        keywords: &["php-mcp", "mcp-php-sdk", "php-sdk"],
    },
    Framework {
        id: "laravel-mcp",
        label: "Laravel MCP",
        lang: "php",
        keywords: &["laravel-mcp", "laravel mcp"],
    },
    Framework {
        id: "mcp-php",
        label: "mcp-php",
        lang: "php",
        keywords: &["mcp-php"],
    },
    Framework {
        id: "symfony-mcp",
        label: "Symfony MCP",
        lang: "php",
        keywords: &["symfony-mcp"],
    },
];

/// Framework par identifiant (badge UI, tests).
pub fn by_id(id: &str) -> Option<&'static Framework> {
    FRAMEWORKS.iter().find(|f| f.id == id)
}

/// Langage d'un framework connu.
pub fn lang_of(id: &str) -> Option<&'static str> {
    by_id(id).map(|f| f.lang)
}

/// Vrai si `kw` apparaît dans `hay` comme MOT délimité (début/fin de chaîne ou
/// caractère non alphanumérique) — « mcpo » ne doit pas matcher « mcporter ».
fn contains_word(hay: &str, kw: &str) -> bool {
    let mut start = 0usize;
    while let Some(pos) = hay[start..].find(kw) {
        let i = start + pos;
        let before_ok = i == 0
            || !hay[..i]
                .chars()
                .next_back()
                .map(|c| c.is_ascii_alphanumeric())
                .unwrap_or(false);
        let end = i + kw.len();
        let after_ok = end >= hay.len()
            || !hay[end..]
                .chars()
                .next()
                .map(|c| c.is_ascii_alphanumeric())
                .unwrap_or(false);
        if before_ok && after_ok {
            return true;
        }
        start = i + 1;
    }
    false
}

/// Détection par nom annoncé (`serverInfo.name`, insensible à la casse).
/// Le mot-clé le plus long gagne (plus spécifique : « rust-mcp-sdk » avant
/// « mcp-sdk »), puis l'ordre de la table tranche les égalités.
pub fn detect_by_name(name: &str) -> Option<&'static Framework> {
    let n = name.to_ascii_lowercase();
    let mut best: Option<(usize, &'static Framework)> = None;
    for f in FRAMEWORKS {
        for kw in f.keywords {
            if contains_word(&n, kw) {
                let score = kw.len();
                if best.map(|(s, _)| score > s).unwrap_or(true) {
                    best = Some((score, f));
                }
            }
        }
    }
    best.map(|(_, f)| f)
}

/// Signatures comportementales fortes.
/// `protocol` = version de spec renvoyée par initialize ; `health` = sonde
/// `/health` « ok » ; `sse` = réponses initialize encodées SSE ;
/// `sse_endpoint` = endpoint `/sse` legacy co-servi.
pub fn detect_signature(
    protocol: Option<&str>,
    health: bool,
    sse: bool,
    sse_endpoint: bool,
) -> Option<&'static Framework> {
    // LiteMCP : version de spec récente non négociée + /health « ok ».
    if protocol == Some(crate::LITEMCP_PROTOCOL) && health {
        return by_id("litemcp");
    }
    // FastMCP (mcp-proxy httpStream) : les deux signaux SSE + /sse legacy.
    if sse && sse_endpoint {
        return by_id("fastmcp-py");
    }
    None
}

/// Détection complète : signature d'abord (la plus fiable), puis nom.
/// Le langage permet de lever l'ambiguïté FastMCP Python vs TypeScript.
pub fn detect(
    name: Option<&str>,
    protocol: Option<&str>,
    health: bool,
    sse: bool,
    sse_endpoint: bool,
    lang_hint: Option<&str>,
) -> Option<&'static Framework> {
    if let Some(fw) = detect_signature(protocol, health, sse, sse_endpoint) {
        // FastMCP existe en Python et en TypeScript : le langage tranche.
        if fw.id == "fastmcp-py" && lang_hint == Some("node") {
            return by_id("fastmcp-ts");
        }
        return Some(fw);
    }
    name.and_then(detect_by_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_max_cinq_par_langage_et_ids_uniques() {
        use std::collections::{HashMap, HashSet};
        let mut ids = HashSet::new();
        let mut per_lang: HashMap<&str, usize> = HashMap::new();
        for f in FRAMEWORKS {
            assert!(ids.insert(f.id), "id dupliqué : {}", f.id);
            assert!(!f.label.is_empty(), "{}: label vide", f.id);
            assert!(!f.keywords.is_empty(), "{}: aucun mot-clé", f.id);
            assert!(
                !f.keywords.iter().any(|k| k != &k.to_ascii_lowercase()),
                "{}: mot-clé non minuscule",
                f.id
            );
            *per_lang.entry(f.lang).or_insert(0) += 1;
        }
        for lang in [
            "python", "node", "rust", "go", "java", "dotnet", "ruby", "php",
        ] {
            let n = per_lang.get(lang).copied().unwrap_or(0);
            assert!(n > 0, "aucun framework pour {}", lang);
            assert!(n <= 5, "{} a {} frameworks (max 5)", lang, n);
        }
    }

    #[test]
    fn detection_par_nom_et_par_langage() {
        assert_eq!(
            detect_by_name("FastMCP-Python").map(|f| f.id),
            Some("fastmcp-py")
        );
        assert_eq!(detect_by_name("xmcp-server").map(|f| f.id), Some("xmcp"));
        assert_eq!(detect_by_name("my rmcp server").map(|f| f.id), Some("rmcp"));
        assert_eq!(detect_by_name("mcp-go").map(|f| f.id), Some("mcp-go"));
        assert_eq!(
            detect_by_name("spring-ai-mcp").map(|f| f.id),
            Some("spring-ai")
        );
        assert_eq!(
            detect_by_name("Laravel MCP").map(|f| f.id),
            Some("laravel-mcp")
        );
        // langage déduit du framework
        assert_eq!(lang_of("mcp-go"), Some("go"));
        assert_eq!(lang_of("rmcp"), Some("rust"));
        // nom neutre → aucun framework inventé
        assert_eq!(detect_by_name("mon-serveur-metier"), None);
    }

    #[test]
    fn pas_de_faux_positifs_sur_les_paquets_officiels() {
        // Les serveurs npm officiels s'appellent @modelcontextprotocol/server-*
        // → ils ne doivent PAS être pris pour le SDK C#.
        assert_eq!(
            detect_by_name("@modelcontextprotocol/server-everything").map(|f| f.id),
            None,
            "un serveur npm officiel n'est pas le SDK C#"
        );
        assert_eq!(
            detect_by_name("@modelcontextprotocol/server-filesystem").map(|f| f.id),
            None
        );
        // « mcporter » contient « mcpo » : ne doit pas être pris pour mcpo
        assert_eq!(detect_by_name("mcporter").map(|f| f.id), None);
        // mais le vrai paquet mcpo est bien reconnu
        assert_eq!(detect_by_name("mcpo").map(|f| f.id), Some("mcpo"));
    }

    #[test]
    fn mots_cles_les_plus_specifiques_gagnent() {
        // « rust-mcp-sdk » contient « mcp-sdk » : le plus long doit gagner
        assert_eq!(
            detect_by_name("rust-mcp-sdk").map(|f| f.id),
            Some("rust-mcp-sdk")
        );
        assert_eq!(detect_by_name("fast-mcp").map(|f| f.id), Some("fast-mcp"));
    }

    #[test]
    fn signature_litemcp_et_fastmcp_avec_arbitrage_du_langage() {
        assert_eq!(
            detect(None, Some("2026-07-28"), true, false, false, None).map(|f| f.id),
            Some("litemcp")
        );
        // FastMCP avec indices Python → fastmcp-py ; avec Node → fastmcp-ts
        assert_eq!(
            detect(None, Some("2025-06-18"), false, true, true, Some("python")).map(|f| f.id),
            Some("fastmcp-py")
        );
        assert_eq!(
            detect(None, Some("2025-06-18"), false, true, true, Some("node")).map(|f| f.id),
            Some("fastmcp-ts")
        );
        // la signature l'emporte sur un nom trompeur
        assert_eq!(
            detect(Some("xmcp"), Some("2026-07-28"), true, false, false, None).map(|f| f.id),
            Some("litemcp")
        );
        // aucun signal → aucun framework
        assert_eq!(
            detect(Some("serveur-metier"), None, false, false, false, None),
            None
        );
    }
}
