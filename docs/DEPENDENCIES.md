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
| Surveillance | notify 8.2 | CC0-1.0 | ReadDirectoryChangesW sous Windows, inotify sous Linux ; 9.0 encore en RC. Regroupement des événements propre à RAP, sans crate debouncer |
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
ciblés via `[patch.crates-io]`. Sources, licences et différences :
[vendor/README.md](../vendor/README.md). Aucun filtre des erreurs à l'exécution.

GPUI et CPAL utilisent les API natives derrière leurs backends conditionnés par
cible. La compilation Windows nécessite Rust MSVC et les Build Tools C++ / SDK
Windows. Linux n'est pas validé dans ce premier jalon.

Sources : https://docs.rs/gpui/0.2.2/gpui/,
https://docs.rs/walkdir/2.5.0/walkdir/,
https://docs.rs/symphonia/0.5.5/symphonia/,
https://docs.rs/cpal/0.16.0/cpal/.

API Windows vérifiées dans les sources locales épinglées et la documentation Microsoft :
[interop SMTC](https://learn.microsoft.com/en-us/windows/win32/api/systemmediatransportcontrolsinterop/nf-systemmediatransportcontrolsinterop-isystemmediatransportcontrolsinterop-getforwindow),
[zone de notification](https://learn.microsoft.com/en-us/windows/win32/shell/notification-area),
[clés Run](https://learn.microsoft.com/en-us/windows/win32/setupapi/run-and-runonce-registry-keys).

Vignette et commandes de barre des tâches :
[DwmSetIconicThumbnail](https://learn.microsoft.com/en-us/windows/win32/api/dwmapi/nf-dwmapi-dwmseticonicthumbnail),
[ThumbBarAddButtons](https://learn.microsoft.com/en-us/windows/win32/api/shobjidl_core/nf-shobjidl_core-itaskbarlist3-thumbbaraddbuttons).

jwalk 0.9 est déclaré non maintenu : https://github.com/Byron/jwalk.
Il reste une inspiration examinée, pas une dépendance de RAP.

Cast utilise TLS via rustls/tokio-rustls ; oxicast résout les fournisseurs
cryptographiques ring et AWS-LC. Leur code natif est lié statiquement, sans DLL
à installer chez l'utilisateur. Les compilations Windows emploient les Build
Tools déjà requis ; AWS-LC peut aussi utiliser CMake et des objets NASM fournis
par sa distribution. La validation réelle a été effectuée sous Windows MSVC.
Les certificats auto-signés des récepteurs Cast sont acceptés uniquement par ce
client Cast, sur les destinations locales découvertes, sans modifier le TLS des
autres clients. Il n'y a pas d'authentification DeviceAuth du récepteur à ce stade.

Références réseau examinées : [oxicast](https://github.com/denniskribl/oxicast),
[mdns-sd](https://github.com/keepsimple1/mdns-sd),
[Google Cast Media Receiver](https://developers.google.com/cast/docs/web_receiver),
[formats Cast](https://developers.google.com/cast/docs/media).

## Bluetooth Windows

La connexion Bluetooth réutilise Windows et CPAL
déjà présents dans RAP, sans dépendance ou service Amazon supplémentaire.
L'énumération des appareils appairés utilise les API Windows Devices Enumeration,
Bluetooth et Media Devices de `windows` 0.61.3, avec `windows-collections` 0.2.0
et `windows-future` 0.2.1 déclarés uniquement pour Windows (versions déjà
résolues dans le workspace). Aucun service supplémentaire n'est nécessaire.
L'icône Bluetooth provient du SVG Lucide officiel `icons/bluetooth.svg`, sans
modification, et relève de la même attribution dans `assets/NOTICE.md`.
