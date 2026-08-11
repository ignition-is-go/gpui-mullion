# Public API evolution

Mullion is pre-1.0, but its public surface is designed for use by multiple application crates.

## Canonical entry points

- Import everyday types from `gpui_mullion::prelude`; advanced types live in named modules.
- The root facade is an explicit compatibility list rather than module glob exports. Adding a public
  module helper therefore does not silently add a root-level contract.
- Use `MullionView::try_new`, `try_new_with_catalog`, or `try_new_with_workspaces` for persisted or
  untrusted input. Infallible `new` is reserved for host-built input and panics on invalid identity
  or topology.
- `MullionTheme` is the semantic color source. `MullionAppearance` is the only complete resolved
  visual/geometry bundle; `MullionView::with_theme[_provider]` derives it internally, while
  `with_appearance[_provider]` installs exact component tokens.
- Mullion re-exports its exact `gpui` and `gpui_command_palette` revisions for type-identity-safe
  downstream integration.

## Single-look migration

The resolved look API intentionally has no compatibility alias:

- Replace `MullionStyles` with `MullionAppearance`.
- Replace `MullionAppearance::system/light/dark` with
  `MullionView::with_theme_mode(MullionThemeMode::System/Light/Dark)`.
- Replace `MullionAppearance::theme(theme)` with `MullionView::with_theme(theme)`.
- Replace `MullionAppearance::styles(styles)` with `MullionView::with_appearance(appearance)`.
- Replace `MullionAppearance::custom(theme, styles)` by starting with
  `MullionAppearance::from_theme(theme)`, modifying its public component fields, and installing it.
- Application theme adapters should return only `MullionTheme` and connect through
  `with_theme_provider`; exact appearance providers are reserved for live geometry or
  component-specific overrides.

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
