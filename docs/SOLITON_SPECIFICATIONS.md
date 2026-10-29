# Soliton — Lecteur audio en Rust
## Spécifications

**Licence :** GPL-3.0-only · **Langage :** Rust · **Interface :** GPUI ·
**Plateformes :** Windows (cible de validation initiale), Linux (Fedora Workstation)

Ce document fixe le périmètre, les règles produit et les phases restantes.
L'état fonctionnel actuel est décrit dans le [README](../README.md), les choix
techniques dans [ARCHITECTURE.md](ARCHITECTURE.md).

---

# 1. Objectif et principes

Lecteur audio de bureau pour une bibliothèque musicale locale structurée :
interface moderne, rapide et sobre, inspirée de la richesse de MusicBee sans sa
complexité visuelle. Qualité d'exécution avant quantité de fonctionnalités.

- Application native, fonctionnement local, sans compte ni cloud obligatoire.
  Google Cast peut dépendre des services Google du récepteur.
- Bibliothèque strictement en lecture seule ; métadonnées et pochettes issues
  uniquement des fichiers locaux, sans enrichissement en ligne.
- Interface indépendante du moteur audio ; intégrations système isolées.
- Dépendances Rust pures privilégiées. Pas de télémétrie, de fonctions sociales,
  d'achats ni de streaming commercial.
- Interface en français et en anglais.

---

# 2. État d'avancement

Implémenté et validé sous Windows : workspace et paramètres, bibliothèque et
indexation incrémentale, interface de bibliothèque, lecture locale, éditions et
compilations, cache des pochettes, surveillance des sources, interface finalisée,
playlists et file, intégration Windows (SMTC, barre des tâches, notification,
fermeture, démarrage automatique), sorties audio locales et Bluetooth, sorties
DLNA / UPnP et Google Cast.

Linux : version fonctionnelle sous Fedora Workstation (bibliothèque, lecture
locale par la sortie système, interface Wayland/X11, MPRIS, archive installable
par utilisateur ; démarrage automatique réservé à Windows) ; voir [LINUX.md](LINUX.md).

Reste à faire : distribution Windows (phase 10) et Linux (phase 10 bis),
validations matérielles (phase 11). Les livrables sont préparés sans être
publiés : la publication d'une release fait l'objet d'une demande distincte.

---

# 3. Règles produit

Ces règles restent en vigueur pour toute évolution.

## 3.1 Bibliothèque

- Plusieurs dossiers sources : disques internes ou externes, partages SMB et NAS
  traités comme des chemins ordinaires. Pas de client SMB intégré.
- Soliton ne modifie, déplace, renomme ou supprime jamais fichiers, tags ou pochettes
  intégrées. Bases, caches et temporaires restent hors des dossiers sources.
- Cible initiale inférieure à 10 000 pistes, sans limite artificielle ; listes et
  grilles virtualisées ; aucune opération lourde sur le thread GPUI.
- SQLite versionné par migrations. Seuls les paramètres et playlists ne peuvent
  pas être reconstruits depuis les fichiers.
- Une source inaccessible conserve ses albums, métadonnées et pochettes en cache ;
  ses pistes sont signalées indisponibles et non lisibles. Une disparition n'est
  établie qu'après un parcours complet d'une source accessible. « Masquer les
  albums indisponibles » est cochée par défaut ; une source remontée réapparaît
  d'elle-même en quelques secondes.
- Indexation en arrière-plan, incrémentale, annulable, tolérante aux fichiers
  corrompus, avec progression. Surveillance temps réel complétée par une analyse
  au démarrage et périodique.

## 3.2 Métadonnées, éditions et pochettes

- Tags intégrés (`lofty`) ; à défaut, déduction depuis l'arborescence
  `Artiste/Album/01 - Titre`, stockée seulement dans Soliton et distinguée des tags.
  Seuls les dossiers situés sous la racine de la source nomment artiste et album.
  Valeurs manquantes affichées « Artiste inconnu » et « Album inconnu » (regroupés
  par dossier, carte « Artiste inconnu » en fin de liste) ; valeurs déduites en
  italique.
