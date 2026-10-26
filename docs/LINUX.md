# Prototype Linux — développement et validation

Le support Linux est en préparation. Windows conserve son backend GPUI, WASAPI
et ses intégrations natives. Les essais graphiques et matériels Fedora Workstation
et Silverblue restent nécessaires avant d'annoncer Linux comme validé.

L'[audit initial](PORTABILITE_LINUX.md) décrit les obstacles et le périmètre du
portage. Ce guide concerne la version du code après les premières adaptations.

## Ce qui est préparé

- Build de l'application sans référence aux ressources Windows sur un hôte Linux.
- GPUI 0.2.2 avec Wayland et X11, sans changement des versions des dépendances.
- Correctif local de la dépendance Unix `xattr` 0.2.3 pour `libc` récent sous
  Linux, sans changement de version ; voir [vendor/README.md](../vendor/README.md).
- Négociation `xdg-shell` compatible avec la version 1 exposée par WSLg, dans le
  backend Wayland uniquement.
- Décorations clientes Linux : déplacement au clic gauche depuis la barre de
  titre et le logo, double-clic pour agrandir/restaurer, réduction, fermeture et
  redimensionnement par les bords libres. Aucun changement de curseur ajouté.
- Identifiant de fenêtre `io.github.Carlton2001.Soliton`.
- Fermeture via le nettoyage normal de Soliton ; absence de masquage sans moyen
  de restauration.
- Mises à jour Windows indisponibles dans l'interface Linux, sans requête au
  démarrage ni modification du choix enregistré dans les paramètres.
- Découverte audio périodique conservée sans tentative de surveillance Windows.
- Bibliothèque, paramètres et caches indépendants des dossiers musicaux.

## Préparer Fedora Workstation ou Fedora WSL

Depuis la racine du dépôt, avec une session utilisateur normale :

```sh
sudo dnf install gcc gcc-c++ make cmake pkgconf-pkg-config \
  alsa-lib-devel libxkbcommon-devel libxkbcommon-x11-devel \
  freetype-devel fontconfig-devel libxcb-devel wayland-devel rustup
rustup-init -y --default-toolchain 1.98.1 --profile minimal --component rustfmt
. "$HOME/.cargo/env"
```

La version Rust correspond à celle de la validation Windows. Si Rust est déjà
installé par rustup, utiliser plutôt :

```sh
rustup toolchain install 1.98.1 --profile minimal --component rustfmt
```

