# RAP — Rust Audio Player

Lecteur audio local Rust / GPUI, GPL-3.0-only. Phases 0 à 5 implémentées dans
l'ordre et validées sous Windows avec Rust 1.98.1 MSVC.

## Lancer

Installer Rust MSVC, les Build Tools C++ et le SDK Windows, puis :

```powershell
cargo run -p rap-app
```

Après construction, l'exécutable est `target/debug/rap.exe`.

- **Ajouter un dossier** : plusieurs sources, scan en arrière-plan, progression et annulation.
- **Albums** : grille ou liste ; cliquer un album ouvre sa fiche et **Lire** lance sa lecture.
- **Éditions** : un même album présent dans plusieurs dossiers (CD et haute résolution,
  par exemple) apparaît une seule fois ; la fiche propose le choix de l'édition,
  la meilleure disponible en premier. Les sous-dossiers `CD1`, `Disc 2`… forment
  un seul album, et une compilation sans artiste d'album apparaît sous « Artistes divers ».
- Double-cliquer une piste lance la lecture à partir de celle-ci.
- **Artistes**, **Genres**, **Années** : accès aux albums correspondants ; cliquer
  l'artiste d'une fiche ou de la lecture en cours affiche ses albums.
- Recherche par album, titre, artiste ou genre, sans tenir compte de la casse ni des accents.
- **Filtres** : genre, période (décennie ou année), artiste, format et qualité
  (haute résolution, CD, avec perte), combinables ; seuls les filtres actifs
  apparaissent, en puces retirables. **Tri** par artiste, titre, année ou ajout récent.
- Barre permanente : pochette, titre, artiste et album, lecture/pause,
  précédente/suivante, aléatoire, répétition, position et volume glissables, sourdine.
- **Lecture en cours** : grande pochette et commandes essentielles, plein écran
  Windows avec `F11`.
- **Glisser-déposer** de fichiers ou de dossiers sur la fenêtre, et **Ouvrir un
  fichier** : lecture ponctuelle avec tags et pochettes, sans ajout à la bibliothèque.
- **Paramètres** : français/anglais, apparence système/clair/sombre, huit couleurs
  d'accent, taille des pochettes, masquage des albums indisponibles. Tout est conservé.
- Erreurs et avertissements regroupés dans un bandeau consultable.

Clavier : `Ctrl+F` recherche, `Ctrl+O` ouverture ponctuelle, flèches pour les albums,
`Entrée` pour ouvrir une fiche, `Espace` lecture/pause, `Ctrl+←/→` ±10 s, `F11`
plein écran, `Échap` ou bouton Précédent de la souris pour revenir. Le champ de
recherche prend en charge sélection, copier/coller et saisie IME.

## Données et fichiers sources

Sous Windows, SQLite, paramètres et pochettes sont dans `%LOCALAPPDATA%\RAP`.
`RAP_DATA_DIR` permet d'isoler les données de développement. Ce dossier doit rester
hors des sources musicales ; les sources qui le contiennent sont refusées.
RAP ouvre la musique en lecture seule et ne modifie ni tags ni pochettes intégrées.
Une source inaccessible conserve sa collection, signalée indisponible.

La découverte utilise **dua-core 5.0**, successeur recommandé de jwalk : deux workers
de parcours, métadonnées natives réutilisées, extraction des tags dans une file bornée.
Les fichiers inchangés, y compris illisibles, ne sont pas relus. Les caches de pochettes
sont adressés par contenu, en 160 et 512 pixels, régénérés s'ils manquent et nettoyés
quand plus aucune piste ne les référence. Les listes et grilles sont virtualisées.

Les dossiers sources sont surveillés (**notify**) : ajouts, modifications et
suppressions sont pris en compte après deux secondes de calme. Une analyse
incrémentale complète a lieu au démarrage et toutes les 30 minutes, en filet de
sécurité pour les partages réseau. Les sources sont sondées chaque minute sur un
thread dédié : une source hors ligne garde ses albums, signalés indisponibles,
et est réanalysée à son retour. Un fichier n'est retiré de la bibliothèque
qu'après un parcours complet et sans erreur de sa source accessible ; une source
qui paraît soudain vide conserve ses entrées.

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

## Limites

- Lecture mono/stéréo, sortie partagée par défaut. Pas de garantie bit-perfect,
  de mode exclusif, de sélection de sortie ou d'égaliseur à ce stade.
- WAV, FLAC, MP3, AAC-LC/M4A, ALAC, OGG Vorbis et AIFF testés par décodage et seek
  sur des signaux générés. Opus, WMA et APE ne sont pas proposés.
- Les éditions sont rapprochées seulement si artiste de l'album, titre et année
  sont identiques, l'année étant présente ; sinon elles restent des albums distincts.
  Le choix d'édition n'est pas mémorisé entre deux sessions.
- Les notifications sur partages SMB/NAS dépendent du serveur ; l'analyse périodique
  les complète. Pas de réglage de l'intervalle à ce stade.
- L'annulation attend la fin d'une E/S système en cours. Les performances sur
  grandes collections/NAS et l'écoute prolongée restent à valider sur des données réelles.
- Le glisser-déposer lit au plus 2 000 fichiers et n'ajoute jamais de dossier à la
  bibliothèque (choix validé, voir l'architecture). Il n'a pas été testé par un dépôt
  réel automatisé ; la lecture des fichiers déposés est couverte par un test unitaire.
- Intégrations système et distribution installable restent dans les phases suivantes.
  Linux n'a pas été validé.
- GPUI 0.2.2 et `proc-macro-error2` utilisent des correctifs locaux documentés
  dans [vendor/README.md](vendor/README.md) pour la fermeture Windows, la sonde
  DXGI Debug facultative et les incompatibilités futures de Rust.

Voir [les spécifications](RAP_SPECIFICATIONS.md), [l'architecture](docs/ARCHITECTURE.md)
et [les dépendances](docs/DEPENDENCIES.md). Attributions dans [assets/NOTICE.md](assets/NOTICE.md).