- Modèle distinct : fichier, piste, édition, album regroupé, artiste.
- Plusieurs éditions d'un album (par exemple 16/44,1 et 24/96) sont présentées
  ensemble, avec choix de l'édition. Regroupement prudent : en cas d'ambiguïté,
  deux albums plutôt qu'une fusion incorrecte.
- Dans un même dossier et pour un même titre, un artiste d'album manquant ne crée
  pas d'édition supplémentaire si un seul artiste d'album explicite existe et que
  les morceaux sans ce tag portent ce même artiste. Compilations et cas
  contradictoires ou déduits restent séparés.
- Compilations : artiste d'album distinct de l'artiste de piste, album unique.
- Pochettes uniquement intégrées aux fichiers (pas de `folder.jpg`, pas de
  téléchargement), cache local invalidé quand l'image change, visuel neutre à défaut.

## 3.3 Interface

- Minimaliste, proche du natif, pochettes mises en valeur, animations discrètes.
  Éviter panneaux inutiles et options excessives.
- Les contrôles d'un même type partagent dimensions, typographie et états. La
  sélection utilise un fond teinté sans changement de graisse ni de bordure.
  Focus visible au clavier. Titres de page remontés sous la barre de titre par
  une marge haute négative calculée sur la police, sans couper lettres ni accents.
  Espacements : 0,75 rem au-dessus et au-dessous du contenu des pages, entre les
  sections (sans cumul), avec les mêmes marges extérieures en liste et en grille ;
  0,5 rem entre actions liées. Le logo de la sidebar garde une marge haute dédiée.
- Navigation toujours visible dans la sidebar, prolongée jusqu'au bord supérieur
  avec l'icône Soliton au-dessus des boutons ; barre de titre sans nom à sa droite.
  Icônes et libellés :
  Artistes, Albums, Playlists, File de lecture, Paramètres. Pas de bouton Retour ;
  `Alt+←/→` parcourent l'historique. Chaque section principale affiche son titre
  sous la barre de titre compacte et au-dessus de ses contrôles ; la barre de titre
  reste indépendante, haute de 32 pixels logiques avec des symboles système de
  12 pixels logiques. Les fiches (album, playlist) gardent leur propre en-tête ;
  « Lecture en cours » n'en a pas.
- Barre de titre, navigation et lecteur partagent le fond de l'application. Un trait
  sépare la navigation, le lecteur et l'en-tête de chaque page du contenu.
  Recherche à gauche des contrôles de tri ; Grille/Liste à droite, sur la même
  ligne ; la recherche se rétrécit si besoin. Filtres de métadonnées réservés à
  Albums ; genres et années par filtres ; le menu s'ajuste aux valeurs affichées.
- Albums : grille ou liste, choix mémorisé ; sur fenêtre large la fiche s'ouvre dans
  un panneau à droite, sur fenêtre étroite dans une page dédiée.
- Fiche album : grande pochette, métadonnées, durée, pistes, formats, éditions ;
  qualité par piste si elle diffère. Numéros disque.piste toujours affichés.
- Boutons avec icône : libellé complet ou icône seule avec infobulle, jamais d'état
  intermédiaire. Tous les boutons, groupés compris, se réduisent ensemble ; les
  boutons de lecture en dernier. Les entrées de menu gardent leur libellé. Actions secondaires des morceaux dans un menu « Actions ».
- Lecteur : progression au-dessus ; pochette et métadonnées à gauche ; Aléatoire,
  Précédent, Lecture/Pause (centré), Suivant, Répéter, File ; volume à droite,
  dans un panneau en fenêtre étroite. « Lecture en cours » par le bouton Agrandir,
  qui devient Réduire pour la fermer, comme Échap ; pochette et métadonnées
  réunies dans un cartouche. F11 : plein écran d'écoute, le cartouche seul avec
  progression et commandes de lecture ; Échap, F11 ou son bouton Réduire en
  sortent, comme les liens artiste, album et File.
- Thèmes clair, sombre et système, accent configurable, système de design centralisé.
- Icône : onde continue sur carré arrondi, six palettes au choix (Graphite et
  menthe par défaut), appliquée sans redémarrage, indépendante du thème. Titre de
  fenêtre « Soliton ».
- Pas de lecture de fichiers hors bibliothèque ni de glisser-déposer ; les
  dossiers s'ajoutent depuis les paramètres ou une vue vide.

