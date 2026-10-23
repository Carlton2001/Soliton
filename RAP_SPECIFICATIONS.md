# RAP — Rust Audio Player
## Cahier des charges fonctionnel et technique

**Version du document :** 0.1  
**Statut :** Spécifications initiales  
**Licence du projet :** GPL-3.0-only  
**Langage :** Rust  
**Interface graphique :** GPUI  
**Plateformes :** Windows en priorité, Linux ultérieurement

---

# 1. Présentation du projet

## 1.1 Objectif

RAP (Rust Audio Player) est un lecteur audio de bureau développé en Rust avec le framework graphique GPUI.

L'application est destinée aux utilisateurs possédant une bibliothèque musicale locale structurée et souhaitant profiter de leur collection dans une interface moderne, rapide, élégante et simple.

RAP privilégie trois objectifs fondamentaux :

1. **La bibliothèque musicale :** organisation, exploration et présentation des albums.
2. **La qualité audio :** lecture haute fidélité et gestion des périphériques. Le mode exclusif et la restitution bit-perfect sont hors du périmètre V1 retenu.
3. **La simplicité :** interface épurée, sans accumulation de fonctionnalités secondaires.

RAP s'inspire de la richesse fonctionnelle de MusicBee, mais cherche à éviter sa complexité visuelle et ses nombreuses options peu utiles.

## 1.2 Principes directeurs

- Application native et performante.
- Fonctionnement local, sans compte utilisateur ni service cloud obligatoire pour la bibliothèque et la lecture locale. La sortie Google Cast optionnelle peut dépendre des services du récepteur Google ; ne pas garantir son fonctionnement hors Internet sans validation.
- Bibliothèque musicale strictement en lecture seule.
- Aucune modification des fichiers musicaux.
- Métadonnées et pochettes exclusivement issues des fichiers locaux.
- Architecture modulaire et extensible.
- Interface indépendante du moteur audio.
- Compatibilité Windows et Linux prévue dès la conception.
- Dépendances Rust pures privilégiées.
- Absence de télémétrie.
- Pas de fonctionnalités sociales.
- Pas de gestion des achats ni de services de streaming commercial.
- Architecture multilangue, Français et Anglais pour commencer.

## 1.3 Priorités

La première version doit être une application réellement utilisable, et non une simple démonstration technique.

L'objectif est d'obtenir rapidement :

- Une bibliothèque musicale fonctionnelle.
- Une interface GPUI complète et cohérente.
- Une lecture audio locale fiable.
- Une navigation fluide.
- Des paramètres persistants.
- Une base technique permettant d'ajouter progressivement les fonctionnalités avancées.

---

# 2. Plateformes et distribution

## 2.1 Windows

Windows constitue la plateforme principale de développement et de validation initiale.

Distribution prévue :

- Installateur Windows.
- Archive ZIP portable.
- Exécutable natif.
- Aucune installation séparée de runtime nécessaire dans la mesure du possible.

Le format exact de l'installateur sera déterminé ultérieurement.

## 2.2 Linux

Linux doit être pris en compte dans l'architecture dès le départ.

Cependant, la compilation et la validation Linux ne sont pas exigées à chaque étape initiale.

Les fonctionnalités spécifiques au système doivent être isolées derrière des abstractions.

Formats de distribution Linux à déterminer ultérieurement.

## 2.3 Mises à jour

RAP doit pouvoir vérifier la disponibilité d'une nouvelle version.

Comportement souhaité :

- Vérification configurable.
- Notification lorsqu'une version plus récente existe.
- Affichage de la version disponible.
- Ouverture de la page de téléchargement.
- Pas d'installation automatique dans la V1.

Le fournisseur de mises à jour reste à confirmer. GitHub Releases constitue une possibilité.

---

# 3. Bibliothèque musicale

## 3.1 Sources musicales

RAP doit accepter plusieurs dossiers sources.

Sources prises en charge :

- Disques internes.
- Disques externes.
- Partages réseau SMB.
- NAS accessibles via le système de fichiers.

Les sources réseau doivent être traitées comme des chemins de fichiers ordinaires lorsqu'elles sont montées ou accessibles par le système.

RAP n'a pas besoin d'implémenter directement un client SMB dans la V1.

## 3.2 Taille de bibliothèque

Volume initial ciblé : moins de 10 000 pistes.

L'architecture ne doit cependant pas introduire de limitation artificielle.

Les listes et grilles doivent utiliser des mécanismes de virtualisation lorsque cela est pertinent.

Les opérations lourdes ne doivent jamais bloquer le thread GPUI.

## 3.3 Base de données

Utilisation recommandée : SQLite avec `rusqlite`.

La base conserve notamment :

- Sources musicales.
- Chemins des fichiers.
- Métadonnées extraites.
- Caractéristiques audio.
- Albums et artistes.
- Relations entre pistes et éditions.
- Références vers les pochettes mises en cache.
- État de disponibilité des fichiers.
- Informations nécessaires à l'indexation incrémentale.

La base doit être versionnée avec un système de migrations.

Les données peuvent être reconstruites à partir des fichiers musicaux, à l'exception des paramètres propres à RAP et des playlists personnelles.

## 3.4 Lecture seule

RAP ne doit jamais :

- Modifier les tags audio.
- Réécrire les fichiers musicaux.
- Déplacer les pistes.
- Renommer les fichiers.
- Supprimer les fichiers de la collection.
- Modifier les pochettes intégrées.

Les opérations de lecture doivent utiliser des accès non destructifs.

Les fichiers temporaires, caches et bases de données doivent être stockés en dehors des dossiers musicaux.

---

# 4. Métadonnées

## 4.1 Source principale

Les métadonnées proviennent exclusivement des tags intégrés aux fichiers.

