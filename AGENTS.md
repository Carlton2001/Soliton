# Soliton: agent instructions

Soliton is a high-performance Rust/GPUI audio library player for large local collections (FLAC, MP3, WAV, etc.), focused on rich metadata and fast navigation backed by a local database.

## References

- [Specifications](docs/SOLITON_SPECIFICATIONS.md): scope, product rules, phases. Read the relevant sections before each change; do not duplicate them here. Report contradictions before implementing the affected behavior, and present high-impact architecture choices before implementing them.
- GPUI reference: the sibling project `../Oxyde-GPUI` (components, visual conventions, `cx.listener` closures). Check compatibility with Soliton's GPUI version, reuse selectively after review, respect licenses and attributions, never depend on the Oxyde app.

## Standing rules

- No project warnings from `cargo check`. Search with `rg`, never `grep`.
- Keep expensive work and file I/O off the UI thread.
- Put OS integrations behind dedicated interfaces with target-gated implementations and dependencies; no Windows types in common code.
- Prefer native interactions and left click for primary actions.
- Icons: official Lucide SVGs from `lucide-icons/lucide/icons/<name>.svg`, paths unmodified. If the source is unreachable, report it.
- Never add or change a cursor without an explicit user request; new interactions keep the default cursor.
- Commit messages in English. **Every push to GitHub (branches and tags, any tool) needs an explicit request for that push.** A request to edit, commit or merge does not grant it, and a past approval does not cover the next push.
- Every UI string ships in French and English from the start.

## Invariants

- Never modify, move, rename or delete library files, their tags or embedded artwork. Databases, caches and temporary files live outside source folders.
- An unreachable source is not proof of deletion: keep its entries and mark them unavailable.
- The audio engine stays independent of GPUI. The audio callback does no allocation, I/O, logging or blocking lock waits.
- Indexing is cancellable; expensive reads and processing use bounded concurrency.

## Code organization

- Separate domain, audio engine, persistence, indexing and UI; business rules stay independent of GPUI components.
- One identifiable responsibility per module. Facades declare and assemble; they do not accumulate traversal or rendering.
- Guideline: 200 to 600 production lines per file, no minimum. Above 800, consider splitting before any significant extension and justify keeping it. No artificial splits; tests do not count.
- Extract coherent sections; share a component only when several uses share presentation or behavior. Preserve minimal visibility, GPUI identifiers and lifecycles. Keep refactoring within the change.
- New unit tests go in the module's `tests/` folder, without changing their logical Rust parent or widening visibility. When moving tests, keep scenarios, attributes and discovery.

## Validation

- After Rust changes: `cargo fmt --check`, `cargo check` and the relevant tests. State which checks were not run and why.
- Prioritize critical behavior: metadata, album and edition grouping, incremental indexing, source availability, audio engine states.
- Keep platform-specific code isolated.