## 3.4 Lecture

- Moteur indépendant de GPUI, piloté par commandes et événements ; callback audio
  sans allocation, E/S, journalisation ni verrou bloquant.
- Album, sélection, aléatoire, répétition piste/album, navigation. Gapless non
  prioritaire.
- Playlists manuelles nommées et file modifiable. Une occurrence par fichier :
  les ajouts répétés sont ignorés. Une piste indisponible reste visible dans sa
  playlist. Pas de playlists automatiques ni d'import/export.
- Session persistée (piste, position, volume, sortie, modes) sans historique ni
  statistiques d'écoute.

## 3.5 Sorties et réseau

- Sortie partagée locale (`cpal`, PipeWire sous Linux), sélection persistée,
  changement sans perte de piste ni de position, aucun repli silencieux vers une
  autre sortie.
- Avec « Périphérique système par défaut », une sortie perdue (casque éteint)
  laisse la lecture continuer sur la nouvelle sortie par défaut du système ;
  pause seulement si aucune ne s'ouvre. Une pause demandée par le système
  (WirePlumber sous Linux) est respectée.
- Sortie choisie perdue ou absente : pause à la même position et avertissement
  affiché jusqu'à la reprise. La reprise se fait sur la sortie système par
  défaut ; Soliton revient sur la sortie choisie dès qu'elle réapparaît, sans
  changer la préférence enregistrée.
- Linux : sinks PipeWire listés en direct, Bluetooth compris ; la sortie système
  par défaut suit les paramètres Son. Jamais de carte ALSA brute.
- Bluetooth via les paramètres Windows et les sorties exposées par le système.
  Soliton ne prétend pas établir la connexion, n'annonce ni lossless ni bit-perfect et
  ne change pas la sortie par défaut de Windows.
- DLNA / UPnP et Google Cast : couches de contrôle distinctes, sélection commune,
  serveur HTTP limité au média actif et au récepteur choisi, aucun chemin accepté
  depuis le réseau. Formats non acceptés signalés, pas de transcodage. Une seule
  sortie active, pas de multiroom.

## 3.6 Intégration système

- Windows : SMTC avec pochette, touches multimédias, vignette de barre des tâches
  avec commandes, icône de notification (Afficher, Lecture/Pause, Précédent,
  Suivant, Quitter), fermeture configurable (quitter, masquer, demander) avec
  commande Quitter explicite, démarrage automatique optionnel.
