//! The markup of the palette.
//!
//! The ids start with the palette id `P`:
//!
//! | Id | Element |
//! |---|---|
//! | `P` | the `<dialog>` |
//! | `P-input` | the combobox |
//! | `P-recent` | the hidden recent ids |
//! | `P-list` | the listbox |
//! | `P-status` | the status line |
//! | `P-group-N` | the label of group N |
//! | `P-option-N` | option N |
//! | `P-action` | the hidden `POST` form of the actions |

use autumn_web::widgets::{ModalConfig, kbd, kbd_combo, modal};
use maud::{DOCTYPE, Markup, html};

use crate::command::{Command, CommandTarget};
use crate::palette::{Csrf, Inner, PALETTE_PATH, pack};
use crate::search::SearchResults;

/// The htmx triggers of the combobox. `command-palette-open` comes from the
/// pack JS when the dialog opens. A plain `input` trigger (no `changed`)
/// sends the same query again after the dialog opens again.
const TRIGGERS: &str = "input delay:150ms, command-palette-open";

/// The mark in `Labels::count` that the number replaces.
const COUNT_MARK: &str = concat!("{", "count", "}");

/// The `kbd` of the hotkey. The pack JS changes the `mod` key to `⌘` on
/// Apple platforms.
fn hotkey_kbd(palette: &Inner) -> Markup {
    let labels = palette.hotkey.labels();
    let primary = palette.hotkey.uses_primary();
    html! {
        kbd class="autumn-kbd-combo" {
            @for (i, label) in labels.iter().enumerate() {
                @if i > 0 { "+" }
                @if i == 0 && primary {
                    kbd class="autumn-kbd" data-command-palette-mod { (label) }
                } @else {
                    kbd class="autumn-kbd" { (label) }
                }
            }
        }
    }
}

pub fn invoker(palette: &Inner) -> Markup {
    let keys = palette.hotkey.aria_keyshortcuts();
    html! {
        a class="autumn-command-palette-link" href=(PALETTE_PATH)
            data-command-palette-link=(palette.id) {
            span class="autumn-command-palette-invoker-label" { (palette.labels.invoker) }
            (hotkey_kbd(palette))
        }
        button type="button" class="autumn-command-palette-invoker" hidden
            data-command-palette-open=(palette.id) aria-haspopup="dialog"
            aria-keyshortcuts=(keys) {
            span class="autumn-command-palette-invoker-label" { (palette.labels.invoker) }
            (hotkey_kbd(palette))
        }
    }
}

/// The search panel. `results` is `None` in the dialog: the server fills
/// the listbox when the dialog opens.
fn panel(palette: &Inner, results: Option<&SearchResults<'_>>) -> Markup {
    let id = &palette.id;
    let input_id = format!("{id}-input");
    let list_id = format!("{id}-list");
    let recent_id = format!("{id}-recent");
    let in_dialog = results.is_none();
    let expanded = results.is_some_and(|r| !r.is_empty());
    let active = expanded.then(|| format!("{id}-option-0"));
    let class = if in_dialog {
        "autumn-command-palette-panel"
    } else {
        "autumn-command-palette-panel autumn-command-palette-preview"
    };
    html! {
        div class=(class) data-command-palette=(id)
            data-command-palette-hotkey=(palette.hotkey.spec())
            data-command-palette-recent-max=(palette.limits.recent)
            data-command-palette-error=(palette.labels.error) {
            label class="autumn-visually-hidden" for=(input_id) { (palette.labels.input) }
            // `type="text"`: in a search input, the first Escape clears
            // the text. Here Escape closes the dialog.
            input id=(input_id) class="autumn-command-palette-input" type="text" name="q"
                value=[results.map(|r| r.query.as_str())]
                role="combobox" aria-expanded=(if expanded { "true" } else { "false" })
                aria-autocomplete="list" aria-controls=(list_id)
                aria-activedescendant=[active.as_deref()]
                autocomplete="off" spellcheck="false" autofocus[in_dialog]
                placeholder=(palette.labels.placeholder)
                hx-get=(PALETTE_PATH) hx-trigger=(TRIGGERS) hx-target=(format!("#{list_id}"))
                hx-swap="innerHTML" hx-include=(format!("#{recent_id}")) hx-sync="this:replace";
            input type="hidden" id=(recent_id) name="recent" value="";
            div id=(list_id) class="autumn-command-palette-list" role="listbox"
                aria-label=(palette.labels.results)
                data-autumn-activedescendant data-autumn-orientation="vertical" {
                @if let Some(results) = results {
                    (options(palette, results, None))
                }
            }
            p id=(format!("{id}-status")) class="autumn-command-palette-status" role="status" {
                @if let Some(results) = results {
                    (status_text(palette, results))
                }
            }
        }
    }
}

