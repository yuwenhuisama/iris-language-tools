const fs = require('node:fs');

let buffer = '';
process.stdin.on('data', chunk => {
  buffer += chunk.toString();
  if (buffer.includes('"initialize"')) {
    fs.writeFileSync(`${__filename}.initialized`, JSON.stringify({ pid: process.pid }));
    buffer = '';
  }
});
process.stdin.resume();
