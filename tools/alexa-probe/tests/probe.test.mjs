import test from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { createHash } from 'node:crypto';
import { Probe, publicBase } from '../src/probe.mjs';
import { gateway, range } from '../src/gateway.mjs';
import { createHandler } from '../skill/index.mjs';

const key = 'a'.repeat(64);

async function fixture(t, options = {}) {
  const directory = await mkdtemp(join(tmpdir(), 'rap-alexa-'));
  const path = join(directory, 'sample.flac');
  // Synthetic transport bytes, not an acoustic validation fixture.
  const bytes = Buffer.concat([Buffer.from('fLaC'), Buffer.alloc(4096, 42)]);
  await writeFile(path, bytes);
  const probe = await Probe.open(path, 'https://rap.example', options);
  const server = await gateway(probe, key, 0);
  t.after(async () => {
    await probe.close();
    await new Promise(resolve => { server.close(resolve); server.closeAllConnections(); });
    await rm(directory, { recursive: true, force: true });
  });
  const local = `http://127.0.0.1:${server.address().port}`;
  const post = (route, value, secret = key) => fetch(`${local}/probe/${route}`, {
    method: 'POST', headers: { Authorization: `Bearer ${secret}` }, body: JSON.stringify(value),
  });
  return { probe, server, local, post, path, bytes };
}

test('original bytes, SHA-256, ranges and HEAD; source is unchanged', async t => {
  const { probe, local, post, path, bytes, server } = await fixture(t);
  assert.equal(server.address().address, '127.0.0.1');
  const attempt = probe.begin('studio-serial');
  const launch = { code: attempt.code, deviceId: 'opaque-studio-id', requestId: 'request-1' };
  const response = await post('launch', launch);
  assert.equal(response.status, 200);
  const descriptor = await response.json();
  const media = `${local}${new URL(descriptor.url).pathname}`;
  assert.equal(probe.snapshot().state, 'loading'); // HTTP acknowledgement is not playback.
  const head = await fetch(media, { method: 'HEAD' });
  assert.equal(head.headers.get('content-length'), String(bytes.length));
  assert.equal(probe.snapshot().bytesServed, 0);
  const full = await fetch(media);
  assert.equal(full.headers.get('content-type'), 'audio/flac');
  const original = Buffer.from(await full.arrayBuffer());
  assert.deepEqual(original, bytes);
  assert.equal(createHash('sha256').update(original).digest('hex'), probe.snapshot().sha256);
  const partial = await fetch(media, { headers: { Range: 'bytes=100-199' } });
  assert.equal(partial.status, 206);
  assert.equal(partial.headers.get('content-range'), `bytes 100-199/${bytes.length}`);
  assert.deepEqual(Buffer.from(await partial.arrayBuffer()), bytes.subarray(100, 200));
  assert.equal((await fetch(media, { headers: { Range: 'bytes=99999-' } })).status, 416);
  assert.deepEqual(await readFile(path), bytes);
  assert.equal(probe.snapshot().transcoded, false);
  assert.equal(probe.snapshot().bytesServed, bytes.length + 100);
});

test('no arbitrary files or unauthenticated commands, tokens revoked on replacement and stop', async t => {
  const { probe, post, local } = await fixture(t);
  const first = probe.begin('studio');
  assert.equal((await post('launch', {}, 'wrong')).status, 401);
  assert.equal((await fetch(`${local}/media/${first.token}/audio.flac`)).status, 404);
  probe.launch({ code: first.code, deviceId: 'd', requestId: 'r' });
  assert.equal((await fetch(`${local}/session.json`)).status, 404);
  assert.equal((await fetch(`${local}/media/${first.token}/audio.flac?file=C:/secret`)).status, 404);
  const second = probe.begin('other-studio');
  assert.equal((await fetch(`${local}/media/${first.token}/audio.flac`)).status, 404);
  probe.launch({ code: second.code, deviceId: 'd2', requestId: 'r2' });
  probe.revoke();
  assert.equal((await fetch(`${local}/media/${second.token}/audio.flac`)).status, 404);
});

test('late/wrong-device events cannot change a newer attempt; actual Alexa events drive state', async t => {
  const { probe } = await fixture(t);
  const first = probe.begin('studio');
  const launch = { code: first.code, deviceId: 'd', requestId: 'r' };
  assert.deepEqual(probe.launch(launch), probe.launch(launch));
  assert.throws(() => probe.launch({ ...launch, requestId: 'replay' }));
  const event = { deviceId: 'd', token: first.token, type: 'PlaybackStarted' };
  assert.equal(probe.event({ ...event, deviceId: 'wrong' }), false);
  assert.equal(probe.snapshot().state, 'loading');
  assert.equal(probe.event(event), true);
  assert.equal(probe.snapshot().state, 'playing');
  probe.event({ ...event, type: 'PlaybackStopped', offsetMs: 1000 });
  assert.equal(probe.resume(event).offsetMs, 1000);
  assert.equal(probe.snapshot().state, 'loading');
  probe.event(event);
  probe.begin('new-studio');
  assert.equal(probe.event({ ...event, type: 'PlaybackFinished' }), false);
  assert.equal(probe.snapshot().state, 'waiting');
});

