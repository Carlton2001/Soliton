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

## Phase 5 — Interface finalisée — 20 octobre 2026

Arbitrages validés avant implémentation : filtres derrière un bouton avec puces
pour les filtres actifs (une valeur par filtre), palette de huit accents, préférences
de taille des pochettes, thème système, tri et masquage des albums indisponibles.

**Écart aux spécifications (§8.10).** Le cahier des charges prévoit que le
glisser-déposer distingue lecture ponctuelle et ajout d'un dossier à la bibliothèque.
Sur décision de l'utilisateur, le dépôt sert uniquement à la lecture ponctuelle ;
les sources s'ajoutent seulement par « Ajouter un dossier ». Le cahier des charges
a été mis à jour le 22 octobre 2026 : la fonction est retirée.

**Lecture ponctuelle (retirée le 22 octobre 2026).** RAP gère une bibliothèque : le
glisser-déposer, « Ouvrir un fichier » et `Ctrl+O` ont été supprimés, ainsi que les pages
Genres et Années, redondantes avec les filtres. Historique :
`rap_library::read_loose_files` parcourt fichiers et dossiers
déposés ou ouverts (liens non suivis, 2 000 fichiers au plus), lit tags et pochettes
sur l'exécuteur d'arrière-plan de GPUI, sans base de données. Les identifiants sont
négatifs pour ne jamais correspondre à une piste indexée. Les pochettes vont dans
`covers/loose`, sous-dossier ignoré par le nettoyage du cache et vidé au démarrage.
Une nouvelle demande annule la précédente.

