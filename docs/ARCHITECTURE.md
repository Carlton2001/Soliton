# Architecture

## Crates

| Crate | Rôle |
| --- | --- |
| `soliton-core` | Types métier, contrats, commandes, événements, configuration, clé de comparaison du texte. Sans GPUI. |
| `soliton-library` | Parcours, tags, SQLite, regroupement, cache des pochettes, surveillance, recherche. |
| `soliton-audio` | Décodage, sortie CPAL, file de lecture, arbitrage entre sortie locale et réseau. |
| `soliton-network` | Découverte SSDP/mDNS, contrôle DLNA et Cast, serveur HTTP des médias. Sans GPUI ni décodage. |
| `soliton-platform` | Intégrations système derrière `Desktop` ; code natif confiné à `windows/` et `linux/`. |
| `soliton-app` | Assemblage des services et interface GPUI. |

SQLite reste un module de `soliton-library`. Paramètres, base et pochettes sont dans
le dossier local Soliton (`SOLITON_DATA_DIR` pour isoler le développement), jamais dans
une source. Un fichier de configuration invalide est signalé, pas écrasé ; chaque
écriture est synchronisée sur le disque avant de remplacer la précédente.

GPUI 0.2.2 et proc-macro-error2 2.0.1 sont conservés localement avec des correctifs
ciblés : voir [vendor/README.md](../vendor/README.md). Oxyde a servi de référence
(pools bornés, annulation, fontes, `SetWindowSubclass`, `cx.listener`) sans copie
de code applicatif.

## Bibliothèque

**Threads.** Un worker unique d'écriture SQLite reçoit commandes, événements de
surveillance et échéances ; entre deux lots d'un parcours, il laisse passer les
modifications de playlists, les autres travaux attendent la fin du parcours. Les
paramètres ont leur propre worker, qui n'écrit que la dernière valeur. Un lecteur
WAL prépare instantanés, facettes, index de pistes et recherches ; il ne
reconstruit qu'au changement de `PRAGMA data_version`.
Pendant l'indexation, l'interface relit l'instantané toutes les 750 ms.

**Parcours.** `dua-core` (isolé dans `discovery.rs`) : deux workers, deux lots en
attente par worker, 64 entrées découvertes en attente ; un à quatre lecteurs de
tags, 16 travaux en attente. Un scan à la fois, liens non suivis, métadonnées
natives réutilisées sans second `stat`.

**Incrémental.** Taille et date de modification évitent les relectures ; les
échecs d'extraction sont mémorisés par taille, date et version d'extracteur. Un
chemin sans représentation Unicode est signalé et ignoré sans interrompre le
parcours ni empêcher ses conclusions.

**Disparitions.** Après un parcours complet sans erreur de parcours, sur une
source vérifiée accessible avant et après, les fichiers non rencontrés sont
retirés. Annulation, erreur de parcours ou source devenue inaccessible empêchent
toute conclusion. Une racine sans aucun fichier audio est traitée comme un volume
non monté.

**Surveillance.** `notify`, file bornée à 256 événements ; une perte déclenche une
analyse complète. Regroupement : 2 s de calme, 15 s au plus, 4 096 chemins au plus,
convertis en dossiers à analyser. Sonde `read_dir` sur thread dédié, délai 5 s,
chaque minute ; les sources hors ligne sont sondées en plus toutes les 5 s
(délai 1 s) et redeviennent disponibles dès le début de leur analyse, sans
rafraîchissement manuel ; analyse complète au démarrage et toutes les 30 minutes.

**Schéma.** Migrations séquentielles dans `schema.rs` (`user_version`), chacune
transactionnelle avec `foreign_key_check`. Sources, artistes, albums, éditions,
pistes logiques, fichiers physiques, playlists et entrées (chemin unique par
playlist, référence nullable à la piste pour survivre au retrait du catalogue).