test('missing playback confirmation and expired grants fail closed', async t => {
  let time = 0;
  const { probe, local } = await fixture(t, { now: () => time, lifetimeMs: 90_000 });
  const first = probe.begin('studio');
  time = 46_000;
  assert.equal(probe.snapshot().state, 'failed');
  assert.throws(() => probe.launch({ code: first.code, deviceId: 'd', requestId: 'r' }));
  const next = probe.begin('studio');
  probe.launch({ code: next.code, deviceId: 'd', requestId: 'r2' });
  probe.event({ deviceId: 'd', token: next.token, type: 'PlaybackStarted' });
  time += 91_000;
  assert.equal((await fetch(`${local}/media/${next.token}/audio.flac`)).status, 404);
  assert.equal(probe.snapshot().state, 'failed');
});

test('file changes revoke playback and source errors never expose paths', async t => {
  const { probe, local, path } = await fixture(t);
  const attempt = probe.begin('studio');
  probe.launch({ code: attempt.code, deviceId: 'd', requestId: 'r' });
  await writeFile(path, 'changed');
  const response = await fetch(`${local}/media/${attempt.token}/audio.flac`);
  assert.equal(response.status, 404);
  assert.equal(probe.snapshot().error, 'SOURCE_CHANGED');
  assert.ok(!(await response.text()).includes(path));
});

test('range and public URL validation', () => {
  assert.deepEqual(range('bytes=-10', 100), [90, 99]);
  assert.deepEqual(range('bytes=90-999', 100), [90, 99]);
  for (const value of ['bytes=-0', 'bytes=10-9', 'bytes=0-1,4-5', 'bytes=9007199254740993-']) {
    assert.equal(range(value, 100), null);
  }
  for (const value of ['http://example.com', 'https://user:secret@example.com',
    'https://example.com:8443', 'https://example.com?secret=x', 'https://example.com/rap']) {
    assert.throws(() => publicBase(value));
  }
  assert.equal(publicBase('https://example.com/'), 'https://example.com');
});

test('Lambda-to-gateway contract, confirmed pause/resume and end-of-file revocation', async t => {
  const { probe, local } = await fixture(t);
  const env = { RAP_ALEXA_PUBLIC_URL: 'https://rap.example', RAP_ALEXA_BRIDGE_KEY: key,
    RAP_ALEXA_SKILL_ID: 'amzn1.ask.skill.test', RAP_ALEXA_USER_ID: 'user' };
  const handler = createHandler(env, (url, options) => fetch(`${local}${new URL(url).pathname}`, options));
  const attempt = probe.begin('studio-serial');
  let sequence = 0;
  const event = request => ({ context: { System: { application: { applicationId: env.RAP_ALEXA_SKILL_ID },
    user: { userId: 'user' }, device: { deviceId: 'studio-device' } },
    AudioPlayer: { token: attempt.token } }, request: {
      locale: 'fr-FR', requestId: `r${++sequence}`, timestamp: new Date(1000 * sequence).toISOString(), ...request,
    } });
  const response = await handler(event({ type: 'IntentRequest', intent: {
    name: 'PlayProbeIntent', slots: { code: { value: attempt.code } },
  } }));
  assert.equal(response.response.directives[0].audioItem.stream.token, attempt.token);
  assert.equal(probe.snapshot().state, 'loading');
  await handler(event({ type: 'AudioPlayer.PlaybackStarted', token: attempt.token }));
  assert.equal(probe.snapshot().state, 'playing');
  await handler(event({ type: 'AudioPlayer.PlaybackStopped', token: attempt.token, offsetInMilliseconds: 1200 }));
  const resumed = await handler(event({ type: 'IntentRequest', intent: { name: 'AMAZON.ResumeIntent' } }));
  assert.equal(resumed.response.directives[0].audioItem.stream.offsetInMilliseconds, 1200);
  await handler(event({ type: 'AudioPlayer.PlaybackStarted', token: attempt.token }));
  const older = event({ type: 'AudioPlayer.PlaybackStopped', token: attempt.token,
    timestamp: new Date(1000).toISOString() });
  await handler(older);
  assert.equal(probe.snapshot().state, 'playing');
  await handler(event({ type: 'AudioPlayer.PlaybackFinished', token: attempt.token }));
  assert.equal(probe.snapshot().state, 'finished');
  assert.equal(await probe.media(`/media/${attempt.token}/audio.flac`), null);
});