Bibliothèque Rust recommandée : `lofty`.

Métadonnées principales :

- Titre.
- Artiste.
- Artiste de l'album.
- Album.
- Année ou date.
- Genre.
- Numéro de piste.
- Numéro de disque.
- Durée.
- Informations techniques du fichier.

Les tags doivent être interprétés en tenant compte des différents standards et formats.

## 4.2 Métadonnées manquantes

Lorsque certaines informations sont absentes, RAP peut utiliser l'arborescence des dossiers comme source de secours.

Exemple :

`Musique/Artiste/Album/01 - Titre.flac`

RAP peut en déduire :

- Artiste.
- Album.
- Numéro de piste.
- Titre.

Ces informations sont calculées uniquement dans la base de RAP.

Aucune correction n'est écrite dans les fichiers.

Les métadonnées extraites des tags doivent être distinguées des métadonnées déduites.

## 4.3 Métadonnées en ligne

Aucun enrichissement en ligne n'est prévu.

RAP ne doit pas interroger automatiquement :

- MusicBrainz.
- Discogs.
- Last.fm.
- Services de récupération de pochettes.
- Autres bases musicales distantes.

La bibliothèque doit fonctionner intégralement hors ligne.

---

# 5. Albums et éditions

## 5.1 Modèle logique

RAP doit distinguer :

- Le fichier physique.
- La piste musicale.
- L'édition d'un album.
- L'album regroupé.
- L'artiste.

Cette distinction est essentielle pour gérer plusieurs versions d'un même album.

## 5.2 Éditions multiples

Exemple :

Un utilisateur possède :

- Un album en FLAC 16 bits / 44,1 kHz.
- Le même album en FLAC 24 bits / 96 kHz.

RAP doit pouvoir les présenter comme deux éditions d'un même album.

L'utilisateur doit pouvoir sélectionner l'édition souhaitée.

## 5.3 Identification des éditions

Les critères possibles comprennent :

- Artiste de l'album.
- Titre normalisé.
- Année.
- Organisation des dossiers.
- Nombre de pistes.
- Numérotation des disques.
- Caractéristiques audio.

Le regroupement doit rester prudent.

Dans un même dossier et pour un même titre d’album renseigné, un tag d’artiste
d’album manquant ne crée pas une édition supplémentaire si un seul artiste
d’album explicite existe et que tous les morceaux sans ce tag portent ce même
artiste dans leurs tags. Les compilations et les cas contradictoires ou déduits
restent séparés. Cette réconciliation utilise les métadonnées stockées dans RAP
et ne modifie aucun tag source.

En cas d'ambiguïté, il vaut mieux présenter deux albums distincts plutôt que fusionner incorrectement des éditions.

Aucune suppression ou modification des fichiers ne doit intervenir.

## 5.4 Compilations

RAP doit distinguer l'artiste de l'album de l'artiste de chaque piste.

Les compilations doivent pouvoir apparaître comme un album unique même lorsque les pistes ont des artistes différents.

---

# 6. Pochettes

## 6.1 Sources

Les pochettes proviennent exclusivement des images intégrées aux fichiers audio.

RAP ne doit pas rechercher automatiquement `folder.jpg`, `cover.jpg` ou d'autres images externes.

Aucun téléchargement de pochette n'est prévu.

## 6.2 Extraction

Les images embarquées doivent être extraites lors de l'indexation ou à la demande.

Une pochette commune peut être utilisée pour toutes les pistes d'un même album lorsque cela est cohérent.

## 6.3 Cache

RAP doit conserver un cache local des pochettes.

Objectifs :

- Éviter les lectures répétées des fichiers audio.
- Accélérer l'affichage des grilles.
- Réduire les accès au NAS.
- Permettre l'affichage des albums lorsque la source est déconnectée.

Prévoir des miniatures adaptées aux différentes tailles d'affichage.

Le cache doit être invalidé lorsque les images sources changent.

## 6.4 Pochettes absentes

En l'absence de pochette intégrée, afficher un visuel neutre cohérent avec le thème de RAP.

---

# 7. Indexation et surveillance

## 7.1 Première analyse

Lors de l'ajout d'une source :

1. Parcourir récursivement les dossiers.
2. Identifier les fichiers audio reconnus.
3. Extraire les métadonnées.
4. Extraire les caractéristiques techniques.
5. Identifier les pochettes.
6. Construire les albums et éditions.
7. Enregistrer les résultats dans SQLite.
8. Actualiser progressivement l'interface.

L'analyse doit s'effectuer en arrière-plan.

Une progression doit être affichée.

## 7.2 Analyses suivantes

RAP doit effectuer une vérification au démarrage.

L'indexation doit être incrémentale afin d'éviter de relire systématiquement tous les fichiers inchangés.

Les critères de détection peuvent inclure :

- Chemin.
- Taille.
- Date de modification.
- Informations conservées lors de la précédente analyse.

## 7.3 Surveillance temps réel

Bibliothèque candidate : `notify`.

RAP doit surveiller les dossiers musicaux lorsque le système le permet.

Les événements doivent être regroupés pour éviter les réindexations répétées.

Les notifications réseau n'étant pas toujours fiables, une analyse périodique ou manuelle doit pouvoir compléter la surveillance.

## 7.4 Sources indisponibles

Lorsqu'un disque ou NAS devient inaccessible :

- Les albums restent visibles.
- Les métadonnées restent disponibles.
- Les pochettes en cache restent affichées.
- Les pistes sont signalées comme indisponibles.
- La lecture est désactivée pour les fichiers inaccessibles.

Une source temporairement indisponible ne doit jamais entraîner la suppression automatique de son contenu dans la base.

La disparition définitive d'un fichier ne peut être validée qu'après vérification de l'accessibilité de sa source.

---

# 8. Interface graphique

