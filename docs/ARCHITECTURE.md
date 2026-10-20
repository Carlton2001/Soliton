# Architecture et décisions

## Phase 0

Workspace Rust 2024 : `rap-core` (domaine, contrats, configuration), `rap-library`
(modules de persistance et d'indexation), `rap-audio` (décodage et sortie),
`rap-platform` (adaptations système), `rap-app` (assemblage et GPUI).
Aucune dépendance GPUI dans les services. SQLite reste un module de rap-library,
sans crate supplémentaire avant qu'un second consommateur ne la justifie.

GPUI 0.2.2 crates.io est évalué en premier ; Oxyde utilise une révision Git plus
récente au même numéro de version. Les API sont vérifiées dans les sources retenues.
Les paramètres, la base et les pochettes résident dans le dossier local RAP du
système, jamais dans les sources musicales. Les erreurs de configuration sont
signalées, sans écrasement silencieux d'un fichier invalide.

## Parcours envisagé pour la phase 1

L'examen d'Oxyde (`systems/folder_sizes.rs`, `commands/duplicates.rs`,
`utils/audio_metadata.rs`) montre des pools jwalk dédiés et bornés, l'annulation
coopérative, ainsi que lofty et la priorité aux pochettes intégrées de face avant.
RAP adopte ces principes avec une implémentation propre : un scan à la fois,
parcours distinct des lecteurs de tags, files bornées, SQLite sur un seul thread
propriétaire. Les échecs ne sont pas filtrés silencieusement.
Pas de copie de code applicatif Oxyde ni de dépendance à cette application.

La vérification de maintenance invalide toutefois le choix initial de jwalk :
0.9.0 est déprécié et le dépôt est archivé depuis le 5 août 2026
(https://github.com/Byron/jwalk). Après examen du successeur recommandé par
l'auteur, choix retenu : dua-core 5.0.0, moteur autonome MIT sans le CLI dua.
Deux workers de parcours, deux lots en attente par worker ; un à quatre lecteurs
de tags, 16 travaux/résultats en attente, 64 entrées découvertes en attente.
Le backend est isolé dans `discovery.rs`. Les métadonnées natives retournées
par dua-core sont réutilisées sans second stat de chaque fichier sous Windows.
La comparaison de performances SSD/NAS reste à faire sur collection réelle.

L'artiste d'album est distinct de l'artiste de piste. Éditions et surveillance :
voir la phase 4.

## Phases 1 et 2

SQLite versionné par `user_version` : sources, artistes, albums, éditions, pistes
logiques et fichiers physiques. Un fichier illisible n'arrête pas le scan.
Les entrées ne sont jamais supprimées : après un parcours complet sans erreur,
les fichiers non rencontrés sont marqués indisponibles. Un parcours annulé ou
partiel n'établit aucune disparition. Une source inaccessible conserve sa collection.
La taille et la date de modification permettent déjà d'éviter les relectures
inutiles. La surveillance et le regroupement d'éditions restent en phase 4.

L'application emploie un worker de commandes pour l'écriture, un lecteur WAL pour
les instantanés/recherches/facettes, et le pipeline borné de scan. La progression
est coalescée ; les instantanés sont relus toutes les 750 ms pendant l'indexation.
Les grilles (rangées adaptatives), listes, artistes et pistes utilisent
`uniform_list` de GPUI. Recherche Unicode, filtres combinés par genre/année/format.
Textes centralisés FR/EN et palette clair/sombre. Les réglages sont sauvegardés
sur le worker, les pochettes chargées par le chargeur d'images asynchrone de GPUI.

Le champ de texte reprend l'exemple officiel GPUI sous Apache-2.0 avec sélection,
copier/coller et IME. Attribution et licence dans assets/. Les SVG Lucide officiels
sont embarqués sans modification. Aucun changement de curseur n'est introduit.

## Phase 3

Symphonia décode au fil des paquets sur un worker audio dédié. CPAL/WASAPI ouvre
la sortie partagée par défaut ; on privilégie la fréquence et le nombre de canaux
natifs si le périphérique les expose en f32, puis sa configuration par défaut.
Rubato effectue une conversion sinc sur ce même worker quand les fréquences
diffèrent. Le signal est mono/stéréo ; les autres configurations échouent
explicitement. Le mode exclusif et les garanties bit-perfect restent hors jalon.

Le callback audio effectue uniquement des lectures atomiques, dépile des samples
d'un tampon lock-free préalloué et applique le volume. Ni fichiers, ni logs,
ni allocations, ni verrous bloquants. La quantité de PCM en attente est bornée
à deux secondes de sortie plus un paquet en cours de conversion. Une commande
ne doit pas attendre le décodage d'un fichier entier. Les accès NAS restent
soumis aux délais d'E/S du système : l'annulation est coopérative entre ces accès.

Le seek réinitialise le décodeur et le convertisseur, et remplace le flux de sortie
pour ne pas restituer d'anciens samples. La position vient des samples effectivement
consommés, hors silence de sous-alimentation. Lecture d'albums/sélections simples,
aléatoire sans doublons, répétition de piste ou d'album. Les événements sont bornés
et coalescés ; les erreurs restent dans l'état de lecture jusqu'à une nouvelle action.

Les versions Symphonia 0.5.5, CPAL 0.16 et Rubato 0.16.2 sont épinglées pour cette
première intégration aux API examinées. Il existe des versions majeures plus récentes ;
une migration devra conserver ces tests de décodage et de sortie avant changement.

## Validation du jalon — 20 octobre 2026

- Windows MSVC, Rust 1.98.1 : `cargo fmt --check`, `cargo check --workspace --locked`
  et construction des deux exécutables réussis, sans avertissement RAP.
- 20 tests réussis avec `cargo test --workspace --locked -- --include-ignored`,
  après génération des fixtures. Le test matériel s'exécute à volume nul.
- Décodage et seek vérifiés en WAV, FLAC, MP3, AAC/M4A, ALAC, Vorbis et AIFF.
  Le test AAC a conduit à initialiser la sortie après le premier paquet décodé,
  certains conteneurs ne donnant pas immédiatement leur disposition de canaux.
- Test de sous-alimentation : les trames stéréo restent indivisibles, sans décalage
  de canaux ; une pause conserve les données et ne fait pas avancer la position.
- Fenêtre GPUI réellement ouverte puis fermée proprement, captures de contrôle
  de la grille, de la liste et de la fiche album, français/sombre et anglais/clair.
  La commande de lecture de l'interface est vérifiée sur un signal synthétique
  à volume nul : titre courant, bouton Pause et progression actualisés.
- Redémarrage et scan des sept fichiers synthétiques : sept inchangés, aucune
  relecture de tags, aucune erreur. Les tests de bibliothèque couvrent aussi 180
  fichiers, les sources déconnectées et la conservation des octets sources.

La validation ne remplace pas une écoute prolongée, un test NAS réel ou un benchmark
sur grande collection. Linux n'est pas validé.

## Correctifs de démarrage et fermeture Windows — 20 octobre 2026

Les premiers essais ne rejetaient pas les diagnostics d'erreur si le processus
sortait avec le code 0. Le test natif vérifie maintenant aussi les deux journaux.

Deux packages sont conservés localement et épinglés via `[patch.crates-io]` :
GPUI 0.2.2 et proc-macro-error2 2.0.1. Les licences, sources et modifications
sont documentées dans [vendor/README.md](../vendor/README.md).

L'absence de Graphics Tools/DXGI Debug est traitée comme un diagnostic debug,
avec le même repli D3D fonctionnel. Les erreurs réelles de création du périphérique
restent signalées. La destruction Windows marque le HWND détruit, débranche les
callbacks GPUI et révoque le glisser-déposer avant sa disparition. La destruction
différée ne touche plus à une fenêtre déjà détruite. La réexportation de proc_macro
et les littéraux f32 sont corrigés pour les futures versions de Rust.

Validation après correction : formatage, `cargo check --workspace --locked`, build
des exécutables et 20 tests réussis sans avertissement. Les trois fermetures natives
(lecture en cours à volume nul, immédiatement après ouverture, fenêtre réduite)
retournent 0 sans erreur dans les journaux. Les essais natifs ont été exécutés hors
bac à sable après un échec STATUS_INVALID_HANDLE de ce dernier, également observé
dans le test CPAL inchangé.

## Phase 4 — Robustesse de la bibliothèque — 20 octobre 2026

Arbitrages validés avant implémentation (décisions ouvertes 4 et 5 du cahier
des charges) : regroupement artiste + titre + année, retrait des fichiers disparus
après confirmation, analyse périodique de 30 minutes, « Artistes divers » pour
les compilations sans artiste d'album.

**Schéma v2.** Migrations séquentielles dans `schema.rs` (v1 SQL, v2 Rust), chacune
dans une transaction, contrôles de clés étrangères désactivés pendant la migration
(SQLite embarqué les active par défaut) puis `foreign_key_check`. Une édition a une
identité stable (source, dossier d'album, titre et artiste d'album taggé normalisés) ;
plusieurs éditions par album. Les pistes existantes sont regroupées sans accès aux
fichiers ; `files.extractor` provoque une relecture unique des tags à l'analyse
suivante. Les identifiants de pistes sont conservés.

**Regroupement** (`grouping.rs`, fonctions pures). Dossier d'album = dossier parent,
en sautant un niveau `CD1`/`Disc 2`/`Disque 3`, sans sortir de la source. L'album
d'une édition est recalculé à chaque lot validé à partir de ses pistes : artiste
d'album taggé, sinon compilation si drapeau de compilation ou artistes de piste
différents, sinon artiste de piste ou dossier. Fusion entre dossiers seulement si
artiste, titre normalisé (casse, espaces), statut de compilation et année la plus
fréquente sont identiques ; un titre déduit du dossier ou une année absente ne
fusionne jamais. Les éditions sont classées disponibles d'abord, puis par résolution.

**Disparitions.** Les erreurs de lecture d'un fichier n'empêchent plus de conclure ;
seules les erreurs de parcours, l'annulation ou une source devenue inaccessible
pendant l'analyse l'empêchent. La source est vérifiée avant et après le parcours.
Un parcours complet de la racine sans aucun fichier audio est traité comme un volume
non monté : entrées conservées et marquées indisponibles. Les échecs d'extraction
sont mémorisés par taille, date et version pour ne pas relire un fichier corrompu
inchangé.

**Surveillance et sondes.** Le worker unique d'écriture (`indexing.rs`) attend à la
fois les commandes, les événements notify et ses échéances. Le gestionnaire notify
ne bloque jamais : file bornée à 256 événements, perte signalée par un drapeau et
traitée comme une analyse complète. Les événements sont regroupés (2 s de calme,
15 s au plus, 4 096 chemins au plus) puis convertis en dossiers à analyser ; un
dossier supprimé est remplacé par son plus proche ancêtre existant et la disparition
n'est conclue que dans ce périmètre. Une source n'est surveillée qu'après avoir
répondu. La sonde `read_dir` s'exécute sur un thread dédié avec un délai de 5 s,
au plus une sonde en attente par chemin. Les analyses d'arrière-plan n'affichent
pas de progression ; l'interface actualise sa vue sur événement. Elles sont annulées
à la fermeture.

**Pochettes.** Cache adressé par contenu, donc invalidé de fait quand l'image intégrée
change. Une entrée de cache manquante provoque la relecture du fichier. Après chaque
analyse de toutes les sources, les miniatures qu'aucune piste ne référence (sources
hors ligne comprises) et les fichiers temporaires de plus d'une heure sont supprimés,
uniquement dans le dossier de cache de RAP.

**Recherche.** Le lecteur WAL garde sa connexion ouverte et ne reconstruit instantané,
facettes et textes en minuscules que lorsque `PRAGMA data_version` change ; une frappe
ne fait plus que comparer des chaînes préparées. Les filtres portent sur toutes les
éditions.

**Casse et accents.** Une seule clé de comparaison (`rap_core::text::fold` :
décomposition NFKD, retrait des diacritiques combinants, ligatures œ/æ/ß développées,
minuscules, espaces normalisés) sert au regroupement, aux tris, à la recherche et aux
filtres. Les marques qui changent la lettre elle-même (kana voisés) sont conservées.
La migration v3 recalcule depuis la base les identités d'éditions et d'albums. Les facettes
Artistes et Genres réunissent les graphies d'une même valeur et affichent la plus
fréquente (la première rencontrée en cas d'égalité) ; filtrer sur l'une trouve toutes
les autres.

Validation : `cargo fmt --check`, `cargo check --workspace --locked` sans avertissement,
37 tests réussis avec `--include-ignored`, tests audio matériels compris. Smoke test natif réussi, avec
migration d'une base v1 existante. Collection FLAC générée : éditions 16/44,1 et 24/96
réunies avec sélecteur, compilation sous « Artistes divers ». Application en cours
d'exécution : un album ajouté et une édition supprimée sont pris en compte sans
action manuelle. Non validé : NAS/SMB réel, grandes collections, déconnexion
physique d'un disque, Linux.
