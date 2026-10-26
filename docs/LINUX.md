# Soliton sous Linux

Soliton fonctionne sous Linux : bibliothèque, lecture locale, interface GPUI
Wayland/X11, contrôles média du bureau (MPRIS), démarrage automatique et
installation par utilisateur. Windows conserve son backend GPUI, WASAPI et ses
intégrations natives. L'[audit initial](PORTABILITE_LINUX.md) décrit les
obstacles et le périmètre du portage.

Fedora Workstation (GNOME, Wayland, PipeWire) est l'environnement de
développement et de validation. Silverblue reste à vérifier sur un hôte réel.

## Fonctionnement

- **Fenêtre** : décorations clientes ; déplacement au clic gauche depuis la barre
  de titre et le logo, double-clic pour agrandir/restaurer, réduction, fermeture
  et redimensionnement par une marge transparente autour de la fenêtre, qui porte
  l'ombre portée et le curseur de redimensionnement des bords libres. État
  agrandi restauré au démarrage (GPUI ne l'applique pas à l'ouverture sous Linux).
  Identifiant `io.github.Carlton2001.Soliton`, partagé par la fenêtre, l'entrée de
  bureau, l'icône et MPRIS.
- **Audio** : Symphonia → Rubato → CPAL 0.16/ALSA `default` → PipeWire
  (`pipewire-alsa`) ou PulseAudio (`alsa-plugins-pulseaudio`). Seule la sortie
  système par défaut est proposée : le périphérique, Bluetooth compris, se choisit
  dans les paramètres Son. Les autres entrées CPAL étaient des alias de cette
  sortie ou des cartes ALSA brutes (`plughw`) contournant PipeWire, et leur
  énumération ouvrait chaque périphérique toutes les 5 s. La sélection native des
  sinks PipeWire est reportée.
- **MPRIS** (`org.mpris.MediaPlayer2.soliton`, suffixe `.instance<pid>` pour une
  seconde instance) : titre, artiste, album, durée, pochette du cache Soliton,
  état, position et `Seeked` ; lecture, pause, bascule, arrêt, précédent,
  suivant, `Seek`, `SetPosition`, `Raise` et `Quit`. Les touches multimédias de
  GNOME et son panneau média passent par ce service. Aucune URI ni fichier hors
  bibliothèque n'est accepté. Sans bus de session, la lecture continue et un
  avertissement est affiché.
- **Démarrage automatique** : entrée XDG
  `~/.config/autostart/io.github.Carlton2001.Soliton.desktop`, écrite ou retirée
  seulement par la case des paramètres. Elle désigne l'exécutable courant.
- **Fermeture** : GNOME n'a pas de zone de notification ; la fermeture quitte
  toujours Soliton par le nettoyage normal, sans masquage.
- **Icône** : quand Soliton a été installé par `install.sh` (marqueur
  `X-Soliton-Install=user` dans son entrée de bureau), la palette choisie remplace
  l'icône `~/.local/share/icons/hicolor/scalable/apps/io.github.Carlton2001.Soliton.svg`.
  Une entrée ou une icône installée autrement n'est jamais modifiée.
- **Indisponible sous Linux** : mises à jour intégrées (téléchargement et
  installateur Windows), Bluetooth dédié, notifications natives de sorties audio,
  accent système. Audio exclusif et Flatpak restent hors périmètre ; DLNA/Cast
  sont compilés mais non validés sous Linux.
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
`./uninstall.sh` retire ces fichiers et l'entrée de démarrage automatique, et
conserve `~/.local/share/Soliton`. Il refuse d'agir sur une entrée de bureau
qu'il n'a pas installée.

Prérequis d'exécution : Vulkan, session Wayland ou X11, D-Bus et portail XDG
(choix des dossiers), PipeWire avec `pipewire-alsa` ou PulseAudio avec sa
passerelle ALSA. Le binaire est lié à `libasound`, `libxcb` et `libxkbcommon`
(Vulkan, Wayland et Fontconfig sont chargés à l'exécution) ; construit sur
Fedora 44, il requiert la glibc de cette version ou plus récente.

## Préparer Fedora pour le développement

```sh
sudo dnf install gcc gcc-c++ make cmake pkgconf-pkg-config \
  alsa-lib-devel libxkbcommon-devel libxkbcommon-x11-devel \
  freetype-devel fontconfig-devel libxcb-devel wayland-devel rustup
rustup-init -y --default-toolchain 1.98.1 --profile minimal --component rustfmt
. "$HOME/.cargo/env"
```

Avec rustup déjà présent : `rustup toolchain install 1.98.1 --profile minimal
--component rustfmt`. Références Fedora :
[Rust](https://developer.fedoraproject.org/tech/languages/rust/rust-installation.html),
[ALSA](https://packages.fedoraproject.org/pkgs/alsa-lib/alsa-lib-devel/),
[xkbcommon](https://packages.fedoraproject.org/pkgs/libxkbcommon/libxkbcommon-devel/),
[Wayland](https://packages.fedoraproject.org/pkgs/wayland/wayland-devel/),
[pipewire-alsa](https://packages.fedoraproject.org/pkgs/pipewire/pipewire-alsa/).

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
cargo +1.98.1 test -p soliton-platform session_bus -- --ignored
```

## Construire l'archive

```sh
RUSTUP_TOOLCHAIN=1.98.1 scripts/build-linux.sh
```

Le script vérifie l'inventaire des licences (`scripts/check-third-party-licenses.sh`,
mêmes empreintes que la version PowerShell), exige une entrée `<release>` de la
version courante dans les métadonnées AppStream, construit en release, assemble
`target/distribution/output/Soliton-<version>-x86_64-linux.tar.gz`, contrôle son
contenu et écrit son empreinte SHA-256. `--skip-build` réutilise le binaire release
existant. Les sources de l'entrée de bureau, des métadonnées et des scripts sont
dans `installer/linux/`. La régénération des notices reste faite par
`scripts/update-third-party-licenses.ps1` ; elle couvre toutes les cibles, dont
Linux.

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

Les essais WSL précédents (Windows MSVC et Fedora 44 WSL, mêmes tests et
lancements temporisés) restent valables pour Windows.

À vérifier encore sur matériel réel : accélération GPU, interactions manuelles de
la barre de titre et du redimensionnement sous Wayland, panneau média et touches
multimédias GNOME, case de démarrage automatique et ouverture de session,
rafraîchissement de l'icône du lanceur, portail de choix de dossiers, surveillance
lors du retrait/remontage d'une source et des quotas inotify, changement de sortie
dans GNOME pendant la lecture, Silverblue (binaire de l'archive sur l'hôte,
`ldd`).
