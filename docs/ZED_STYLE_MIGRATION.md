# Zed-style public API migration

The pre-1.0 compatibility surface was removed in favor of the patterns used by the pinned Zed GPUI revision.

- Construct and retain `Entity<MullionView<D>>` explicitly. It implements `Render`, `Focusable`, and emits `PaneEvent<D>`, `WorkspaceChanged`, and `WorkspaceEvent<D>`.
- Use `try_new`, `try_new_with_catalog`, and `try_new_with_workspaces` when inputs need validation. The optional/boolean pseudo-constructors and hidden aliases were removed.
- Use `focus_presentation`, `workspaces`, `try_switch_workspace`, `WorkspaceSet::try_switch`, and `WorkspaceSet::try_persist_active` as the single canonical APIs.
- Icons are typed `LucideIcon` values rendered with the `IconElement::new(icon)` builder. `IconElement` is decorative by default and supports stable ID, size, color, and an opt-in accessible label.
- Keyboard activation dispatches typed Mullion actions before semantic mutation; pointer and accessibility activation reach the same caller-owned entity.

No pane tree, workspace, theme, command-palette, native, or WebAssembly serialization contract changed.
