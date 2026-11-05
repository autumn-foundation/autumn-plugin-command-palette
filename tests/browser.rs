//! The palette in Chromium (autumn#3416).
//!
//! - The pack passes the browser conformance (axe, console, keyboard).
//! - The APG combobox in the dialog: hotkeys, invoker, arrows, Enter,
//!   Escape, focus return, recent items, actions and axe on the open dialog.
//! - With JavaScript off, the invoker is a link to the search page.
//!
//! `Page::press` sends no modifier keys. The tests send ⌘K and Ctrl+K as a
//! `KeyboardEvent` from the page.
//!
//! Run: `cargo test --features system-tests --test browser -- --include-ignored`

use std::time::Duration;

use autumn_command_palette_plugin::{
    ASSETS, Command, CommandPalette, CommandRegistry, Hotkey, pack,
};
use autumn_web::AppState;
use autumn_web::config::AutumnConfig;
use autumn_web::plugin_conformance::widgets::browser::run_widget_browser_conformance;
use autumn_web::prelude::*;
use autumn_web::reexports::axum::extract::State;
use autumn_web::system_test::{Page, SystemTest, SystemTestRunner};
use maud::{DOCTYPE, Markup, html};

const INPUT: &str = "#command-palette-input";
const DIALOG: &str = "document.getElementById('command-palette')";

fn axe_source() -> String {
    let path =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/axe-core/axe.min.js");
    std::fs::read_to_string(&path).unwrap_or_else(|err| panic!("read {}: {err}", path.display()))
}

fn registry() -> CommandRegistry {
    CommandRegistry::new()
        .add(Command::route("home", "Dashboard", "/app").shortcut(&["G", "D"]))
        .add(Command::route("posts", "Posts", "/posts").keywords(["articles"]))
        .add(Command::route("settings", "Settings", "/settings").group("Account"))
        .add(Command::action("posts.new", "New post", "/posts/new").hint("Write a draft"))
}

#[get("/app")]
async fn app_page(State(state): State<AppState>) -> Markup {
    let palette = state.extension::<CommandPalette>().expect("palette");
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "App" }
                link rel="stylesheet" href=(autumn_web::ui::WIDGETS_CSS_PATH);
                script src="/static/js/htmx.min.js" defer {}
                script src="/static/js/autumn-widgets.js" defer {}
                (pack().head())
            }
            body {
                header { (*palette) }
                main {
                    h1 { "App" }
                    label for="note" { "Note" }
                    input id="note" type="text";
                }
            }
        }
    }
}

/// A page without htmx: the palette cannot load results.
#[get("/bare")]
async fn bare_page(State(state): State<AppState>) -> Markup {
    let palette = state.extension::<CommandPalette>().expect("palette");
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                title { "Bare" }
                script src="/static/js/autumn-widgets.js" defer {}
                (pack().head())
            }
            body { main { h1 { "Bare" } (*palette) } }
        }
    }
}

#[get("/posts")]
async fn posts_page() -> Markup {
    html! { (DOCTYPE) html lang="en" { head { title { "Posts" } } body { main { h1 { "Posts page" } } } } }
}

#[post("/posts/new")]
async fn new_post() -> Markup {
    html! { (DOCTYPE) html lang="en" { head { title { "Created" } } body { main { h1 { "Post created" } } } } }
}

async fn start(palette: CommandPalette, csrf: bool) -> (SystemTestRunner, Page) {
    let mut config = AutumnConfig {
        profile: Some("test".into()),
        ..Default::default()
    };
    config.security.csrf.enabled = csrf;
    let state = AppState::for_test().with_profile("test");
    state.insert_extension(config);
    state.insert_extension(palette);
    let runner = Box::pin(
        SystemTest::new()
            .state(state)
            .routes(routes![
                autumn_command_palette_plugin::route::command_palette_search,
                app_page,
                bare_page,
                posts_page,
                new_post
            ])
            .plugin_assets(&ASSETS)
            .build(),
    )
    .await
    .expect("start runner");
    let page = runner.page().await.expect("page");
    (runner, page)
}

async fn eval_bool(page: &Page, js: &str) -> bool {
    page.evaluate(js)
        .await
        .ok()
        .and_then(|r| r.into_value::<bool>().ok())
        .unwrap_or(false)
}

async fn eval_string(page: &Page, js: &str) -> String {
    page.evaluate(js)
        .await
        .ok()
        .and_then(|r| r.into_value::<String>().ok())
        .unwrap_or_default()
}

