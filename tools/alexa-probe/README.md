# Prototype Wi-Fi Echo Studio / Echo Studio Wi-Fi probe

**Archive — piste abandonnée le 21 octobre 2026.** RAP utilise désormais le
parcours Bluetooth natif Windows pour ces appareils. Ne pas suivre ce guide
pour ajouter une sortie Bluetooth : ni connexion Amazon ni tunnel ne sont requis.
**Archived:** the Wi-Fi approach was abandoned in favor of native Windows
Bluetooth pairing and local audio output selection. This tool is not needed.

Outil de validation **expérimental**, séparé de l'application RAP. Sélection
d'une Echo du compte Amazon en ligne de commande, puis essai d'un seul FLAC ou
WAV en lecture seule, sans transcodage. Aucun matériel supplémentaire requis.
Le sélecteur graphique de RAP n'est pas encore raccordé : l'essai sur le Studio
doit d'abord confirmer la faisabilité. Une réussite HTTP ne prouve pas une
lecture réussie, ni la qualité du traitement interne de l'enceinte.

**Ce guide s'adresse au développeur du prototype.** Une fonctionnalité Alexa
distribuée dans RAP ne doit demander aux utilisateurs finaux ni inscription
Amazon Developer/AWS, ni déploiement de skill, ni gestion manuelle d'un tunnel.
Une skill commune et son infrastructure gérée demandent une conception et une
validation supplémentaires ; la configuration mono-utilisateur ci-dessous ne
constitue pas un service prêt à publier pour plusieurs utilisateurs.

## Installation Windows

Node.js 22 ou supérieur. Depuis ce dossier :

```powershell
npm.cmd ci --ignore-scripts --no-audit --no-fund
npm.cmd test
npm.cmd run login
```

Ouvrir l'adresse locale affichée (`http://127.0.0.1:8767/`) dans le navigateur
et se connecter au compte Amazon du Studio, avec la double authentification
habituelle. Ce proxy d'authentification non officiel écoute uniquement sur
`127.0.0.1`. Ne jamais l'exposer par le tunnel. Aucun mot de passe ou code MFA
n'est demandé dans le terminal ou le chat. La session est enregistrée dans
`%LOCALAPPDATA%\RAP\alexa-probe`, hors de la bibliothèque et du dépôt. Ce dossier
contient des jetons sensibles : ne pas le partager. Fermer le programme de
connexion après l'authentification ; il s'arrête normalement seul.

Le mode français utilise `amazon.fr`, l'anglais `amazon.com`. Les autres régions
ne sont pas encore proposées. Ajouter `-- --lang en` aux commandes npm pour
l'interface anglaise et les enceintes configurées en anglais américain.

## HTTPS sans domaine : tunnel temporaire

Depuis la racine du dépôt, dans un terminal séparé, lancer :

```powershell
.\scripts\start-alexa-tunnel.ps1
```

Le script utilise le binaire Windows x64 officiel `cloudflared` 2026.10.3,
téléchargé dans `target/tools` avec vérification SHA-256. Il n'installe aucun
programme système. Il démarre un tunnel **sur le port du média uniquement**.
Pour un téléchargement sans démarrer le tunnel, utiliser `-DownloadOnly`.

Avec une installation existante de `cloudflared`, l'équivalent est :

```powershell
cloudflared tunnel --url http://127.0.0.1:8766
```

Conserver l'URL `https://….trycloudflare.com` affichée et laisser le terminal
ouvert pendant le test. Le serveur de média sera lancé à l'étape suivante ; des
erreurs de connexion avant son démarrage sont normales. Aucun domaine ni compte
Cloudflare n'est requis pour ce [Quick Tunnel](https://developers.cloudflare.com/tunnel/get-started/quick-tunnels/).
L'URL change au redémarrage. Ne pas activer une authentification interactive
Cloudflare : l'Echo et Lambda ne peuvent pas remplir un formulaire de connexion.

Dans cette configuration de test, les octets audio transitent via Cloudflare et
Internet, sans réencodage par le prototype. Ce n'est pas un transport purement
local. L'URL du média contient une capacité aléatoire valable au plus 30 minutes,
révoquée à l'arrêt/remplacement ; les routes de la skill exigent une clé séparée.
Aucun dossier musical n'est parcouru ou publié. Aucun tunnel n'est lancé par RAP.

## Créer la skill de développement, sans publication

