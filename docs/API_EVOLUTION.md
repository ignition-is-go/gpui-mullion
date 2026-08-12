# Public API evolution

Mullion is pre-1.0, but its public surface is designed for use by multiple application crates.

## Canonical entry points

- Import everyday types from `gpui_mullion::prelude`; advanced types live in named modules.
- The root facade is an explicit compatibility list rather than module glob exports. Adding a public
  module helper therefore does not silently add a root-level contract.
- Use `MullionView::try_new`, `try_new_with_catalog`, or `try_new_with_workspaces` for persisted or
  untrusted input. Infallible `new` is reserved for host-built input and panics on invalid identity
  or topology.
- `MullionTheme` is the only look type. It contains semantic colors and all resolved component
  colors and geometry; install it explicitly with the application-global `set_mullion_theme`.
- Mullion re-exports its `gpui` and shared local `gpui_command_palette` dependencies for
  type-identity-safe downstream integration.
- `WorkspaceControls::editable().on_changed(...)` is the storage-neutral workspace boundary:
  consumers receive complete snapshots and own async persistence. `replace_workspaces` applies
  validated server state without calling the persistence hook back.

## Single-theme migration

The look API intentionally has no compatibility aliases:

- Replace `MullionStyles` or `MullionAppearance` with `MullionTheme`.
- Replace per-view appearance setters and providers with one application-level
  `set_mullion_theme(cx, complete_theme)` installation.
- Replace theme-mode APIs with `MullionTheme::light()`, `dark()`, or
  `system(window_appearance)`, then explicitly install the complete result before rendering.
- Build application themes with `MullionTheme::custom(...)`, then customize geometry directly on
  the complete theme before installing it.
- Application adapters should return only `MullionTheme` or `Arc<MullionTheme>`.
  Replacing the global does not repaint by itself; the application batches all theme installs and refreshes windows once.

## Stability classes

- `PaneNode`, identifiers, workspace snapshots, commands, and their Serde representations are
  persisted protocols. Their golden compatibility tests must pass before release.
- Public style records and catalog projection records are literal data-transfer types. Their public
  fields are intentional; incompatible field or meaning changes require a migration release.
- Public errors and emitted event enums are `non_exhaustive`. Consumers must retain a fallback arm,
  allowing Mullion to report new validation failures and events additively.
- Closure-backed GPUI chrome and factories are foreground-local. The catalog's `Activity` renderer
  is the deliberate stateless fallback; `ActivityFactoryRegistry` is the view-owned durable entity
  layer. Keeping them separate prevents catalog metadata from owning GPUI entity lifecycles.

## Compatibility shims

Hidden aliases and boolean/optional wrappers remain only for the known migration suite. New code
must use the documented typed `try_*` APIs. They are not part of the canonical facade and may be
removed at the next explicitly breaking pre-1.0 version after downstream migration.

## Release checks

The crate denies missing rustdoc, broken intra-doc links, and unsafe code. CI/release validation must
run the full feature suite, strict Clippy, rustdoc with warnings denied, the public-facade compile
checks, deterministic 1/8/29/128-pane scaling checks, native builds, release WebAssembly build, and
real Chrome canvas interaction smoke tests.
