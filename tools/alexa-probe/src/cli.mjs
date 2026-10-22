import { parseArgs } from 'node:util';
import { createInterface } from 'node:readline/promises';
import { stdin, stdout } from 'node:process';
import { Account, bridgeKey } from './account.mjs';
import { Probe } from './probe.mjs';
import { gateway } from './gateway.mjs';

const text = {
  help: ['Usage : node src/cli.mjs login|setup|serve --file <fichier.flac> --public-url <https://…> [--lang fr|en]',
    'Usage: node src/cli.mjs login|setup|serve --file <file.flac> --public-url <https://…> [--lang fr|en]'],
  login: ['Ouvrez http://127.0.0.1:8767/ dans votre navigateur pour vous connecter à Amazon.',
    'Open http://127.0.0.1:8767/ in your browser to sign in to Amazon.'],
  connected: ['Connexion Amazon enregistrée localement.', 'Amazon session saved locally.'],
  setup: ['Clé à copier uniquement dans RAP_ALEXA_BRIDGE_KEY sur AWS Lambda. Ne pas la partager :',
    'Copy this key only into RAP_ALEXA_BRIDGE_KEY on AWS Lambda. Do not share it:'],
  listening: ['Flux prêt sur 127.0.0.1:8766. Aucun tunnel ni règle pare-feu créé.',
    'Media ready on 127.0.0.1:8766. No tunnel or firewall rule created.'],
  devices: ['Enceintes du compte Amazon (présence sur le réseau local non vérifiée) :',
    'Speakers on the Amazon account (local network presence not verified):'],
  select: ['Numéro de l’enceinte, ou q pour quitter : ', 'Speaker number, or q to quit: '],
  offline: ['indisponible', 'unavailable'],
  groups: ['groupe existant', 'existing group'],
  commands: ['Commandes : play, pause, resume, stop, status, exit. Réglez le volume de l’enceinte avant le test.',
    'Commands: play, pause, resume, stop, status, exit. Set the speaker volume before testing.'],
  invalid: ['Choix invalide ou enceinte indisponible.', 'Invalid choice or speaker unavailable.'],
  received: ['Commande envoyée ; attente de la confirmation Alexa.', 'Command sent; awaiting Alexa confirmation.'],
  error: ['Échec du test / de la commande :', 'Test / command failed:'],
  unavailable: ['Configuration ou service indisponible ; consulter le guide du prototype.',
    'Configuration or service unavailable; see the prototype guide.'],
  status: ['État', 'State'],
  states: {
    idle: ['inactif', 'idle'], waiting: ['attente de la skill', 'waiting for skill'],
    loading: ['attente du démarrage audio', 'waiting for audio start'],
    playing: ['lecture confirmée par Alexa', 'playback confirmed by Alexa'],
    paused: ['pause confirmée par Alexa', 'pause confirmed by Alexa'],
    finished: ['terminé', 'finished'], stopped: ['arrêt / accès au fichier révoqué', 'stopped / file access revoked'],
    failed: ['échec', 'failed'],
  },
};

const { values, positionals } = parseArgs({ allowPositionals: true, options: {
  file: { type: 'string' }, 'public-url': { type: 'string' }, lang: { type: 'string', default: 'fr' },
  help: { type: 'boolean', default: false },
} });
if (!['fr', 'en'].includes(values.lang)) throw new Error('INVALID_LANGUAGE');
const language = values.lang;
const index = language === 'fr' ? 0 : 1;
const t = key => text[key][index];
const safeError = error => /^[A-Z_]+$/.test(error.message) ? error.message : t('unavailable');

async function main() {
  if (values.help || !positionals.length) return console.log(t('help'));
  const command = positionals[0];
  if (positionals.length !== 1 || !['login', 'setup', 'serve'].includes(command)) throw new Error('INVALID_COMMAND');
  if (command === 'setup') {
    console.log(t('setup'));
    console.log(await bridgeKey());
    return;
  }
  if (command === 'login') {
    console.log(t('login'));
    const account = await Account.connect({ interactive: true, language });
    account.close();
    console.log(t('connected'));
    return;
  }
  if (!values.file || !values['public-url']) return console.log(t('help'));
  const probe = await Probe.open(values.file, values['public-url']);
  let account, server, terminal, timer, selected;
  let stopped = false, attempted = false;
  const stop = async () => {
    if (!selected || !attempted || stopped) return;
    probe.revoke();
    await account.transport(selected.id, 'stop');
    stopped = true;
  };
  try {
    account = await Account.connect({ language });
    const devices = await account.devices();
    console.log(t('devices'));
    const clean = value => value.replace(/[\x00-\x1f\x7f]/g, '');
    devices.forEach((device, i) => console.log(`${i + 1}. ${clean(device.name)}` +
      (device.online ? '' : ` (${t('offline')})`) +
      (device.groups.length ? ` [${t('groups')}: ${device.groups.map(clean).join(', ')}]` : '')));
    terminal = createInterface({ input: stdin, output: stdout });
    const answer = await terminal.question(t('select'));
    if (answer === 'q') return;
    selected = devices[Number(answer) - 1];
    if (!/^\d+$/.test(answer) || !selected?.online) throw new Error('DEVICE_UNAVAILABLE');
    server = await gateway(probe, await bridgeKey());
    console.log(t('listening'));
    console.log(t('commands'));
    const status = () => {
      const snapshot = probe.snapshot();
      console.log(`${t('status')}: ${text.states[snapshot.state][index]} | ${snapshot.format.toUpperCase()} | ` +
        `${snapshot.bytesServed}/${snapshot.size} bytes | SHA-256 ${snapshot.sha256}` +
        (snapshot.error ? ` | ${snapshot.error}` : ''));
    };
    probe.on('status', status);
    timer = setInterval(() => probe.expire(), 1000);
    terminal.on('SIGINT', () => terminal.close());
    for (;;) {
      let input;
      try { input = (await terminal.question('RAP > ')).trim(); }
      catch { break; }
      if (input === 'exit') break;
      try {
        if (input === 'status') { status(); continue; }
        if (input === 'stop') { await stop(); continue; }
        if (input === 'play') {
          if (probe.current) await stop();
          const attempt = probe.begin(selected.id);
          attempted = true;
          stopped = false;
          try { await account.start(selected.id, attempt.code, language); }
          catch (error) { probe.revoke('failed', safeError(error)); throw error; }
        } else if (input === 'pause' && probe.current?.state === 'playing') {
          await account.transport(selected.id, 'pause');
        } else if (input === 'resume' && probe.current?.state === 'paused') {
          await account.transport(selected.id, 'play');
        } else { console.log(t('invalid')); continue; }
        console.log(t('received'));
      } catch (error) { console.log(t('error'), safeError(error)); }
    }
  } finally {
    if (timer) clearInterval(timer);
    terminal?.close();
    // Revoke even if Amazon is unreachable. No automatic restore of another app.
    if (attempted && !stopped) {
      try { await stop(); } catch { console.error(t('error'), 'AMAZON_STOP_FAILED'); }
    }
    account?.close();
    await probe.close();
    if (server) await new Promise(resolve => { server.close(resolve); server.closeAllConnections(); });
  }
}

main().catch(error => { console.error(t('error'), safeError(error)); process.exitCode = 1; });