## 8.1 Technologie

Framework : GPUI.

L'interface doit rester indépendante des traitements audio, réseau et d'indexation.

## 8.2 Style

Direction graphique :

- Minimaliste.
- Sobre.
- Moderne.
- Proche des applications natives.
- Mise en valeur des pochettes.
- Typographie lisible.
- Espacements cohérents.
- Animations discrètes.

Éviter les panneaux inutiles et les interfaces excessivement configurables.

## 8.3 Organisation générale

L'interface principale comporte :

- Une navigation explicite et toujours visible sous la barre de titre, avec icônes et libellés.
- Une zone centrale de bibliothèque.
- Une recherche globale.
- Des filtres contextuels.
- Une barre de lecture permanente en bas.

Sections principales :

- Accueil.
- Albums.
- Artistes.
- Playlists manuelles.
- File de lecture.
- Paramètres.

Genres et années sont accessibles par les filtres de la vue Albums.

La vue Accueil, placée avant Artistes dans la navigation, propose trois panneaux
simultanés : artistes, albums de l’artiste sélectionné, puis pistes de l’album
sélectionné. Les listes défilent indépendamment. Les actions de lecture, les
éditions, la recherche et les filtres réutilisent les contrôles existants. La
navigation principale et le lecteur restent visibles. Les couleurs et les
contrôles ne changent pas. La dernière sélection est restaurée au redémarrage.
Les colonnes Artistes et Albums sont dimensionnées automatiquement au contenu
pour afficher les noms complets sur une ligne. La largeur des albums s’adapte
aux albums visibles de l’artiste sélectionné, y compris après filtrage.
Elles sont incompressibles et
ne disposent pas de redimensionnement manuel. La colonne des pistes occupe
l’espace restant et permet un défilement horizontal indépendant si nécessaire.
Les ascenseurs verticaux restent au bord droit de chaque colonne. Les sélections
occupent la largeur disponible entre les marges intérieures. L’en-tête décoratif
« Votre collection / Choisir un disque » n’est pas affiché sur Accueil.

## 8.4 Vue Albums

Deux modes :

**Grille**

- Pochettes.
- Titres.
- Artistes.
- Navigation visuelle.

**Liste**

- Petites pochettes.
- Titres.
- Artistes.
- Années.
- Genres.
- Informations techniques utiles.

Le choix d'affichage doit être mémorisé.

## 8.5 Vue Artistes

Plusieurs modes de présentation.

Au minimum :

- Liste alphabétique.
- Vue permettant de consulter les albums d'un artiste.

L'architecture doit permettre d'ajouter une grille d'artistes ultérieurement.

## 8.6 Fiche album

La fiche album doit présenter :

- Grande pochette.
- Titre.
- Artiste.
- Année.
- Genre.
- Durée totale.
- Nombre de pistes.
- Formats et caractéristiques audio.
- Liste des pistes avec durées.
- Éditions disponibles.

La qualité audio doit être affichée par piste si elle diffère au sein d'un album.

## 8.7 Recherche

Recherche globale instantanée.

Champs concernés :

- Artistes.
- Albums.
- Titres.
- Genres.

Prévoir des filtres combinables :

- Genre.
- Année ou période.
- Artiste.
- Format.
- Qualité audio.

Les filtres doivent rester discrets lorsqu'ils ne sont pas utilisés.

## 8.8 Thèmes

Prévoir :

- Thème clair.
- Thème sombre.
- Couleur d'accent configurable.

Centraliser les couleurs, espacements et styles dans un système de design cohérent.

## 8.9 Lecture

Barre de lecture permanente avec :

- Pochette.
- Titre.
- Artiste.
- Lecture/Pause.
- Piste précédente/suivante.
- Progression.
- Volume.
- Aléatoire.
- Répétition.

Simplification retenue le 23 octobre 2026 : la file de lecture possède une page
dédiée, accessible par la navigation principale. Le tiroir de file et la vue de
session séparée sont supprimés. F11 affiche la page active en plein écran en
conservant la navigation et la barre de lecture. Les actions comportent un libellé,
sauf les commandes usuelles du lecteur (précédent, lire/pause, suivant, aléatoire,
répéter et couper/rétablir le son) et Lire/Pause dans les listes de pistes : ces
icônes disposent d'une infobulle. Les actions secondaires des morceaux sont
réunies dans un menu « Actions ».

## 8.10 Interactions

Interactions indispensables :

- Double-clic pour lancer la lecture.
- Navigation fluide au clavier et à la souris.

RAP gère une bibliothèque : il ne propose ni lecture ponctuelle de fichiers hors bibliothèque, ni glisser-déposer. Les dossiers s'ajoutent depuis les paramètres.

---

# 9. Moteur audio

## 9.1 Architecture

Le moteur audio doit fonctionner indépendamment de GPUI.

Il reçoit des commandes et publie des événements.

Commandes principales :

- Play.
- Pause.
- Stop.
- Seek.
- Next.
- Previous.
- SetVolume.
- SetOutputDevice.
- SetShuffle.
- SetRepeatMode.
- SetEqualizer (évolution ultérieure, hors V1).

Événements principaux :

- PlaybackStarted.
- PlaybackPaused.
- PlaybackStopped.
- TrackChanged.
- PositionChanged.
- OutputChanged.
- PlaybackError.

Le décodage et les opérations de sortie ne doivent pas bloquer le thread graphique.

## 9.2 Formats ciblés

Formats souhaités :

- MP3.
- FLAC.
- AAC / M4A.
- OGG Vorbis.
- Opus.
- WAV.
- AIFF.
- WMA.
- APE / Monkey's Audio.

Le support effectif dépendra des décodeurs retenus.

La V1 doit prioriser les formats correctement pris en charge par les bibliothèques Rust sélectionnées.

