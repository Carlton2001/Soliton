# Architecture

## Crates

| Crate | Rôle |
| --- | --- |
| `rap-core` | Types métier, contrats, commandes, événements, configuration, clé de comparaison du texte. Sans GPUI. |
| `rap-library` | Parcours, tags, SQLite, regroupement, cache des pochettes, surveillance, recherche. |
| `rap-audio` | Décodage, sortie CPAL, file de lecture, arbitrage entre sortie locale et réseau. |
| `rap-network` | Découverte SSDP/mDNS, contrôle DLNA et Cast, serveur HTTP des médias. Sans GPUI ni décodage. |
| `rap-platform` | Intégrations système derrière `Desktop` ; types Windows confinés à `windows/`. |
| `rap-app` | Assemblage des services et interface GPUI. |

SQLite reste un module de `rap-library`. Paramètres, base et pochettes sont dans
le dossier local RAP (`RAP_DATA_DIR` pour isoler le développement), jamais dans
une source. Un fichier de configuration invalide est signalé, pas écrasé.

GPUI 0.2.2 et proc-macro-error2 2.0.1 sont conservés localement avec des correctifs
ciblés : voir [vendor/README.md](../vendor/README.md). Oxyde a servi de référence
(pools bornés, annulation, fontes, `SetWindowSubclass`, `cx.listener`) sans copie
de code applicatif.

## Bibliothèque

**Threads.** Un worker unique d'écriture SQLite reçoit commandes, événements de
surveillance et échéances. Un lecteur WAL prépare instantanés, facettes, index de
pistes et recherches ; il ne reconstruit qu'au changement de `PRAGMA data_version`.
Pendant l'indexation, l'interface relit l'instantané toutes les 750 ms.

**Parcours.** `dua-core` (isolé dans `discovery.rs`) : deux workers, deux lots en
attente par worker, 64 entrées découvertes en attente ; un à quatre lecteurs de
tags, 16 travaux en attente. Un scan à la fois, liens non suivis, métadonnées
natives réutilisées sans second `stat`.

**Incrémental.** Taille et date de modification évitent les relectures ; les
échecs d'extraction sont mémorisés par taille, date et version d'extracteur.

**Disparitions.** Après un parcours complet sans erreur de parcours, sur une
source vérifiée accessible avant et après, les fichiers non rencontrés sont
retirés. Annulation, erreur de parcours ou source devenue inaccessible empêchent
toute conclusion. Une racine sans aucun fichier audio est traitée comme un volume
non monté.

**Surveillance.** `notify`, file bornée à 256 événements ; une perte déclenche une
analyse complète. Regroupement : 2 s de calme, 15 s au plus, 4 096 chemins au plus,
convertis en dossiers à analyser. Sonde `read_dir` sur thread dédié, délai 5 s,
chaque minute ; analyse complète au démarrage et toutes les 30 minutes.

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

**Texte.** `rap_core::text::fold` (NFKD, sans diacritiques, ligatures développées,
minuscules, espaces normalisés) sert au regroupement, aux tris, à la recherche et
aux facettes, qui affichent la graphie la plus fréquente.

**Pochettes.** Cache adressé par contenu, miniatures 160 et 512 px ; entrée
manquante régénérée ; miniatures orphelines et temporaires de plus d'une heure
supprimées après chaque analyse complète, dans le cache RAP uniquement.

## Audio

Symphonia décode paquet par paquet sur un worker dédié ; Rubato convertit la
fréquence sur ce même worker. CPAL/WASAPI en mode partagé, f32, fréquence et canaux
natifs si possible. Mono/stéréo uniquement.

Le callback ne fait que des lectures atomiques, dépile un tampon lock-free
préalloué de trames complètes et applique le volume. PCM en attente borné à deux
secondes. Le seek réinitialise décodeur, convertisseur et flux. La position vient
des samples réellement consommés.

Le moteur possède la file : occurrences à identifiant stable, dédoublonnage par
chemin, provenance de playlist conservée, instantanés immuables partagés par `Arc`.
Modifier la suite ne rouvre pas le flux courant.

**Sorties locales.** Identifiées par leur nom (seul identifiant portable de CPAL
0.16) ; noms ambigus non sélectionnables. `None` = périphérique par défaut, résolu
à chaque ouverture. Découverte toutes les 5 s sur un worker borné. Un changement
prépare la nouvelle session avant de remplacer l'ancienne ; un échec la conserve.
Pas de reprise automatique après déconnexion.

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
Buffering. RAP ne pilote ni ne ferme une session qu'il n'a pas ouverte.

La sortie réseau vaut pour la session ; le retour local reprend à la position
distante. Aucune panne ne bascule vers une autre sortie.

## Interface

Listes, grilles et sélecteurs virtualisés (`uniform_list`). Textes centralisés FR/EN
dans `i18n.rs`. Inter embarquée, chiffres tabulaires (`tnum`). Champ de texte issu
de l'exemple GPUI (Apache-2.0). SVG Lucide officiels non modifiés. Accents vérifiés
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
  appliqués à la fenêtre, à l'aperçu et à la notification.
- **Démarrage** : valeur RAP sous `HKCU\…\CurrentVersion\Run`, écrite seulement par
  une action de l'utilisateur.
- **Fermeture** : annulation des travaux, arrêt du moteur, retrait des abonnements
  et de l'icône, sauvegarde finale, puis `cx.quit()`.
