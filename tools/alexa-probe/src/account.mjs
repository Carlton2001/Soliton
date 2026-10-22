import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { homedir } from 'node:os';
import { randomBytes } from 'node:crypto';

export function stateDirectory() {
  if (process.platform === 'win32' && process.env.LOCALAPPDATA) {
    return join(process.env.LOCALAPPDATA, 'RAP', 'alexa-probe');
  }
  return join(homedir(), '.local', 'state', 'rap', 'alexa-probe');
}

export async function bridgeKey(directory = stateDirectory()) {
  await mkdir(directory, { recursive: true, mode: 0o700 });
  const path = join(directory, 'bridge-key');
  try { await writeFile(path, randomBytes(32).toString('hex'), { flag: 'wx', mode: 0o600 }); }
  catch (error) { if (error.code !== 'EEXIST') throw error; }
  const key = (await readFile(path, 'utf8')).trim();
  if (!/^[a-f0-9]{64}$/.test(key)) throw new Error('INVALID_BRIDGE_KEY');
  return key;
}

export function call(remote, method, args, timeoutMs = 15_000) {
  return new Promise((resolve, reject) => {
    const timer = setTimeout(() => reject(new Error('AMAZON_TIMEOUT')), timeoutMs);
    const done = (error, result) => {
      clearTimeout(timer);
      if (error) reject(new Error('AMAZON_REQUEST_FAILED'));
      else resolve(result);
    };
    try { remote[method](...args, done); }
    catch { done(new Error()); }
  });
}

export class Account {
  constructor(remote) { this.remote = remote; }

  static async connect({ interactive = false, language = 'fr', directory = stateDirectory() } = {}) {
    await mkdir(directory, { recursive: true, mode: 0o700 });
    let cookie;
    try { cookie = JSON.parse(await readFile(join(directory, 'session.json'), 'utf8')); }
    catch (error) {
      if (error.code !== 'ENOENT' && !(interactive && error instanceof SyntaxError)) {
        throw new Error('INVALID_SESSION');
      }
    }
    if (!interactive && !cookie) throw new Error('LOGIN_REQUIRED');
    const { default: AlexaRemote } = await import('alexa-remote2');
    const remote = new AlexaRemote();
    // Silence both upstream and its proxy's default logger: these logs can
    // contain account cookies, OAuth redirects or request headers.
    let saved = Promise.resolve();
    let saveFailed = false;
    remote.on('cookie', (value, _csrf, macDms) => {
      const data = remote.cookieData ?? { localCookie: value, macDms };
      saved = saved.then(() => writeFile(join(directory, 'session.json'), JSON.stringify(data),
        { mode: 0o600 })).catch(() => { saveFailed = true; });
    });
    try {
      await new Promise((resolve, reject) => {
        const timer = setTimeout(() => reject(new Error('AMAZON_TIMEOUT')), interactive ? 15 * 60_000 : 30_000);
        remote.init({ cookie, amazonPage: language === 'fr' ? 'amazon.fr' : 'amazon.com',
          alexaServiceHost: language === 'fr' ? 'alexa.amazon.fr' : 'pitangui.amazon.com',
          acceptLanguage: language === 'fr' ? 'fr-FR' : 'en-US',
          amazonPageProxyLanguage: language === 'fr' ? 'fr_FR' : 'en_US',
          proxyOnly: interactive, setupProxy: interactive,
          proxyOwnIp: '127.0.0.1', proxyListenBind: '127.0.0.1', proxyPort: 8767,
          formerDataStorePath: join(directory, 'registration.json'),
          deviceAppName: 'RAP Alexa probe', bluetooth: false,
          logger: () => {},
          useWsMqtt: false, cookieRefreshInterval: 0,
          autoQueryActivityOnTrigger: false }, error => {
          // The login library reports the browser challenge through the callback
          // once, and calls it again after successful authentication.
          if (interactive && error?.message?.startsWith('Please open http://127.0.0.1:8767/')) return;
          clearTimeout(timer);
          if (error) reject(new Error('AMAZON_LOGIN_FAILED'));
          else resolve();
        });
      });
      await saved;
      if (saveFailed) throw new Error('SESSION_SAVE_FAILED');
      return new Account(remote);
    } catch (error) {
      remote.stop();
      remote.stopProxyServer();
      throw error;
    }
  }

  async devices() {
    const result = await call(this.remote, 'getDevices', []);
    if (!Array.isArray(result?.devices)) throw new Error('INVALID_DEVICE_LIST');
    const seen = new Set();
    return result.devices.filter(d => {
      if (d.deviceFamily !== 'ECHO' || !d.serialNumber || seen.has(d.serialNumber) ||
          (d.clusterMembers?.length ?? 0) > 0 || !d.capabilities?.includes('AUDIO_PLAYER')) return false;
      seen.add(d.serialNumber);
      return true;
    }).map(d => ({ id: d.serialNumber, name: d.accountName || 'Echo', online: d.online === true,
      groups: Object.values(this.remote.serialNumbers)
        .filter(group => group.clusterMembers?.includes(d.serialNumber))
        .map(group => group.accountName || 'Echo') }))
      .sort((a, b) => a.name.localeCompare(b.name) || a.id.localeCompare(b.id));
  }

  async start(serial, code, language = 'fr') {
    // Require an exact cached serial, never a potentially ambiguous friendly name.
    if (!Object.hasOwn(this.remote.serialNumbers, serial)) throw new Error('DEVICE_UNAVAILABLE');
    const text = language === 'fr'
      ? `demande à rap musique de lancer le test numéro ${code}`
      : `ask rap music to start test number ${code}`;
    return call(this.remote, 'sendSequenceCommand', [serial, 'textCommand', text]);
  }

  async transport(serial, command) {
    if (!Object.hasOwn(this.remote.serialNumbers, serial)) throw new Error('DEVICE_UNAVAILABLE');
    if (command === 'stop') return call(this.remote, 'sendSequenceCommand', [serial, 'deviceStop', null]);
    if (!['pause', 'play'].includes(command)) throw new Error('INVALID_COMMAND');
    return call(this.remote, 'sendCommand', [serial, command, null]);
  }

  close() { this.remote.stop(); this.remote.stopProxyServer(); }
}
