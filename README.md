# RAP — Rust Audio Player

Lecteur audio local Rust / GPUI, GPL-3.0-only. Phases 0 à 7, puis phase 9 implémentées dans
l'ordre et validées sous Windows avec Rust 1.98.1 MSVC.
La phase 8 (API locale de contrôle) est reportée.

La piste Alexa Wi-Fi a été abandonnée au profit de la sortie Bluetooth le
21 octobre 2026.

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
- **Artistes** : accès aux albums correspondants ; cliquer
  l'artiste d'une fiche ou de la lecture en cours affiche ses albums.
- Recherche par album, titre, artiste ou genre, sans tenir compte de la casse ni des accents.
- **Filtres** : genre, période (décennie ou année), artiste, format et qualité
  (haute résolution, CD, avec perte), combinables ; seuls les filtres actifs
  apparaissent, en puces retirables. **Tri** par artiste, titre, année ou ajout récent.
- Barre permanente : pochette, titre, artiste et album, lecture/pause,
  précédente/suivante, aléatoire, répétition, position et volume glissables, sourdine.
- **Lecture en cours** : grande pochette et commandes essentielles, plein écran
  Windows avec `F11`.
- **Paramètres** : français/anglais, apparence système/clair/sombre, huit couleurs
  d'accent, taille des pochettes, masquage des albums indisponibles. Tout est conservé.
- **Sortie audio** : choix du périphérique système par défaut ou d'une sortie locale
  mémorisée dans les paramètres. La liste se rafraîchit automatiquement et manuellement.
  Un changement conserve la piste, la position et l'état lecture/pause ; un choix
  refusé conserve la sortie précédente. Une sortie préférée absente reste mémorisée.
  Le périphérique actif, la fréquence, les canaux et le format d'échantillons envoyés
  par RAP s'affichent dans les paramètres et la lecture en cours.
- **Bluetooth** : dans **Paramètres → Sortie audio → Appareils Bluetooth appairés**,
  RAP liste les appareils déjà appairés, y compris déconnectés, avec leur état.
  **Connecter dans Windows** et **Ajouter ou connecter un appareil Bluetooth**
  ouvrent les paramètres Windows ; la connexion s'effectue dans ces paramètres.
  Activez le Bluetooth du PC et le mode appairage de l'enceinte, puis ajoutez-la
  ou reconnectez-la dans Windows. Sur une Echo compatible, dites « Alexa,
  appairage Bluetooth ». Revenez dans RAP et sélectionnez sa sortie disponible
  sous l'appareil ou dans la liste audio au-dessus. Les listes s'actualisent en
  arrière-plan toutes les cinq secondes ou à la demande. Le choix est mémorisé
  sans changer la sortie par défaut de Windows. Les autres enceintes et casques audio Bluetooth sont
  également utilisables lorsqu'ils sont exposés comme sorties par Windows.
  Un appareil Alexa découvert en Wi-Fi n'est pas nécessairement un récepteur
  audio Bluetooth. Aucun compte Amazon ni service distant n'est utilisé par RAP.
  Le codec dépend de Windows et du périphérique : ni lossless ni bit-perfect
  ne sont garantis, et le format PCM affiché est celui envoyé au système.
  Sur un ensemble de deux Echo et d'un Echo Sub, le Bluetooth alimente uniquement
  l'Echo connectée au PC, sans transmettre au reste de l'ensemble Alexa.
- **Sorties réseau** : dans les paramètres audio, **Rechercher les appareils**
  découvre les récepteurs DLNA/UPnP et Google Cast du réseau local. Choisir un
  appareil y envoie la lecture ; les mêmes commandes de RAP et de Windows
  pilotent la piste, le volume et la navigation. La NVIDIA SHIELD est prise en
  charge via son récepteur Cast intégré. Le choix réseau vaut pour la session ;
  le périphérique local préféré reste mémorisé. Choisir à nouveau une sortie
  locale reprend sur le PC en conservant la position et l'état lecture/pause.
- Erreurs et avertissements regroupés dans un bandeau consultable.
- **Windows** : contrôles multimédias système (SMTC), titre, artiste, album,
  pochette et progression ; lecture/pause, arrêt, précédente/suivante et seek.
  Les touches multimédias sont prises en charge via SMTC.
