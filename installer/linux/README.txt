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
dossiers et flock (util-linux). install.sh signale les dépendances système
manquantes et affiche une commande adaptée à la distribution lorsqu'elle est
reconnue. Dans un terminal, vous pouvez confirmer l'exécution de cette commande.
Si flock manque toujours, installez-le puis relancez le script.

Utilisation : ajoutez vos dossiers de musique dans Paramètres. Vos fichiers,
tags et pochettes ne sont jamais modifiés. Dans Paramètres → À propos, vérifiez
les mises à jour puis cliquez sur « Mettre à jour Soliton ». Soliton vérifie la
signature et redémarre après l'installation. Cette fonction utilise curl, les
certificats du système et tar avec gzip.
Le binaire est construit sur Fedora 44 x86_64. Le script vérifie que les
bibliothèques du système, dont glibc, permettent de charger le binaire avant
de l’installer ou de le remplacer. Pour Ubuntu 24.04, Linux Mint 22 et
Debian 13, utilisez le paquet .deb de la version.

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
(PulseAudio alone is not supported), an XDG portal for folder selection and
flock (util-linux).
install.sh reports missing system dependencies and prints a command for
recognized distributions. In a terminal, you can confirm running the command.
If flock is still missing, install it and rerun the script.

Usage: add your music folders in Settings. Your files, tags and artwork are
never modified. In Settings → About, check for updates and click “Update
Soliton”. Soliton verifies the signature and restarts after installation.
This feature uses curl, system CA certificates and tar with gzip support.
The binary is built on Fedora 44 x86_64. The script checks that the system
libraries, including glibc, can load it before installing or replacing it.
For Ubuntu 24.04, Linux Mint 22 and Debian 13, use the release's .deb package.

Licence : GPL-3.0-only (LICENSE). Dependencies: licenses/dependencies.
