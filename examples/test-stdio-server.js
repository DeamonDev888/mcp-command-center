// Fixture stdio MCP pour tester mcp-control (JSON-RPC par lignes, zéro dépendance).
// Usage : node examples/test-stdio-server.js
const readline = require('readline');
const rl = readline.createInterface({ input: process.stdin });
rl.on('line', line => {
  let m; try { m = JSON.parse(line); } catch { return; }
  if (m.method === 'initialize') {
    console.log(JSON.stringify({ jsonrpc: '2.0', id: m.id, result: {
      protocolVersion: '2024-11-05', capabilities: { tools: {} },
      serverInfo: { name: 'fake-stdio-mcp', version: '0.1.0' },
    } }));
  } else if (m.method === 'tools/list') {
    console.log(JSON.stringify({ jsonrpc: '2.0', id: m.id, result: { tools: [
      { name: 'ping', description: 'renvoie pong (fixture de test stdio)', inputSchema: { type: 'object' } },
      { name: 'echo', description: 'renvoie le texte passe en argument (fixture)', inputSchema: { type: 'object' } },
    ] } }));
  }
  // notifications/initialized : rien à répondre
});
console.error('fake-stdio-mcp prêt');
