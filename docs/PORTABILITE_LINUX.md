# Évaluation de la portabilité Linux de Soliton

Date de l'analyse : 26 octobre 2026.

Ce rapport conserve les constats de l'audit initial. Les premières adaptations
et la procédure de validation sont décrites dans [le guide Linux](LINUX.md).

Environnement de développement envisagé : Fedora Workstation.
Environnement de validation envisagé : Fedora Silverblue.

## Conclusion

Un premier prototype Linux paraît accessible sans refonte générale. Le moteur
audio et la bibliothèque sont déjà largement portables. Deux obstacles concrets
ressortent : le script de compilation de l'application et les commandes de sa
barre de titre. La principale incertitude restante concerne l'exécution de GPUI
sur le matériel Fedora.

La complexité globale du prototype est faible à moyenne, sous réserve des essais
graphiques et audio natifs. Les intégrations système complètes constituent des
travaux distincts et reportables.

## Périmètre et méthode

Analyse réalisée en lecture seule : spécifications, six crates du workspace,
manifestes, dépendances verrouillées, sources GPUI vendues et backends CPAL
inspectés. Les arbres de dépendances ont été consultés avec
`cargo tree --locked --offline` pour les cibles Windows et Linux depuis un hôte
Windows.

Aucun code n'a été modifié pendant l'analyse. Aucune compilation ni exécution
Linux n'a été effectuée : les constats statiques ne constituent pas une validation
Linux. Le présent document est l'export du rapport demandé après cette analyse.

Les contraintes retenues sont la préservation de Windows, l'absence de
refactorisation générale et la préférence pour les dépendances Rust pures lorsque
c'est raisonnable. Audio exclusif, DLNA/Cast et packaging Flatpak sont hors du
prototype proposé.

## 1. Workspace et dépendances

Le [workspace Cargo](../Cargo.toml) sépare correctement domaine, bibliothèque,
audio, réseau, plateforme et interface :

| Crate | Évaluation |
| --- | --- |
| `soliton-core` | Domaine, configuration et règles largement portables ; politique de mises à jour encore Windows |
| `soliton-library` | Accès par `Path`/`PathBuf`, Lofty, SQLite et surveillance multiplateforme |
| `soliton-audio` | Indépendant de GPUI ; décodage et conversion portables, sortie abstraite par CPAL |
| `soliton-network` | Interfaces et bibliothèques réseau portables en principe ; fonctionnement Linux non validé et hors prototype |
| `soliton-platform` | API natives isolées derrière des contrats communs et des modules conditionnels |
| `soliton-app` | Widgets GPUI réutilisables ; adaptations du build, de la fenêtre et des capacités disponibles |

Versions importantes : GPUI **0.2.2**, CPAL **0.16.0**, Symphonia **0.5.5**,
Lofty **0.25.4**, `notify` **8.2.0**, `rusqlite` **0.36** avec SQLite embarqué.
Les versions résolues font référence dans [Cargo.lock](../Cargo.lock).

Le minimum Rust déclaré est 1.89, mais la CI Windows utilise 1.98.1. Reprendre
cette dernière version limiterait les variables du premier portage. Les correctifs
locaux de GPUI doivent être conservés pour préserver Windows ; voir
[vendor/README.md](../vendor/README.md).

Aucune feature Soliton ne permet actuellement d'exclure le réseau.
`soliton-audio` dépend toujours de `soliton-network` : reporter DLNA/Cast ne retire
donc pas leurs dépendances de compilation, notamment TLS avec `ring` et AWS-LC.

Le projet ne sera pas entièrement Rust pur : SQLite embarqué, certaines
dépendances graphiques et TLS comportent du code natif. Les remplacer augmenterait
inutilement le périmètre. Lofty, Symphonia et Rubato permettent en revanche de
conserver l'essentiel du traitement musical en Rust, sans FFmpeg à l'exécution.

## 2. Dépendances Windows et Win32

| Origine | Dépendances et usages Windows | Conséquence Linux |
| --- | --- | --- |
| `soliton-platform` | `windows` 0.61.3, `windows-registry`, `windows-collections`, `windows-future` : WinRT/COM, SMTC, Bluetooth, périphériques, registre, Shell, GDI/DWM, icônes et barre des tâches | Déclarations et implémentations conditionnées par cible ; pas de types Win32 exposés au code commun |
| Téléchargement et installation | WinHTTP, lancement d'un installateur `.exe`, options de processus Windows | Implémentations Linux absentes, mais compilation commune prévue |
| Build de l'application | `embed-resource`, génération des ressources et icônes Windows | Conditionnement incomplet : blocage de compilation décrit ci-dessous |
| GPUI | Direct3D/DirectWrite, familles `windows`, `windows-core`, `windows-numerics`, registre et bibliothèques associées | Backends Linux distincts déjà présents |
| CPAL et surveillance | `windows` 0.54 pour WASAPI ; `windows-sys` 0.60 pour `notify` | Remplacés respectivement par ALSA et inotify |
| Dépendances transitives | `windows-sys`, `winapi`, `winapi-util` via fichiers, dossiers, réseau, exécuteurs, terminal et fontes ; Schannel via certificats système ; registre via `zed-reqwest` | Branches propres à Windows, sans besoin de réécrire ces bibliothèques |

