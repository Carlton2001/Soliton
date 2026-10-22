import { createHash, randomInt, randomBytes } from 'node:crypto';
import { open } from 'node:fs/promises';
import { extname } from 'node:path';
import { EventEmitter } from 'node:events';

export function publicBase(value) {
  const url = new URL(value);
  if (url.protocol !== 'https:' || (url.port && url.port !== '443') ||
      url.username || url.password || url.search || url.hash || url.pathname !== '/' ||
      ['localhost', '127.0.0.1', '[::1]'].includes(url.hostname)) {
    throw new Error('HTTPS_PUBLIC_URL_REQUIRED');
  }
  return url.href.replace(/\/$/, '');
}

export class Probe extends EventEmitter {
  static async open(path, base, { now = Date.now, lifetimeMs = 30 * 60_000 } = {}) {
    base = publicBase(base);
    const extension = extname(path).toLowerCase();
    if (!['.flac', '.wav'].includes(extension)) throw new Error('FLAC_OR_WAV_REQUIRED');
    const file = await open(path, 'r');
    try {
      const stat = await file.stat();
      if (!stat.isFile() || stat.size < 12) throw new Error('INVALID_AUDIO_FILE');
      const header = Buffer.alloc(12);
      await file.read(header, 0, 12, 0);
      const flac = header.subarray(0, 4).toString() === 'fLaC';
      const wav = ['RIFF', 'RF64'].includes(header.subarray(0, 4).toString()) &&
        header.subarray(8, 12).toString() === 'WAVE';
      if (!(extension === '.flac' ? flac : wav)) throw new Error('INVALID_AUDIO_HEADER');
      const hash = createHash('sha256');
      for await (const chunk of file.createReadStream({ start: 0, autoClose: false })) hash.update(chunk);
      const after = await file.stat();
      if (after.size !== stat.size || after.mtimeMs !== stat.mtimeMs) throw new Error('SOURCE_CHANGED');
      return new Probe(file, stat, extension, base, hash.digest('hex'), now, lifetimeMs);
    } catch (error) {
      await file.close();
      throw error;
    }
  }

  constructor(file, stat, extension, base, sha256, now, lifetimeMs) {
    super();
    Object.assign(this, { file, stat, extension, base, sha256, now, lifetimeMs });
    this.current = null;
    this.streams = new Set();
    this.last = { state: 'idle', bytesServed: 0 };
  }

  snapshot() {
    this.expire();
    const s = this.current ?? this.last;
    return { state: s.state, offsetMs: s.offsetMs ?? 0, bytesServed: s.bytesServed,
      size: this.stat.size, sha256: this.sha256, format: this.extension.slice(1),
      transcoded: false, error: s.error ?? null };
  }

  expire() {
    const s = this.current;
    if (s && (this.now() >= s.expires ||
        (['waiting', 'loading'].includes(s.state) && this.now() >= s.startDeadline))) {
      this.revoke('failed', 'PLAYBACK_TIMEOUT');
    }
  }

  revoke(state = 'stopped', error = null) {
    if (this.current) this.last = { ...this.current, state, error };
    this.current = null;
    for (const stream of this.streams) stream.destroy();
    this.streams.clear();
    this.emit('status', this.snapshot());
  }

  begin(serial) {
    this.revoke();
    this.current = { serial, token: randomBytes(32).toString('hex'),
      code: String(randomInt(100000, 1000000)), deviceId: null, requestId: null,
      state: 'waiting', offsetMs: 0, bytesServed: 0,
      expires: this.now() + this.lifetimeMs, startDeadline: this.now() + 45_000 };
    this.emit('status', this.snapshot());
    return { code: this.current.code, token: this.current.token };
  }

  descriptor(s) {
    return { token: s.token, url: `${this.base}/media/${s.token}/audio${this.extension}`,
      offsetMs: s.offsetMs, title: `RAP — ${this.extension.slice(1).toUpperCase()}` };
  }

  launch({ code, deviceId, requestId }) {
    this.expire();
    const s = this.current;
    if (!s || !deviceId || !requestId || s.code !== String(code) ||
        (s.deviceId && (s.deviceId !== deviceId || s.requestId !== requestId))) {
      throw new Error('STALE_OR_WRONG_LAUNCH');
    }
    // Duplicate delivery of the same Alexa request is idempotent.
    if (!s.deviceId) {
      s.deviceId = deviceId;
      s.requestId = requestId;
      s.state = 'loading';
      s.lastEventTime = 0;
      s.eventIds = new Set();
      this.emit('status', this.snapshot());
    }
    return this.descriptor(s);
  }

  resume({ deviceId, token }) {
    this.expire();
    const s = this.current;
    if (!s || s.deviceId !== deviceId || s.token !== token || s.state !== 'paused') {
      throw new Error('NO_PAUSED_PROBE');
    }
    s.state = 'loading';
    s.startDeadline = this.now() + 45_000;
    this.emit('status', this.snapshot());
    return this.descriptor(s);
  }

  event({ deviceId, token, type, offsetMs, error, timestamp, requestId }) {
    this.expire();
    const s = this.current;
    if (!s || s.deviceId !== deviceId || s.token !== token) return false;
    const states = { PlaybackStarted: 'playing', PlaybackStopped: 'paused',
      PlaybackFinished: 'finished', PlaybackFailed: 'failed' };
    if (!Object.hasOwn(states, type)) return false;
    if (requestId && s.eventIds.has(requestId)) return false;
    const eventTime = Date.parse(timestamp);
    if (Number.isFinite(eventTime)) {
      if (eventTime < s.lastEventTime) return false;
      s.lastEventTime = eventTime;
    }
    if (requestId) {
      s.eventIds.add(requestId);
      if (s.eventIds.size > 64) s.eventIds.delete(s.eventIds.values().next().value);
    }
    if (Number.isSafeInteger(offsetMs) && offsetMs >= 0) s.offsetMs = offsetMs;
    s.state = states[type];
    if (type === 'PlaybackFailed') {
      // Do not propagate remote error messages containing private URLs.
      const allowed = ['MEDIA_ERROR_UNKNOWN', 'MEDIA_ERROR_INVALID_REQUEST',
        'MEDIA_ERROR_SERVICE_UNAVAILABLE', 'MEDIA_ERROR_INTERNAL_SERVER_ERROR',
        'MEDIA_ERROR_INTERNAL_DEVICE_ERROR'];
      this.revoke('failed', allowed.includes(error) ? error : 'PLAYBACK_FAILED');
    } else if (type === 'PlaybackFinished') {
      this.revoke('finished');
    } else {
      this.emit('status', this.snapshot());
    }
    return true;
  }

  async media(path) {
    this.expire();
    const s = this.current;
    if (!s || !s.deviceId || path !== `/media/${s.token}/audio${this.extension}`) return null;
    const stat = await this.file.stat();
    if (this.current !== s) return null;
    if (stat.size !== this.stat.size || stat.mtimeMs !== this.stat.mtimeMs) {
      this.revoke('failed', 'SOURCE_CHANGED');
      return null;
    }
    return s;
  }

  async close() {
    this.revoke();
    await this.file.close();
  }
}