- **Aperçu de barre des tâches** : au survol de l'icône RAP, vignette compacte
  avec pochette, titre, artiste, album et progression ; boutons natifs Précédent,
  Lecture/Pause, Suivant et Arrêt, disponibles aussi lorsque RAP est réduit.
  La vignette suit le thème et l'accent. Sa barre de progression est indicative.
- **Zone de notification** : clic gauche pour restaurer RAP ; menu au clic droit
  avec Afficher, Lecture/Pause, Précédente, Suivante et Quitter.
- **Fermeture** : quitter (par défaut), masquer en continuant la lecture ou demander.
  Le bouton **Quitter RAP** des paramètres et le menu de notification quittent
  toujours complètement. Si l'icône est indisponible, RAP reste visible.
- **Démarrage automatique** : option Windows dans les paramètres, désactivée tant
  qu'elle n'a pas été activée. Le réglage concerne l'utilisateur courant et le
  chemin de l'exécutable ; réactiver l'option après avoir déplacé RAP.

Clavier : `Ctrl+F` recherche, flèches pour les albums,
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
./scripts/smoke-windows.ps1 -CloseScenario Hidden
./scripts/smoke-windows.ps1 -CloseScenario Ask
./scripts/smoke-windows.ps1 -Taskbar
./scripts/smoke-windows.ps1 -AudioOutputs
./scripts/smoke-windows.ps1 -NetworkOutputs -LocalOutputCount 2
cargo run -p rap-network --example discover
cargo test -p rap-platform -- --include-ignored
```

Le test de sortie native utilise un volume nul. Le smoke test ouvre puis ferme RAP
sur une base et une collection synthétiques dans `target/`, et capture uniquement
sa fenêtre. Il rejette un code de sortie non nul ou des erreurs dans les journaux.
`-AudioOutputs` vérifie aussi la sélection et la sauvegarde d'une sortie locale,
le retour au périphérique système et l'affichage en lecture ; `-AudioOutputRow 1`
choisit la deuxième sortie de la liste. Le débranchement physique reste à valider.
FFmpeg ne fait pas partie des dépendances de l'application.

Test Bluetooth explicite sur une sortie déjà appairée/connectée dans Windows
(remplacer le nom par celui affiché dans RAP) :

```powershell
cargo run -p rap-audio --example bluetooth_probe -- "Haut-parleurs (Echo Studio-231)"
```

Ce test crée dans un dossier temporaire un signal de cinq secondes à faible
niveau, sélectionne exactement la sortie demandée et vérifie la progression,
la pause/reprise, la fin de lecture et la conservation du choix. Il ne modifie
ni le volume système ni la bibliothèque. Il faut confirmer séparément l'écoute
sur la bonne enceinte ; aucune déduction sur le codec Bluetooth ou le lossless
n'est faite à partir du format PCM.

Validation du 21 octobre 2026 : `cargo fmt --all --check`, `cargo check
--workspace --all-targets` sans avertissement de projet et 66 tests automatisés réussis
(7 scénarios matériels/fixtures ignorés par défaut). Construction Windows et
test réel du moteur sur `Haut-parleurs (Echo Studio-231)` réussis : flux
44 100 Hz, deux canaux, F32 avant encodage Bluetooth ; progression,
pause/reprise et fin de lecture vérifiées. Le premier signal n'a pas été entendu.
Après mise à jour, l'utilisateur confirme la lecture sur ses casques Bluetooth
et sur l'Echo principale de son ensemble ; la seconde Echo et l'Echo Sub ne
reçoivent pas cette entrée Bluetooth, conformément à la limitation Alexa.
L'énumération native retrouve les quatre appareils appairés et déconnectés du PC.
Déconnexion physique pendant la lecture non testée.
Les avertissements du linker MSVC signalent uniquement la création de fichiers
`.lib`/`.exp`. Sous le sandbox, désactiver le cache incrémental avec
`$env:CARGO_INCREMENTAL = '0'` évite l'avertissement de copie au lieu de lien dur.

`-NetworkOutputs` recherche et capture les appareils réseau sans sélectionner un
récepteur. `-LocalOutputCount` indique le nombre de sorties locales affichées.
Pour une validation matérielle explicite sur un récepteur Cast **inactif**, avec
un fichier synthétique silencieux de douze secondes :

```powershell
cargo run -p rap-network --example smoke_cast -- <identifiant-découvert> <fichier-synthétique>
cargo run -p rap-audio --example network_session -- <identifiant-découvert> <fichier-synthétique>
```

Ces scénarios restaurent le volume initial. WAV et FLAC ont été vérifiés sur la
SHIELD ; DLNA est couvert par un récepteur simulé, en attente de test matériel.
Les tests natifs de `rap-platform` nécessitent un bureau Windows interactif : ils
interrogent réellement SMTC, lisent sa pochette et envoient des commandes. Le test
du registre utilise une clé temporaire `HKCU\Software\RAP-Test-<pid>` supprimée
ensuite ; il ne touche pas à l'option réelle de démarrage automatique.

Outil de diagnostic sans interface :

```powershell
cargo run -p rap-app --bin rap-index -- <dossier-donnees-RAP> [dossier-musique]
```

## Limites

- Réseau : IPv4 sur les interfaces privées ou de lien local ; une seule sortie
  active à la fois, aucun transcodage ni synchronisation multiroom. Le PC doit
  rester allumé et joignable par le récepteur. Les autres modèles Cast et les
  téléviseurs DLNA dépendent de leurs formats, capacités et réglages réseau.
  La découverte seule ne lance aucune application sur les appareils. La lecture
  locale fonctionne sans Internet ; le récepteur Cast peut dépendre de Google.

- Lecture mono/stéréo, sortie partagée. Égaliseur, mode exclusif, bit-perfect et
  validation de formats supplémentaires exclus du périmètre retenu pour la phase 7.
- CPAL 0.16 identifie les sorties par leur nom : une sortie renommée doit être
  sélectionnée à nouveau ; les noms identiques sont signalés et non sélectionnables.
  Le choix système par défaut est résolu à chaque ouverture du flux (piste, seek,
  sélection de sortie), sans migration automatique d'un flux déjà actif.
  Les caractéristiques affichées décrivent ce que RAP envoie, avant les traitements
  éventuels du mélangeur système et du matériel.
- WAV, FLAC, MP3, AAC-LC/M4A, ALAC, OGG Vorbis et AIFF testés par décodage et seek
  sur des signaux générés. Opus, WMA et APE ne sont pas proposés.
- Les éditions sont rapprochées seulement si artiste de l'album, titre et année
  sont identiques, l'année étant présente ; sinon elles restent des albums distincts.
  Le choix d'édition n'est pas mémorisé entre deux sessions.
- Les notifications sur partages SMB/NAS dépendent du serveur ; l'analyse périodique
  les complète. Pas de réglage de l'intervalle à ce stade.
- L'annulation attend la fin d'une E/S système en cours. Les performances sur
  grandes collections/NAS et l'écoute prolongée restent à valider sur des données réelles.
- Les touches multimédias physiques, un vrai redémarrage d'Explorer et le lancement
  après ouverture de session restent à vérifier manuellement. Windows peut aussi
  désactiver le lancement via ses propres paramètres de démarrage.
- L'aperçu de barre des tâches est validé par les API natives DWM et des commandes
  de boutons simulées. Le survol et les clics physiques sur le panneau Windows
  restent à vérifier manuellement, ainsi qu'un redémarrage réel d'Explorer.
- La distribution installable reste dans une phase suivante. Linux n'a pas été validé.
- GPUI 0.2.2 et `proc-macro-error2` utilisent des correctifs locaux documentés
  dans [vendor/README.md](vendor/README.md) pour la fermeture Windows, la sonde
  DXGI Debug facultative et les incompatibilités futures de Rust.

Voir [les spécifications](RAP_SPECIFICATIONS.md), [l'architecture](docs/ARCHITECTURE.md)
et [les dépendances](docs/DEPENDENCIES.md). Attributions dans [assets/NOTICE.md](assets/NOTICE.md).