**Regroupement** (`grouping.rs`, fonctions pures). Dossier d'album = parent, en
sautant un niveau `CD1`/`Disc 2`/`Disque 3`. Identité d'édition : source, dossier,
titre et artiste d'album normalisés. Album : artiste d'album taggé, sinon
compilation (drapeau ou artistes différents, « Artistes divers »), sinon artiste de
piste ou dossier. Fusion entre dossiers seulement si artiste, titre, compilation et
année majoritaire sont identiques ; titre déduit ou année absente : jamais de fusion.

**Texte.** `soliton_core::text::fold` (NFKD, sans diacritiques, ligatures développées,
minuscules, espaces normalisés) sert au regroupement, aux tris, à la recherche et
aux facettes, qui affichent la graphie la plus fréquente.

**Pochettes.** Cache adressé par contenu, miniatures 160 et 512 px ; entrée
manquante régénérée ; miniatures orphelines et temporaires de plus d'une heure
supprimées après chaque analyse complète, dans le cache Soliton uniquement.

## Audio

Symphonia décode paquet par paquet sur un worker dédié ; Rubato convertit la
fréquence sur ce même worker. CPAL/WASAPI en mode partagé, f32, fréquence et canaux
natifs si possible. Sources mono/stéréo uniquement ; une sortie de plus de deux
canaux (5.1, 7.1) reçoit la stéréo à l'avant gauche et droit, les autres canaux
restent silencieux.

Le callback ne fait que des lectures atomiques, dépile un tampon lock-free
préalloué de trames complètes et applique le volume. PCM en attente borné à deux
secondes. Le seek garde le flux ouvert : le worker arrête la consommation, attend
deux callbacks complets, vide le tampon puis réinitialise décodeur et
convertisseur ; les seeks successifs sont regroupés. La position vient des
samples réellement consommés.

Un paquet impossible à décoder est sauté et signalé une fois par piste ; huit de
suite rendent le fichier illisible. En lecture automatique, une piste illisible ou
indisponible est passée, au plus un tour de file ; une panne de sortie ou de
réseau arrête la lecture.

Le moteur possède la file : occurrences à identifiant stable, dédoublonnage par
chemin, provenance de playlist conservée, instantanés immuables partagés par `Arc`.
Modifier la suite ne rouvre pas le flux courant.

**Sorties locales.** Un backend par cible derrière la même interface interne
(`backend` dans `soliton-audio`) : CPAL hors Linux, identifié par le nom (seul
identifiant portable de CPAL 0.16) et découvert toutes les 5 s sur un worker
borné ; PipeWire natif sous Linux, identifié par le nom du nœud et suivi en
direct par le registre. Identifiants ambigus non sélectionnables. `None` =
périphérique par défaut, résolu à chaque ouverture sous CPAL, suivi en continu
par PipeWire. Un changement
prépare la nouvelle session avant de remplacer l'ancienne ; un échec la conserve.
Une sortie perdue ou absente met en pause à la même position (`interruption`,
effacée à la reprise) ; si une sortie avait été choisie, les flux s'ouvrent sur
la sortie par défaut (`fallback`) jusqu'à ce que la liste la montre de nouveau,
puis la session y revient. Jamais de lecture sur une autre sortie sans action
de l'utilisateur.

**Bluetooth.** Énumération Windows (Devices Enumeration, Bluetooth, Media Devices)
hors thread UI ; association aux sorties par identifiant de conteneur, sans
rapprochement de noms.

## Réseau

Démarré seulement à la recherche d'appareils. Runtime Tokio de deux threads, huit
threads bloquants au plus ; commandes limitées à 12 s (16 s côté appelant). La
progression suit les acquittements du récepteur ; une génération par chargement
écarte les états d'une piste précédente.

**HTTP.** Serveur sur l'interface IPv4 du récepteur, port temporaire, UUID aléatoire
par média, accès réservé à l'IP choisie, piste active et sa pochette uniquement.
GET, HEAD, OPTIONS, plages d'octets ; huit corps simultanés, blocs de 64 Kio.
Révocation au remplacement, à l'arrêt et à la déconnexion.