1. Se connecter à la [console développeur Alexa](https://developer.amazon.com/alexa/console/ask)
   avec le compte du Studio. Créer une skill française, expérience « Music &
   Audio », modèle **Custom**, hébergement **Provision your own**, à partir de
   zéro. Relever son identifiant `amzn1.ask.skill.…`.
2. Dans **Interfaces**, activer **AudioPlayer**, puis enregistrer. Dans l'éditeur
   JSON de l'interaction model, importer [`skill/fr-FR.json`](skill/fr-FR.json),
   enregistrer et construire le modèle. L'invocation est « rap musique ».
3. Dans AWS, créer une fonction Lambda Node.js 22 en région Europe (Irlande),
   coller [`skill/index.mjs`](skill/index.mjs) dans **index.mjs**, définir le
   handler **index.handler**, puis déployer le code. Ne créer ni Function URL
   ni API Gateway. Régler le timeout Lambda à 8 secondes.
4. Ajouter à la Lambda le déclencheur **Alexa Skills Kit** en activant la
   vérification du Skill ID et en renseignant exactement l'identifiant de
   l'étape 1. Dans la skill, configurer l'endpoint Europe avec l'ARN de cette
   Lambda, puis enregistrer.
5. Activer le test en mode **Development**. Dans le simulateur, invoquer
   « ouvre rap musique ». Dans le JSON de la requête reçue, relever
   `context.System.user.userId` (ce n'est ni l'adresse e-mail ni le numéro de
   série du Studio). La première réponse échoue tant que la configuration
   Lambda n'est pas complète : c'est attendu. Ne pas partager cet identifiant.
6. Dans le terminal local, exécuter `node src/cli.mjs setup`. Copier la clé
   affichée uniquement dans les variables d'environnement de la Lambda :

   | Variable | Valeur |
   | --- | --- |
   | `RAP_ALEXA_PUBLIC_URL` | URL HTTPS du tunnel, sans slash final |
   | `RAP_ALEXA_BRIDGE_KEY` | Clé produite par `setup` |
   | `RAP_ALEXA_SKILL_ID` | Identifiant exact de cette skill |
   | `RAP_ALEXA_USER_ID` | Identifiant de l'utilisateur obtenu à l'étape 5 |

La Lambda vérifie l'application et l'utilisateur avant de contacter la
passerelle. Les événements sont associés à l'identifiant opaque du Studio, au
jeton de lecture et à la tentative en cours. La commande initiale transporte un
code de corrélation à six chiffres, expirant après 45 secondes. Le mode
développement doit être activé sur le compte associé au Studio.

Pour l'anglais américain, utiliser [`skill/en-US.json`](skill/en-US.json),
« rap music », `amazon.com` et `--lang en`.

## Premier essai sans toucher à la musique

Depuis la racine du dépôt, générer des signaux de cinq secondes à faible niveau
avec FFmpeg (outil de développement uniquement, aucune conversion pendant la
diffusion). Le script n'écrase pas les fichiers déjà présents :

```powershell
.\scripts\generate-alexa-fixtures.ps1
```

Puis depuis ce dossier, remplacer l'URL d'exemple par celle du tunnel :

```powershell
npm.cmd run probe -- --file ../../target/alexa-fixtures/tone-16-44100.flac --public-url https://exemple.trycloudflare.com
```

Choisir le numéro du Studio dans la liste. Les noms identiques restent des choix
distincts ; les commandes utilisent le numéro de série exact. Les appareils
hors ligne sont signalés et les groupes sont exclus. La liste provient du compte
Amazon : elle ne prouve pas que toutes les enceintes sont sur le même Wi-Fi.
Les appareils sans capacité de lecture autonome, dont l'Echo Sub, sont exclus.
L'appartenance des enceintes à un groupe existant est affichée ; le prototype
ne modifie pas cette association et ne garantit pas le routage d'une paire.

Régler le volume sur l'enceinte avant le test. Entrer **play**. La commande
demande à la skill de lancer le test sur ce Studio. Attendre « lecture confirmée
par Alexa » : « commande envoyée » et « attente du démarrage » ne sont pas un
succès. Si la skill ne s'exécute pas ou si le format est refusé, le programme
affiche un échec ou expire après 45 secondes. Aucun repli MP3/AAC n'est effectué.

Commandes disponibles :

- `status` : état, octets servis, format et SHA-256 du fichier source.
- `pause` et `resume` : commandes sur l'enceinte choisie, confirmations par les
  événements de la skill quand ils sont reçus.
- `stop` : révocation immédiate de l'URL et demande d'arrêt à l'enceinte.
- `exit` : arrêt et fermeture des connexions locales.

Si Amazon est injoignable, l'arrêt distant peut échouer ; l'accès au fichier est
quand même révoqué. Un éventuel tampon déjà chargé dans l'Echo peut continuer
brièvement. Ne pas interpréter les octets servis comme un total audio unique :
les reprises et les requêtes Range peuvent relire les mêmes plages.

Si le premier FLAC est lu et audible, recommencer avec `tone-24-96000.flac` puis
le WAV, en quittant et relançant le prototype pour changer de fichier. Noter le
format, les événements et les éventuels refus. Même une lecture confirmée ne
mesure pas le rééchantillonnage ou les traitements internes du Studio. Si les
FLAC sont refusés, ne pas annoncer une solution sans perte opérationnelle.

À la fin, fermer également le terminal du tunnel. Il n'est pas installé comme
service Windows. Aucune publication de skill ni modification de pare-feu n'est
effectuée par le prototype.

## Tests et limites

`npm.cmd test` utilise un compte et des événements simulés ainsi qu'un vrai
serveur HTTP sur loopback. Les tests contrôlent les octets et SHA-256, Range et
HEAD, la révocation, les fichiers modifiés, les délais, les événements d'un autre
appareil ou d'une ancienne tentative, et les vérifications d'identité Lambda.
Ils ne se connectent pas à Amazon et ne sollicitent aucune enceinte.

Node.js est réservé à cette preuve de faisabilité. Aucune dépendance ni sortie
Alexa n'est ajoutée à l'application Rust tant que l'essai matériel n'est pas
concluant. Linux et la connexion Amazon réelle ne sont pas validés par ces tests.
`alexa-remote2` 8.1.1 (MIT), épinglé dans le lockfile, reste une API non officielle.
Les sessions peuvent expirer et les changements Amazon casser le pilotage.

L'audit npm a conduit à remplacer la dépendance transitive `cookie` 0.6 par
0.7.2. Il reste un avis élevé sur `braces`, sans version corrigée publiée : son
chemin via `micromatch` compile le motif fixe `!/cookie-success` du proxy
d'`alexa-cookie2`, sans motif provenant des requêtes. Le proxy d'authentification est de
plus limité au loopback. Cette analyse est limitée à la configuration et aux
versions épinglées ; ce prototype n'est pas destiné à un déploiement public
général. Le port public du média n'utilise aucune de ces dépendances HTTP.

Validation locale du 21 octobre 2026 sous Windows : 12 tests automatisés, puis
3 contrôles de transport sur de vrais fichiers synthétiques (FLAC 16/44,1,
FLAC 24/96 et WAV 16/44,1), tous réussis. Syntaxe JavaScript/JSON/PowerShell et
empreinte du binaire du tunnel vérifiées. L'audit npm n'est pas vierge : un avis
racine `braces` remonte cinq entrées de sévérité élevée dans l'arbre transitoire,
avec la portée décrite ci-dessus. Aucun test sur le Studio n'a encore eu lieu.
Les commandes Cargo n'ont pas été relancées : aucun Rust ni manifeste Cargo
n'a été modifié.

La connexion Amazon réelle et l'énumération des enceintes ont ensuite réussi
sous Windows le même jour. Cela valide l'authentification et la récupération
des capacités, pas encore le déclenchement de la skill ou la lecture audio.

Après autorisation explicite de l'utilisateur pour ces seuls fichiers, les
trois signaux synthétiques ont aussi été vérifiés à travers un vrai Quick
Tunnel Cloudflare : HTTPS, SHA-256 identique, requêtes Range et révocation après
arrêt, tous réussis. Le contrôle opt-in `tests/public-smoke.mjs --public-url
<URL>` est séparé des tests locaux et ne commande aucune enceinte. Il n'accepte
aucun chemin de fichier arbitraire : seuls les trois signaux générés dans
`target/alexa-fixtures` sont servis pendant sa durée d'exécution. Ce résultat
ne valide pas encore les formats acceptés par le Studio.

## English quick start

This is a separate experimental CLI, not yet a RAP GUI output. Install Node.js
22+, run `npm.cmd ci --ignore-scripts --no-audit --no-fund`, then
`npm.cmd run login -- --lang en` and sign in using the loopback browser proxy.
Run `scripts/start-alexa-tunnel.ps1` from the repository root (official binary,
SHA-256 checked, no system install). It creates a temporary Cloudflare tunnel
pointing **only** to `127.0.0.1:8766` for
HTTPS, deploy `skill/index.mjs` to AWS Lambda with a restricted Alexa Skills Kit
trigger, and import `skill/en-US.json` into a development custom skill. Set the
four Lambda environment variables listed above. The first simulator invocation
provides the request's `context.System.user.userId`; failure before configuring
the variables is expected. Keep all secrets outside chat and version control.

Generate the synthetic fixtures with `scripts/generate-alexa-fixtures.ps1`, then
run `npm.cmd run probe -- --lang en --file <fixture.flac> --public-url <tunnel-url>`.
Choose the speaker number and enter `play`. Only an Alexa `PlaybackStarted`
event confirms playback. Commands are `play`, `pause`, `resume`, `stop`,
`status`, `exit`. Original FLAC/WAV bytes are served unchanged; no lossy fallback
is implemented. Actual Echo Studio format support and internal processing remain
unverified. The temporary tunnel carries audio over the internet. Stop the probe
and tunnel after testing. French instructions above provide the complete setup.
