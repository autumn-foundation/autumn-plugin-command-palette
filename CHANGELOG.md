# Changelog

## [Unreleased]

### Added

- `CommandPalette`: a ⌘K command palette widget pack (autumn#3416).
  - ⌘K (Apple), Ctrl+K, a `Hotkey` that you set, or the invoker button
    can open it.
  - `GET /_palette` sends the results through htmx: groups, `kbd`
    shortcuts and recent items.
  - `CommandRegistry` holds routes and actions. `visible_when` filters a
    command, or all commands, for each request. Only a path on this
    origin gets the CSRF token.
  - The `admin` feature adds the `autumn-admin-plugin` resources.
  - With JavaScript off, the invoker is a link to a plain search page.
  - Three stories, registered through `AppBuilder::widget_pack`.