WMA et APE pourront être reportés si leur prise en charge nécessite des dépendances natives importantes.
La phase 7 n'ajoute ni ne valide de formats supplémentaires ; elle conserve les formats déjà pris en charge.

## 9.3 Décodage

Bibliothèque candidate : `symphonia`.

Prévoir une abstraction permettant d'ajouter ultérieurement d'autres décodeurs.

Ne pas introduire obligatoirement FFmpeg dans la première version.

## 9.4 Sortie audio

Bibliothèque candidate : `cpal` pour les sorties classiques.

Prévoir une abstraction des périphériques et modes de sortie.

Fonctions souhaitées :

- Sélection du périphérique.
- Gestion du volume.
- Détection des capacités.
- Sortie partagée.
- Sortie exclusive et restitution bit-perfect : évolutions ultérieures, hors V1.

## 9.5 Modes audio

**Mode standard**

- Compatibilité maximale.
- Volume logiciel.
- Égaliseur : hors V1.
- Conversion audio si nécessaire.

**Mode audiophile**

Évolution ultérieure, hors du périmètre V1 retenu le 21 octobre 2026.

- Priorité à l'absence de modification du signal.
- Égaliseur désactivé.
- Pas de conversion inutile.
- Mode exclusif lorsque disponible.
- Indication des caractéristiques réelles de sortie.

Le statut bit-perfect ne doit pas être annoncé sans justification technique.

## 9.6 Égaliseur

Évolution ultérieure, hors du périmètre V1 retenu le 21 octobre 2026 : égaliseur audio désactivable.

Il doit fonctionner uniquement dans un chemin de traitement compatible.

L'activation de l'égaliseur est incompatible avec une restitution strictement bit-perfect.

## 9.7 Comportement de lecture

Fonctions :

- Lecture d'un album.
- Lecture d'une sélection de pistes.
- Lecture aléatoire.
- Répétition d'une piste.
- Répétition d'un album ou de la sélection.
- Navigation entre pistes.

Pas de système complexe de playlists dans la V1.

Ajout retenu le 23 octobre 2026 : playlists manuelles nommées, création,
renommage, suppression, ajout de pistes ou d'une édition d'album, retrait et
réorganisation. La file de lecture est consultable et modifiable, avec ajout
après la piste courante ou en fin de file et enregistrement en playlist.
Une playlist et une file ne contiennent qu'une occurrence de chaque fichier :
les ajouts répétés sont ignorés sans déplacer la piste existante. Les fichiers
distincts d'un même titre ou d'une autre édition restent séparés. La reprise
d'une ancienne file supprime ses doublons en conservant le fichier sélectionné.
Les playlists résident dans SQLite, hors des sources musicales ; une piste
indisponible ou retirée du catalogue reste visible dans sa playlist.
Les playlists automatiques et l'import/export ne font pas partie de cet ajout.

La lecture gapless n'est pas prioritaire.

## 9.8 Persistance

RAP peut conserver :

- Dernière piste sélectionnée.
- Dernière position de lecture.
- Volume.
- Périphérique de sortie préféré.
- Mode de répétition.
- Mode aléatoire.

Ces données sont des paramètres de session et non un historique d'écoute.

Aucune statistique d'écoute n'est demandée.

---

# 10. Intégration système

## 10.1 Windows

Prévoir :

- Contrôles multimédias SMTC.
- Affichage de la pochette dans les contrôles système.
- Vignette personnalisée au survol de la barre des tâches : pochette, titre,
  artiste, album et progression, avec boutons Précédent, Lecture/Pause,
  Suivant et Arrêt. La progression affichée dans cette vignette est indicative.
- Gestion des touches multimédias.
- Icône de notification.
- Démarrage automatique optionnel.

## 10.2 Linux

Prévoir ultérieurement :

- Intégration MPRIS.
- Contrôle via D-Bus.
- Icône de notification lorsque l'environnement le permet.
- Démarrage automatique adapté à l'environnement.

## 10.3 Fermeture

Comportement configurable :

- Fermer et arrêter la lecture.
- Masquer la fenêtre et continuer la lecture.
- Demander confirmation.

Prévoir une commande explicite permettant de quitter complètement RAP.

## 10.4 Zone de notification

Menu minimal :

- Afficher RAP.
- Lecture/Pause.
- Précédent.
- Suivant.
- Quitter.

La zone de notification ne doit pas être la seule manière de récupérer une fenêtre masquée si elle n'est pas disponible.

---

# 11. Diffusion réseau

## 11.1 DLNA / UPnP

RAP doit pouvoir découvrir et contrôler des périphériques compatibles.

Fonctions visées :

- Découverte des appareils.
- Sélection d'une sortie réseau.
- Lecture.
- Pause.
- Arrêt.
- Navigation.
- Contrôle du volume lorsque disponible.
- Gestion de plusieurs périphériques.

Une architecture de sortie réseau distincte des sorties locales est recommandée.

La phase 9 couvre également Google Cast / Chromecast (§11.5), avec une implémentation distincte du contrôle DLNA / UPnP et une sélection de sortie commune dans RAP.

## 11.2 Serveur de diffusion

RAP doit pouvoir exposer les fichiers musicaux autorisés via HTTP sur le réseau local pour les récepteurs DLNA / UPnP et Google Cast.

Prévoir :

- Serveur HTTP intégré.
- Accès limité aux contenus explicitement autorisés.
- Gestion des formats acceptés par les appareils.
- Protection contre les accès arbitraires aux fichiers.
- Gestion des interruptions réseau.

Le transcodage pourra être ajouté ultérieurement.

La lecture initiale privilégie les fichiers directement acceptés par le récepteur. Un format non pris en charge doit être signalé explicitement ; la prise en charge locale d'un format ne garantit pas sa lecture sur une sortie réseau.

