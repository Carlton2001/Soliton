// Run after scripts/generate-alexa-fixtures.ps1. This test uses real audio
// containers but does not connect to Amazon or play audio on any device.
import test from 'node:test';
import assert from 'node:assert/strict';
import { readFile } from 'node:fs/promises';
import { fileURLToPath } from 'node:url';
import { Probe } from '../src/probe.mjs';
import { gateway } from '../src/gateway.mjs';

for (const name of ['tone-16-44100.flac', 'tone-24-96000.flac', 'tone-16-44100.wav']) {
  test(`generated audio container: ${name}, exact HTTP bytes`, async () => {
    const path = fileURLToPath(new URL(`../../../target/alexa-fixtures/${name}`, import.meta.url));
    const probe = await Probe.open(path, 'https://rap.example');
    let server;
    try {
      const attempt = probe.begin('synthetic-studio');
      probe.launch({ code: attempt.code, deviceId: 'synthetic-device', requestId: 'synthetic-request' });
      server = await gateway(probe, 'f'.repeat(64), 0);
      const result = await fetch(`http://127.0.0.1:${server.address().port}/media/${attempt.token}/audio${probe.extension}`);
      assert.equal(result.status, 200);
      assert.deepEqual(Buffer.from(await result.arrayBuffer()), await readFile(path));
      assert.equal(probe.snapshot().transcoded, false);
    } finally {
      await probe.close();
      if (server) await new Promise(resolve => { server.close(resolve); server.closeAllConnections(); });
    }
  });
}
