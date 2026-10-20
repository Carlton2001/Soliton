# RAP — Rust Audio Player

Lecteur audio local Rust / GPUI, GPL-3.0-only. Premier jalon : phases 0 à 3,
implémentées dans l'ordre et validées sous Windows avec Rust 1.98.1 MSVC.

## Lancer

Installer Rust MSVC, les Build Tools C++ et le SDK Windows, puis :

```powershell
cargo run -p rap-app
```

Après construction, l'exécutable est `target/debug/rap.exe`.

- **Ajouter un dossier** : plusieurs sources, scan en arrière-plan, progression et annulation.
- **Albums** : grille ou liste ; cliquer un album ouvre sa fiche et **Lire** lance sa lecture.
- Double-cliquer une piste lance la lecture à partir de celle-ci.
- **Artistes**, **Genres**, **Années** : accès aux albums correspondants.
- Recherche par album, titre, artiste ou genre ; filtres combinables genre/année/format.
- Barre permanente : lecture/pause/arrêt, précédente/suivante, position cliquable,
  déplacements de 10 s, volume, aléatoire, répétition de piste ou d'album.
- **Ouvrir un fichier** : lecture ponctuelle d'un fichier ou d'une sélection, sans indexation.
- **Paramètres** : français/anglais et thème clair/sombre. Langue, thème, mode
  grille/liste et volume sont conservés.

Clavier : `Ctrl+F` recherche, `Ctrl+O` ouverture ponctuelle, flèches pour les albums,
`Entrée` pour ouvrir une fiche, `Espace` lecture/pause, `Échap` retour à la bibliothèque.
Le champ de recherche prend en charge sélection, copier/coller et saisie IME.

## Données et fichiers sources

Sous Windows, SQLite, paramètres et pochettes sont dans `%LOCALAPPDATA%\RAP`.
`RAP_DATA_DIR` permet d'isoler les données de développement. Ce dossier doit rester
hors des sources musicales ; les sources qui le contiennent sont refusées.
RAP ouvre la musique en lecture seule et ne modifie ni tags ni pochettes intégrées.
Une source inaccessible conserve sa collection, signalée indisponible.

La découverte utilise **dua-core 5.0**, successeur recommandé de jwalk : deux workers
de parcours, métadonnées natives réutilisées, extraction des tags dans une file bornée.
Les fichiers inchangés ne sont pas relus. Les caches de pochettes sont adressés par
contenu, en 160 et 512 pixels. Les listes et grilles sont virtualisées.

## Vérifier

```powershell
cargo fmt --check
cargo check --workspace --locked
cargo test --workspace --locked
```

Tests audio complémentaires, avec FFmpeg installé **uniquement pour générer les
fixtures de développement** et un périphérique de sortie disponible :

```powershell
./scripts/generate-audio-fixtures.ps1
cargo test -p rap-audio -- --include-ignored
./scripts/smoke-windows.ps1
./scripts/smoke-windows.ps1 -Language English -Light -List
./scripts/smoke-windows.ps1 -CloseScenario Early
./scripts/smoke-windows.ps1 -CloseScenario Minimized
```

Le test de sortie native utilise un volume nul. Le smoke test ouvre puis ferme RAP
sur une base et une collection synthétiques dans `target/`, et capture uniquement
sa fenêtre. Il rejette un code de sortie non nul ou des erreurs dans les journaux.
FFmpeg ne fait pas partie des dépendances de l'application.

Outil de diagnostic sans interface :

```powershell
cargo run -p rap-app --bin rap-index -- <dossier-donnees-RAP> [dossier-musique]
```

## Limites du premier jalon

- Lecture mono/stéréo, sortie partagée par défaut. Pas de garantie bit-perfect,
  de mode exclusif, de sélection de sortie ou d'égaliseur à ce stade.
- WAV, FLAC, MP3, AAC-LC/M4A, ALAC, OGG Vorbis et AIFF testés par décodage et seek
  sur des signaux générés. Opus, WMA et APE ne sont pas proposés.
- Les éditions de dossiers distincts restent séparées ; rapprochement d'éditions,
  surveillance automatique et gestion affinée des compilations relèvent de la phase 4.
- L'annulation attend la fin d'une E/S système en cours. Les performances sur
  grandes collections/NAS et l'écoute prolongée restent à valider sur des données réelles.
- Glisser-déposer, plein écran, filtres de qualité audio, intégrations système et
  distribution installable restent dans les phases suivantes. Linux n'a pas été validé.
- GPUI 0.2.2 et `proc-macro-error2` utilisent des correctifs locaux documentés
  dans [vendor/README.md](vendor/README.md) pour la fermeture Windows, la sonde
  DXGI Debug facultative et les incompatibilités futures de Rust.

Voir [les spécifications](RAP_SPECIFICATIONS.md), [l'architecture](docs/ARCHITECTURE.md)
et [les dépendances](docs/DEPENDENCIES.md). Attributions dans [assets/NOTICE.md](assets/NOTICE.md).
