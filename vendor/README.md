# Correctifs de dépendances

Copies des packages publiés sur crates.io, sans changement de version ni d'API.
Leurs licences et leurs sources sont conservées. Les caches Cargo de la machine
ne sont pas modifiés. `Cargo.toml` applique ces copies via `[patch.crates-io]`.
Les crates tierces restent exclues du workspace RAP.

## GPUI 0.2.2 — Apache-2.0

Source : https://crates.io/crates/gpui/0.2.2

- `src/platform/windows/directx_devices.rs` : l'échec de la sonde facultative
  DXGI Debug est un diagnostic debug. La création réelle du périphérique garde
  son traitement d'erreur et le repli sans Graphics Tools reste actif.
- `src/platform/windows/events.rs` (barre de titre personnalisée) : comme dans
  les versions ultérieures de GPUI, un clic non client n'est consommé que si un
  gestionnaire arrête sa propagation, et non plus dès que le focus appelle
  `prevent_default` ; le déplacement et les boutons natifs restent ainsi actifs.
  Le bord supérieur de redimensionnement est testé avant les zones de contrôle.
- `src/platform/windows/events.rs` : `WM_DESTROY` marque la fenêtre détruite,
  retire ses callbacks et révoque le glisser-déposer avant la perte du HWND.
- `src/platform/windows/window.rs` : le destructeur retire les callbacks vers
  l'entité GPUI supprimée et ne redétruit pas un HWND déjà détruit. Une seconde
  vérification est effectuée à l'exécution de la tâche de destruction différée.
- `src/taffy.rs` : préciser les deux littéraux `f32` pour éviter le repli de types
  que Rust 1.98 signale comme incompatibilité future (rust-lang/rust#154024).

Ces changements corrigent les diagnostics au démarrage et la double destruction
observés sous Windows. Aucun filtre global des journaux, aucun désarmement des
assertions debug et aucun changement de curseur ne sont appliqués.

## proc-macro-error2 2.0.1 — MIT OR Apache-2.0

Source : https://crates.io/crates/proc-macro-error2/2.0.1

`src/lib.rs` : rendre `extern crate proc_macro` public afin que sa réexportation
publique soit valide (E0365, rust-lang/rust#127909). Pas de changement de logique
des macros. Les autres sources du package sont inchangées.

Lors d'une mise à jour upstream, réexaminer ces modifications et les retirer une
fois les mêmes comportements corrigés et validés dans la version retenue.