- La notification ne doit pas être le seul moyen de retrouver une fenêtre masquée.
- Linux : MPRIS (panneau média et touches multimédias du bureau, sans ouverture
  d'URI), icône du lanceur suivant la palette pour une installation par
  `install.sh`. Sans zone de notification sous GNOME, la fermeture quitte
  toujours et aucun démarrage automatique n'est proposé. Notification reportée.

---

# 4. Non planifié

- API locale de contrôle (ancienne phase 8) : HTTP sur `127.0.0.1` uniquement,
  service distinct du serveur de diffusion, authentification obligatoire en cas
  d'accès distant.
- Mode exclusif, bit-perfect, égaliseur.
- Opus, WMA, APE et autres formats demandant des décodeurs complémentaires.
- Transcodage réseau, multiroom, Alexa Wi-Fi.
- Linux : mises à jour intégrées, paquets pour d'autres familles que celles
  de la phase 10 bis (Arch, openSUSE…).

---

# 5. Phases restantes

Chaque phase aboutit à un état compilable et vérifiable. Présenter les choix
d'architecture avant implémentation.

## Phase 10 — Distribution Windows

**10.1 Build Release**
- Profil `release` (optimisations, symboles séparés), sans console.
- Métadonnées de l'exécutable : version, éditeur, icône.

**10.2 Mode portable**
- Données à côté de l'exécutable lorsque celui-ci est lancé depuis une archive
  portable (marqueur à définir) ; sinon `%LOCALAPPDATA%\Soliton`.
- Démarrage automatique et chemins cohérents dans les deux modes.

**10.3 Installateur**
- Choix du format (décision ouverte 1), installation par utilisateur sans
  élévation si possible, raccourcis, désinstallation propre qui conserve les
  données de l'utilisateur sauf demande explicite.
- Aucun runtime séparé à installer.

**10.4 Vérification des mises à jour**
- Désactivable, hors thread UI, sans installation automatique.
- Notification avec la version disponible. Sur clic explicite, téléchargement
  de l'installateur Windows, vérification de sa signature, fermeture propre de
  Soliton et relance après installation. En mode portable, ouverture de la page de
  téléchargement pour remplacer l'archive en conservant le dossier `data`.

**10.5 Documentation et licences**
- Guide utilisateur court (FR/EN).
- Inventaire des licences des dépendances livrées et attributions intégrées
  à la distribution.

**Livrable :** archive portable et installateur Windows publiables.

## Phase 10 bis — Distribution Linux

Dans l'ordre : RPM, puis .deb, puis Flatpak. Identifiant
`io.github.Carlton2001.Soliton`, installation système sous `/usr`, données
utilisateur dans `~/.local/share/Soliton` conservées à la désinstallation. Le mode
portable reste propre à Windows ; l'icône du lanceur suit la palette seulement
pour une installation par `install.sh`.

Le rendu GPUI exige Vulkan (repli logiciel lavapipe accepté sur matériel
ancien) : déclarer `vulkan-loader`/`libvulkan1` et `mesa-vulkan-drivers` dans
les paquets, et faire avertir `install.sh` en l'absence de `libvulkan.so.1`.

**10 bis.1 RPM** — Fedora Workstation 44 et suivantes (binaire construit sur
Fedora 44, glibc 2.43) : `.spec` et `rpmbuild` à partir du
binaire release de l'archive, validé par `rpmlint`, installation et
désinstallation.

**10 bis.2 .deb** — Ubuntu 24.04 et suivantes, Linux Mint 22, Debian 13 :
binaire compilé dans un conteneur Ubuntu 24.04 (glibc 2.39), dépendances
détectées par `dpkg-shlibdeps`, validé par `lintian`, installation et
désinstallation.

**10 bis.3 Flatpak** — Flathub, Silverblue compris : runtime Freedesktop 26.08,
compilation depuis les sources hors ligne, permissions minimales (affichage,
PipeWire, réseau, MPRIS), lecture seule du dossier Musique et des points de
montage, portail pour les autres dossiers ; données séparées des installations
natives.

**Livrable :** paquets construits et vérifiés, prêts à publier.

## Phase 11 — Validation matérielle

- Collection réelle de plusieurs milliers de pistes : indexation initiale,
  redémarrage, performances SSD/NAS.
- NAS/SMB réel : déconnexion et retour, notifications, analyse périodique.
- Déconnexion physique d'un disque et d'une sortie audio pendant la lecture.
- Écoute prolongée.
- Touches multimédias physiques, survol et clics de la vignette de barre des
  tâches, redémarrage d'Explorer, lancement à l'ouverture de session.
- Récepteur DLNA physique ; coupure Wi-Fi en diffusion Cast.

**Livrable :** limitations connues mises à jour dans le README, correctifs des
défauts trouvés.

---

# 6. Décisions ouvertes

1. Format de l'installateur Windows.
2. Fournisseur des mises à jour (GitHub Releases envisagé) et format de version.
3. ~~Signature de l'exécutable Windows et des paquets Linux.~~ Tranché le
   2026-10-29 : solutions gratuites uniquement (Minisign pour les mises à jour,
   GPG pour les paquets Linux), sans certificat Authenticode. Voir
   `docs/DISTRIBUTION.md`.
4. Périmètre multi-appareils réseau au-delà d'une sortie active.

Ne pas trancher ces points sans les présenter.

---

# 7. Risques

- **GPUI** : pré-1.0, API instable ; correctifs locaux dans `vendor/`.
- **Réseau** : comportements variables selon les appareils ; Cast sans SDK
  officiel Rust, testé seulement sur la NVIDIA SHIELD.
- **NAS** : notifications et performances variables ; l'analyse périodique sert
  de repli.
- **Linux** : intégrations différentes ; aucune API Windows dans le code commun.
  Backends GPUI Linux incomplets (handle de fenêtre X11, état agrandi à
  l'ouverture, activation Wayland) : contourner dans Soliton plutôt que modifier
  GPUI.
- **Distribution** : antivirus et SmartScreen sans signature de l'exécutable.
