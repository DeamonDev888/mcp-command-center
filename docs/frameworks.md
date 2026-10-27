# Frameworks MCP reconnus

MCP Control identifie **37 frameworks MCP** automatiquement,
à partir du nom que le serveur s'annonce (`serverInfo.name`), du paquet lancé
(pour les serveurs stdio) et de quelques signatures protocolaires.

> Cette page est générée depuis `src/frameworks.rs` — la source de vérité.

## Par langage

### Python (5)

| Framework | Détecté par |
|---|---|
| **FastMCP** | `fastmcp-python`, `fastmcp python` |
| **FastAPI-MCP** | `fastapi-mcp`, `fastapi mcp` |
| **mcpo** | `mcpo` |
| **mcp-proxy** | `mcp-proxy`, `mcp_proxy` |
| **MCP SDK** | `mcp-python-sdk`, `python-sdk`, `mcp python sdk` |

### Node.js / TypeScript (6)

| Framework | Détecté par |
|---|---|
| **FastMCP** | `fastmcp-typescript`, `fastmcp typescript` |
| **xmcp** | `xmcp` |
| **mcp-framework** | `mcp-framework` |
| **MCP SDK** | `@modelcontextprotocol/sdk`, `modelcontextprotocol/sdk`, `typescript-sdk`, `…/server`, `…/sdk`, `@modelcontextprotocol/server` |
| **Fastify MCP** | `fastify-mcp`, `fastify mcp` |
| **ViteMCP** | `vitemcp`, `vite-mcp` |

### Rust (5)

| Framework | Détecté par |
|---|---|
| **LiteMCP** | `litemcp`, `lite-mcp` |
| **rmcp** | `rmcp`, `rust-sdk`, `modelcontextprotocol/rust` |
| **rust-mcp-sdk** | `rust-mcp-sdk`, `rust_mcp_sdk` |
| **tower-mcp** | `tower-mcp`, `tower_mcp` |
| **mcp-rs** | `mcp-rs`, `mcp_rs` |

### Go (5)

| Framework | Détecté par |
|---|---|
| **mcp-go** | `mcp-go`, `mark3labs` |
| **Go SDK** | `modelcontextprotocol/go`, `go-sdk`, `golang-sdk` |
| **mcp-golang** | `mcp-golang` |
| **gomcp** | `gomcp` |
| **go-mcp** | `go-mcp`, `go_mcp` |

### Java (5)

| Framework | Détecté par |
|---|---|
| **Spring AI** | `spring-ai`, `spring ai`, `springaialibaba` |
| **MCP Java SDK** | `mcp-java-sdk`, `java-sdk`, `io.modelcontextprotocol` |
| **Quarkus MCP** | `quarkus-mcp`, `quarkus mcp`, `quarkus` |
| **LangChain4j** | `langchain4j`, `langchain4j-mcp` |
| **mcp-java** | `mcp-java`, `mcp4j` |

### .NET (3)

| Framework | Détecté par |
|---|---|
| **mcp-dotnet** | `mcp-dotnet`, `dotnet-mcp` |
| **mcpdotnet** | `mcpdotnet`, `mcp.net` |
| **csharp-mcp** | `csharp-mcp`, `mcp-server-dotnet` |

### Ruby (4)

| Framework | Détecté par |
|---|---|
| **MCP Ruby** | `mcp-ruby`, `ruby-sdk`, `modelcontextprotocol/ruby` |
| **fast-mcp** | `fast-mcp`, `fast_mcp` |
| **ruby-mcp** | `ruby-mcp`, `ruby_mcp` |
| **mcp-rb** | `mcp-rb` |

### PHP (4)

| Framework | Détecté par |
|---|---|
| **PHP MCP SDK** | `php-mcp`, `mcp-php-sdk`, `php-sdk` |
| **Laravel MCP** | `laravel-mcp`, `laravel mcp` |
| **mcp-php** | `mcp-php` |
| **Symfony MCP** | `symfony-mcp` |

## Comment la détection fonctionne

1. **Signature forte** — le nom du serveur ou le paquet lancé correspond à un
   mot-clé connu (recherche par MOT DÉLIMITÉ : `mcpo` ne matche pas `mcporter`).
2. **Nom annoncé** — `serverInfo.name` du serveur.
3. **Arbitrage LiteMCP** — entre FastMCP Python et FastMCP TypeScript, le
   **langage du process** tranche (le nom ne suffit pas).

Quand aucun framework n'est identifié, **aucun badge n'est affiché** —
l'app n'invente jamais de framework.