Les principaux utilitaires transitifs concernés comprennent `dirs`, `dua-core`,
`tempfile`, `walkdir`, `same-file`, `if-addrs`, `mio`, Tokio, `socket2`,
`mdns-sd`, `async-io`, `polling`, `stacker` et les composants de journalisation.
Les familles auxiliaires `windows-targets`, `windows-link`, `windows-result`,
`windows-strings`, `windows-threading` et les macros Windows font partie de ces
chaînes de dépendances.

Les paquets Windows présents dans `Cargo.lock` ne constituent pas, à eux seuls,
des blocages Linux. Un arbre calculé depuis Windows peut également conserver des
dépendances Windows des outils exécutés sur l'hôte de compilation.

Sources principales : [manifeste plateforme](../crates/soliton-platform/Cargo.toml),
[intégration Windows](../crates/soliton-platform/src/windows/mod.rs),
[téléchargement](../crates/soliton-platform/src/windows_download.rs),
[mise à jour](../crates/soliton-platform/src/updater.rs),
[manifeste GPUI](../vendor/gpui-0.2.2/Cargo.toml).

## 3. Blocages réels du prototype

### Script de compilation — complexité faible

Dans [build.rs](../crates/soliton-app/build.rs), un `return` évite l'exécution sous
Linux, mais l'appel à `embed_resource` reste soumis à la résolution Rust. Cette
dépendance n'est déclarée que pour Windows. Sur un hôte Linux, le script doit donc
échouer à compiler.

Adaptation ciblée : conditionner le code Windows lui-même avec `#[cfg(...)]`, en
incluant ses constantes pour éviter les avertissements. La branche de génération
des ressources Windows doit conserver son comportement.

### Commandes de fenêtre — complexité moyenne

La [barre de titre](../crates/soliton-app/src/title_bar.rs) utilise uniquement
`WindowControlArea` pour déplacement, réduction, agrandissement et fermeture.
Les backends GPUI [Wayland](../vendor/gpui-0.2.2/src/platform/linux/wayland/window.rs)
et [X11](../vendor/gpui-0.2.2/src/platform/linux/x11/window.rs) ignorent ce mécanisme
de hit-test.

Adaptation ciblée : prévoir les interactions Linux explicites et vérifier les
décorations, notamment sous GNOME/Wayland. GPUI 0.2.2 fournit déjà les opérations
nécessaires, notamment `start_window_move`, `start_window_resize`, `zoom_window`
et `minimize_window`. La fermeture doit passer par le cycle de nettoyage Soliton.
Aucune migration de framework n'est justifiée pour ce prototype.

## 4. Compatibilité graphique et prérequis Fedora

La compatibilité Linux est prévue dans la dépendance, mais n'est pas validée pour
Soliton. Les features GPUI par défaut activent Wayland et X11 ; Linux utilise
Blade/Vulkan et Cosmic Text. Les widgets Soliton, listes virtualisées, SVG et
fontes Inter embarquées ne présentent pas de dépendance Windows directe.

L'ajout de dossiers utilise déjà le portail XDG : une session D-Bus et un portail
fonctionnel sont nécessaires. Le code pertinent se trouve dans
[le backend Linux GPUI](../vendor/gpui-0.2.2/src/platform/linux/platform.rs) et
[le choix de source Soliton](../crates/soliton-app/src/app.rs).

