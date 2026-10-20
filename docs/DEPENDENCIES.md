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

jwalk 0.9 est déclaré non maintenu : https://github.com/Byron/jwalk.
Il reste une inspiration examinée, pas une dépendance de RAP.
