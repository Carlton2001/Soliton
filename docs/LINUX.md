# Soliton sous Linux

Soliton fonctionne sous Linux : bibliothèque, lecture locale, interface GPUI
Wayland/X11, contrôles média du bureau (MPRIS) et installation par utilisateur. Windows conserve son backend GPUI, WASAPI et ses
intégrations natives. L'[audit initial](PORTABILITE_LINUX.md) décrit les
obstacles et le périmètre du portage.

Fedora Workstation (GNOME, Wayland, PipeWire) est l'environnement de
développement et de validation. Les paquets RPM, .deb et Flatpak
(Silverblue compris) sont planifiés en phase 10 bis des spécifications.

## Fonctionnement

- **Fenêtre** : décorations clientes ; déplacement au clic gauche depuis la barre
  de titre et le logo, double-clic pour agrandir/restaurer, réduction, fermeture
  et redimensionnement par une marge transparente autour de la fenêtre, qui porte
  l'ombre portée et le curseur de redimensionnement des bords libres. Taille et
  état agrandi restaurés au démarrage (GPUI n'applique pas l'état agrandi à
  l'ouverture sous Linux). Sous Wayland, le compositeur choisit la position et
  GPUI ne connaît aucun écran avant l'ouverture : la taille enregistrée est alors
  reprise sans contrôle d'écran.
  Identifiant `io.github.Carlton2001.Soliton`, partagé par la fenêtre, l'entrée de
  bureau, l'icône et MPRIS.
- **Audio** : Symphonia → Rubato → flux PipeWire natif (`pipewire` 0.10, sans
  CPAL ni ALSA sous Linux), 32 bits flottants, mono ou stéréo au débit de la
  piste ; PipeWire convertit vers le format de la sortie. Les sorties sont les
  sinks PipeWire (nom du nœud persisté, description affichée, icône Bluetooth
  pour les appareils `bluez5`), suivis en direct par le registre : un casque
  appairé apparaît aussitôt, sans sondage. « Périphérique système par défaut »
  suit immédiatement la sortie choisie dans les paramètres Son ; si cette sortie
  disparaît en erreur (casque Bluetooth éteint), le flux est rouvert sur la
  nouvelle sortie par défaut sans pause. WirePlumber met toutefois en pause, par
  MPRIS, les lecteurs dont la sortie disparaît (`linking.pause-playback`, actif
  par défaut, Firefox compris) ; Soliton respecte ce réglage du système.
  `wpctl settings --save linking.pause-playback false` le désactive pour toutes
  les applications. Une sortie
  choisie est ciblée par `target.object` avec `node.dont-reconnect` et
  `node.dont-fallback` : PipeWire ne la remplace jamais. Sa disparition met la
  lecture en pause, avec un avertissement jusqu'à la reprise ; la reprise joue
  sur la sortie par défaut, puis Soliton revient sur la sortie choisie dès
  qu'elle réapparaît (casque rallumé). Le format affiché nomme la sortie réellement liée au flux.
  PulseAudio seul (sans PipeWire) n'est plus pris en charge.
- **MPRIS** (`org.mpris.MediaPlayer2.soliton`, suffixe `.instance<pid>` pour une
  seconde instance) : titre, artiste, album, durée, pochette du cache Soliton,
  état, position et `Seeked` ; lecture, pause, bascule, arrêt, précédent,
  suivant, `Seek`, `SetPosition`, `Raise` et `Quit`. Les touches multimédias de
  GNOME et son panneau média passent par ce service. Aucune URI ni fichier hors
  bibliothèque n'est accepté. Sans bus de session, la lecture continue et un
  avertissement est affiché.
- **Accent système** : lu par le portail XDG Settings
  (`org.freedesktop.appearance`, clé `accent-color`, GNOME 47+ et KDE Plasma,
  Flatpak compris) sur le fil d'intégration, relu chaque seconde. Les variantes
  claire et sombre gardent la teinte et plafonnent la luminosité OKLab à 0,5
  (thème clair) ou la portent à 0,85 (thème sombre), pour rester lisibles sous
  les libellés noirs ou blancs. Sans accent publié, l'accent bleu s'applique.
- **Fermeture** : GNOME n'a pas de zone de notification ; la fermeture quitte
  toujours Soliton par le nettoyage normal, sans masquage. Pour la même raison,
  aucun démarrage automatique n'est proposé : la fenêtre s'ouvrirait à chaque
  ouverture de session. La section d'intégration du bureau des paramètres est
  donc absente.
- **Icône** : quand Soliton a été installé par `install.sh` (marqueur
  `X-Soliton-Install=user` dans son entrée de bureau), la palette choisie remplace
  l'icône `~/.local/share/icons/hicolor/scalable/apps/io.github.Carlton2001.Soliton.svg`.
  Une entrée ou une icône installée autrement n'est jamais modifiée.
- **Indisponible sous Linux** : démarrage automatique, mises à jour intégrées
  (téléchargement et installateur Windows), Bluetooth dédié, notifications
  natives de sorties audio. Audio exclusif et Flatpak restent hors périmètre ; Google Cast
  est validé, DLNA est compilé mais non validé sous Linux.
- **Surveillance** : inotify sur chaque dossier des sources. Si la limite du
  système (`fs.inotify.max_user_watches`) empêche de surveiller toute une
  source, ses surveillances partielles sont retirées : elle n'est couverte que
  par les analyses (démarrage, toutes les 30 min, rafraîchissement), un
  avertissement est journalisé et la surveillance est retentée après chaque
  analyse. Une limite atteinte en cours de route (nouveau dossier) déclenche une
  analyse complète.
- **Sources** : un dossier démonté garde ses albums, signalés indisponibles et
  masqués par défaut ; remonté, il réapparaît en quelques secondes sans
  rafraîchissement (sonde des sources hors ligne toutes les 5 s).
- Bibliothèque, paramètres et caches dans `~/.local/share/Soliton`
  (`SOLITON_DATA_DIR` pour isoler un essai), jamais dans les sources musicales.

## Installer depuis l'archive

```sh
tar -xzf Soliton-0.1.0-x86_64-linux.tar.gz
cd Soliton-0.1.0-x86_64-linux
./install.sh
```

Installation pour l'utilisateur courant, sans root : `~/.local/bin/soliton`,
entrée de bureau, métadonnées AppStream, icône et licences dans
`~/.local/share`. Relancer `install.sh` d'une archive plus récente met à jour.
`./uninstall.sh` retire ces fichiers et conserve `~/.local/share/Soliton`. Il refuse d'agir sur une entrée de bureau
qu'il n'a pas installée.

Prérequis d'exécution : Vulkan, session Wayland ou X11, D-Bus et portail XDG
(choix des dossiers), PipeWire 0.3.49 ou plus récent. Le binaire est lié à
`libpipewire-0.3`, `libxcb` et `libxkbcommon`
(Vulkan, Wayland et Fontconfig sont chargés à l'exécution) ; construit sur
Fedora 44, il requiert la glibc de cette version ou plus récente.

## Préparer Fedora pour le développement

```sh
sudo dnf install gcc gcc-c++ make cmake pkgconf-pkg-config \
  pipewire-devel clang-devel libxkbcommon-devel libxkbcommon-x11-devel \
  freetype-devel fontconfig-devel libxcb-devel wayland-devel rustup
rustup-init -y --default-toolchain 1.98.1 --profile minimal --component rustfmt
. "$HOME/.cargo/env"
```

Avec rustup déjà présent : `rustup toolchain install 1.98.1 --profile minimal
--component rustfmt`. `clang-devel` sert à générer les liaisons PipeWire
(`bindgen`). Références Fedora :
[Rust](https://developer.fedoraproject.org/tech/languages/rust/rust-installation.html),
[PipeWire](https://packages.fedoraproject.org/pkgs/pipewire/pipewire-devel/),
[xkbcommon](https://packages.fedoraproject.org/pkgs/libxkbcommon/libxkbcommon-devel/),
[Wayland](https://packages.fedoraproject.org/pkgs/wayland/wayland-devel/).

## Compiler, tester, lancer

```sh
cargo +1.98.1 fmt --check
cargo +1.98.1 check --workspace --all-targets --locked
cargo +1.98.1 test --workspace --locked
export SOLITON_DATA_DIR="$HOME/.local/share/Soliton-dev"
RUST_LOG=info cargo +1.98.1 run -p soliton-app --bin Soliton --locked
```

`WAYLAND_DISPLAY= cargo run …` force X11 (XWayland). `Soliton-index` vérifie
l'indexation sans fenêtre :

```sh
cargo +1.98.1 run -p soliton-app --bin Soliton-index --locked -- \
  "$SOLITON_DATA_DIR" /chemin/vers/une/collection-de-test
```

Tests explicites sur une session de bureau (volume nul pour l'audio) :

```sh
cargo +1.98.1 test -p soliton-audio native_output -- --ignored
cargo +1.98.1 test -p soliton-audio unavailable_preference -- --ignored
cargo +1.98.1 test -p soliton-audio chosen_sink -- --ignored
cargo +1.98.1 test -p soliton-audio engine_pauses -- --ignored
cargo +1.98.1 test -p soliton-audio default_following -- --ignored
cargo +1.98.1 test -p soliton-audio default_output_loss -- --ignored
cargo +1.98.1 test -p soliton-platform session_bus -- --ignored
cargo +1.98.1 test -p soliton-library watch_limit -- --ignored
cargo +1.98.1 test -p soliton-app remounted_source -- --ignored
```

`chosen_sink`, `engine_pauses` et `default_following` créent un sink PipeWire
muet temporaire. `default_following` en fait brièvement la sortie par défaut du
système (`pw-metadata`, paquet `pipewire-utils`) : les autres applications y
passent pendant environ une seconde, puis la sortie configurée est rétablie,
même en cas d'échec. `watch_limit` se relance dans un espace de noms
utilisateur (`unshare`) dont la limite inotify est abaissée, sans toucher à
celle du système. `remounted_source` renomme puis rétablit un dossier source
temporaire et attend son retour sans analyse manuelle (environ 5 s).

## Construire l'archive

```sh
RUSTUP_TOOLCHAIN=1.98.1 scripts/build-linux.sh
```

Le script vérifie l'inventaire des licences (`scripts/check-third-party-licenses.sh`,
mêmes empreintes que la version PowerShell), exige une entrée `<release>` de la
version courante dans les métadonnées AppStream, construit en release, assemble
`target/distribution/output/Soliton-<version>-x86_64-linux.tar.gz`, contrôle son
contenu (binaire sans symboles) et écrit son empreinte SHA-256. `--skip-build` réutilise le binaire release
existant. Les sources de l'entrée de bureau, des métadonnées et des scripts sont
dans `installer/linux/`.

## Construire le RPM

```sh
RUSTUP_TOOLCHAIN=1.98.1 scripts/build-rpm.sh
```

Prérequis : `sudo dnf install rpm-build desktop-file-utils appstream`. Le script
construit d'abord l'archive (`--skip-build` est transmis), puis le paquet
`target/distribution/output/soliton-<version>-1.fc44.x86_64.rpm` et son empreinte
SHA-256 à partir de `installer/linux/rpm/soliton.spec`. Le binaire est celui de
l'archive, construit sur Fedora 44 : le paquet vise Fedora 44 et suivantes.

Le paquet installe le binaire, l'entrée de bureau, les métadonnées AppStream et
l'icône sous `/usr`, les licences sous `/usr/share/licenses/soliton`. L'entrée
système ne porte pas le marqueur de `install.sh` : son icône garde la palette
par défaut. Les données restent dans `~/.local/share/Soliton`, partagées avec
l'archive et conservées par `dnf remove`. Si l'archive est aussi installée,
son entrée dans `~/.local/share/applications` masque celle du paquet.

Les deux entrées déclarent `StartupNotify=false` : GPUI ne consomme pas le jeton
de démarrage (`XDG_ACTIVATION_TOKEN`, `DESKTOP_STARTUP_ID`), et GNOME garderait
le curseur d'attente jusqu'à son délai d'expiration.

Le paquet déclare `vulkan-loader`, `mesa-vulkan-drivers` et
`libwayland-client`, chargés à l'exécution ; les autres bibliothèques sont
détectées par `rpmbuild`. Le champ `License` est calculé par
`scripts/rpm-license.py` à partir des crates liées sous Linux (`cargo metadata`,
dépendances de construction et de développement exclues) et des ressources
embarquées (Inter, Lucide). La date de `%changelog` est celle du dernier commit et
les notes de version restent dans les métadonnées AppStream.

## Construire le .deb

```sh
scripts/build-deb.sh
```

Le binaire du paquet est compilé sous Ubuntu 24.04 (glibc 2.39) : le paquet vise
Ubuntu 24.04 et suivantes, Linux Mint 22 et Debian 13. Hors d'Ubuntu 24.04, le
script se relance dans un conteneur `podman` construit depuis
`installer/linux/deb/Containerfile` (Rust 1.98.1, outils Debian), avec son propre
dossier `target/ubuntu-24.04` et un volume `soliton-ubuntu-cargo` pour le cache
des crates. Il produit `target/distribution/output/soliton_<version>-1_amd64.deb`
et son empreinte SHA-256, puis le vérifie avec `lintian`.

Disposition et données identiques au RPM ; les licences sont dans
`/usr/share/doc/soliton` (`copyright` au format Debian, notices des crates).
`dpkg-shlibdeps` détecte les bibliothèques liées ; `libvulkan1`,
`mesa-vulkan-drivers` et `libwayland-client0`, chargées à l'exécution, sont
déclarées, ainsi que `pipewire-bin` (qui fournit `client.conf`, sans lequel le
client PipeWire échoue) et une police système (`fonts-dejavu-core` ou
équivalent) pour les caractères absents d'Inter et de JetBrains Mono, toutes deux
embarquées. Le RPM déclare `default-fonts-core-sans` pour la même raison. Le champ `Maintainer` vaut `carlton2001@gmail.com`, ou
`SOLITON_MAINTAINER`.

## Construire le Flatpak

```sh
scripts/build-flatpak.sh
```

Prérequis : `flatpak-builder` (paquet Fedora) et, pour `flatpak-builder-lint`,
`flatpak install --user flathub org.flatpak.Builder` ; le script installe le SDK
Freedesktop 26.08 et ses extensions `rust-stable` et `llvm22` (libclang pour les
liaisons PipeWire). La compilation suit le manifeste
`installer/linux/flatpak/io.github.Carlton2001.Soliton.yml`, depuis le dernier
commit de `main` et hors ligne : `scripts/flatpak-cargo-sources.py` régénère
`cargo-sources.json` depuis `Cargo.lock` (archives crates.io vérifiées par leur
empreinte, format de `flatpak-cargo-generator`). Le script produit
`target/distribution/output/Soliton-<version>-x86_64.flatpak` et passe
`flatpak-builder-lint`.

Pour Flathub, `scripts/flathub-manifest.sh vX.Y.Z` écrit dans `target/flathub`
une copie du manifeste dont la source est le dépôt GitHub à ce tag et son commit,
avec `cargo-sources.json` ; la release les joint aussi. La soumission (pull request
sur `flathub/flathub`) est faite par l'auteur lui-même : Flathub refuse les
soumissions produites par une IA. Les URL des captures d'écran de la metainfo
pointent vers le tag ; les scripts de montée de version les mettent à jour.

Permissions minimales : Wayland (X11 en secours), `dri` pour Vulkan, socket
PipeWire seule pour l'audio, réseau pour Cast et DLNA, nom MPRIS. Fichiers en
lecture seule : dossier Musique, `/run/media`, `/media` et `/mnt`. Le portail
renvoie toujours un chemin du portail de documents ; quand le dossier est lisible
directement, Soliton le remplace par son chemin réel (`GetHostPaths`), ce qui
rétablit notifications et disponibilité des sources. Un autre dossier passe
par le portail de documents : indexé et lu, mais ses ajouts ne sont vus qu'à
l'analyse périodique (30 min) ou au redémarrage, et son indisponibilité n'est pas
détectée. Pour l'éviter, l'utilisateur peut élargir l'accès :
`flatpak override --user --filesystem=home:ro io.github.Carlton2001.Soliton`.
Les données sont dans `~/.var/app/io.github.Carlton2001.Soliton/data/Soliton`,
séparées des installations natives.

Après un changement de dépendances, régénérer les notices avant de construire
(cargo-about 0.9.2 compilé une fois depuis crates.io ; Python 3 est présent sur
Fedora) :

```sh
cargo +1.98.1 install cargo-about --version 0.9.2 --locked --features cli
RUSTUP_TOOLCHAIN=1.98.1 scripts/update-third-party-licenses.sh
```

Le script produit les mêmes notices et le même inventaire que
`scripts/update-third-party-licenses.ps1` ; l'inventaire couvre toutes les cibles.

## Validation

Le 26 octobre 2026, Fedora 44 Workstation, GNOME 50 Wayland, PipeWire 1.6, Rust
1.98.1, dans une machine virtuelle VMware (rendu Vulkan logiciel llvmpipe) :

| Vérification | Résultat |
| --- | --- |
| Format, `cargo check --workspace --all-targets --locked` sans avertissement | Réussis |
| Suite de tests du workspace | 135 réussis, 5 ignorés par défaut |
| Tests natifs explicites : deux tests audio, MPRIS sur le bus de session | Réussis |
| `soliton-platform` pour `x86_64-pc-windows-msvc` (`cargo check`, tests compris) | Réussi |
| Lecture/pause/seek/suivant/arrêt CPAL → ALSA → PipeWire, volume nul | Réussie |
| MPRIS sur le bus de session : propriétés, `PlayPause`, `SetPosition` (test) | Réussi |
| MPRIS depuis `gdbus` sur l'application : métadonnées et pochette, lecture, suivant, seek relatif, pause | Réussi |
| Fenêtre Wayland et X11 : rendu, échelle 2×, polices, contrôles Linux | Réussis |
| Restauration de l'état agrandi (X11) | Réussie après correctif |
| Indexation : FLAC 16/44,1 et 24/96 regroupés en deux éditions, MP3 multidisque, compilation WAV, pochettes | Réussie |
| Absence de flux PipeWire éphémères et de messages ALSA après la politique de sortie | Vérifiée |
| Archive release : installation dans un `HOME` isolé, entrée de bureau validée, lancement, icône suivant la palette, désinstallation conservant les données, refus d'une entrée étrangère | Réussis |
| AppStream (`appstreamcli validate`) et entrée de bureau (`desktop-file-validate`) | Valides ; remarque pédante sur les majuscules de l'identifiant |

Le 28 octobre 2026, sur matériel réel (Fedora 44 Workstation, GNOME Wayland,
Intel UHD Raptor Lake via Mesa, PipeWire 1.6.10, Rust 1.98.1) :

| Vérification | Résultat |
| --- | --- |
| Format, `cargo check` et `cargo clippy` du workspace sans avertissement | Réussis |
| Suite de tests du workspace | Réussie, 11 tests natifs ignorés par défaut |
| Tests natifs : lecture/pause/seek/suivant/arrêt et sélection explicite sur PipeWire, préférence indisponible sans repli, MPRIS | Réussis |
| Test natif : sink temporaire choisi, lié, puis retiré ; échec signalé sans déplacement vers une autre sortie | Réussi |
| Test natif du moteur : pause à la perte du sink choisi, reprise sur la sortie par défaut, retour au sink recréé | Réussi |
| `soliton-audio` et `soliton-library` pour `x86_64-pc-windows-msvc` (`cargo xwin check`, tests compris) | Réussi |
| Restauration de la taille et de l'état agrandi sous Wayland | Réussie après correctif |
| Binaire lié à `libpipewire-0.3`, plus à `libasound` (`ldd`) | Vérifié |
| Choix d'un casque Bluetooth dans Soliton, son extinction (pause, avertissement, reprise sur la sortie par défaut) et son retour | Réussis (essai manuel) |
| Démontage d'une source : albums indisponibles ; remontage : retour sans rafraîchissement (test explicite et correctif) | Réussis |
| Notices régénérées par `update-third-party-licenses.sh`, identiques octet pour octet à la génération Windows sur le même verrou | Vérifié |
| `build-linux.sh` : vérification des licences, build release, archive et empreinte | Réussi |
| Rendu GPUI accéléré : Blade sur l'adaptateur Vulkan « Intel(R) Graphics (RPL-S) », présentation MAILBOX, sans avertissement | Vérifié |
| Test natif : lecture sans sortie choisie déplacée vers une nouvelle sortie par défaut du système puis ramenée, sans pause ni repli, sortie affichée suivie | Réussi |
| Limite inotify atteinte (espace de noms utilisateur, limite 8) : surveillances partielles retirées, quota rendu à une autre source (test et correctif) | Réussis |
| Interactions de la barre de titre (déplacement, double-clic, boutons) et redimensionnement sous Wayland | Réussis (essais manuels) |
| Panneau média et touches multimédias GNOME | Réussis (essais manuels) |
| Portail de choix de dossiers à l'ajout d'une source | Réussi (essai manuel) |
| Icône du lanceur suivant la palette (copie installée) | Réussie (essai manuel) |
| Sortie par défaut : casque Bluetooth allumé, la lecture le suit ; casque éteint, session rouverte sur la nouvelle sortie par défaut (« Format envoyé » mis à jour), puis pause demandée par WirePlumber (`linking.pause-playback`, observé par `dbus-monitor`) | Réussi (essai manuel) ; pause du système respectée |
| Test natif du moteur : perte de la sortie par défaut, session rouverte sur la nouvelle sans pause ni avertissement, position gardée | Réussi |
| Collection réelle de 7 206 pistes sur NTFS (`ntfs-3g`, 99 dossiers) : 372 surveillances inotify utilisées sur tout le système pour une limite de 557 977 | Vérifié |
| Google Cast : découverte et lecture sur un appareil Cast | Réussi (essai manuel) |
| Colonnes de la grille ajustées au redimensionnement, marge de redimensionnement comprise | Réussi (essai manuel) |

Le 28 octobre 2026, paquet RPM dans un conteneur `fedora:44` minimal :

| Vérification | Résultat |
| --- | --- |
| `dnf install` du paquet : dépendances résolues, aucune bibliothèque manquante (`ldd`) | Réussi |
| `rpmlint` 2.8.0 | Aucune erreur ; absence de page de manuel et de documentation signalée |
| `dnf remove` : fichiers retirés, `~/.local/share/Soliton` conservé | Réussi |
| Paquet installé sur la machine de développement (Fedora 44, GNOME Wayland) : lanceur, icône, fenêtre associée au dock, panneau média, bibliothèque existante retrouvée | Réussi (essai manuel) ; avertissement attendu de `dnf` pour un paquet local non signé |
| Lancement depuis la vue des applications avec `StartupNotify=false` : plus de curseur d'attente, fenêtre au premier plan | Réussi (essai manuel) |
| Chiffres seuls en JetBrains Mono embarquée (numéros, durées, lecteur, curseurs), infobulles en Inter : aucun chiffre tronqué | Réussi (essai manuel, 29 octobre 2026) |

Le 28 octobre 2026, paquet .deb dans des conteneurs `ubuntu:24.04`, `debian:13`
et `ubuntu:26.04` minimaux :

| Vérification | Résultat |
| --- | --- |
| Compilation release sous Ubuntu 24.04 (PipeWire 1.0) | Réussie ; binaire limité à glibc 2.39 |
| `apt install` du paquet : dépendances résolues, aucune bibliothèque manquante (`ldd`), Vulkan et Wayland présents | Réussi sur les trois |
| `lintian` | Aucune erreur ; absence de page de manuel et de bogue clos signalée |
| Paquet dans une toolbox Ubuntu 24.04 vierge : `client.conf` de PipeWire et police absents, client audio en échec puis arrêt de GPUI | Corrigé : `pipewire-bin` déclaré, infobulles en Inter ; démarrage vérifié sans aucune police système (conteneur Wayland) |
| Paquet corrigé dans la toolbox Ubuntu 24.04 (binaire Ubuntu vérifié par `/proc/<pid>/root`) : rendu, lecture audio, panneau média, bibliothèque retrouvée | Réussi (essai manuel, 29 octobre 2026) ; intégration au bureau Ubuntu (lanceur, dock) non vérifiée |
| `apt purge` : fichiers retirés, `~/.local/share/Soliton` conservé | Réussi sur les trois |

Le 29 octobre 2026, Flatpak installé sur la machine de développement, sans
permission de fichiers (sources accordées par le portail) :

| Vérification | Résultat |
| --- | --- |
| Rendu, polices, lecture audio, panneau média et pochette, lanceur | Réussis (essai manuel) ; icône fixe, comme pour le RPM |
| Ajout d'une source par le portail et indexation | Réussi (essai manuel) |
| Ajout de fichiers pendant l'exécution | Non détecté avant le redémarrage : pas de notification inotify sur le portail de documents |
| Disque de la source débranché | Entrées conservées, mais source non signalée indisponible : erreur à la lecture |

Le 29 octobre 2026, Flatpak reconstruit avec les permissions actuelles (Musique,
`/run/media`, `/media`, `/mnt` en lecture seule) :

| Vérification | Résultat |
| --- | --- |
| Ajout par le portail d'un dossier d'un disque monté (`/run/media/…/_FLAC`) | Réussi (essai manuel) : source enregistrée sous son chemin réel, plus sous `/run/flatpak/doc` |
| Casque Bluetooth : choix dans Soliton et lecture | Réussi (essai manuel) |

Les essais WSL précédents (Windows MSVC et Fedora 44 WSL, mêmes tests et
lancements temporisés) restent valables pour Windows.
