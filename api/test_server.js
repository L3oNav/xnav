const http = require('http');

const host = '127.0.0.1';
const port = 8080;


const server = http.createServer((req, res) => {
  res.statusCode = 200;
  res.setHeader('Content-Type', 'application/json');
  res.setHeader('server', 'Node.js');
  console.log(req.headers);
  res.end("I'm the server behind the proxy");
})

server.listen(port, host, () => {
  console.log(`Server is running on http://${host}:${port}`);
})