**DLNA.** SSDP borné ; XML et SOAP plafonnés à 1 MiB, sans DTD ; URL HTTP vers l'IP
annonçante, sans proxy ni redirection. AVTransport, RenderingControl si présent,
ConnectionManager pour les types MIME.

**Cast.** `oxicast` et Default Media Receiver, reconnexion automatique désactivée.
Seul FINISHED enchaîne la piste suivante ; un IDLE pendant LOADING reste en
Buffering. Soliton ne pilote ni ne ferme une session qu'il n'a pas ouverte.

La sortie réseau vaut pour la session ; le retour local reprend à la position
distante. Aucune panne ne bascule vers une autre sortie.

## Interface

Listes, grilles et sélecteurs virtualisés (`uniform_list`). Textes centralisés FR/EN
dans `i18n.rs`, y compris les erreurs : moteur et bibliothèque produisent des
`soliton_core::failure::Failure` (nature sans texte, détail système conservé) que
l'interface traduit sous le libellé de leur origine. Inter embarquée, chiffres tabulaires (`tnum`). Champ de texte issu
de l'exemple GPUI (Apache-2.0). SVG Lucide officiels non modifiés,
chargés avec un trait de 1,5 au lieu de 2 (seul cet attribut change). Accents vérifiés
à un contraste d'au moins 4,5:1. Erreurs conservées (50 au plus) dans un bandeau.

## Intégration Windows

Un worker possède l'appartement WinRT MTA, SMTC et une fenêtre cachée pour la
notification et les messages Shell. États coalescés dans une file d'une entrée,
commandes système dans une file de 64. Les touches multimédias passent par SMTC.

- **SMTC** : pochette lue depuis le cache (8 MiB max) dans un
  `InMemoryRandomAccessStream`, car `StorageFile` refuse les chemins `\\?\`.
- **Barre des tâches** (`windows/taskbar/`) : image 640×360 préparée sur le worker
  chaque seconde et à chaque changement ; la sous-classe du HWND ne fait qu'une copie
  GDI bornée et l'envoi des commandes. Rétablie sur `TaskbarButtonCreated`.
- **Notification** : rétablie sur `TaskbarCreated` ; si elle échoue, le masquage
  est interdit.
- **Icônes** : six SVG de `assets/logos/` rastérisés à sept tailles par `build.rs`,
  appliqués à la fenêtre, à l'aperçu et à la notification. Si l'interface ne
  répond pas encore, l'icône de la fenêtre est retentée toutes les 250 ms ; un
  échec n'est signalé qu'après 10 s.
- **Démarrage** : valeur Soliton sous `HKCU\…\CurrentVersion\Run`, écrite seulement par
  une action de l'utilisateur.
- **Fermeture** : annulation des travaux, arrêt du moteur, retrait des abonnements
  et de l'icône, sauvegarde finale, puis `cx.quit()`.

## Intégration Linux

`Desktop` expose des capacités distinctes : `HIDES` (zone de notification, Windows
seulement), `AUTOSTART` (Windows seulement : sans zone de notification, GNOME
ouvrirait la fenêtre à chaque session). Sous Linux, un worker possède la connexion D-Bus de
session (`zbus` bloquant, déjà présent via GPUI) ; l'interface lui envoie l'état
coalescé dans une file d'une entrée, les commandes reviennent dans une file de 64.
Le handle de fenêtre n'est pas demandé : GPUI ne le fournit pas sous X11.

- **MPRIS** : `org.mpris.MediaPlayer2` et `.Player` ; seuls état, métadonnées et
  capacités émettent `PropertiesChanged`, la position est lue à la demande et un
  écart de plus de 1,5 s par rapport à la lecture attendue émet `Seeked`. Un
  identifiant de piste par occurrence protège `SetPosition` des requêtes périmées.
- **Icône du lanceur** : réécrite seulement si l'entrée de bureau porte le marqueur
  d'`install.sh` ; la date du thème `hicolor` est mise à jour pour les caches.
- **Fenêtre** : `Raise` active la fenêtre par GPUI ; l'état agrandi enregistré est
  réappliqué après ouverture.
