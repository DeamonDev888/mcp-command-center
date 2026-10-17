// Fixture MCP en transport SSE (réponses text/event-stream) — zéro dépendance.
const http = require('http');
const sse = (id, result) => `data: ${JSON.stringify({ jsonrpc: '2.0', id, result })}\n\n`;

const srv = http.createServer((req, res) => {
  let buf = '';
  req.on('data', (d) => (buf += d));
  req.on('end', () => {
    if (!buf.includes('"method"')) {
      res.writeHead(404);
      res.end();
      return;
    }
    let msg = {};
    try {
      msg = JSON.parse(buf.slice(buf.indexOf('{')));
    } catch (e) {
      res.writeHead(400);
      res.end();
      return;
    }
    res.writeHead(200, { 'Content-Type': 'text/event-stream', 'Cache-Control': 'no-cache', Connection: 'close' });
    if (msg.method === 'initialize') {
      res.end(sse(msg.id, { protocolVersion: '2024-11-05', capabilities: {}, serverInfo: { name: 'sse-fixture', version: '0.1.0' } }));
    } else {
      res.end(sse(msg.id, { tools: [{ name: 'ping', description: 'outil de test SSE' }] }));
    }
  });
});
srv.listen(3170, '::1', () => console.log('sse-fixture sur [::1]:3170'));
