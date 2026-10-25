# Distribution et mises à jour / Distribution and updates

Les scripts et workflows sont adaptés d'Oxyde-GPUI, Copyright (C) 2026
Carlton2001, sous GPL-3.0. Ils ne dépendent pas de son checkout.

## Destination des mises à jour

Le dépôt de référence est `Carlton2001/Soliton`. Les builds locaux utilisent cette
destination par défaut. Les workflows passent automatiquement `github.repository`
au build via `SOLITON_UPDATE_REPOSITORY`. Pour un build local visant un autre dépôt :

```powershell
$env:SOLITON_UPDATE_REPOSITORY = 'proprietaire/depot'
cargo run -p soliton-app
```

Cette variable se définit **à la compilation** ; la modifier déclenche la
recompilation du binaire. Tant qu'aucune release stable avec `latest.json` n'est
publiée, la vérification ne peut pas proposer de mise à jour. Ne pas employer le
dépôt Oxyde.

Soliton vérifie une fois au démarrage, si l'option est activée, et sur demande dans
la dernière section des paramètres. La requête WinHTTP s'exécute sur un worker,
avec délais réseau et réponse limitée à 64 Kio. Les erreurs restent visibles
dans les paramètres et la vérification peut être relancée.

`latest.json`, joint à la dernière release stable publiée, contient une version
numérique `major.minor.patch`. Les versions égales ou plus anciennes ne sont pas
proposées. Le manifeste contient l'URL exacte de l'installateur Soliton de cette
version dans le dépôt compilé et sa signature Minisign au format Tauri. Sur clic
explicite, Soliton télécharge au maximum 512 Mio, vérifie la signature avec la clé
publique compilée, puis lance Setup. Setup attend la fin du processus Soliton avant
de remplacer les fichiers et relance l'application. En cas d'échec, Soliton reste
ouvert et permet de réessayer. Les paquets ne sont pas signés Authenticode.

L'icône `info.svg` dans la sidebar apparaît uniquement lorsqu'une mise à jour
est disponible, avec la couleur d'accent. Son infobulle annonce la mise à jour et son clic
lance l'installation uniquement si une vraie release a été détectée. En mode
portable, il ouvre la page de téléchargement et indique de conserver `data`.

## Construire sous Windows x64

Rust MSVC 1.98.1, Build Tools C++ et SDK Windows sont requis. Pour l'installateur,
installer aussi Inno Setup 6.3 ou ultérieur.

```powershell
./scripts/install-cargo-about.ps1
./scripts/build-distribution.ps1
# Ou seulement l'archive portable :
./scripts/build-distribution.ps1 -PortableOnly
```

Les résultats sont dans `target/distribution/output/` : ZIP portable, Setup,
empreintes SHA-256. Le job de release ajoute la signature et `latest.json` après
signature de Setup. Les licences sont régénérées et vérifiées
avant assemblage. Le runtime C MSVC est lié statiquement ; le script refuse une
dépendance résiduelle aux DLL vcruntime/msvcp. `-SkipBuild` réassemble les binaires
existants, toujours avec actualisation des licences ; il faut avoir compilé
avec les mêmes options et la destination de mises à jour voulue.

Extraire le ZIP en entier. Son dossier `Soliton/data` active le mode portable ; les
données et caches restent à côté de l'exécutable. L'installation classique est
par utilisateur dans `%LOCALAPPDATA%/Programs/Soliton`, sans élévation, avec données
dans `%LOCALAPPDATA%/Soliton`. La désinstallation conserve ces données. `SOLITON_DATA_DIR`
reste prioritaire pour le développement. Les fichiers musicaux restent en lecture
seule dans les deux modes.

## GitHub

- `tests.yml` : formatage, check de toutes les cibles Rust et tests Windows,
  sur main et les pull requests. Aucun test matériel implicite.
- `security.yml` : audit RustSec Windows x64, sur modifications des dépendances,
  chaque lundi et manuellement. Les vulnérabilités ou crates retirées bloquent
  les releases ; les rapports sont conservés en artifacts.
- `distribution.yml` : réchauffement du cache release et assemblage sur changement
  des manifestes/scripts de distribution, ou déclenchement manuel.
- `release.yml` : sur tag `vX.Y.Z`, compare le tag au workspace, teste, audite,
  construit une fois, puis crée une release **en brouillon** contenant ces mêmes
  fichiers. La publier manuellement pour la rendre visible à la vérification.

Les actions tierces sont épinglées aux révisions reprises d'Oxyde. Les permissions
sont en lecture, sauf le job final de release. Les jobs réutilisables sont partagés
entre main, PR et tags. L'environnement GitHub `release` peut recevoir des règles
de protection. Configurer une paire de clés propre à Soliton, sans réutiliser celles
d'Oxyde. La variable **du dépôt** `SOLITON_UPDATE_PUBLIC_KEY` contient la clé publique
Tauri (texte Minisign encodé en base64) et est compilée dans les distributions.
Les secrets de l'environnement `release` `SOLITON_SIGNING_PRIVATE_KEY` et
`SOLITON_SIGNING_PRIVATE_KEY_PASSWORD` sont transmis au signer Tauri épinglé 2.10.1.
Le workflow refuse de publier sans clé/signature. Pour un build local, passer
aussi `SOLITON_UPDATE_PUBLIC_KEY` à la compilation. Aucune clé privée n'est embarquée.
La rotation de la clé doit préserver la capacité des anciennes versions à
vérifier la signature des nouveaux installateurs.

Le remote `origin` vise `https://github.com/Carlton2001/Soliton.git`. Autoriser
Actions et configurer les clés et éventuellement l'environnement `release` dans
ce dépôt. Préparer une version sur un arbre propre :

```powershell
./scripts/bump-version.ps1 -Version 0.2.0
```

Ce script modifie la version du workspace et Cargo.lock, crée un commit anglais et
un tag local ; il ne pousse rien. Toute opération de push reste manuelle et exige
une demande explicite de l'utilisateur dans une session Codex.

## English

Release builds get their update repository from `github.repository`. Local builds
default to `Carlton2001/Soliton`; set `SOLITON_UPDATE_REPOSITORY` before compilation to override it.
Checks run off the UI thread and can be disabled. Clicking the notification
downloads and verifies the signed installer before starting Setup and closing
Soliton cleanly. Setup waits for Soliton to exit and relaunches it. Portable builds open
the release page instead, keeping updates manual to preserve their data folder.
The sidebar icon is visible in the accent color only when an update is available.
Startup checks are enabled by default and respect the saved preference. Configure the repository public key
variable and the release signing secrets described above before publishing.

Run `scripts/build-distribution.ps1` on Windows x64 to build a per-user Inno Setup
installer and a portable ZIP with licenses and SHA-256 checksums. The portable
`data` directory keeps application data next to the executable; installed builds
use LocalAppData. Uninstall preserves user data. A `vX.Y.Z` tag runs tests, audit
and packaging before creating a draft release. Publish it manually to expose
`latest.json` to update checks. No push is performed by the version bump script.
