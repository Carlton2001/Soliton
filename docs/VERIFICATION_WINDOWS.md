# Vérifications Windows après le support Linux

Commits concernés : `76ea35f Make Soliton functional on Linux`, puis la sortie
PipeWire native sous Linux.

Sous Linux, tout a été validé. Côté Windows, seul `soliton-platform` a été
vérifié par `cargo check` croisé, tests compris. L'application n'a pas pu être
compilée pour Windows depuis Linux (shaders GPUI, `aws-lc-sys`).

## Ce qui touche Windows

- **Démarrage des intégrations** : `Desktop::start` reçoit maintenant la fenêtre
  GPUI (`&impl HasWindowHandle`) au lieu d'un `RawWindowHandle`. La plateforme
  lit elle-même le handle sous Windows. Ce chemin démarre le panneau média
  (SMTC), la vignette de barre des tâches, la zone de notification et les icônes.
- **Capacités `Desktop`** : `Desktop::SUPPORTED` est complété par
  `Desktop::HIDES` et `Desktop::AUTOSTART`. Sous Windows, le comportement doit
  rester identique : fermeture configurable, masquage, démarrage automatique.
- **Inventaire des licences** : les empreintes de `Cargo.lock`,
  `crates/soliton-app/Cargo.toml` et `crates/soliton-platform/Cargo.toml` ont
  changé. Les notices régénérées sont identiques.
- **Sortie audio** : le code CPAL a été déplacé dans `cpal_backend.rs`, désormais
  compilé hors Linux seulement, sans changement de comportement voulu. Le rendu
  du callback reçoit la conversion d'échantillon en paramètre. Les sorties
  portent un libellé (égal au nom sous Windows) et un indicateur Bluetooth
  (toujours faux sous Windows). `soliton-audio` a été vérifié par
  `cargo xwin check` pour `x86_64-pc-windows-msvc`, tests compris.
- **Perte de sortie** (commun à Windows) : la lecture passe en pause au lieu de
  l'état d'échec ; la reprise utilise la sortie par défaut, puis Soliton revient
  sur la sortie choisie quand l'énumération la montre de nouveau.
- **Restauration de la fenêtre** : sans écran connu à l'ouverture, la position
  enregistrée est reprise sans contrôle. Windows connaît ses écrans dès
  l'ouverture : le contrôle s'applique toujours.
- **Sans effet sous Windows** : la restauration de l'état agrandi au démarrage
  reste limitée à Linux.

## 1. Compilation et tests

Après récupération du commit :

```powershell
cargo +1.98.1 fmt --check
cargo +1.98.1 check --workspace --all-targets --locked
cargo +1.98.1 test --workspace --locked
./scripts/check-third-party-licenses.ps1
```

- [ ] `fmt --check` réussi
- [ ] `check` réussi, sans avertissement du projet
- [ ] Tests du workspace réussis
- [ ] Inventaire des licences à jour

## 2. Test natif de l'intégration Windows

Ce test passe par le nouveau chemin du handle de fenêtre, qui a été adapté :

```powershell
cargo +1.98.1 test -p soliton-platform native_tray_smtc_artwork_commands_and_cleanup -- --ignored
```

- [ ] Réussi

Tests audio natifs, à volume nul, inchangés mais utiles en contrôle :

```powershell
cargo +1.98.1 test -p soliton-audio native_output -- --ignored
cargo +1.98.1 test -p soliton-audio unavailable_preference -- --ignored
```

- [ ] Réussis

## 3. Essai manuel de l'application

```powershell
cargo +1.98.1 run -p soliton-app --bin Soliton --locked
```

Fenêtre et icônes :

- [ ] L'icône de la fenêtre et de la barre des tâches s'affiche au démarrage.
- [ ] Changer la palette dans Paramètres → Icône de l'application met à jour la
  fenêtre, la barre des tâches et la zone de notification.

Panneau média et barre des tâches :

- [ ] Pendant une lecture, le panneau média Windows affiche titre, artiste et
  pochette.
- [ ] Lecture/Pause, Précédent et Suivant fonctionnent depuis le panneau et les
  touches multimédias.
- [ ] Les boutons de la vignette de barre des tâches fonctionnent.

Paramètres :

- [ ] La section s'appelle toujours « Intégration Windows ».
- [ ] Elle contient « À la fermeture » (Quitter / Masquer / Demander) et la case
  « Lancer Soliton à l'ouverture de session ».
- [ ] La case de démarrage automatique s'active et se désactive (valeur
  `Soliton` sous `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`).
- [ ] La section « Sortie audio » liste toujours les périphériques, pas seulement
  la sortie système, sans la mention « choisir dans les paramètres Son ».
- [ ] Choisir un périphérique pendant la lecture garde piste et position ; le
  format affiché et l'infobulle de sortie du lecteur nomment ce périphérique.
- [ ] Le texte sous la liste reste « Le choix système par défaut s'applique au
  démarrage d'une piste… ».
- [ ] Une sortie choisie puis débranchée met la lecture en pause, avec
  l'avertissement « Lecture en pause » au-dessus de la barre de lecture. La
  reprise joue sur la sortie par défaut ; rebrancher la sortie y ramène la
  lecture dès que Windows signale le périphérique.

Fermeture :

- [ ] « Masquer » cache la fenêtre ; l'icône de notification la restaure.
- [ ] « Demander » affiche la boîte de confirmation avec l'option Masquer.
- [ ] « Quitter », depuis la fenêtre ou depuis la notification, ferme proprement.

## 4. Distribution Windows

`scripts/build-distribution.ps1` construisait `--bin soliton` depuis le renommage
RAP → Soliton, alors que la cible Cargo s'appelle `Soliton`. Le workflow
« Windows distribution » échouait pour cette raison ; le nom est corrigé.

```powershell
./scripts/build-distribution.ps1 -PortableOnly
```

- [ ] La distribution portable se construit (ou le workflow « Windows
  distribution » réussit).

## Résultat

En cas d'échec, noter la commande, la sortie ou le comportement observé.
