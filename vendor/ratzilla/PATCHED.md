# Patched ratzilla 0.3.1

A copy of [ratzilla](https://github.com/ratatui/ratzilla) 0.3.1 from crates.io
(MIT OR Apache-2.0, © Orhun Parmaksız and contributors), used through
`[patch.crates-io]` in the top-level `Cargo.toml`.

Two changes, both in `src/backend/dom.rs` and marked `space-travel patch`:

1. `Backend::size` returns the grid size measured from the grid's parent
   element, the same calculation used when the grid is built. Upstream returns
   an estimate from the window size (on phones, the whole screen), which
   disagrees with the grid inside an iframe, so ratatui drew past its end.
2. `draw` skips cells outside the grid instead of indexing out of bounds,
   which panicked on the first frame after a resize.

Both are worth sending upstream; drop this copy once a release includes them.