/// The footer: the keys of the listbox.
fn footer(palette: &Inner) -> Markup {
    let labels = &palette.labels;
    html! {
        p class="autumn-command-palette-keys" {
            span class="autumn-command-palette-key" { (kbd("↑")) (kbd("↓")) " " (labels.hint_move) }
            span class="autumn-command-palette-key" { (kbd("Enter")) " " (labels.hint_select) }
            span class="autumn-command-palette-key" { (kbd("Esc")) " " (labels.hint_close) }
        }
    }
}

pub fn dialog(palette: &Inner) -> Markup {
    let config = ModalConfig::new()
        .light_dismiss(true)
        .class("autumn-command-palette-dialog")
        .close_button(palette.labels.close.clone())
        .footer(footer(palette));
    modal(
        &palette.id,
        &palette.labels.title,
        &panel(palette, None),
        &config,
    )
}

/// The text of the status line: the count, or the empty text.
fn status_text(palette: &Inner, results: &SearchResults<'_>) -> Markup {
    let count = results.len();
    html! {
        @if count == 0 {
            span class="autumn-command-palette-empty" { (palette.labels.empty) }
        } @else {
            span class="autumn-visually-hidden" {
                @if count == 1 {
                    (palette.labels.count_one)
                } @else {
                    (palette.labels.count.replace(COUNT_MARK, &count.to_string()))
                }
            }
        }
    }
}

/// `true` when `command` has a hint or a shortcut.
fn has_detail(command: &Command) -> bool {
    command.hint_text().is_some() || !command.shortcut_keys().is_empty()
}

/// The content of one option. The name is the title. The hint and the keys
/// are the description (`{option_id}-desc`).
fn option_body(command: &Command, option_id: &str) -> Markup {
    let keys: Vec<&str> = command.shortcut_keys().iter().map(String::as_str).collect();
    html! {
        span class="autumn-command-palette-title" { (command.title()) }
        @if has_detail(command) {
            span id=(format!("{option_id}-desc")) class="autumn-command-palette-detail"
                aria-hidden="true" {
                @if let Some(hint) = command.hint_text() {
                    span class="autumn-command-palette-hint" { (hint) }
                }
                (kbd_combo(&keys))
            }
        }
    }
}

/// `true` for a path on this origin: it starts with one `/`. Only such a
/// path gets the CSRF token. A `//host` or `/\host` path goes to another
/// host in a browser.
fn same_origin(path: &str) -> bool {
    path.starts_with('/') && !path.starts_with("//") && !path.starts_with("/\\")
}

/// The groups and options of the listbox. The first option is active.
fn options(palette: &Inner, results: &SearchResults<'_>, csrf: Option<&Csrf>) -> Markup {
    let id = &palette.id;
    let form_id = format!("{id}-action");
    let external_id = format!("{id}-action-external");
    let action_paths: Vec<&str> = results
        .commands()
        .filter_map(|command| match command.target() {
            CommandTarget::Action(path) => Some(path.as_str()),
            CommandTarget::Route(_) => None,
        })
        .collect();
    let has_local = action_paths.iter().any(|path| same_origin(path));
    let has_external = action_paths.iter().any(|path| !same_origin(path));
    // Number the options across the groups.
    let mut next = 0usize;
    let numbered: Vec<Vec<(usize, &Command)>> = results
        .groups
        .iter()
        .map(|group| {
            group
                .commands
                .iter()
                .map(|&command| {
                    next += 1;
                    (next - 1, command)
                })
                .collect()
        })
        .collect();
    html! {
        @for (g, (group, items)) in results.groups.iter().zip(&numbered).enumerate() {
            @let label_id = format!("{id}-group-{g}");
            div class="autumn-command-palette-group" role="group" aria-labelledby=(label_id) {
                div id=(label_id) class="autumn-command-palette-group-label" { (group.label) }
                @for &(n, command) in items {
                    @let option_id = format!("{id}-option-{n}");
                    @let selected = if n == 0 { "true" } else { "false" };
                    @let described = has_detail(command).then(|| format!("{option_id}-desc"));
                    @match command.target() {
                        CommandTarget::Route(href) => {
                            a id=(option_id) class="autumn-command-palette-option" role="option"
                                tabindex="-1" aria-selected=(selected)
                                data-command-id=(command.id()) href=(href)
                                aria-describedby=[described.as_deref()] {
                                (option_body(command, &option_id))
                            }
                        }
                        CommandTarget::Action(path) => {
                            button id=(option_id) class="autumn-command-palette-option" role="option"
                                tabindex="-1" aria-selected=(selected)
                                data-command-id=(command.id()) type="submit"
                                form=(if same_origin(path) { &form_id } else { &external_id })
                                formaction=(path) formmethod="post"
                                aria-describedby=[described.as_deref()] {
                                (option_body(command, &option_id))
                            }
                        }
                    }
                }
            }
        }
        @if has_local {
            form id=(form_id) method="post" hidden {
                @if let Some(csrf) = csrf {
                    input type="hidden" name=(csrf.field) value=(csrf.token);
                }
            }
        }
        @if has_external {
            form id=(external_id) method="post" hidden {}
        }
    }
}

