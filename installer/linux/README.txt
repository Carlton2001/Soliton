Soliton pour Linux / Soliton for Linux
======================================

Installation (utilisateur courant, sans root) :

    ./install.sh

Le menu propose d'installer Soliton ou, si l'installation utilisateur existe,
de la mettre à jour, de la réinstaller ou de la désinstaller. La désinstallation
conserve la bibliothèque, les paramètres et les playlists.
Pour lancer une action directement : ./install.sh --install ou
./install.sh --uninstall. ./uninstall.sh reste également disponible.
La désinstallation complète efface aussi la base de bibliothèque, les playlists,
les réglages et le cache du dossier de données natif. Les autres fichiers restent
en place. Confirmez ce choix dans le menu ou lancez ./install.sh --uninstall --purge.

Prérequis : session Wayland ou X11 avec Vulkan, PipeWire 0.3.49 ou plus récent
(PulseAudio seul n'est pas pris en charge), portail XDG pour le choix des
dossiers. install.sh avertit si Vulkan est absent.

Utilisation : ajoutez vos dossiers de musique dans Paramètres. Vos fichiers,
tags et pochettes ne sont jamais modifiés. Les mises à jour s'installent en
relançant install.sh depuis une archive plus récente.
Le binaire est construit sur Fedora 44 x86_64 ; les distributions plus
anciennes peuvent manquer de la version de glibc requise.

Installation (current user, no root):

    ./install.sh

The menu offers installation or, if a per-user installation exists, an update,
reinstall, or uninstall. Uninstalling keeps the library, settings and playlists.
To run an action directly, use ./install.sh --install or ./install.sh --uninstall.
./uninstall.sh is also available.
A complete uninstall also deletes the library database, playlists, settings
and cache from the native data directory. Other files remain in place.
Confirm the choice in the menu or run ./install.sh --uninstall --purge.

Requirements: Wayland or X11 session with Vulkan, PipeWire 0.3.49 or later
(PulseAudio alone is not supported), and an XDG portal for folder selection.
install.sh warns when Vulkan is missing.

Usage: add your music folders in Settings. Your files, tags and artwork are
never modified. Update by running install.sh from a newer archive.
The binary is built on Fedora 44 x86_64; older distributions may lack the
required glibc version.

Licence : GPL-3.0-only (LICENSE). Dependencies: licenses/dependencies.
