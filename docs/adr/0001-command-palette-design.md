# ADR 0001: The command palette is a core dialog, a core combobox and one route

- Status: Accepted
- Date: 2026-11-05
- Issue: autumn#3416

## Context

The palette must open with ⌘K, get results from the server through htmx,
and work with JavaScript off. It must pass the same axe and keyboard gate
as core widgets.

## Decision

- The dialog is the core `modal()` with `light_dismiss`. The browser gives
  the focus trap, Escape and focus return.
- The input is an APG combobox. The listbox uses the keyboard core
  (`data-autumn-activedescendant`). The pack JS has no arrow-key code.
- One route, `GET /_palette`, has a fixed path. An htmx request gets the
  listbox content. Other requests get the search page. `SystemTest` takes
  only `Route`s, so the route is a `#[get]` route.
- A route command is `<a role="option" href>`. An action command is a
  `<button role="option" type="submit">` for a hidden `POST` form with the
  CSRF token. Enter clicks it, so the browser does the navigation.
- Recent ids are in `localStorage`. The server resolves them against the
  registry and `visible_when`.
- The ranking uses one pass for each rank. This is simple to prove: the
  Verus model in `verification/palette.rs` is the same loop.

## Consequences

- One palette for each app. The path `/_palette` is reserved.
- An admin command needs an explicit filter. This prevents a leak of admin
  links.
- `Page::press` sends no modifier keys. The browser tests send ⌘K and
  Ctrl+K as a page `KeyboardEvent`.