## 11.3 Plusieurs appareils

La gestion de plusieurs appareils ne doit pas être confondue avec une synchronisation multiroom parfaite.

La synchronisation audio stricte entre plusieurs enceintes est une fonctionnalité avancée, hors périmètre initial.

## 11.4 Enceintes Bluetooth, dont Amazon Echo

Le 21 octobre 2026, l'utilisateur a abandonné la piste Alexa Wi-Fi / skill et
demandé la connexion Bluetooth. Le prototype Wi-Fi n'est pas intégré à RAP et
ne doit pas être démarré pour le parcours Bluetooth.

Sous Windows, RAP ouvre les paramètres Bluetooth natifs pour ajouter, appairer
ou reconnecter une enceinte ou un casque. Windows gère la découverte, les
confirmations d'appairage et les codecs. RAP réutilise les sorties audio CPAL
exposées par le système, leur actualisation en arrière-plan et leur sélection
persistée. Aucune connexion à Amazon, skill, passerelle distante ou souscription
n'est requise par RAP pour cette sortie.

La liste Bluetooth doit inclure les appareils déjà appairés même déconnectés,
avec leur état et une action « Connecter dans Windows » ouvrant les paramètres
natifs. Cette action ne doit pas prétendre établir la connexion elle-même.
L'énumération et son actualisation périodique s'effectuent hors du thread UI.
Les sorties audio sont associées aux appareils par leur identifiant de conteneur
Windows ; aucun rapprochement approximatif de noms ne doit choisir une sortie.
Une sortie associée n'est sélectionnable que si le moteur la voit disponible et
non ambiguë. Les appareils déconnectés restent listés ; leurs sorties absentes
ne deviennent pas sélectionnables pour autant.

Le périphérique doit accepter l'audio Bluetooth du PC (A2DP). La découverte
d'un appareil Alexa sur le Wi-Fi ou en Bluetooth ne garantit pas cette capacité.
RAP ne sélectionne pas automatiquement le nouvel appareil et ne change pas le
périphérique audio par défaut de Windows. Les échecs de sélection conservent
la sortie fonctionnelle précédente ; une sortie absente reste mémorisée.

La sortie Bluetooth utilise le codec négocié par Windows et le périphérique.
Ne garantir ni lossless ni bit-perfect ; le format PCM affiché par RAP précède
le transport Bluetooth et n'indique pas son codec. Une seule sortie est active ;
pas de synchronisation multiroom ni de couplage stéréo entre Echo géré par RAP.
L'entrée Bluetooth d'une Echo n'est pas redistribuée à la seconde Echo et à
l'Echo Sub de son ensemble Alexa ; la sortie cible uniquement l'Echo connectée.
Les textes d'aide et les actions doivent être disponibles en français et anglais.