Références Fedora : [Rust](https://developer.fedoraproject.org/tech/languages/rust/rust-installation.html),
[ALSA](https://packages.fedoraproject.org/pkgs/alsa-lib/alsa-lib-devel/),
[xkbcommon](https://packages.fedoraproject.org/pkgs/libxkbcommon/libxkbcommon-devel/),
[xkbcommon X11](https://packages.fedoraproject.org/pkgs/libxkbcommon/libxkbcommon-x11-devel/),
[FreeType](https://packages.fedoraproject.org/pkgs/freetype/freetype-devel/),
[Wayland](https://packages.fedoraproject.org/pkgs/wayland/wayland-devel/).

Dans WSL, garder les artefacts sur le système de fichiers Linux pour éviter de
mélanger les builds avec Windows et réduire les accès au volume Windows :

```sh
export CARGO_TARGET_DIR="$HOME/.cache/soliton/target"
export CARGO_INCREMENTAL=0
```

Cette configuration est locale au shell. Les sources peuvent rester dans le
checkout Windows monté sous `/mnt/c/`. Une compilation et des tests WSL ne
remplacent pas les essais d'une session GNOME Workstation.

## Compiler et tester

```sh
cargo +1.98.1 fmt --check
cargo +1.98.1 check --workspace --all-targets --locked
cargo +1.98.1 test --workspace --locked
cargo +1.98.1 build -p soliton-app --bin Soliton --locked
```

Les tests audio demandant une sortie et les fixtures multicodecs sont ignorés
par défaut. Les tests ordinaires n'ont pas besoin d'une fenêtre ou d'un GPU.
Le workflow GitHub Linux vérifie compilation et tests sans valider le bureau.

## Lancer avec des données isolées

Vérifier Vulkan, une session graphique Wayland ou X11, D-Bus et le portail de
choix de fichiers. Sous GNOME, le service de portail approprié doit être actif.
Un environnement WSL minimal ne fournit pas nécessairement ces services.

```sh
export SOLITON_DATA_DIR="$HOME/.local/share/Soliton-linux-prototype"
RUST_LOG=info cargo +1.98.1 run -p soliton-app --bin Soliton --locked
```

Ce dossier doit rester hors des sources musicales. Ne pas réutiliser la base
Windows pour les premiers essais : ses chemins absolus et ses noms de sorties
ne correspondent pas à ceux de Linux.

Le binaire `Soliton-index` permet aussi une vérification de l'indexation sans
ouvrir de fenêtre :

```sh
cargo +1.98.1 run -p soliton-app --bin Soliton-index --locked -- \
  "$SOLITON_DATA_DIR" /chemin/vers/une/collection-de-test
```

## Audio standard

La chaîne reste Symphonia → Rubato → CPAL 0.16/ALSA → greffon PipeWire.
Sur une session Fedora PipeWire, vérifier la présence de `pipewire-alsa` et le
routage du périphérique ALSA `default` vers PipeWire. Sélectionner d'abord la
sortie système par défaut dans Soliton. La liste CPAL peut également contenir des
sorties matérielles ALSA ; elle n'est pas une liste native des sinks PipeWire.

Le paquet [pipewire-alsa](https://packages.fedoraproject.org/pkgs/pipewire/pipewire-alsa/)
fournit la passerelle. Pour un serveur PulseAudio, la passerelle correspondante est
[alsa-plugins-pulseaudio](https://packages.fedoraproject.org/pkgs/alsa-plugins/alsa-plugins-pulseaudio/).

Les tests natifs existants peuvent être lancés explicitement sur la machine de
validation ; ils utilisent un volume nul :

```sh
cargo +1.98.1 test -p soliton-audio native_output_play_pause_seek_next_and_stop -- --ignored
cargo +1.98.1 test -p soliton-audio unavailable_preference_never_falls_back_and_output_failure_can_be_recovered -- --ignored
```

## Essais restant à réaliser sur Workstation

1. Barre de titre : clics gauche, déplacement, réduction, agrandissement,
   restauration, redimensionnement, placement sur plusieurs écrans et fermeture.
2. Mise à l'échelle GNOME, thèmes clair/sombre et saisie avec accents.
3. Ajout de dossiers par le portail, indexation, tags, pochettes, éditions,
   annulation et reprise incrémentale.
4. Surveillance : ajouts, modifications, suppressions, quotas inotify et
   retrait/remontage d'une source sans perte du catalogue hors ligne.
5. Lecture WAV/FLAC/MP3, pause, seek, suivant, volume, fréquences 44,1/48 kHz,
   changement et déconnexion de sortie.
6. Redémarrage et persistance des paramètres, playlists et file.

Sous Silverblue, Toolbx convient à la préparation et aux premiers essais.
Vérifier ensuite le binaire depuis l'hôte, ses bibliothèques avec `ldd`, le GPU,
le portail et le son. Une réussite dans Toolbx n'est pas une validation de l'hôte.
Voir la [documentation Toolbx](https://containertoolbx.org/).

## Fonctions différées

MPRIS/D-Bus, touches multimédias globales, notification, démarrage automatique,
gestion Bluetooth dédiée, sélection native de sinks PipeWire et distribution Linux
ne font pas partie de cette première étape. Audio exclusif, DLNA/Cast et Flatpak
restent hors du périmètre de validation Linux actuel. Les dépendances réseau
restent compilées, sans changement de l'architecture du moteur.

## Résultats de validation sur cette machine

Le 26 octobre 2026, avec Rust 1.98.1 :

| Environnement | Vérification | Résultat |
| --- | --- | --- |
| Windows MSVC | Format et `cargo check --workspace --all-targets --locked` | Réussis, sans avertissement du projet |
| Windows MSVC | Suite de tests du workspace | 141 tests réussis, 9 tests ignorés par défaut |
| Windows MSVC | Test audio natif lecture/pause/seek/suivant/arrêt, volume nul | Réussi |
| Windows MSVC | Démarrage/fermeture précoce, masquage/restauration, confirmation, fermeture minimisée et barre des tâches | Réussis |
| Windows MSVC | Six icônes, persistance, icônes natives fenêtre/proxy et zone de notification | Réussis |
| Windows MSVC | Inventaire des licences et tests des scripts de distribution/mises à jour | Réussis |
| Fedora 44 WSL | `cargo check --workspace --all-targets --locked` et construction des deux exécutables | Réussis |
| Fedora 44 WSL | Suite de tests du workspace, dont surveillance inotify réelle | 127 tests réussis, 4 tests ignorés par défaut |
| Fedora 44 WSL | Tests amont de la copie xattr corrigée | 4 tests et 1 doctest réussis |
| Fedora 44 WSL | Indexation des 7 fixtures audio synthétiques | Réussie, sans erreur |
| Fedora 44 WSLg | Test audio natif à volume nul, CPAL/ALSA vers PulseAudio WSLg | Réussi avec configuration ALSA locale au processus |
| Fedora 44 WSLg | Lancement de la fenêtre, Wayland puis X11 | Réussis pendant 12 secondes chacun, sans panic ; arrêt imposé par le test |

Ces résultats ne valident pas les interactions d'une session GNOME Workstation,
le routage PipeWire natif, les montages amovibles/NAS ni Silverblue. Le workflow
GitHub est ajouté mais n'a pas encore été exécuté à distance.

Le rendu WSLg observé utilise llvmpipe : les adaptateurs D3D12 exposés par WSL
ne proposent pas l'extension Vulkan `VK_EXT_inline_uniform_block` demandée par
Blade. Ce lancement ne valide donc pas l'accélération matérielle Linux.
Le portail de paramètres n'est pas disponible dans cette distribution WSL
minimale ; le choix de dossiers par portail reste à vérifier sur Workstation.
Les essais de lancement temporisés ne vérifient pas la fermeture depuis les
contrôles Linux, ni l'ensemble des interactions de fenêtre.

Pour reproduire le contrôle audio WSLg sans remplacer la configuration ALSA du
compte, créer un fichier dédié hors des sources musicales :

```sh
mkdir -p "$HOME/.cache/soliton"
cat > "$HOME/.cache/soliton/wsl-pulse.conf" <<'EOF'
</usr/share/alsa/alsa.conf>
pcm.!default { type pulse }
ctl.!default { type pulse }
EOF
ALSA_CONFIG_PATH="$HOME/.cache/soliton/wsl-pulse.conf" \
  cargo +1.98.1 test --workspace --locked \
  native_output_play_pause_seek_next_and_stop -- --ignored
```

Ce contrôle nécessite `alsa-plugins-pulseaudio` et le `PULSE_SERVER` fourni par
WSLg. L'énumération CPAL essaie aussi les interfaces optionnelles JACK et OSS :
leurs diagnostics « Unknown PCM » n'ont pas empêché le test de lecture.
