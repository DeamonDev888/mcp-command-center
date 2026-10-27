// Fixture HTTP MCP pour tester la découverte par scan de mcp-control.
// FastMCP-like : bind IPv6 [::1], POST /mcp, réponse chunked (teste le dechunker du client).
// Usage : node examples/test-http-server.js [port]
const port = parseInt(process.argv[2] || '3166', 10);
require('http').createServer((req, res) => {
  if (req.method === 'POST' && req.url === '/mcp') {
    let body = '';
    req.on('data', c => { body += c; });
    req.on('end', () => {
      let msg; try { msg = JSON.parse(body); } catch { res.writeHead(400); res.end(); return; }
      let result;
      if (msg.method === 'initialize') {
        result = { protocolVersion: '2024-11-05', capabilities: { tools: {} },
                   serverInfo: { name: 'fake-http-mcp', version: '0.1.0' } };
      } else if (msg.method === 'tools/list') {
        result = { tools: [
          { name: 'lancer Les dés', description: 'fixture', inputSchema: { type: 'object' } },
        ].filter(t => !t.name.includes(' ')).concat([
          { name: 'roll_dice', description: 'lance un dé (fixture de test http)', inputSchema: { type: 'object' } },
        ]) };
      } else { result = {}; }
      res.writeHead(200, { 'Content-Type': 'application/json' }); // sans Content-Length => chunked
      res.write(JSON.stringify({ jsonrpc: '2.0', id: msg.id, result }) + '\n');
      res.end();
    });
  } else { res.writeHead(404); res.end('not found'); }
}).listen(port, '::1', () => console.log(`fake-http-mcp sur [::1]:${port}`));