Références : [paramètres Windows](https://learn.microsoft.com/en-us/windows/apps/develop/launch/launch-settings),
[audio Bluetooth Windows](https://learn.microsoft.com/en-us/windows-hardware/drivers/bluetooth/bluetooth-classic-audio),
[appairage Echo](https://www.aboutamazon.com/news/devices/connect-alexa-bluetooth).

## 11.5 Google Cast / Chromecast

Ajout au périmètre de la phase 9 à la demande de l'utilisateur le 21 octobre 2026.

La NVIDIA SHIELD constitue une cible de validation explicite, via son récepteur Google Cast intégré, sans application DLNA intermédiaire à installer. Aucune marque ou modèle de Smart TV n'est imposé.

Fonctions visées :

- Découverte des récepteurs Google Cast disponibles sur le réseau local.
- Sélection dans RAP avec identification du protocole et distinction des sorties locales et DLNA / UPnP.
- Démarrage d'une session de lecture et envoi des médias locaux autorisés.
- Lecture, pause, arrêt, navigation et changement de position lorsque pris en charge.
- Consultation de l'état et de la progression réels du récepteur.
- Contrôle du volume lorsque disponible.
- Gestion des déconnexions, refus de session et formats incompatibles.
- Transmission des métadonnées et pochettes locales lorsque le récepteur le permet.

La couche de contrôle Cast est distincte de celle de DLNA / UPnP ; le serveur de diffusion local et les modèles de présentation peuvent être partagés. Cette diffusion ne dépend pas de l'API de contrôle reportée en phase 8.

Le choix du client Cast compatible avec Rust / Windows, du récepteur et des dépendances doit être validé avant implémentation. Le récepteur média par défaut de Google est une option à étudier, sans imposer d'application réceptrice RAP personnalisée. La compatibilité doit être testée sur la SHIELD ; elle ne constitue pas une garantie pour tous les appareils Chromecast, Google TV ou les groupes d'enceintes. La synchronisation multiroom reste hors périmètre initial.

Références : [Google Cast sur SHIELD](https://support-shield.nvidia.com/shield-tv-pro-user-guide/Google_Cast.htm), [API Google Cast](https://developers.google.com/cast/docs/reference), [récepteurs média](https://developers.google.com/cast/docs/web_receiver), [formats acceptés](https://developers.google.com/cast/docs/media).

---

# 12. API locale

Fonctionnalité reportée à la demande de l'utilisateur le 21 octobre 2026, hors du périmètre V1 retenu. La phase 9 utilise son propre service de diffusion réseau et ne dépend pas de cette API de contrôle.

## 12.1 Objectif

Permettre à d'autres applications locales de contrôler RAP.

Technologie candidate : `axum`.

## 12.2 Fonctions envisagées

- Consulter l'état de lecture.
- Démarrer la lecture.
- Mettre en pause.
- Arrêter.
- Changer de piste.
- Modifier le volume.
- Consulter les albums.
- Consulter les périphériques.
- Sélectionner une sortie.

## 12.3 Sécurité

L'API doit écouter exclusivement sur l'interface locale par défaut.

Adresses recommandées :

- `127.0.0.1`.
- Éventuellement `::1`.

Aucune exposition publique par défaut.

Si un accès distant est ajouté, il devra disposer de mécanismes d'authentification et d'autorisation appropriés.

L'API de contrôle et le serveur de diffusion DLNA / Google Cast doivent être considérés comme deux services distincts.

---

# 13. Architecture Rust

## 13.1 Workspace Cargo

Organisation initiale recommandée :

```text
RAP/
├── Cargo.toml
├── Cargo.lock
├── AGENTS.md
├── LICENSE
├── README.md
├── crates/
│   ├── rap-app/
│   ├── rap-core/
│   ├── rap-library/
│   ├── rap-audio/
│   ├── rap-network/
│   └── rap-platform/
├── assets/
├── docs/
└── tests/
```

## 13.2 rap-app

Responsabilités :

- Démarrage de l'application.
- Interface GPUI.
- Navigation.
- Composants graphiques.
- Thèmes.
- Gestion des interactions.
- Présentation des données.
- Assemblage des services.

## 13.3 rap-core

Responsabilités :

- Types métier communs.
- Contrats entre modules.
- Commandes.
- Événements.
- Configuration.
- Gestion des erreurs communes.

Cette crate ne doit pas dépendre de GPUI.

## 13.4 rap-library

Responsabilités :

- Sources musicales.
- Parcours des dossiers.
- Extraction des tags.
- Indexation.
- Base SQLite.
- Cache des pochettes.
- Recherche.
- Regroupement des albums.
- Détection des modifications.

## 13.5 rap-audio

Responsabilités :

- Décodage.
- Pipeline audio.
- Buffers.
- Contrôle de lecture.
- Périphériques.
- Volume.
- Égaliseur (évolution ultérieure, hors V1).
- Gestion des erreurs audio.

## 13.6 rap-platform

Responsabilités :

- Fonctions spécifiques Windows.
- Fonctions spécifiques Linux.
- Intégration multimédia.
- Zone de notification.
- Démarrage automatique.
- Adaptations système.

## 13.7 Réseau

Les fonctionnalités réseau pourront initialement être organisées en modules dédiés.

La phase 9 introduit `rap-network` pour la découverte DLNA / Google Cast, le contrôle des récepteurs et la diffusion HTTP des seuls médias autorisés. Le moteur `rap-audio` conserve la file de lecture et l'arbitrage entre une sortie locale et une sortie réseau active.

Éviter de créer des crates sans responsabilité réelle.

---

# 14. Dépendances candidates

| Domaine | Solution envisagée |
|---|---|
| Interface | gpui |
| Métadonnées | lofty |
| SQLite | rusqlite |
| Décodage | symphonia |
| Audio | cpal |
| Surveillance | notify |
| Sérialisation | serde |
| HTTP | axum |
| Asynchrone | tokio |
| Logs | tracing |
| Linux MPRIS | zbus |

Ces dépendances ne sont pas définitivement validées.

Codex devra vérifier avant intégration :

- Versions disponibles.
- Maintenance.
- Compatibilité Windows.
- Perspectives Linux.
- Licences.
- Fonctionnalités réellement prises en charge.
- Dépendances natives éventuelles.

Les bibliothèques Rust pures sont à privilégier lorsque cela reste raisonnable.

---

# 15. Réutilisation d'Oxyde

RAP peut reprendre des composants ou utilitaires du projet Oxyde GPUI.

Éléments potentiellement réutilisables :

- Système de thèmes.
- Composants GPUI génériques.
- Dialogues de sélection.
- Gestion des paramètres.
- Utilitaires de chemins.
- Journalisation.
- Composants de listes.
- Gestion de tâches asynchrones.

La réutilisation doit rester sélective.

RAP ne doit pas dépendre directement de l'application Oxyde.

Il faut privilégier la copie adaptée ou l'extraction de composants réellement génériques, en respectant leurs licences et attributions.

Aucune réutilisation ne doit être décidée sans examiner le code concerné.

---

# 16. Performances et robustesse

## 16.1 Interface

Le thread GPUI ne doit jamais effectuer directement :

- Scan récursif important.
- Lecture massive de métadonnées.
- Décodage audio.
- Accès réseau bloquant.
- Traitement coûteux des pochettes.

## 16.2 Bibliothèque

L'indexation doit :

- Être incrémentale.
- Éviter les accès inutiles.
- Pouvoir être interrompue.
- Tolérer les fichiers corrompus.
- Continuer lorsqu'un fichier individuel échoue.
- Fournir une progression.

## 16.3 Audio

Le moteur doit :

- Éviter les interruptions dues à l'interface.
- Utiliser des buffers adaptés.
- Gérer les périphériques déconnectés.
- Signaler les erreurs sans faire planter l'application.
- Préserver la réactivité de l'interface.

## 16.4 Réseau

Les déconnexions NAS, DLNA ou Google Cast ne doivent pas bloquer RAP.

Prévoir des délais d'attente et une gestion explicite des erreurs.

---

# 17. Tests

## 17.1 Stratégie

Les tests unitaires sont prioritaires sur les modules critiques.

Domaines à tester :

- Extraction des métadonnées.
- Reconstruction depuis les chemins.
- Regroupement des albums.
- Gestion des éditions.
- Indexation incrémentale.
- Disponibilité des sources.
- Commandes du moteur audio.
- États de lecture.
- Gestion des erreurs.

## 17.2 Jeux de données

Prévoir des fixtures comprenant :

- Albums complets.
- Tags incomplets.
- Compilations.
- Plusieurs éditions.
- Pochettes intégrées.
- Fichiers sans pochette.
- Fichiers corrompus.
- Formats différents.
- Sources temporairement indisponibles.

## 17.3 Validation manuelle

Tester notamment :

- Première indexation.
- Redémarrage de RAP.
- Ajout de fichiers.
- Suppression de fichiers.
- Déconnexion d'un disque.
- Déconnexion du NAS.
- Lecture prolongée.
- Changement de périphérique.
- Fermeture et restauration de fenêtre.

---

# 18. Organisation du développement avec Codex

## 18.1 Principe

Le développement sera réalisé avec un agent Codex principal.

Les tâches seront exécutées successivement.

Chaque phase devra aboutir à un état compilable et vérifiable.

## 18.2 Règles générales

- Examiner l'existant avant toute modification.
- Respecter les responsabilités des crates.
- Ne pas introduire de dépendances inutiles.
- Ne pas créer d'abstractions sans besoin concret.
- Éviter les fichiers monolithiques.
- Viser 200 à 600 lignes pour les fichiers de production lorsque cela est pertinent.
- Au-delà de 800 lignes, examiner le découpage avant toute extension significative.
- Ne pas découper artificiellement.
- Ne pas compter les tests dans les limites de production.
- Préserver la compatibilité avec les décisions architecturales.
- Ajouter des tests pour les comportements critiques.
- Exécuter les vérifications disponibles après modification.
- Documenter les limitations connues.

## 18.3 Documentation

Maintenir :

- `README.md`.
- `AGENTS.md`.
- Cahier des charges.
- Documentation des choix architecturaux importants.
- Liste des limitations connues.

Éviter de multiplier les documents redondants.

---

# 19. Plan de développement

## Phase 0 — Initialisation

Objectifs :

- Créer le workspace Cargo.
- Configurer les crates.
- Initialiser GPUI.
- Configurer les logs.
- Définir les types communs.
- Mettre en place les paramètres.
- Vérifier la compilation Windows.

**Livrable :** application GPUI vide mais fonctionnelle, avec architecture initiale.

## Phase 1 — Bibliothèque et indexation

Objectifs :

- Mettre en place le multilangue.
- Ajouter des dossiers sources.
- Parcourir les fichiers audio.
- Extraire les métadonnées.
- Créer le schéma SQLite.
- Indexer les pistes.
- Construire artistes et albums.
- Extraire les pochettes.
- Gérer les erreurs.

**Livrable :** bibliothèque indexée et consultable.

## Phase 2 — Interface bibliothèque

Objectifs :

- Navigation principale.
- Vue albums en grille.
- Vue albums en liste.
- Vue artistes.
- Fiche album.
- Affichage des pochettes.
- Recherche.
- Filtres.
- Thèmes clair/sombre.

**Livrable :** interface complète permettant d'explorer la collection.

## Phase 3 — Lecture audio locale

Objectifs :

- Intégrer le décodeur.
- Intégrer la sortie audio.
- Lire un fichier.
- Lire un album.
- Gérer Play/Pause/Stop.
- Gérer Seek.
- Gérer volume.
- Gérer piste suivante/précédente.
- Implémenter aléatoire et répétition.
- Connecter le moteur à GPUI.

**Livrable :** RAP utilisable quotidiennement comme lecteur audio local.

## Phase 4 — Robustesse de la bibliothèque

Objectifs :

- Indexation incrémentale.
- Surveillance des dossiers.
- Gestion des NAS déconnectés.
- Regroupement des éditions.
- Gestion des compilations.
- Cache des pochettes.
- Optimisation des recherches.

**Livrable :** bibliothèque robuste et persistante.

## Phase 5 — Interface finalisée

Objectifs :

- Plein écran de la page active, sans vue de lecture distincte.
- Barre de lecture finalisée.
- Filtres combinables.
- Préférences d'affichage.
- Couleur d'accent.
- Amélioration de l'ergonomie.
- Gestion des états vides et des erreurs.

**Livrable :** interface V1 aboutie.

## Phase 6 — Intégration Windows

Objectifs :

- SMTC.
- Pochette dans les contrôles système.
- Vignette de barre des tâches personnalisée et commandes de lecture natives.
- Touches multimédias.
- Icône de notification.
- Comportement configurable à la fermeture.
- Démarrage automatique optionnel.

**Livrable :** intégration Windows fonctionnelle.

## Phase 7 — Sorties audio locales

Objectifs :

- Découverte et sélection des périphériques de sortie locaux.
- Choix du périphérique système par défaut ou d'un périphérique préféré mémorisé.
- Changement de sortie en conservant la piste, la position et l'état lecture/pause.
- Gestion des périphériques indisponibles sans changement silencieux de destination.
- Affichage du périphérique actif et des caractéristiques du flux ouvert par RAP : fréquence, canaux et format d'échantillons.
- Sélection extensible aux sorties réseau de la phase 9, avec des implémentations locales et réseau distinctes.

L'égaliseur, le mode exclusif, le bit-perfect et la validation de formats supplémentaires sont exclus de cette phase et du périmètre V1 retenu.

**Livrable :** sélection des sorties locales et informations de sortie fonctionnelles, validées sur les configurations prises en charge.

## Phase 8 — API locale

**Statut : reportée, hors V1**, à la demande de l'utilisateur le 21 octobre 2026. Le développement passe directement à la phase 9 ; les objectifs ci-dessous sont conservés pour une évolution ultérieure.

Objectifs :

- Serveur HTTP local.
- Commandes de lecture.
- Consultation de l'état.
- Consultation des albums.
- Consultation des sorties.
- Gestion des erreurs.
- Tests des endpoints.

**Livrable :** API locale documentée.

## Phase 9 — DLNA / UPnP et Google Cast / Chromecast

Objectifs :

- Découverte des récepteurs DLNA / UPnP et Google Cast.
- Sélection d'une sortie réseau dans une interface commune, avec protocole identifiable.
- Serveur de diffusion local des fichiers et pochettes autorisés, utilisable par les deux protocoles.
- Contrôle de lecture, état et progression du récepteur sélectionné.
- Gestion de plusieurs périphériques.
- Gestion des déconnexions.
- Validation des formats compatibles.
- Validation Google Cast sur la NVIDIA SHIELD, via son récepteur intégré.

**Livrable :** diffusion DLNA / UPnP fonctionnelle sur les appareils validés et diffusion Google Cast validée sur la NVIDIA SHIELD.

## Phase 10 — Distribution

Objectifs :

- Compilation Release.
- Installateur Windows.
- Version ZIP portable.
- Gestion des chemins de configuration.
- Vérification des mises à jour.
- Documentation utilisateur.
- Vérification des licences.

**Livrable :** première distribution publique Windows.

---

# 20. Priorités de livraison

## Premier jalon utilisable

Les phases 0 à 3 constituent le premier jalon majeur.

RAP doit alors permettre :

- D'ajouter une bibliothèque.
- De parcourir les albums.
- De consulter les pochettes.
- De rechercher un album ou un artiste.
- De lancer la lecture.
- De contrôler le volume.
- De naviguer entre les pistes.

Ce jalon doit être réellement fonctionnel, même si certaines fonctionnalités avancées manquent.

## V1 Windows complète

La V1 cible les fonctionnalités des phases 0 à 10, à l'exception de la phase 8 reportée, et sous réserve de validation technique.

Les fonctionnalités complexes doivent pouvoir être reportées individuellement sans bloquer la distribution.

## Évolutions ultérieures

- Linux.
- Diffusion Alexa par Wi-Fi uniquement sur nouvelle demande explicite ; la
  sortie Bluetooth utilise déjà le parcours des sorties audio locales (§11.4).
- Formats audio nécessitant des décodeurs complémentaires.
- Transcodage DLNA avancé.
- Synchronisation multiroom.
- Améliorations audiophiles propres aux plateformes.
- Fonctionnalités supplémentaires validées par l'usage.

---

# 21. Risques techniques

## 21.1 GPUI

La disponibilité et la maturité des composants nécessaires doivent être vérifiées.

Certains contrôles devront éventuellement être développés spécifiquement.

## 21.2 Bit-perfect

La restitution bit-perfect dépend de la totalité du chemin audio.

Elle ne peut pas être garantie uniquement par le décodeur.

Les modes exclusifs doivent faire l'objet de tests matériels.

## 21.3 WMA et APE

Le support de ces formats peut nécessiter des bibliothèques complémentaires.

Ne pas compromettre la simplicité de distribution pour les intégrer prématurément.

## 21.4 DLNA et Google Cast

Les appareils présentent des comportements et capacités variables.

Il faudra tester sur du matériel réel.

Google Cast nécessite un client distinct de DLNA / UPnP. Les API émettrices officiellement documentées concernent Android, iOS et le Web ; une intégration native Rust / Windows doit faire l'objet d'une validation spécifique. Le fonctionnement hors Internet du récepteur Google ne doit pas être supposé.

## 21.5 NAS

Les notifications de fichiers et les performances peuvent varier.

Prévoir une stratégie de repli par analyse périodique.

## 21.6 Linux

Certaines intégrations système seront différentes de Windows.

Éviter toute dépendance directe aux API Windows dans les crates communes.

---

# 22. Critères de qualité

RAP devra respecter les principes suivants :

- Démarrage rapide.
- Interface réactive.
- Lecture audio stable.
- Bibliothèque persistante.
- Pas de modification des fichiers musicaux.
- Gestion correcte des sources déconnectées.
- Gestion des erreurs sans plantage.
- Consommation mémoire raisonnable.
- Architecture lisible.
- Code Rust idiomatique.
- Tests sur les modules critiques.
- Absence de fonctionnalités inutiles.

---

# 23. Décisions encore ouvertes

Les éléments suivants nécessitent une validation technique ou un arbitrage ultérieur :

1. Versions exactes des crates.
2. Stratégie définitive de sortie audio exclusive (évolution ultérieure, hors V1).
3. Formats effectivement disponibles en Rust pur.
4. Algorithme précis de regroupement des éditions.
5. Politique de vérification des NAS.
6. Format de distribution Windows.
7. Méthode de notification des nouvelles versions.
8. Périmètre exact de la diffusion réseau multi-appareils, hors synchronisation multiroom initiale.
9. Détails de l'API HTTP.
10. Modalités de réutilisation des composants d'Oxyde.
11. Client Google Cast Rust / Windows, stratégie de récepteur et formats validés sur la NVIDIA SHIELD.

Ces décisions ne doivent pas être inventées par Codex sans justification.

Les choix ayant un impact important sur l'architecture doivent être présentés avant implémentation.

---

# 24. Consignes de lancement pour Codex

Avant de commencer le développement :

1. Lire intégralement ce document.
2. Examiner le workspace existant s'il existe.
3. Vérifier les dépendances envisagées.
4. Proposer l'architecture détaillée.
5. Identifier les risques techniques immédiats.
6. Préparer un plan d'implémentation de la phase 0.
7. Ne pas implémenter plusieurs phases simultanément.
8. Maintenir le projet compilable à chaque étape.
9. Ne pas introduire de fonctionnalités non demandées.
10. Demander un arbitrage lorsque les spécifications sont ambiguës.

**Objectif final :** construire un lecteur audio natif, rapide, élégant et fiable, dont la complexité technique reste invisible pour l'utilisateur.

RAP doit privilégier la qualité d'exécution plutôt que la quantité de fonctionnalités.