pub fn fragment(palette: &Inner, results: &SearchResults<'_>, csrf: Option<&Csrf>) -> Markup {
    html! {
        (options(palette, results, csrf))
        p id=(format!("{}-status", palette.id)) hx-swap-oob="innerHTML" {
            (status_text(palette, results))
        }
    }
}

pub fn page_body(palette: &Inner, results: &SearchResults<'_>, csrf: Option<&Csrf>) -> Markup {
    let id = &palette.id;
    let labels = &palette.labels;
    let input_id = format!("{id}-page-q");
    html! {
        div class="autumn-command-palette-page" {
            h1 class="autumn-command-palette-page-title" { (labels.page_title) }
            form class="autumn-command-palette-page-search" role="search" method="get"
                action=(PALETTE_PATH) {
                label class="autumn-command-palette-page-label" for=(input_id) { (labels.input) }
                input id=(input_id) class="autumn-command-palette-page-input" type="search"
                    name="q" value=(results.query);
                button class="autumn-command-palette-page-submit" type="submit" { (labels.submit) }
            }
            @if results.is_empty() {
                p class="autumn-command-palette-empty" { (labels.empty) }
            }
            @for (g, group) in results.groups.iter().enumerate() {
                @let title_id = format!("{id}-page-group-{g}");
                section class="autumn-command-palette-page-group" aria-labelledby=(title_id) {
                    h2 id=(title_id) class="autumn-command-palette-page-group-title" { (group.label) }
                    ul class="autumn-command-palette-page-list" {
                        @for command in &group.commands {
                            li class="autumn-command-palette-page-item" {
                                (page_item(command, csrf))
                            }
                        }
                    }
                }
            }
        }
    }
}

/// One command on the search page: a link, or a `POST` form.
fn page_item(command: &Command, csrf: Option<&Csrf>) -> Markup {
    let keys: Vec<&str> = command.shortcut_keys().iter().map(String::as_str).collect();
    html! {
        @match command.target() {
            CommandTarget::Route(href) => {
                a class="autumn-command-palette-page-link" href=(href) { (command.title()) }
            }
            CommandTarget::Action(path) => {
                form class="autumn-command-palette-page-action" method="post" action=(path) {
                    @if let Some(csrf) = csrf.filter(|_| same_origin(path)) {
                        input type="hidden" name=(csrf.field) value=(csrf.token);
                    }
                    button class="autumn-command-palette-page-button" type="submit" { (command.title()) }
                }
            }
        }
        @if let Some(hint) = command.hint_text() {
            span class="autumn-command-palette-hint" { (hint) }
        }
        (kbd_combo(&keys))
    }
}

pub fn page(palette: &Inner, results: &SearchResults<'_>, csrf: Option<&Csrf>) -> Markup {
    let body = page_body(palette, results, csrf);
    let title = &palette.labels.page_title;
    if let Some(layout) = &palette.layout {
        return layout(title, body);
    }
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                title { (title) }
                link rel="stylesheet" href=(autumn_web::ui::WIDGETS_CSS_PATH);
                (pack().head())
            }
            body {
                main class="autumn-command-palette-page-main" { (body) }
            }
        }
    }
}

pub fn preview(palette: &Inner, results: &SearchResults<'_>) -> Markup {
    panel(palette, Some(results))
}
