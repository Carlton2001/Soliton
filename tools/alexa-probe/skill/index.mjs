// Deploy this file to AWS Lambda with an Alexa Skills Kit trigger restricted to
// RAP_ALEXA_SKILL_ID. Do not expose a Lambda function URL or an HTTP trigger.

export function createHandler(env, request = fetch) {
  const base = new URL(env.RAP_ALEXA_PUBLIC_URL);
  if (base.protocol !== 'https:' || (base.port && base.port !== '443') ||
      base.username || base.password || base.search || base.hash || base.pathname !== '/' ||
      !/^amzn1\.ask\.skill\./.test(env.RAP_ALEXA_SKILL_ID ?? '') ||
      !env.RAP_ALEXA_USER_ID || !/^[a-f0-9]{64}$/.test(env.RAP_ALEXA_BRIDGE_KEY ?? '')) {
    throw new Error('INVALID_CONFIGURATION');
  }
  const prefix = base.href.replace(/\/$/, '');
  async function rpc(path, data) {
    const result = await request(`${prefix}/probe/${path}`, { method: 'POST',
      redirect: 'error', signal: AbortSignal.timeout(4000),
      headers: { Authorization: `Bearer ${env.RAP_ALEXA_BRIDGE_KEY}`, 'Content-Type': 'application/json' },
      body: JSON.stringify(data) });
    if (!result.ok) throw new Error('BRIDGE_UNAVAILABLE');
    return result.json();
  }
  const reply = response => ({ version: '1.0', response: { shouldEndSession: true, ...response } });
  const say = (text) => reply({ outputSpeech: { type: 'PlainText', text } });
  const play = (media) => {
    if (!media.url?.startsWith(`${prefix}/media/`) || !/^[a-f0-9]{64}$/.test(media.token) ||
        !Number.isSafeInteger(media.offsetMs) || media.offsetMs < 0) throw new Error('INVALID_MEDIA');
    return reply({ directives: [{ type: 'AudioPlayer.Play', playBehavior: 'REPLACE_ALL',
      audioItem: { stream: { token: media.token, url: media.url, offsetInMilliseconds: media.offsetMs },
        metadata: { title: media.title } } }] });
  };
  return async event => {
    const system = event.context?.System;
    if (system?.application?.applicationId !== env.RAP_ALEXA_SKILL_ID ||
        system?.user?.userId !== env.RAP_ALEXA_USER_ID || !system?.device?.deviceId) {
      throw new Error('UNAUTHORIZED_ALEXA_REQUEST');
    }
    const req = event.request;
    const deviceId = system.device.deviceId;
    const fr = req?.locale?.startsWith('fr');
    try {
      if (req?.type === 'IntentRequest' && req.intent?.name === 'PlayProbeIntent') {
        const code = req.intent.slots?.code?.value;
        if (!/^\d{6}$/.test(code ?? '')) throw new Error('INVALID_CODE');
        return play(await rpc('launch', { code, deviceId, requestId: req.requestId }));
      }
      if ((req?.type === 'IntentRequest' && req.intent?.name === 'AMAZON.ResumeIntent') ||
          req?.type === 'PlaybackController.PlayCommandIssued') {
        return play(await rpc('resume', { deviceId, token: event.context?.AudioPlayer?.token }));
      }
      if (req?.type === 'AudioPlayer.PlaybackNearlyFinished') return reply({});
      if (['AudioPlayer.PlaybackStarted', 'AudioPlayer.PlaybackStopped',
        'AudioPlayer.PlaybackFinished', 'AudioPlayer.PlaybackFailed'].includes(req?.type)) {
        await rpc('event', { deviceId, token: req.token, type: req.type.slice('AudioPlayer.'.length),
          offsetMs: req.offsetInMilliseconds ?? req.currentPlaybackState?.offsetInMilliseconds,
          error: req.error?.type, timestamp: req.timestamp, requestId: req.requestId });
        return reply({});
      }
      if ((req?.type === 'IntentRequest' && ['AMAZON.StopIntent', 'AMAZON.CancelIntent',
        'AMAZON.PauseIntent'].includes(req.intent?.name)) || req?.type === 'PlaybackController.PauseCommandIssued') {
        return reply({ directives: [{ type: 'AudioPlayer.Stop' }] });
      }
      if (req?.type === 'SessionEndedRequest') return reply({});
      return say(fr ? 'Lancez le test depuis RAP sur votre ordinateur.' : 'Start the test from RAP on your computer.');
    } catch {
      // Never log or speak URLs, account identifiers, tokens or server errors.
      if (req?.type?.startsWith('AudioPlayer.')) throw new Error('BRIDGE_EVENT_FAILED');
      return say(fr ? 'Le test RAP est indisponible ou a expiré.' : 'The RAP test is unavailable or has expired.');
    }
  };
}

export const handler = async event => createHandler(process.env)(event);