Sur Workstation, prévoir une chaîne C/C++, `pkg-config`, CMake, les fichiers de
développement ALSA et xkbcommon, ainsi que FreeType. À l'exécution, vérifier Vulkan,
Fontconfig, Wayland/X11 et les portails. Fedora fournit notamment
[alsa-lib-devel](https://packages.fedoraproject.org/pkgs/alsa-lib/alsa-lib-devel/)
et [libxkbcommon-x11-devel](https://packages.fedoraproject.org/pkgs/libxkbcommon/libxkbcommon-x11-devel/).
La liste exacte des bibliothèques liées devra être confirmée sur le premier
binaire Linux.

Le projet voisin Oxyde-GPUI a été examiné comme référence. Il utilise désormais
une révision Git de GPUI et `gpui_platform`, avec une API différente de la version
crates.io 0.2.2 de Soliton. Ses composants ne doivent pas être repris sans
vérification de compatibilité.

## 5. Fichiers, chemins, métadonnées et surveillance

- [AppPaths](../crates/soliton-platform/src/paths.rs) utilise
  `dirs::data_local_dir()` : le stockage suit déjà XDG, normalement
  `~/.local/share/Soliton`. `SOLITON_DATA_DIR` permet d'isoler les essais. Séparer
  ultérieurement configuration et cache n'est pas indispensable au prototype.
- Les chemins utilisent `Path`/`PathBuf`, la canonicalisation et
  `MAIN_SEPARATOR`. Les contrôles empêchant le chevauchement entre sources et
  stockage applicatif sont déjà présents.
- [Les métadonnées](../crates/soliton-library/src/metadata.rs) restent lues par
  Lofty. L'absence de date de création possède un repli sur la date de
  modification. Les sources musicales sont ouvertes en lecture seule.
- Les chemins non UTF-8 sont explicitement signalés puis ignorés, avec un
  [scénario de test Unix](../crates/soliton-library/src/tests/database.rs)
  existant. C'est une limitation connue, pas un nouveau blocage.
- [La surveillance](../crates/soliton-library/src/watch.rs) utilise `notify`, donc
  inotify sous Linux, avec regroupement, gestion des débordements et analyses
  périodiques. Les quotas inotify et les notifications des montages réseau
  restent à tester ; ces limites sont documentées par
  [notify](https://docs.rs/notify/8.2.0/notify/).
- Les protections contre les suppressions après parcours incomplet ou source
  inaccessible existent déjà dans
  [l'indexeur](../crates/soliton-library/src/indexer.rs). Tester impérativement
  démontage/remontage et réinstallation des surveillances. Une racine redevenue
  accessible ne prouve pas à elle seule que le volume attendu est remonté.
- Les NAS doivent être accessibles comme chemins montés. Les URI `smb://` ne sont
  pas prises en charge ; aucun client SMB intégré n'est nécessaire au prototype.
- Utiliser une base Linux distincte pour les essais : les chemins absolus et les
  noms de sorties enregistrés sous Windows ne deviennent pas automatiquement
  valides sous Linux.

## 6. Chemin audio le plus court

**Symphonia → Rubato → CPAL/ALSA → greffon `pipewire-alsa` → PipeWire.**

Le [code de sortie](../crates/soliton-audio/src/output.rs) utilise les interfaces
portables CPAL et plusieurs formats d'échantillons. Le moteur est indépendant de
GPUI et conserve son callback fondé sur des atomiques et une file préallouée.

Les sources inspectées de CPAL 0.16 utilisent ALSA comme backend Linux par défaut ;
aucun backend PulseAudio ou PipeWire direct n'est activé par Soliton. Fedora
fournit précisément le
[greffon ALSA vers PipeWire](https://packages.fedoraproject.org/pkgs/pipewire/pipewire-alsa/).
Pour PulseAudio, le
[greffon ALSA correspondant](https://packages.fedoraproject.org/pkgs/alsa-plugins/alsa-plugins-pulseaudio/)
offre une autre passerelle.

Commencer par la sortie CPAL par défaut, après vérification de son routage vers
PipeWire. L'énumération CPAL expose également des sorties matérielles ALSA : elle
ne représente pas fidèlement les sorties et profils PipeWire. Une sélection native
par sink peut attendre.

Les notifications de périphériques Windows sont facultatives : le moteur conserve
déjà un [sondage toutes les cinq secondes](../crates/soliton-audio/src/engine.rs)
lorsqu'elles ne sont pas disponibles. Le refus de la surveillance native Linux
produit un avertissement de journal, sans empêcher la lecture.

Les points à valider sont la négociation des formats, les fréquences 44,1/48 kHz,
le seek, les sous-alimentations, la fermeture du flux et la déconnexion de sortie.
L'ouverture de la sortie de remplacement avant fermeture de l'ancienne doit aussi
être vérifiée avec les greffons ALSA, sans modifier préventivement ce comportement
commun.

## 7. Adaptations mineures et fonctions reportables

| Fonction | État actuel / adaptation | Complexité qualitative |
| --- | --- | --- |
| SMTC, vignette Windows, notification, démarrage automatique | Interfaces déjà isolées ; intégration désactivée hors Windows. La fermeture quitte correctement l'application | Aucun portage requis pour le prototype |
| MPRIS/D-Bus | Futur backend derrière le contrat `Desktop` | Moyenne, reportable |
| Bluetooth dédié | Interface déjà masquée hors Windows ; gestion initiale dans GNOME, lecture via la sortie système | Aucun portage requis pour la lecture standard ; intégration dédiée reportable |
| Mises à jour | Vérification proposée par défaut alors que téléchargement, manifeste et installateur sont Windows ; désactiver cette capacité sous Linux | Faible |
| Notifications de sorties audio | Repli par sondage déjà présent ; notifications natives Linux facultatives | Faible pour conserver le repli, intégration native reportable |
| Accent système, icône du bureau, placement exact Wayland | Finitions reportables ; conserver un thème et un accent explicites | Faible à moyenne selon le niveau d'intégration |
| Organisation XDG séparant données, configuration et cache | Stockage XDG commun déjà utilisable | Reportable |
| Audio exclusif, DLNA/Cast, Flatpak | Hors du prototype demandé ; dépendances réseau toujours compilées dans l'état actuel | Non estimée dans ce périmètre |

Sources : [contrat Desktop](../crates/soliton-platform/src/desktop.rs),
[cycle de vie de l'application](../crates/soliton-app/src/system.rs),
[Bluetooth](../crates/soliton-platform/src/bluetooth.rs),
[interface de mise à jour](../crates/soliton-app/src/updates.rs),
[politique du manifeste de mise à jour](../crates/soliton-core/src/updates.rs).

## 8. Risques de régression Windows

Les principaux risques seraient :

- Une modification globale de GPUI ou de ses features, ou la perte des correctifs
  locaux Windows.
- Un changement commun de sélection audio, de négociation de format, de seek ou
  de remplacement des flux pour résoudre un problème uniquement Linux.
- Une normalisation des chemins ou une évolution du schéma qui modifierait les
  identités des sources, fichiers, albums ou éditions sous Windows.
- Une généralisation précipitée de `Desktop::SUPPORTED`. L'activation future de
  MPRIS devrait distinguer les capacités disponibles plutôt que rendre toutes les
  fonctions `Desktop` actives ensemble.
- Une modification du script de build qui altérerait les ressources, icônes ou
  métadonnées de l'exécutable Windows.

Conserver les branches Windows et ajouter des adaptations ciblées réduit ces
risques. La CI Windows existante doit être maintenue, complétée par des essais
ciblés sur barre de titre, fermeture, intégrations et audio lorsque ces parties
sont touchées.

## 9. Plan de portage progressif

Après le socle de compilation, bibliothèque, fenêtre et audio peuvent être
vérifiés indépendamment. Chaque jalon doit conserver un état compilable et ne
doit pas imposer la réalisation des intégrations reportées.

| Jalon | Résultat vérifiable | Complexité |
| --- | --- | --- |
| 1. Socle Fedora Workstation | Corriger le build conditionnel, installer les dépendances ; `cargo check --workspace --all-targets --locked` sans avertissement projet ; conserver le contrôle Windows | Faible |
| 2. Bibliothèque | Tests domaine/bibliothèque ; indexation, tags, éditions, pochettes, annulation et reprise incrémentale avec stockage isolé | Faible |
| 3. Fenêtre GPUI | Commandes au clic gauche, déplacement, redimensionnement, fermeture, choix de dossier, saisie et mise à l'échelle sous Wayland ; X11 en complément si disponible | Moyenne |
| 4. Lecture standard | WAV/FLAC/MP3, pause, seek, suivant, volume et arrêt via PipeWire ; réutiliser les tests audio natifs actuellement ignorés | Faible à moyenne |
| 5. Robustesse | Retrait/remontage d'une source, permissions, événements perdus, sortie audio indisponible et redémarrage sans repli silencieux | Moyenne |
| 6. Validation Silverblue | Même révision, même version Fedora et architecture ; vérifier bibliothèques dynamiques, GPU, portail, fichiers et son sur l'hôte | Moyenne |

Les tests existants couvrent déjà le domaine, le regroupement, les sources
indisponibles, les chemins non Unicode, le décodage et les états du moteur. Des
tests natifs audio sont ignorés par défaut et doivent être déclenchés explicitement
avec une sortie disponible. Les fixtures multicodecs nécessitent une génération
préalable ; le script existant utilise PowerShell et FFmpeg, uniquement comme
outils de développement. Le binaire `Soliton-index` sert de diagnostic sans fenêtre,
mais appartient à `soliton-app` et conserve donc ses dépendances de compilation.

Toolbx convient aux essais préliminaires sur Silverblue : il donne accès au
domicile, aux sockets graphiques et à D-Bus, comme l'indique la
[documentation Toolbx](https://containertoolbx.org/). Une validation uniquement
dans Toolbx ne prouve toutefois pas l'exécution du binaire sur l'hôte. Il faut
distinguer les deux contextes et vérifier les bibliothèques dynamiques du binaire
avec `ldd` avant les essais sur l'hôte. Aucun packaging Flatpak n'est nécessaire
à cette validation.

À chaque modification Rust future : `cargo fmt --check`, contrôles et tests
pertinents sur Linux, maintien de la CI Windows, puis essais matériels ciblés.
Ces vérifications n'ont pas été exécutées pendant cet audit en lecture seule.