async fn poll(page: &Page, js: &str) -> bool {
    for _ in 0..100 {
        if eval_bool(page, js).await {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    false
}

async fn must(page: &Page, js: &str, why: &str) {
    assert!(poll(page, js).await, "{why} (`{js}`)");
}

/// Load the app page and wait for htmx, the core and the pack.
async fn open_app(page: &Page) {
    page.visit("/app").await.expect("visit");
    must(page, "document.readyState === 'complete'", "page loads").await;
    must(page, "window.__autumnWidgets === true", "core runs").await;
    must(
        page,
        "document.querySelector('.autumn-command-palette-panel').hasAttribute('data-command-palette-ready')",
        "pack runs",
    )
    .await;
}

/// Send a key with modifiers from the focused element. `Page::press` has no
/// modifiers.
async fn chord(page: &Page, key: &str, ctrl: bool, meta: bool) -> bool {
    send_key(page, key, ctrl, meta, false).await
}

/// Send one `keydown`. `true` when a handler called `preventDefault`.
async fn send_key(page: &Page, key: &str, ctrl: bool, meta: bool, repeat: bool) -> bool {
    eval_bool(
        page,
        &format!(
            "(() => {{ const t = document.activeElement || document.body; \
             const e = new KeyboardEvent('keydown', {{ key: '{key}', ctrlKey: {ctrl}, \
             metaKey: {meta}, repeat: {repeat}, bubbles: true, cancelable: true }}); \
             return !t.dispatchEvent(e); }})()"
        ),
    )
    .await
}

/// The `aria-activedescendant` of the combobox, or `''`.
const ACTIVE_ID: &str = "(document.getElementById('command-palette-input').getAttribute('aria-activedescendant') || '')";

async fn wait_for_options(page: &Page) {
    must(
        page,
        "document.querySelectorAll('#command-palette-list [role=option]').length > 0",
        "the server sends options",
    )
    .await;
    page.expect_hx_settle().await.expect("settle");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn the_pack_passes_browser_conformance() {
    let report = run_widget_browser_conformance(&pack(), &axe_source())
        .await
        .expect("browser conformance runs");
    assert!(report.passed(), "{}", report.to_text_report());
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn ctrl_k_opens_an_apg_combobox_in_a_dialog() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    open_app(&page).await;
    must(
        &page,
        &format!("!{DIALOG}.open"),
        "the dialog starts closed",
    )
    .await;
    assert!(chord(&page, "k", true, false).await, "Ctrl+K is handled");
    must(&page, &format!("{DIALOG}.open"), "Ctrl+K opens the dialog").await;
    must(
        &page,
        "document.activeElement && document.activeElement.id === 'command-palette-input'",
        "focus is on the combobox",
    )
    .await;
    wait_for_options(&page).await;
    must(
        &page,
        "document.getElementById('command-palette-input').getAttribute('aria-expanded') === 'true'",
        "aria-expanded is true with options",
    )
    .await;
    must(
        &page,
        &format!("{ACTIVE_ID} === 'command-palette-option-0'"),
        "the first option is active",
    )
    .await;

    // ArrowDown and ArrowUp move aria-activedescendant. Focus stays on the input.
    page.press("ArrowDown").await.expect("down");
    must(
        &page,
        &format!("{ACTIVE_ID} === 'command-palette-option-1'"),
        "ArrowDown moves",
    )
    .await;
    page.press("ArrowUp").await.expect("up");
    page.press("ArrowUp").await.expect("up");
    let last = eval_string(
        &page,
        "[...document.querySelectorAll('#command-palette-list [role=option]')].pop().id",
    )
    .await;
    must(
        &page,
        &format!("{ACTIVE_ID} === '{last}'"),
        "ArrowUp wraps to the last option",
    )
    .await;
    must(
        &page,
        "document.activeElement.id === 'command-palette-input'",
        "focus stays on the combobox",
    )
    .await;
    must(
        &page,
        &format!("document.getElementById('{last}').getAttribute('aria-selected') === 'true'"),
        "the active option is selected",
    )
    .await;

    // The open dialog with results passes axe.
    let violations = page.axe_violations(&axe_source()).await.expect("axe");
    assert!(violations.is_empty(), "{violations:#?}");

    // Ctrl+K again closes it.
    assert!(chord(&page, "k", true, false).await);
    must(
        &page,
        &format!("!{DIALOG}.open"),
        "Ctrl+K again closes the dialog",
    )
    .await;
    page.expect_no_console_errors()
        .await
        .expect("no console errors");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn meta_k_opens_the_palette() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    open_app(&page).await;
    assert!(chord(&page, "k", false, true).await, "⌘K is handled");
    must(&page, &format!("{DIALOG}.open"), "⌘K opens the dialog").await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn the_invoker_opens_and_escape_returns_focus() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    open_app(&page).await;
    must(
        &page,
        "!document.querySelector('[data-command-palette-open]').hidden && \
         document.querySelector('.autumn-command-palette-link').hidden",
        "the JS shows the button and hides the link",
    )
    .await;
    page.click("[data-command-palette-open]")
        .await
        .expect("click");
    must(
        &page,
        &format!("{DIALOG}.open"),
        "the invoker opens the dialog",
    )
    .await;
    wait_for_options(&page).await;
    page.press("Escape").await.expect("escape");
    must(
        &page,
        &format!("!{DIALOG}.open"),
        "Escape closes the dialog",
    )
    .await;
    must(
        &page,
        "document.activeElement && document.activeElement.matches('[data-command-palette-open]')",
        "focus goes back to the invoker",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn typing_asks_the_server_and_enter_goes_to_the_route() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    open_app(&page).await;
    assert!(chord(&page, "k", true, false).await);
    wait_for_options(&page).await;
    page.fill(INPUT, "articles").await.expect("fill");
    must(
        &page,
        "(() => { const o = [...document.querySelectorAll('#command-palette-list [role=option]')]; \
         return o.length === 1 && o[0].dataset.commandId === 'posts'; })()",
        "the server filters the commands",
    )
    .await;
    must(
        &page,
        &format!("{ACTIVE_ID} === 'command-palette-option-0'"),
        "the new first option is active",
    )
    .await;
    must(
        &page,
        "document.getElementById('command-palette-status').textContent.includes('1 result')",
        "the status tells the count",
    )
    .await;
    page.press("Enter").await.expect("enter");
    page.expect_text("Posts page")
        .await
        .expect("Enter goes to the route");

    // The pick is now a recent item.
    open_app(&page).await;
    assert!(chord(&page, "k", true, false).await);
    wait_for_options(&page).await;
    must(
        &page,
        "(() => { const g = document.querySelector('#command-palette-list [role=group]'); \
         return !!g && g.textContent.includes('Recent') && \
         g.querySelector('[role=option]').dataset.commandId === 'posts'; })()",
        "the recent group shows the last pick first",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn enter_on_an_action_posts_with_the_csrf_token() {
    let (_runner, page) = start(CommandPalette::new(registry()), true).await;
    open_app(&page).await;
    assert!(chord(&page, "k", true, false).await);
    wait_for_options(&page).await;
    page.fill(INPUT, "new post").await.expect("fill");
    must(
        &page,
        "(() => { const o = document.querySelector('#command-palette-list [role=option]'); \
         return !!o && o.dataset.commandId === 'posts.new'; })()",
        "the action shows",
    )
    .await;
    page.press("Enter").await.expect("enter");
    page.expect_text("Post created")
        .await
        .expect("Enter sends the POST");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn a_custom_hotkey_opens_it_but_not_in_a_text_field() {
    let palette = CommandPalette::new(registry()).hotkey(Hotkey::parse("/").expect("valid"));
    let (_runner, page) = start(palette, false).await;
    open_app(&page).await;
    page.click("#note").await.expect("focus the text field");
    page.press("/").await.expect("slash");
    tokio::time::sleep(Duration::from_millis(200)).await;
    assert!(
        !eval_bool(&page, &format!("{DIALOG}.open")).await,
        "`/` types in a text field"
    );
    must(
        &page,
        "document.getElementById('note').value === '/'",
        "the text field gets `/`",
    )
    .await;
    eval_bool(
        &page,
        "(() => { document.activeElement.blur(); return true; })()",
    )
    .await;
    page.press("/").await.expect("slash");
    must(&page, &format!("{DIALOG}.open"), "`/` opens the dialog").await;
    assert!(
        !chord(&page, "k", true, false).await,
        "Ctrl+K is not the hotkey now"
    );
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn without_javascript_the_invoker_is_a_search_page_link() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    page.set_javascript_enabled(false).await.expect("JS off");
    page.visit("/app").await.expect("visit");
    must(
        &page,
        "document.querySelector('.autumn-command-palette-link').checkVisibility() && \
         !document.querySelector('[data-command-palette-open]').checkVisibility()",
        "the link shows and the button does not",
    )
    .await;
    // `Page::click` needs page scripts, so follow the link and send the
    // form by URL.
    let href = eval_string(
        &page,
        "document.querySelector('.autumn-command-palette-link').getAttribute('href')",
    )
    .await;
    assert_eq!(href, "/_palette");
    page.visit(&href).await.expect("follow the link");
    page.expect_text("Search commands")
        .await
        .expect("the search page");
    must(
        &page,
        "(() => { const f = document.querySelector('form[role=search]'); \
         return !!f && f.method === 'get' && f.getAttribute('action') === '/_palette' && \
         !!f.querySelector('input[name=q]') && !!f.querySelector('button[type=submit]'); })()",
        "the page has a plain GET search form",
    )
    .await;
    page.visit("/_palette?q=articles")
        .await
        .expect("send the form");
    must(
        &page,
        "!!document.querySelector('a[href=\"/posts\"]') && \
         document.querySelector('input[name=q]').value === 'articles'",
        "the page lists the match as a link",
    )
    .await;
}

/// axe needs page scripts, so this test checks the search page with
/// JavaScript on.
#[tokio::test]
#[ignore = "requires Chromium"]
async fn the_search_page_passes_axe() {
    let (_runner, page) = start(CommandPalette::new(registry()), true).await;
    page.visit("/_palette?q=post").await.expect("visit");
    must(&page, "document.readyState === 'complete'", "page loads").await;
    let violations = page.axe_violations(&axe_source()).await.expect("axe");
    assert!(violations.is_empty(), "{violations:#?}");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn enter_right_after_typing_runs_the_new_first_option() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    open_app(&page).await;
    assert!(chord(&page, "k", true, false).await);
    wait_for_options(&page).await;
    // Enter before the 150 ms delay ends: the old first option is Dashboard.
    eval_bool(
        &page,
        "(() => { const i = document.getElementById('command-palette-input'); \
         i.value = 'articles'; i.dispatchEvent(new Event('input', { bubbles: true })); \
         return true; })()",
    )
    .await;
    must(
        &page,
        &format!("{ACTIVE_ID} === ''"),
        "typing clears the active option",
    )
    .await;
    page.press("Enter").await.expect("enter");
    page.expect_text("Posts page")
        .await
        .expect("Enter runs the first option of the new list");
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn no_match_collapses_the_list_and_says_so() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    open_app(&page).await;
    assert!(chord(&page, "k", true, false).await);
    wait_for_options(&page).await;
    page.fill(INPUT, "zebra").await.expect("fill");
    must(
        &page,
        "document.querySelectorAll('#command-palette-list [role=option]').length === 0",
        "no option",
    )
    .await;
    must(
        &page,
        "document.getElementById('command-palette-input').getAttribute('aria-expanded') === 'false'",
        "aria-expanded is false",
    )
    .await;
    must(&page, &format!("{ACTIVE_ID} === ''"), "no active option").await;
    must(
        &page,
        "document.getElementById('command-palette-status').textContent.includes('No commands match.')",
        "the status says so",
    )
    .await;
    page.press("Enter").await.expect("enter");
    must(
        &page,
        &format!("{DIALOG}.open"),
        "Enter with no option does nothing",
    )
    .await;
    page.press("Escape").await.expect("escape");
    must(
        &page,
        &format!("!{DIALOG}.open"),
        "Escape closes the dialog",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn the_pointer_moves_the_active_option() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    open_app(&page).await;
    assert!(chord(&page, "k", true, false).await);
    wait_for_options(&page).await;
    page.hover("#command-palette-option-2")
        .await
        .expect("hover");
    must(
        &page,
        &format!("{ACTIVE_ID} === 'command-palette-option-2'"),
        "hover makes the option active",
    )
    .await;
    must(
        &page,
        "document.getElementById('command-palette-option-0').getAttribute('aria-selected') === 'false'",
        "only one option is selected",
    )
    .await;
    must(
        &page,
        "document.activeElement.id === 'command-palette-input'",
        "focus stays on the combobox",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn ctrl_k_closes_and_focus_goes_back() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    open_app(&page).await;
    page.click("#note").await.expect("focus the text field");
    assert!(
        chord(&page, "k", true, false).await,
        "Ctrl+K works in a text field"
    );
    must(&page, &format!("{DIALOG}.open"), "the dialog opens").await;
    assert!(
        !send_key(&page, "k", true, false, true).await,
        "a repeated key does not toggle"
    );
    must(&page, &format!("{DIALOG}.open"), "the dialog stays open").await;
    assert!(chord(&page, "k", true, false).await);
    must(&page, &format!("!{DIALOG}.open"), "Ctrl+K closes it").await;
    must(
        &page,
        "document.activeElement && document.activeElement.id === 'note'",
        "focus goes back to the text field",
    )
    .await;
}

#[tokio::test]
#[ignore = "requires Chromium"]
async fn without_htmx_the_link_stays() {
    let (_runner, page) = start(CommandPalette::new(registry()), false).await;
    page.visit("/bare").await.expect("visit");
    must(&page, "window.__autumnWidgets === true", "core runs").await;
    must(
        &page,
        "document.querySelector('.autumn-command-palette-panel').hasAttribute('data-command-palette-ready')",
        "pack runs",
    )
    .await;
    assert!(
        eval_bool(
            &page,
            "!document.querySelector('.autumn-command-palette-link').hidden && \
             document.querySelector('[data-command-palette-open]').hidden"
        )
        .await,
        "the link stays and the button stays hidden"
    );
    assert!(
        !chord(&page, "k", true, false).await,
        "the hotkey does nothing"
    );
}
