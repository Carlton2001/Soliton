# Développement et état actuel

Lecteur audio local Rust / GPUI, GPL-3.0-only. Phases 0 à 7 et 9 implémentées,
validées sous Windows avec Rust 1.98.1 MSVC. Distribution Windows préparée ;
restent la validation des paquets et les validations matérielles.
La phase 8 (API locale de contrôle) est reportée.

Les commandes ci-dessous sont à exécuter depuis la racine du dépôt.

Le prototype Linux est en préparation sans changement des backends Windows.
Pour Fedora Workstation, WSL et Silverblue, voir [LINUX.md](LINUX.md).

## Lancer

Installer Rust MSVC, les Build Tools C++ et le SDK Windows, puis :

```powershell
cargo run -p soliton-app
```

## Fonctions

- **Bibliothèque** : plusieurs dossiers sources, indexation en arrière-plan
  annulable, surveillance des changements, analyse au démarrage et toutes les
  30 minutes. Une source hors ligne garde ses albums, signalés indisponibles.
- **Albums et artistes** : grille ou liste, fiche album, éditions multiples
  réunies (la meilleure disponible en premier), compilations sous « Artistes divers ».
- **Recherche et filtres** : sans casse ni accents ; filtres genre, période,
  artiste, format et qualité ; tri par artiste, titre, année ou ajout récent.
- **Lecture** : barre permanente, Lecture en cours, plein écran `F11`, aléatoire,
  répétition, playlists manuelles et file modifiable.
- **Sorties** : périphérique local mémorisé, Bluetooth via les paramètres Windows,
  DLNA / UPnP et Google Cast (validé sur NVIDIA SHIELD).
- **Windows** : SMTC et touches multimédias, vignette de barre des tâches avec
  commandes, zone de notification, fermeture configurable, démarrage automatique.
- **Paramètres** : français/anglais, thème système/clair/sombre, accent, taille des
  pochettes, icône de l'application.

Clavier : `Ctrl+F` recherche, `Espace` lecture/pause, `Ctrl+←/→` ±10 s,
`Ctrl+↑/↓` volume, `Alt+←/→` historique, `Échap` retour, `Tab` et `Entrée` pour
les contrôles.

## Données

SQLite, paramètres et pochettes sont dans `%LOCALAPPDATA%\Soliton` (`SOLITON_DATA_DIR`
pour isoler le développement), hors des sources musicales. Soliton ouvre la musique
en lecture seule et ne modifie ni tags ni pochettes intégrées.

## Vérifier

```powershell
cargo fmt --check
cargo check --workspace --locked
cargo test --workspace --locked
```

Tests natifs et matériels (FFmpeg sert uniquement à générer les fixtures) :

```powershell
./scripts/generate-audio-fixtures.ps1
cargo test -p soliton-audio -- --include-ignored
cargo test -p soliton-platform -- --include-ignored
./scripts/smoke-windows.ps1
./scripts/smoke-icons-windows.ps1
```

`smoke-windows.ps1` ouvre Soliton sur des données synthétiques dans `target/`, capture
sa fenêtre et échoue sur un code de sortie ou une erreur de journal. Options :
`-Language English -Light -List`, `-CloseScenario Early|Minimized|Hidden|Ask`,
`-Taskbar`, `-Collections`, `-AudioOutputs [-AudioOutputRow n]`,
`-NetworkOutputs -LocalOutputCount n`.

Exemples matériels explicites (volume nul ou faible, récepteur Cast inactif) :

```powershell
cargo run -p soliton-audio --example bluetooth_probe -- "<nom de sortie>"
cargo run -p soliton-network --example discover
cargo run -p soliton-network --example smoke_cast -- <identifiant> <fichier>
cargo run -p soliton-audio --example network_session -- <identifiant> <fichier>
cargo run -p soliton-app --bin Soliton-index -- <dossier-donnees-Soliton> [dossier-musique]
```

Sous le bac à sable, `$env:CARGO_INCREMENTAL = '0'` évite l'avertissement de liens durs.

## Limites

- Sortie partagée, sources mono/stéréo (stéréo à l'avant sur une sortie 5.1/7.1) ; pas d'égaliseur, de mode exclusif ni de bit-perfect.
- Formats testés : WAV, FLAC, MP3, AAC-LC/M4A, ALAC, Vorbis, AIFF. Pas d'Opus, WMA, APE.
- Les sorties CPAL sont identifiées par leur nom : une sortie renommée doit être
  choisie à nouveau. Le format affiché est celui envoyé par Soliton au système.
- Bluetooth : codec choisi par Windows, ni lossless ni bit-perfect garantis.
- Réseau : IPv4 local, une sortie active, sans transcodage ni multiroom ; DLNA
  testé seulement sur récepteur simulé.
- Éditions fusionnées seulement si artiste, titre et année sont identiques ; le
  choix d'édition n'est pas mémorisé.
- Distribution Windows et vérification des mises à jour : voir le
  [guide de distribution](DISTRIBUTION.md). Linux non validé.
- Validations matérielles en attente : voir la phase 11 des spécifications.

Voir [les spécifications](SOLITON_SPECIFICATIONS.md), [l'architecture](ARCHITECTURE.md),
[les dépendances](DEPENDENCIES.md) et [les correctifs GPUI](../vendor/README.md).
Attributions dans [assets/NOTICE.md](../assets/NOTICE.md).
Collecte et vérification des licences des crates et ressources :
[licenses/dependencies/README.md](../licenses/dependencies/README.md).
