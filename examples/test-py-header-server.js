// Fixture : serveur MCP HTTP qui se présente comme uvicorn (test inférence de langage).
const http = require('http');
const srv = http.createServer((req, res) => {
  let b = ''; req.on('data', d => b += d); req.on('end', () => {
    if (!b.includes('"method"')) { res.writeHead(404); res.end(); return; }
    let m = {}; try { m = JSON.parse(b.slice(b.indexOf('{'))); } catch (e) { res.writeHead(400); res.end(); return; }
    const result = m.method === 'initialize'
      ? { protocolVersion: '2024-11-05', capabilities: {}, serverInfo: { name: 'py-header-fixture', version: '0.1.0' } }
      : { tools: [{ name: 'echo', description: 'test' }] };
    const body = JSON.stringify({ jsonrpc: '2.0', id: m.id, result });
    res.writeHead(200, { 'Content-Type': 'application/json', 'Server': 'uvicorn', 'Content-Length': Buffer.byteLength(body) });
    res.end(body);
  });
});
srv.listen(3171, '::1', () => console.log('py-header-fixture sur [::1]:3171'));
