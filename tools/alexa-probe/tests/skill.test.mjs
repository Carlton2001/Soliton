import test from 'node:test';
import assert from 'node:assert/strict';
import { createHandler } from '../skill/index.mjs';
import { Account } from '../src/account.mjs';

const env = { RAP_ALEXA_PUBLIC_URL: 'https://rap.example', RAP_ALEXA_BRIDGE_KEY: 'b'.repeat(64),
  RAP_ALEXA_SKILL_ID: 'amzn1.ask.skill.example', RAP_ALEXA_USER_ID: 'allowed-user' };
function envelope(request, deviceId = 'studio-id') {
  return { context: { System: { application: { applicationId: env.RAP_ALEXA_SKILL_ID },
    user: { userId: env.RAP_ALEXA_USER_ID }, device: { deviceId } },
    AudioPlayer: { token: 'a'.repeat(64) } }, request: { locale: 'fr-FR', requestId: 'r1', ...request } };
}
const launch = { type: 'IntentRequest', intent: { name: 'PlayProbeIntent', slots: { code: { value: '123456' } } } };

test('skill rejects different accounts/apps before contacting the bridge', async () => {
  const handler = createHandler(env, () => { throw new Error('must not call'); });
  const event = envelope(launch);
  event.context.System.user.userId = 'other';
  await assert.rejects(handler(event), /UNAUTHORIZED/);
  event.context.System.user.userId = env.RAP_ALEXA_USER_ID;
  event.context.System.application.applicationId = 'other';
  await assert.rejects(handler(event), /UNAUTHORIZED/);
});

test('correlated launch and resume retain the original media URL and offset', async () => {
  const calls = [];
  const handler = createHandler(env, async (url, options) => {
    calls.push({ url, options });
    return { ok: true, json: async () => ({ url: `https://rap.example/media/${'a'.repeat(64)}/audio.flac`,
      token: 'a'.repeat(64), offsetMs: 1234, title: 'RAP' }) };
  });
  const response = await handler(envelope(launch));
  assert.equal(response.response.shouldEndSession, true);
  assert.equal(response.response.outputSpeech, undefined);
  assert.equal(response.response.directives[0].audioItem.stream.offsetInMilliseconds, 1234);
  assert.deepEqual(JSON.parse(calls[0].options.body), { code: '123456', deviceId: 'studio-id', requestId: 'r1' });
  assert.equal(calls[0].options.redirect, 'error');
  assert.equal(calls[0].options.headers.Authorization, `Bearer ${env.RAP_ALEXA_BRIDGE_KEY}`);
  await handler(envelope({ type: 'IntentRequest', intent: { name: 'AMAZON.ResumeIntent' } }));
  assert.equal(calls[1].url, 'https://rap.example/probe/resume');
});

test('Alexa playback failures are forwarded with token and device, without speaking private errors', async () => {
  let received;
  const handler = createHandler(env, async (_url, options) => {
    received = JSON.parse(options.body);
    return { ok: true, json: async () => ({ accepted: true }) };
  });
  const response = await handler(envelope({ type: 'AudioPlayer.PlaybackFailed', token: 'probe-token',
    error: { type: 'MEDIA_ERROR_INVALID_REQUEST', message: 'secret-url' } }));
  assert.equal(received.deviceId, 'studio-id');
  assert.equal(received.token, 'probe-token');
  assert.equal(received.error, 'MEDIA_ERROR_INVALID_REQUEST');
  assert.ok(!JSON.stringify(received).includes('secret-url'));
  assert.equal(response.response.outputSpeech, undefined);
});

test('unavailable bridge fails explicitly in both languages and never returns a play directive', async () => {
  const handler = createHandler(env, async () => { throw new Error('secret-token'); });
  for (const locale of ['fr-FR', 'en-US']) {
    const event = envelope(launch);
    event.request.locale = locale;
    const response = await handler(event);
    assert.equal(response.response.directives, undefined);
    assert.ok(!JSON.stringify(response).includes('secret-token'));
    assert.ok(response.response.outputSpeech.text.includes(locale === 'fr-FR' ? 'indisponible' : 'unavailable'));
  }
});

test('account selection uses exact serials, excludes groups, preserves offline entries', async () => {
  const calls = [];
  const remote = { serialNumbers: { s1: {}, s2: {}, group: { accountName: 'Existing pair', clusterMembers: ['s1', 's2'] } },
    getDevices: cb => cb(null, { devices: [
      { deviceFamily: 'ECHO', serialNumber: 's1', accountName: 'Studio', online: true, capabilities: ['AUDIO_PLAYER'] },
      { deviceFamily: 'ECHO', serialNumber: 's2', accountName: 'Studio', online: false, capabilities: ['AUDIO_PLAYER'] },
      { deviceFamily: 'ECHO', serialNumber: 'sub', accountName: 'Echo Sub', online: true, capabilities: ['AUDIO_CONTROLS'] },
      { deviceFamily: 'ECHO', serialNumber: 'group', clusterMembers: ['s1', 's2'] },
      { deviceFamily: 'WHA', serialNumber: 'wha' },
    ] }),
    sendSequenceCommand: (...args) => { calls.push(args.slice(0, -1)); args.at(-1)(null, {}); },
  };
  const account = new Account(remote);
  const devices = await account.devices();
  assert.deepEqual(devices.map(d => [d.id, d.online]), [['s1', true], ['s2', false]]);
  assert.deepEqual(devices[0].groups, ['Existing pair']);
  await account.start('s1', '123456', 'fr');
  assert.deepEqual(calls[0], ['s1', 'textCommand', 'demande à rap musique de lancer le test numéro 123456']);
  await assert.rejects(account.start('Studio', '123456'), /DEVICE_UNAVAILABLE/);
  await account.transport('s1', 'stop');
  assert.deepEqual(calls[1], ['s1', 'deviceStop', null]);
});
