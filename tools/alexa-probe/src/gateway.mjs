import { createServer } from 'node:http';
import { timingSafeEqual } from 'node:crypto';
import { pipeline } from 'node:stream/promises';

export function range(value, size) {
  if (!value) return [0, size - 1];
  const match = /^bytes=(\d*)-(\d*)$/.exec(value);
  if (!match || !(match[1] || match[2])) return null;
  let start = match[1] ? Number(match[1]) : Math.max(0, size - Number(match[2]));
  let end = match[1] && match[2] ? Math.min(size - 1, Number(match[2])) : size - 1;
  if (!Number.isSafeInteger(start) || !Number.isSafeInteger(end) ||
      (match[2] && !Number.isSafeInteger(Number(match[2]))) ||
      start < 0 || end < start || start >= size) return null;
  return [start, end];
}

function authorized(header, key) {
  const actual = Buffer.from(header ?? '');
  const expected = Buffer.from(`Bearer ${key}`);
  return actual.length === expected.length && timingSafeEqual(actual, expected);
}

async function body(request) {
  const chunks = [];
  let size = 0;
  for await (const chunk of request) {
    size += chunk.length;
    if (size > 4096) throw new Error('BODY_TOO_LARGE');
    chunks.push(chunk);
  }
  const result = JSON.parse(Buffer.concat(chunks).toString());
  if (!result || Array.isArray(result) || typeof result !== 'object') throw new Error('INVALID_BODY');
  return result;
}

export async function gateway(probe, key, port = 8766) {
  if (!/^[a-f0-9]{64}$/.test(key)) throw new Error('INVALID_BRIDGE_KEY');
  const server = createServer(async (request, response) => {
    response.setHeader('Cache-Control', 'no-store');
    const json = (status, value) => {
      response.writeHead(status, { 'Content-Type': 'application/json' });
      response.end(JSON.stringify(value));
    };
    try {
      // Match raw paths exactly: no filesystem paths, query strings or listings.
      if (request.method === 'POST' && ['/probe/launch', '/probe/resume', '/probe/event'].includes(request.url)) {
        if (!authorized(request.headers.authorization, key)) return json(401, { error: 'UNAUTHORIZED' });
        const data = await body(request);
        if (request.url === '/probe/launch') return json(200, probe.launch(data));
        if (request.url === '/probe/resume') return json(200, probe.resume(data));
        return json(200, { accepted: probe.event(data) });
      }
      if (!['GET', 'HEAD'].includes(request.method)) return json(405, { error: 'METHOD' });
      const session = await probe.media(request.url);
      if (!session) return json(404, { error: 'NOT_FOUND' });
      if (probe.streams.size >= 8) return json(503, { error: 'BUSY' });
      const span = range(request.headers.range, probe.stat.size);
      if (!span) {
        response.writeHead(416, { 'Content-Range': `bytes */${probe.stat.size}` });
        return response.end();
      }
      const [start, end] = span;
      response.setHeader('Content-Type', probe.extension === '.flac' ? 'audio/flac' : 'audio/wav');
      response.setHeader('Content-Length', end - start + 1);
      response.setHeader('Accept-Ranges', 'bytes');
      if (request.headers.range) response.setHeader('Content-Range', `bytes ${start}-${end}/${probe.stat.size}`);
      response.writeHead(request.headers.range ? 206 : 200);
      if (request.method === 'HEAD') return response.end();
      response.setTimeout(30_000, () => response.destroy());
      const stream = probe.file.createReadStream({ start, end, autoClose: false });
      probe.streams.add(stream);
      stream.on('data', chunk => { session.bytesServed += chunk.length; });
      try { await pipeline(stream, response); }
      finally { probe.streams.delete(stream); }
    } catch (error) {
      if (response.headersSent) response.destroy();
      else json(error.message === 'BODY_TOO_LARGE' ? 413 : 409, { error: 'REQUEST_REJECTED' });
    }
  });
  server.maxConnections = 32;
  server.requestTimeout = 15_000;
  server.headersTimeout = 10_000;
  await new Promise((resolve, reject) => {
    server.once('error', reject);
    server.listen(port, '127.0.0.1', resolve);
  });
  return server;
}
