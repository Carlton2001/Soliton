# Soliton: agent instructions

Soliton is a Rust/GPUI player for large local music libraries, with rich
metadata and fast navigation backed by a local database.

## References

- Use the [product decisions](docs/SOLITON_SPECIFICATIONS.md) as an optional
  reference for behavior the request leaves open. When an explicit request
  changes a decision, update the relevant documentation alongside the change.
- Use the sibling project `../Oxyde-GPUI` as a reference for components, visual
  conventions and `cx.listener` closures. Check compatibility with Soliton's
  GPUI version before reusing code. Respect licenses and attributions and keep
  Soliton independent of the Oxyde app.

## Invariants

- Never modify, move, rename or delete library files, their tags or embedded
  artwork. Keep databases, caches and temporary files outside source folders.
- Keep the entries of unreachable sources and mark them unavailable.
- Keep expensive work and file I/O off the UI thread. Indexing must be
  cancellable, with bounded concurrency for expensive reads and processing.
- Keep the audio engine independent of GPUI. The audio callback must avoid
  allocation, I/O, logging and blocking lock waits.

## Interface

- Prefer native interactions and left click for primary actions. Keep the
  default cursor for new interactions and add or change a cursor only on
  explicit request.
- Ship every UI string in French and English from the start.
- Use official Lucide SVGs from `lucide-icons/lucide/icons/<name>.svg` with paths
  unchanged. Report it if the upstream icon source is unreachable.

## Code organization

- Present high-impact architecture choices before implementing them.
- Separate domain, audio engine, persistence, indexing and UI. Keep business
  rules independent of GPUI components.
- Isolate OS integrations behind dedicated interfaces. Gate implementations and
  dependencies by target, with Windows types confined to Windows code.
- Give each module one identifiable responsibility. Facades declare and
  assemble their parts and delegate traversal and rendering.
- Use 200 to 600 production lines per file as a guideline, with no minimum.
  Exclude tests from the count. Above 800 lines, consider splitting before
  extending the file significantly. Explain the decision if it stays together.
- Split along coherent responsibilities and keep refactoring within scope.
  Share a component only when multiple uses share presentation or behavior.
  Keep visibility narrow and preserve GPUI identifiers and lifecycles.
- Put new unit tests in the module's `tests/` folder while preserving their
  logical Rust parent and visibility. Preserve scenarios, attributes and test
  discovery when moving tests.

## Validation

- After Rust changes, run `cargo fmt --check`, `cargo check` and the relevant
  tests. Resolve project warnings from `cargo check`. Report skipped checks and
  explain why they were skipped.
- Prioritize metadata, album and edition grouping, incremental indexing, source
  availability and audio engine states when validating behavior.
- After changing `Cargo.lock` or any `Cargo.toml`, run
  `scripts/update-third-party-licenses.ps1`, then
  `scripts/check-third-party-licenses.ps1`, then `scripts/audit-security.ps1`.
  The audit must pass before committing, with zero vulnerabilities or yanked
  crates. Include the regenerated `licenses/dependencies` in the commit.

## Contribution workflow

- Use `rg` for searches.
- Write commit messages in English.
- **Every push to GitHub requires an explicit request for that specific push.**
  This applies to branches and tags through any tool.
