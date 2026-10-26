Soliton pour Linux / Soliton for Linux
======================================

Installation (utilisateur courant, sans root) :

    ./install.sh

Soliton apparaît ensuite dans la vue des applications. Relancer install.sh
depuis une nouvelle archive met à jour l'installation. ./uninstall.sh la retire
et conserve bibliothèque, paramètres et playlists dans ~/.local/share/Soliton.

Prérequis : session Wayland ou X11 avec Vulkan, PipeWire (pipewire-alsa) ou
PulseAudio avec sa passerelle ALSA, portail XDG pour le choix des dossiers.
Le binaire est construit sur Fedora 44 x86_64 ; les distributions plus
anciennes peuvent manquer de la version de glibc requise.

Installation (current user, no root):

    ./install.sh

Soliton then appears in the applications overview. Running install.sh from a
newer archive updates it. ./uninstall.sh removes it and keeps the library,
settings and playlists in ~/.local/share/Soliton.

Requirements: Wayland or X11 session with Vulkan, PipeWire (pipewire-alsa) or
PulseAudio with its ALSA bridge, and an XDG portal for folder selection.
The binary is built on Fedora 44 x86_64; older distributions may lack the
required glibc version.

Licence : GPL-3.0-only (LICENSE). Dependencies: licenses/dependencies.
