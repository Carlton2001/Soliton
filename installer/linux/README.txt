Soliton pour Linux / Soliton for Linux
======================================

Installation (utilisateur courant, sans root) :

    ./install.sh

Soliton apparaît ensuite dans la vue des applications. Relancer install.sh
depuis une nouvelle archive met à jour l'installation. ./uninstall.sh la retire
et conserve bibliothèque, paramètres et playlists dans ~/.local/share/Soliton.

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

Soliton then appears in the applications overview. Running install.sh from a
newer archive updates it. ./uninstall.sh removes it and keeps the library,
settings and playlists in ~/.local/share/Soliton.

Requirements: Wayland or X11 session with Vulkan, PipeWire 0.3.49 or later
(PulseAudio alone is not supported), and an XDG portal for folder selection.
install.sh warns when Vulkan is missing.

Usage: add your music folders in Settings. Your files, tags and artwork are
never modified. Update by running install.sh from a newer archive.
The binary is built on Fedora 44 x86_64; older distributions may lack the
required glibc version.

Licence : GPL-3.0-only (LICENSE). Dependencies: licenses/dependencies.
