// Explicit opt-in QA: publishes only the named, generated development fixtures
// for the duration of the check. Never contacts Amazon or commands a speaker.
import { parseArgs } from 'node:util';
import { fileURLToPath } from 'node:url';
import { createHash, randomBytes } from 'node:crypto';
import { Probe } from '../src/probe.mjs';
import { gateway } from '../src/gateway.mjs';

const { values } = parseArgs({ options: { 'public-url': { type: 'string' } } });
if (!values['public-url']) throw new Error('PUBLIC_URL_REQUIRED');

for (const name of ['tone-16-44100.flac', 'tone-24-96000.flac', 'tone-16-44100.wav']) {
  const path = fileURLToPath(new URL(`../../../target/alexa-fixtures/${name}`, import.meta.url));
  const probe = await Probe.open(path, values['public-url']);
  let server;
  try {
    const attempt = probe.begin('transport-check-only');
    const media = probe.launch({ code: attempt.code, deviceId: 'transport-check-only', requestId: 'transport-check-only' });
    server = await gateway(probe, randomBytes(32).toString('hex'));
    const response = await fetch(media.url, { redirect: 'error', signal: AbortSignal.timeout(20_000) });
    if (!response.ok) throw new Error(`HTTPS_STATUS_${response.status}`);
    const bytes = Buffer.from(await response.arrayBuffer());
    if (createHash('sha256').update(bytes).digest('hex') !== probe.sha256 || bytes.length !== probe.stat.size) {
      throw new Error('HTTPS_BYTES_CHANGED');
    }
    const partial = await fetch(media.url, { redirect: 'error', signal: AbortSignal.timeout(20_000),
      headers: { Range: 'bytes=0-11' } });
    if (partial.status !== 206 || (await partial.arrayBuffer()).byteLength !== 12) throw new Error('HTTPS_RANGE_FAILED');
    probe.revoke();
    const revoked = await fetch(media.url, { redirect: 'error', signal: AbortSignal.timeout(20_000) });
    if (revoked.status !== 404) throw new Error('HTTPS_REVOCATION_FAILED');
    console.log(`${name}: HTTPS + SHA-256 + Range + revocation OK / révocation OK`);
  } finally {
    await probe.close();
    if (server) await new Promise(resolve => { server.close(resolve); server.closeAllConnections(); });
  }
}
