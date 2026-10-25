# Dépendances retenues

Les versions résolues de toutes les dépendances sont dans Cargo.lock. L'examen
initial porte sur les manifestes publiés et les sources locales, complété par
les documentations officielles. Pas de dépendance directe à Oxyde.

| Composant | Choix | Licence | Points de validation |
| --- | --- | --- | --- |
| GUI | gpui 0.2.2 crates.io | Apache-2.0 | Backend Windows D3D, Linux X11/Wayland ; pré-1.0, API instable |
| Tags | lofty 0.25.4 | MIT / Apache-2.0 | Rust pur, tags et images intégrées, propriétés audio activées |
| Base | rusqlite 0.36, bundled | MIT | SQLite C compilé et embarqué, sans DLL SQLite à installer |
| Parcours | dua-core 5.0.0 | MIT | Successeur recommandé de jwalk ; workers et résultats bornés, liens non suivis |
| Texte | unicode-normalization 0.1 | MIT / Apache-2.0 | Rust pur, déjà présent dans l'arbre ; clé de comparaison sans accents |
| Canaux | crossbeam-channel 0.5 | MIT / Apache-2.0 | Files bornées entre services |
| Surveillance | notify 8.2 | CC0-1.0 | ReadDirectoryChangesW sous Windows, inotify sous Linux ; 9.0 encore en RC. Regroupement des événements propre à Soliton, sans crate debouncer |
| Décodage | symphonia 0.5.5 | MPL-2.0 | Rust pur, MP3/FLAC/AAC-LC/ALAC/Vorbis/PCM/AIFF ; pas Opus/WMA/APE |
| Sortie | cpal 0.16 | Apache-2.0 | WASAPI Windows ; ALSA requiert bibliothèques de développement sous Linux |
| Conversion | rubato 0.16.2 | MIT | Conversion sinc hors callback, compensation du délai et de la longueur finale |
| Tampon PCM | crossbeam-queue 0.3 | MIT / Apache-2.0 | File préallouée de trames mono/stéréo complètes, pas de décalage des canaux après sous-alimentation |
| Intégration Windows | windows 0.61.3 | MIT / Apache-2.0 | Déjà résolu par GPUI ; SMTC via interop HWND, Shell_NotifyIcon, menu natif ; uniquement sur cible Windows |
| Démarrage utilisateur | windows-registry 0.5.3 | MIT / Apache-2.0 | Déjà résolu ; clé Run de l'utilisateur courant, sans élévation à l'exécution |
| Identifiant de fenêtre | raw-window-handle 0.6 | MIT / Apache-2.0 | Contrat portable déjà employé par GPUI ; aucun HWND dans le code commun |
| Icône de notification | resvg 0.45.1 | MIT / Apache-2.0 | Déjà résolu par GPUI ; rastérisation du SVG Lucide officiel `play.svg`, sans changement du tracé |
| Aperçu de barre des tâches | image 0.25.10, windows 0.61.3 | MIT / Apache-2.0 | Versions déjà résolues ; décodage PNG du cache sur le worker, DIB/GDI, DWM et ITaskbarList3 ; glyphes Lucide existants via resvg |
| Réseau asynchrone | tokio 1.54, axum 0.8.9 | MIT | Deux threads de runtime, huit threads bloquants maximum ; serveur HTTP sur l'interface du récepteur choisi, aucun endpoint de contrôle |
| Client UPnP HTTP | reqwest 0.13.5, roxmltree 0.20 | MIT / Apache-2.0 | HTTP local uniquement, proxy et redirections désactivés, réponses plafonnées à 1 MiB ; XML sans DTD |
| Découverte Cast | mdns-sd 0.20.3 | MIT / Apache-2.0 | Windows dans la CI amont ; découverte mDNS IPv4 indépendante du décodage et de GPUI |
| Google Cast | oxicast 0.0.3 | MIT / Apache-2.0 | Adaptateur isolé, requêtes bornées, reconnexion automatique désactivée, récepteur média standard testé sur SHIELD ; pas de SDK officiel Rust |
| Capacités HTTP | uuid 1.28, futures-util 0.3 | MIT / Apache-2.0 | UUID aléatoires par média, révocation au remplacement/arrêt, huit corps HTTP simultanés maximum |
| Interfaces SSDP | if-addrs 0.15, socket2 0.6 | MIT / Apache-2.0 | Huit interfaces IPv4 au maximum, quatre lectures de descriptions simultanées ; aucun type système dans les contrats communs |

GPUI 0.2.2 et proc-macro-error2 2.0.1 sont conservés localement avec des correctifs
ciblés via `[patch.crates-io]` : [vendor/README.md](../vendor/README.md).

La compilation Windows nécessite Rust MSVC, les Build Tools C++ et le SDK Windows.
Cast utilise rustls avec ring et AWS-LC, liés statiquement (aucune DLL à installer).
Les certificats auto-signés des récepteurs Cast ne sont acceptés que par ce client,
sur les destinations découvertes ; pas d'authentification DeviceAuth.

Bluetooth : API Windows Devices Enumeration, Bluetooth et Media Devices via
`windows` 0.61.3, `windows-collections` 0.2.0 et `windows-future` 0.2.1, déclarés
uniquement pour Windows.

jwalk 0.9 (archivé) a été écarté au profit de dua-core.
