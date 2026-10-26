# Vérifications Windows après le support Linux

Commit concerné : `76ea35f Make Soliton functional on Linux`.

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
- **Sans effet sous Windows** : la politique des sorties audio
  (`OUTPUTS_LISTED` vaut `true`) et la restauration de l'état agrandi sont
  limitées à Linux.

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

Fermeture :

- [ ] « Masquer » cache la fenêtre ; l'icône de notification la restaure.
- [ ] « Demander » affiche la boîte de confirmation avec l'option Masquer.
- [ ] « Quitter », depuis la fenêtre ou depuis la notification, ferme proprement.

## 4. Point à vérifier, antérieur à ce commit

`scripts/build-distribution.ps1` construit `--bin soliton`, alors que la cible
Cargo s'appelle `Soliton`. Cargo distingue la casse : la construction de la
distribution Windows échoue probablement. À contrôler avec :

```powershell
./scripts/build-distribution.ps1 -PortableOnly
```

- [ ] La distribution portable se construit, ou le nom de cible est corrigé.

## Résultat

En cas d'échec, noter la commande, la sortie ou le comportement observé.