**Réglages.** Apparence (système, clair, sombre ; l'ancien booléen `dark` est relu),
accent, taille des pochettes, tri et masquage persistent dans `settings.json`.
Le thème système suit `observe_window_appearance`. Chaque accent a une variante par
thème ; un test vérifie un contraste d'au moins 4,5:1 sur tous les fonds, et le gris
secondaire du thème clair a été foncé pour respecter ce seuil.

**Tri et filtres.** Le tri par ajout récent utilise `files.added` (schéma v4) :
date de création du fichier lue par les lecteurs de tags, sinon date de modification ;
les fichiers indexés auparavant reçoivent leur date de modification. Période et
qualité (plus de 16 bits ou de 48 kHz, CD, avec perte faute de profondeur) sont des
critères du catalogue, comme le masquage des albums sans édition accessible ; les
tris sont stables et gardent l'ordre artiste puis titre en cas d'égalité.

**Interface.** Barre de lecture en trois zones ; curseurs de position et de volume
dessinés sur un canevas comme la barre de défilement, la position n'étant envoyée
au moteur qu'au relâchement. Les boutons Arrêt et ±10 s sont remplacés par la pause
et `Ctrl+←/→`. Vue « lecture en cours » et plein écran Windows (`toggle_fullscreen`).
Menus fermés par un clic extérieur (fond occultant) ou `Échap`. Erreurs et
avertissements sont conservés (50 au plus, doublons consécutifs ignorés) et affichés
dans un bandeau dépliable ; les diagnostics de fichiers sont des avertissements.
États vides : bibliothèque vide, aucun résultat avec bouton d'effacement, facette
vide, sortie audio indisponible, album sans édition accessible. Chemins affichés
sans préfixe `\\?\`. Douze icônes Lucide officielles ajoutées sans modification.

Validation : formatage, `cargo check --workspace --locked` sans avertissement,
45 tests réussis avec `--include-ignored`. Quatre scénarios du smoke test natif réussis.
Pilotage réel de la fenêtre (clics et touches envoyés à la seule fenêtre RAP, son
coupé) : menus de filtres et de tri, puce de période, paramètres, thème clair,
anglais, accent bleu, lecture d'un album, vue en cours de lecture, entrée et sortie
du plein écran. Non validés : dépôt réel de fichiers depuis l'Explorateur, curseurs
glissés à la souris, thème système lors d'un changement Windows, Linux.

## Phase 6 — Intégration Windows — 20 octobre 2026

**Frontière native.** `rap-platform::Desktop` emprunte l'identifiant portable de
la fenêtre principale. Les types HWND, WinRT et Shell restent dans `windows/`,
avec dépendances conditionnées par cible. Le moteur audio ne dépend toujours pas
de l'intégration. Le code Oxyde de fermeture et d'assemblage GPUI a été examiné,
sans copie ; compatibilité vérifiée sur les API de GPUI 0.2.2 conservées localement.

**Worker.** Un thread possède l'appartement WinRT MTA, les abonnements SMTC et
une fenêtre native cachée distincte de GPUI pour le menu et les messages Shell.
Les états de lecture sont coalescés dans une file d'une entrée ; les commandes
système empruntent une file bornée à 64 événements. Aucun callback natif ne
décode d'audio ni ne lit de fichier. Les pochettes viennent uniquement du cache
local RAP ; leur lecture Rust et leur transfert dans un flux mémoire WinRT
s'effectuent sur le worker (voir le correctif ci-dessous). Les métadonnées
ne sont renouvelées qu'au changement de piste/pochette ; `ClearAll` évite de
conserver l'image de la piste précédente. Position et durée sont converties en
ticks de 100 ns avec bornage. Les boutons Play et Pause sont explicites, tandis
que le menu de notification bascule l'état. Les touches multimédias passent par
SMTC, sans second raccourci global risquant de doubler la commande.

**Notification et fenêtre.** Clic gauche : restaurer ; clic droit : menu natif
FR/EN, piloté par les mêmes commandes que l'interface. L'icône est rastérisée
depuis le SVG Lucide officiel existant sur un fond d'accent. `TaskbarCreated`
réinstalle l'icône après un redémarrage d'Explorer. Un échec d'installation
interdit le masquage et demande la restauration de la fenêtre. Le réglage de
fermeture conserve Quitter par défaut ; Masquer et Demander sont persistants.
La confirmation propose Quitter, Masquer si disponible, Annuler ; Échap annule.
Quitter reste accessible dans les paramètres et dans le menu natif.

La fermeture explicite annule les travaux utilisateur, arrête le moteur,
attend le retrait des abonnements et de l'icône sur l'exécuteur d'arrière-plan,
puis termine la sauvegarde sur le worker de bibliothèque avant `cx.quit()`.
La fenêtre empruntée reste vivante pendant ce nettoyage. Une sauvegarde bloquée
ou en erreur est journalisée ; elle n'empêche pas de quitter. Les E/S sources
restent soumises aux délais du système, comme dans les phases précédentes.

**Démarrage.** L'état est relu depuis la valeur RAP de
`HKCU\Software\Microsoft\Windows\CurrentVersion\Run`, pas dupliqué dans JSON.
La lecture du réglage au démarrage ne modifie jamais le registre. Seule une
interaction avec l'option inscrit ou retire cette valeur. Le chemin de
l'exécutable est entouré de guillemets, sans shell et dans la limite Windows
de 260 caractères. Les valeurs des autres applications restent intactes.
Les choix Windows n'introduisent pas de dépendance native dans le code commun ;
sur une autre plateforme la fermeture quitte normalement. Linux reste non validé.

**Validation.** Windows MSVC : `cargo fmt --check`, `cargo check --workspace --locked`
et construction des exécutables réussis sans avertissement du projet ;
`CARGO_INCREMENTAL=0` évite l'avertissement de hard links propre au bac à sable.
Les 50 tests passent avec `--include-ignored`, dont sortie audio native, session
SMTC réellement interrogée (texte Unicode, lecture de la pochette, pause, piste
suivante, seek, changement de piste sans pochette), notification `TaskbarCreated`
simulée et suppression de la fenêtre de notification au nettoyage. L'inscription
au registre est testée sur une clé temporaire dédiée, sans modifier la clé Run.

Les cinq scénarios du smoke test passent : fermeture pendant la lecture,
immédiate, réduite, masquage/restauration et confirmation en anglais/clair/liste.
La capture de la confirmation a été contrôlée visuellement. La restauration
passe par le worker et `ShowWindowAsync`, sans appel de focus synchrone dans la
mise à jour GPUI : les premiers essais avaient révélé des erreurs de réentrance,
corrigées sans modifier GPUI. Les tests natifs ont été exécutés hors bac à sable,
celui-ci reproduisant le `STATUS_INVALID_HANDLE` connu du test audio.

Restent à vérifier manuellement : touches multimédias physiques, clics réels sur
le menu de notification (les événements et commandes natifs sont simulés dans
les tests), redémarrage réel d'Explorer et ouverture de session après activation
du démarrage automatique. Aucune fermeture de session ni relance d'Explorer de
l'utilisateur n'a été provoquée pour ces tests.

### Correctif des pochettes SMTC — 21 octobre 2026

`Library::connect` canonise les chemins du cache : sous Windows ils portent le
préfixe `\\?\`. Le passage direct de ce chemin à `StorageFile::GetFileFromPathAsync`
reproduit `0x800700A1 / UNABLE_TO_MASK_PATH`, y compris sur un chemin court ; le
message Windows évoquant une longueur excessive est trompeur dans ce cas.
Le test natif initial n'utilisait pas de chemin canonique et manquait cette erreur.

Le worker lit désormais le cache avec `std::fs::File` (lecture seule, 8 MiB au
maximum), puis crée un `InMemoryRandomAccessStream` pour SMTC. Aucun chemin n'est
transmis au parseur `StorageFile` et aucune copie temporaire n'est créée. Les
chemins longs restent acceptés ; on ne retire pas leur préfixe. Les fichiers
musicaux et leur contenu ne sont pas touchés.

Validation : reproduction de l'avertissement avant correction, puis test natif
SMTC réussi sur chemin canonique et chemin Unicode de plus de 260 caractères,
avec lecture effective du flux de pochette. Les cinq tests de `rap-platform`
passent, dont la limite de taille et le fichier manquant ; formatage et
`cargo check --workspace --locked` sans avertissement. La recherche des fenêtres
du test est limitée à son PID pour coexister avec un RAP déjà ouvert.

### Aperçu de barre des tâches — 21 octobre 2026

À la demande de l'utilisateur, ajout d'un aperçu compact au survol du bouton
RAP : pochette, titre, artiste, album, temps et progression ; quatre boutons
natifs Précédent, Lecture/Pause, Suivant et Arrêt. Cette intégration utilise
`ITaskbarList3` et DWM, séparément des contrôles SMTC des paramètres rapides.
Elle ne nécessite pas d'installateur. Le périmètre est ajouté au §10.1 et à la
phase 6 du cahier des charges.

`windows/taskbar/` sépare la sous-classe de fenêtre, la barre native, le rendu
compact, les DIB/GDI et les glyphes. Le modèle portable reçoit les quatre
couleurs du thème central de RAP ; les SVG Lucide existants sont rastérisés
sans changement de tracé. Les légendes et infobulles sont FR/EN. Une pochette
absente ou illisible donne un emplacement neutre. Les métadonnées longues sont
tronquées visuellement et le temps accepte les durées de plus d'une heure.

Le worker prépare une image 640×360, rafraîchie une fois par seconde et dès
qu'une piste, un état ou le thème change. Le cache PNG est lu et décodé sur ce
worker, avec limites de dimensions et de mémoire, seulement au changement de
chemin. La sous-classe du HWND principal est installée sur son thread UI, selon
le mécanisme de `SetWindowSubclass` examiné dans Oxyde (`windows_hover_window.rs`),
sans copie de code. Ses callbacks ne font qu'une copie/redimension GDI bornée
de l'image déjà préparée et l'envoi des commandes dans la file existante.
Ils ne lisent aucun fichier et n'attendent pas sur le verrou du cache d'image.
Chaque bitmap, DC, police et icône possède un nettoyage explicite.

Les boutons sont grisés sans piste ; Play devient Pause pendant la lecture.
`TaskbarButtonCreated` provoque le rétablissement des boutons lorsque Windows
recrée le bouton de barre des tâches. Un ajout prématuré avant l'affichage est
retenté ; une erreur de rendu rétablit l'aperçu Windows normal, sans arrêter
l'audio. La sous-classe conserve sa référence partagée jusqu'au détachement
exécuté sur le thread propriétaire, ou jusqu'à `WM_NCDESTROY`. Le nettoyage
normal termine avant la destruction de la fenêtre.

Validation native : Windows accepte la barre de boutons et le bitmap DWM ;
les quatre notifications `THBN_CLICKED` produisent les commandes attendues,
les libellés changent avec l'état/langue, les boutons sans piste sont désactivés
et la sous-classe libère sa référence au détachement. Les tests vérifient aussi
la taille demandée par DWM, l'opacité des pixels, la progression bornée et
l'effacement de la pochette au changement de piste. Une image PNG exportée a
été contrôlée visuellement. Les interactions physiques au survol de la barre
des tâches et le redémarrage réel d'Explorer restent à valider manuellement.

Validation finale Windows MSVC : 56 tests réussis avec `--include-ignored`,
construction des exécutables et `cargo check --workspace --locked` sans
avertissement, formatage vérifié. Le smoke test `-Taskbar -CloseScenario Minimized`
pilote les quatre commandes dans RAP à volume nul ; les captures de pause et
d'arrêt ont été contrôlées. Les scénarios Hidden (anglais/clair/liste) et Early
passent aussi, sans erreur dans les journaux.

## Phase 7 — Sorties audio locales — 21 octobre 2026

Périmètre réduit à la demande de l'utilisateur : sélection des sorties locales
et affichage du flux actif. Égaliseur, exclusif, bit-perfect et validation de
formats supplémentaires sont reportés hors V1. Le choix CPAL 0.16 existant est
conservé ; aucune dépendance n'est ajoutée. Les interactions `cx.listener` du
voisin Oxyde ont été examinées et restent compatibles avec GPUI 0.2.2 de RAP ;
aucun code ni composant de cette application n'a été copié.

`rap-audio::devices` décrit les périphériques et les résout par leur nom, seul
identifiant portable exposé par cette version de CPAL. Les doublons sont affichés
mais refusés à la sélection ; on ne choisit jamais le premier appareil d'un nom
ambigu. Une sortie renommée demande une nouvelle sélection. `None` désigne le
périphérique système par défaut, résolu à chaque ouverture de flux ; un changement
de sortie par défaut dans Windows ne migre pas un flux existant.

Un worker dédié découvre les sorties au démarrage, toutes les cinq secondes et
sur demande. Sa file est bornée ; l'énumération n'interrompt ni GPUI ni le décodage.
Les commandes de sélection rejoignent le worker audio existant. Celui-ci prépare
le nouveau lecteur, le flux et le convertisseur avant de remplacer la session,
puis reprend à la position consommée sur l'ancienne sortie. L'état lecture/pause,
la file, la répétition, l'aléatoire et le volume restent conservés. Un échec garde
l'ancienne session et sa préférence ; ses diagnostics sont distincts d'une panne
de lecture. Aucun nouveau travail n'est introduit dans le callback audio.

Le choix acquitté par le moteur est sauvegardé via le worker de configuration,
avec compatibilité des anciens JSON. Une préférence absente reste mémorisée et
signalée ; la lecture échoue explicitement sans repli silencieux. Une panne de
flux conserve la piste et retire ses caractéristiques actives ; une nouvelle
sélection et une relance permettent de récupérer. Il n'y a pas de reprise
automatique après déconnexion. Les sorties réseau restent une intégration
distincte à ajouter en phase 9, sans assimiler une destination DLNA à un nom CPAL.

L'interface FR/EN distingue la préférence, le périphérique actif et le format
envoyé par RAP (fréquence, canaux, format d'échantillons). Elle ne présente pas
ce format comme une mesure du signal après le mélangeur Windows ou le matériel.
Les informations actives disparaissent à l'arrêt, à la fin de la file et en
cas de panne. Les icônes Lucide déjà présentes sont réutilisées sans modification.

Validation Windows MSVC : `cargo fmt --check`, `cargo check --workspace --locked`
sans avertissement et 53 tests automatiques réussis. Deux tests audio natifs
supplémentaires passent à volume nul : sélection pendant la pause et la lecture,
refus sans perte de session, préférence absente sans repli, panne de callback
simulée et récupération, sortie effacée à la fin de la file. Ils sont exécutés
hors bac à sable à cause du `STATUS_INVALID_HANDLE` CPAL connu. Les tests de
configuration utilisent un répertoire temporaire dans `target/test-temp` pour
éviter les refus de remplacement de fichiers du dossier temporaire système.

Le smoke test `-AudioOutputs` clique dans les paramètres, vérifie la sauvegarde
du choix, revient au périphérique système et actualise la liste. Il passe en
français/sombre et anglais/clair/liste, avec contrôle visuel des deux préférences
et de la vue de lecture. `-AudioOutputRow` permet de choisir une autre ligne
locale dans la configuration testée. Les commandes de la vue de lecture restent
visibles avec les informations de sortie sur plusieurs lignes. Les essais ont
notamment changé entre les sorties casque et haut-parleurs présentes.

Une instance utilisateur verrouillant `target/debug/rap.exe`, la construction
de validation utilise le nom `rap-phase7.exe`, sans fermer cette instance ni
conserver de modification du manifeste. `-Executable` sélectionne cette copie
dans le smoke test. Les données de QA restent dans `target/native-qa-data`,
distinctes de la bibliothèque utilisateur. Le débranchement physique et Linux
restent non validés ; la déconnexion est couverte par le signal de panne simulé.

Validation utilisateur : le fonctionnement et la mise à jour à chaud de la liste
des périphériques de sortie sont confirmés après essai.

## Phase 9 — sorties DLNA / UPnP et Google Cast

La phase 8 reste reportée. `rap-network` rassemble les adaptateurs réseau sans
GPUI ni décodage local. `rap-audio` conserve sa file unique : sélection, précédent,
suivant, aléatoire et répétition utilisent les mêmes règles sur les deux sorties.
Le réseau n'est démarré qu'à la demande de recherche. SSDP et mDNS découvrent les
récepteurs sans les lancer ; la sélection explicite ouvre la session distante.
La SHIELD est une cible de validation, sans filtre sur la marque des récepteurs
Google Cast. Les composants GPUI et `cx.listener` du voisin Oxyde ont été examinés ;
les composants existants de RAP suffisent, sans copie de code ni nouvelle icône.

Le worker audio attend les acquittements hors du thread UI. Un runtime réseau
distinct utilise deux threads et au plus huit threads bloquants ; commandes,
découverte et téléchargements ont des files/concurrences bornées. Les commandes
réseau ont une limite globale de douze secondes, seize secondes côté appelant.
Le callback CPAL reste inchangé. Le volume reçu du matériel revient dans RAP ;
les déplacements et l'état suivent les acquittements, sans simuler une progression.
Une génération par chargement écarte les instantanés d'une ancienne piste.

Les deux protocoles utilisent un serveur HTTP sur l'interface IPv4 du récepteur,
avec port temporaire et UUID aléatoire par média. Seule l'IP choisie peut lire
la piste active ou sa pochette mise en cache ; aucun chemin de fichier n'est
accepté depuis le réseau. GET, HEAD, OPTIONS et plages d'octets sont pris en charge.
Huit corps de réponse au maximum, lus par blocs de 64 Kio. Remplacement, arrêt,
erreur, déconnexion et fermeture révoquent l'accès, y compris les flux en cours.
Il ne s'agit pas de l'API de contrôle de la phase 8. La musique reste en lecture
seule et aucun fichier temporaire n'est créé dans une source.

SSDP limite les interfaces, réponses et descriptions ; les descriptions XML et
réponses SOAP sont plafonnées à 1 MiB, sans DTD. Les URL UPnP doivent utiliser
HTTP et l'IP littérale du périphérique annonçant, sans proxy ni redirection.
AVTransport gère le transport ; RenderingControl le volume lorsqu'il est présent ;
ConnectionManager renseigne les types MIME acceptés. Les capacités manquantes ou
les formats refusés remontent une erreur. Aucun transcodage n'est introduit.

Google Cast utilise oxicast et le Default Media Receiver, avec reconnexion
automatique désactivée. Le lancement à froid peut durer plusieurs secondes.
L'acquittement LOAD de la SHIELD peut encore porter IDLE sans motif pendant
LOADING : RAP conserve alors la génération en Buffering. Seul FINISHED déclenche
l'enchaînement automatique ; CANCELLED arrête et ERROR signale l'échec. Le motif
de fin diffusé est conservé lorsque GET_STATUS renvoie ensuite une liste vide.
Une autre session ou un autre média détecté ne doit pas être piloté comme le nôtre.
La fermeture arrête uniquement la session de récepteur ouverte par RAP.

La sortie réseau vaut pour la session. La préférence CPAL reste sauvegardée.
Le retour local prépare le flux à la position distante avant de fermer la session
réseau, et conserve lecture/pause. Un échec de transfert depuis une sortie locale
laisse son flux utilisable. Une panne réseau ne démarre pas une sortie différente
ou une reconnexion automatique. L'interface indique la connexion en cours et
distingue les erreurs de découverte, de sélection et de lecture, en français/anglais.

Validation matérielle Windows : SHIELD découverte par mDNS puis WAV PCM et FLAC
synthétiques silencieux de douze secondes diffusés sans transcodage. Les exemples
`smoke_cast` et `network_session` vérifient chargement, lecture, pause, seek,
reprise, arrêt, précédent/suivant, répétition de piste à sa fin, boucle de file
et retour à une sortie CPAL en pause. Ils exigent un récepteur inactif avant de
commencer, utilisent un volume nul, restaurent le volume initial et ferment leur
session. Des délais d'attente de chargement ont aussi été observés : une relance
explicite permet de récupérer, avec diagnostic de la commande concernée.

Le DLNA est validé avec un récepteur SOAP simulé et un vrai échange HTTP local ;
aucun récepteur DLNA physique n'était disponible. Les tests couvrent aussi plages
d'octets, autorisations, révocation, conservation des sources, découverte sans
filtre de marque, URL UPnP, XML et états Cast. Les autres modèles Cast, formats
matériels, coupures physiques du Wi-Fi, réseaux IPv6 et Linux restent non validés.
Le réseau initial est limité aux IPv4 privées/de lien local et à une sortie active.

La validation GUI utilise une copie `rap-phase9.exe`, car l'instance utilisateur
verrouille l'exécutable habituel. Le manifeste est restauré après construction ;
les données et médias de test restent isolés dans `target/`. Le smoke test
`-NetworkOutputs` recherche réellement les appareils et capture les paramètres.
La création des bibliothèques d'importation par MSVC peut produire un diagnostic
`linker_messages` avec les dépendances TLS natives ; `cargo check` reste sans
avertissement et aucun avertissement n'est masqué.

`cargo fmt --check` et `cargo check --workspace --locked` passent. Le workspace
compte 63 tests automatiques réussis et sept tests natifs ignorés par défaut.
Les trois tests supplémentaires de `rap-audio` passent lorsqu'ils sont lancés
explicitement (codecs générés et deux scénarios de sorties CPAL). Les quatre
tests natifs de `rap-platform`, inchangés, n'ont pas été relancés pour ce jalon.
Les captures contrôlées montrent la SHIELD dans les paramètres en français/sombre
et anglais/clair/liste ; les deux scénarios GUI se ferment sans erreur.
