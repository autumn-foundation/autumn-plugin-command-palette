//! The palette: its settings, its markup, the pack and the plugin.

use std::borrow::Cow;
use std::fmt;
use std::sync::Arc;

use autumn_web::app::AppBuilder;
use autumn_web::assets::PluginAssets;
use autumn_web::plugin::Plugin;
use autumn_web::plugin_contract::PluginContract;
use autumn_web::reexports::http::request::Parts;
use autumn_web::stories::Story;
use autumn_web::widget_pack::WidgetPack;
use maud::{Markup, Render};

use crate::command::{Command, CommandRegistry, Visibility};
use crate::hotkey::Hotkey;
use crate::search::{self, Limits, SearchResults};

/// The pack name. Classes start with `autumn-command-palette-`.
pub const PACK_NAME: &str = "command-palette";

/// The plugin name.
pub const PLUGIN_NAME: &str = "autumn-command-palette-plugin";

/// The search route. An htmx request gets the listbox. Other requests get
/// the search page.
pub const PALETTE_PATH: &str = "/_palette";

/// The pack CSS and JS.
pub static ASSETS: PluginAssets = PluginAssets::from_files(
    PACK_NAME,
    &[
        ("command-palette.css", include_bytes!("command-palette.css")),
        ("command-palette.js", include_bytes!("command-palette.js")),
    ],
);

/// The texts of the palette. Change a field to translate it.
///
/// `count` holds `{count}`. The palette puts the number of results there.
/// `count_one` is the text for one result.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Labels {
    /// The dialog title. It is visually hidden.
    pub title: String,
    /// The text of the invoker button and link.
    pub invoker: String,
    /// The label of the search input.
    pub input: String,
    /// The input placeholder.
    pub placeholder: String,
    /// The label of the listbox.
    pub results: String,
    /// The label of the recent items group.
    pub recent: String,
    /// The text when no command matches.
    pub empty: String,
    /// The status text for two or more results.
    pub count: String,
    /// The status text for one result.
    pub count_one: String,
    /// The status text when the search request fails.
    pub error: String,
    /// The label of the close button.
    pub close: String,
    /// The title of the search page.
    pub page_title: String,
    /// The submit button of the search page.
    pub submit: String,
    /// The footer text after ↑ ↓.
    pub hint_move: String,
    /// The footer text after Enter.
    pub hint_select: String,
    /// The footer text after Esc.
    pub hint_close: String,
}

impl Default for Labels {
    fn default() -> Self {
        Self {
            title: "Command palette".to_owned(),
            invoker: "Search".to_owned(),
            input: "Search commands".to_owned(),
            placeholder: "Type a command or search…".to_owned(),
            results: "Commands".to_owned(),
            recent: "Recent".to_owned(),
            empty: "No commands match.".to_owned(),
            count: "{count} results".to_owned(),
            count_one: "1 result".to_owned(),
            error: "The search failed. Try again.".to_owned(),
            close: "Close".to_owned(),
            page_title: "Search commands".to_owned(),
            submit: "Search".to_owned(),
            hint_move: "to move".to_owned(),
            hint_select: "to select".to_owned(),
            hint_close: "to close".to_owned(),
        }
    }
}

/// The CSRF token and its form field, for action commands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Csrf {
    /// The form field name, for example `_csrf`.
    pub field: String,
    /// The token.
    pub token: String,
}

/// The layout of the search page: `(title, main content) -> page`.
pub type PageLayout = Arc<dyn Fn(&str, Markup) -> Markup + Send + Sync>;

#[derive(Clone)]
pub struct Inner {
    pub(crate) id: String,
    pub(crate) registry: CommandRegistry,
    pub(crate) hotkey: Hotkey,
    pub(crate) labels: Labels,
    pub(crate) limits: Limits,
    pub(crate) layout: Option<PageLayout>,
    pub(crate) gate: Option<Visibility>,
}

/// A command palette. It is the plugin and the markup.
///
/// Install a clone with `app.plugin(palette.clone())`. Render the other
/// clone in the layout: `(palette)` writes the invoker and the dialog. One
/// palette for each app.
#[derive(Clone)]
pub struct CommandPalette {
    pub(crate) inner: Arc<Inner>,
}

impl CommandPalette {
    /// A palette with the id `command-palette`, the hotkey `mod+k` and the
    /// default [`Labels`] and [`Limits`].
    #[must_use]
    pub fn new(registry: CommandRegistry) -> Self {
        Self {
            inner: Arc::new(Inner {
                id: "command-palette".to_owned(),
                registry,
                hotkey: Hotkey::default(),
                labels: Labels::default(),
                limits: Limits::default(),
                layout: None,
                gate: None,
            }),
        }
    }

    fn with(mut self, f: impl FnOnce(&mut Inner)) -> Self {
        f(Arc::make_mut(&mut self.inner));
        self
    }

    /// Set the element id. Other ids start with it.
    ///
    /// # Panics
    ///
    /// Panics when `id` does not start with an ASCII letter, or has a
    /// character that is not an ASCII letter, a digit, `-` or `_`.
    #[must_use]
    #[track_caller]
    pub fn id(self, id: impl Into<String>) -> Self {
        let id = id.into();
        assert!(
            valid_dom_id(&id),
            "autumn-command-palette: `{id}` is not a valid palette id"
        );
        self.with(|inner| inner.id = id)
    }

    /// Set the hotkey.
    #[must_use]
    pub fn hotkey(self, hotkey: Hotkey) -> Self {
        self.with(|inner| inner.hotkey = hotkey)
    }

    /// Set the texts.
    #[must_use]
    pub fn labels(self, labels: Labels) -> Self {
        self.with(|inner| inner.labels = labels)
    }

    /// Set the result limits.
    #[must_use]
    pub fn limits(self, limits: Limits) -> Self {
        self.with(|inner| inner.limits = limits)
    }

    /// Show the commands only when `visible` returns `true` for the
    /// request. This filter is for all commands, before each
    /// [`Command::visible_when`]. `GET /_palette` needs no login, so
    /// without a filter each command is public.
    #[must_use]
    pub fn visible_when<F>(self, visible: F) -> Self
    where
        F: Fn(&Parts) -> bool + Send + Sync + 'static,
    {
        self.with(|inner| inner.gate = Some(Arc::new(visible)))
    }

    /// Put the search page in the app layout.
    #[must_use]
    pub fn page_layout<F>(self, layout: F) -> Self
    where
        F: Fn(&str, Markup) -> Markup + Send + Sync + 'static,
    {
        self.with(|inner| inner.layout = Some(Arc::new(layout)))
    }

    /// The element id.
    #[must_use]
    pub fn palette_id(&self) -> &str {
        &self.inner.id
    }

    /// The commands.
    #[must_use]
    pub fn registry(&self) -> &CommandRegistry {
        &self.inner.registry
    }

    /// The hotkey.
    #[must_use]
    pub fn current_hotkey(&self) -> &Hotkey {
        &self.inner.hotkey
    }

    /// The texts.
    #[must_use]
    pub fn current_labels(&self) -> &Labels {
        &self.inner.labels
    }

    /// Find the commands for `query`. `recent` holds the recent ids, newest
    /// first. With `parts`, the palette filter and each
    /// [`Command::visible_when`] filter run. Without `parts`, a command with
    /// a filter does not show, and with a palette filter no command shows.
    #[must_use]
    pub fn search(&self, query: &str, recent: &[&str], parts: Option<&Parts>) -> SearchResults<'_> {
        let gate = self.inner.gate.as_ref();
        let visible = |command: &Command| {
            parts.map_or_else(
                || gate.is_none() && !command.has_filter(),
                |parts| gate.is_none_or(|gate| gate(parts)) && command.is_visible(parts),
            )
        };
        search::search(
            &self.inner.registry,
            query,
            recent,
            &visible,
            self.inner.limits,
            &self.inner.labels.recent,
        )
    }

    /// The invoker: a button that opens the dialog, and a link to the
    /// search page. The link shows with JavaScript off.
    #[must_use]
    pub fn invoker(&self) -> Markup {
        crate::render::invoker(&self.inner)
    }

    /// The `<dialog>` with the combobox and the listbox.
    #[must_use]
    pub fn dialog(&self) -> Markup {
        crate::render::dialog(&self.inner)
    }

    /// The listbox content for `results`: the groups, the options and the
    /// status. htmx puts it in the listbox.
    #[must_use]
    pub fn fragment(&self, results: &SearchResults<'_>, csrf: Option<&Csrf>) -> Markup {
        crate::render::fragment(&self.inner, results, csrf)
    }

    /// The main content of the search page.
    #[must_use]
    pub fn page_body(&self, results: &SearchResults<'_>, csrf: Option<&Csrf>) -> Markup {
        crate::render::page_body(&self.inner, results, csrf)
    }

    /// The search page: [`page_body`](Self::page_body) in the
    /// [`page_layout`](Self::page_layout), or in a plain page.
    #[must_use]
    pub fn page(&self, results: &SearchResults<'_>, csrf: Option<&Csrf>) -> Markup {
        crate::render::page(&self.inner, results, csrf)
    }

    /// The panel with the results of `query`, not in a dialog. The stories
    /// use it.
    #[must_use]
    pub fn preview(&self, query: &str) -> Markup {
        crate::render::preview(&self.inner, &self.search(query, &[], None))
    }
}

impl fmt::Debug for CommandPalette {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CommandPalette")
            .field("id", &self.inner.id)
            .field("registry", &self.inner.registry)
            .field("hotkey", &self.inner.hotkey)
            .field("labels", &self.inner.labels)
            .field("limits", &self.inner.limits)
            .field("page_layout", &self.inner.layout.is_some())
            .field("visible_when", &self.inner.gate.is_some())
            .finish()
    }
}

impl Render for CommandPalette {
    fn render(&self) -> Markup {
        maud::html! {
            (self.invoker())
            (self.dialog())
        }
    }
}

impl Plugin for CommandPalette {
    fn name(&self) -> Cow<'static, str> {
        Cow::Borrowed(PLUGIN_NAME)
    }

    fn contract(&self) -> Option<PluginContract> {
        Some(
            PluginContract::new(PLUGIN_NAME)
                .plugin_version(env!("CARGO_PKG_VERSION"))
                .autumn_web("0.10"),
        )
    }

    fn build(self, app: AppBuilder) -> AppBuilder {
        app.widget_pack(pack())
            .routes(autumn_web::routes![crate::route::command_palette_search])
            .state_initializer(move |state| state.insert_extension(self))
    }
}

/// `true` for an HTML id that is safe in a CSS selector.
fn valid_dom_id(id: &str) -> bool {
    id.as_bytes().first().is_some_and(u8::is_ascii_alphabetic)
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// The demo palette of the stories.
#[must_use]
pub fn demo_palette(id: &str) -> CommandPalette {
    CommandPalette::new(
        CommandRegistry::new()
            .add(Command::route("dashboard", "Dashboard", "#dashboard").shortcut(&["G", "D"]))
            .add(Command::route("posts", "Posts", "#posts").keywords(["articles", "blog"]))
            .add(
                Command::route("settings", "Settings", "#settings")
                    .group("Account")
                    .hint("Profile and password"),
            )
            .add(
                Command::action("posts.new", "New post", "#new-post")
                    .shortcut(&["N"])
                    .hint("Write a draft"),
            )
            .add(Command::action("sign-out", "Sign out", "#sign-out").group("Account")),
    )
    .id(id)
}

fn invoker_story() -> Markup {
    demo_palette("command-palette-demo").render()
}

fn results_story() -> Markup {
    demo_palette("command-palette-preview").preview("")
}

fn page_story() -> Markup {
    let palette = demo_palette("command-palette-page");
    palette.page_body(&palette.search("post", &[], None), None)
}

/// The pack: three stories and the assets.
#[must_use]
pub fn pack() -> WidgetPack {
    WidgetPack::new(PACK_NAME)
        .extend([
            Story::new(
                "Command palette",
                "Command palette dialog",
                invoker_story,
                "(CommandPalette::new(registry))",
            ),
            Story::new(
                "Command palette",
                "Command palette results",
                results_story,
                "palette.preview(\"\")",
            ),
            Story::new(
                "Command palette",
                "Command palette search page",
                page_story,
                "palette.page_body(&palette.search(\"post\", &[], None), None)",
            ),
        ])
        .assets(&ASSETS)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::command::Command;
    use autumn_web::plugin_conformance::widgets::run_widget_conformance;

    fn palette() -> CommandPalette {
        CommandPalette::new(
            CommandRegistry::new()
                .add(Command::route("home", "Dashboard", "/").shortcut(&["G", "D"]))
                .add(Command::route("posts", "Posts", "/posts"))
                .add(Command::action("posts.new", "New post", "/posts/new").hint("Write a draft")),
        )
    }

    fn csrf() -> Csrf {
        Csrf {
            field: "_csrf".into(),
            token: "tok123".into(),
        }
    }

    #[test]
    fn invoker_is_a_hidden_button_and_a_link() {
        let html = palette().invoker().into_string();
        assert!(
            html.contains(r#"<a class="autumn-command-palette-link" href="/_palette""#),
            "{html}"
        );
        assert!(
            html.contains(r#"data-command-palette-open="command-palette""#),
            "{html}"
        );
        assert!(html.contains(r#"aria-haspopup="dialog""#), "{html}");
        assert!(
            html.contains(r#"aria-keyshortcuts="Control+K Meta+K""#),
            "{html}"
        );
        assert!(
            html.contains(" hidden"),
            "the button is hidden until the JS runs: {html}"
        );
        assert!(
            html.contains(r#"<kbd class="autumn-kbd" data-command-palette-mod>Ctrl</kbd>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<kbd class="autumn-kbd">K</kbd>"#),
            "{html}"
        );
    }

    #[test]
    fn dialog_is_a_modal_with_an_apg_combobox() {
        let html = palette().dialog().into_string();
        for needle in [
            r#"<dialog id="command-palette" class="autumn-modal autumn-command-palette-dialog""#,
            r#"closedby="any""#,
            r#"aria-modal="true""#,
            r#"data-command-palette-hotkey="mod+k""#,
            r#"role="combobox""#,
            r#"aria-expanded="false""#,
            r#"aria-autocomplete="list""#,
            r#"aria-controls="command-palette-list""#,
            r#"autocomplete="off""#,
            r#"hx-get="/_palette""#,
            r##"hx-target="#command-palette-list""##,
            r##"hx-include="#command-palette-recent""##,
            r#"<input type="hidden" id="command-palette-recent" name="recent""#,
            r#"id="command-palette-list" class="autumn-command-palette-list" role="listbox""#,
            "data-autumn-activedescendant",
            r#"data-autumn-orientation="vertical""#,
            r#"id="command-palette-status""#,
            r#"role="status""#,
            "<kbd class=\"autumn-kbd\">Esc</kbd>",
        ] {
            assert!(html.contains(needle), "missing {needle}: {html}");
        }
        assert!(
            !html.contains("data-autumn-dismiss"),
            "the dialog closes on Escape: {html}"
        );
    }

    #[test]
    fn id_and_hotkey_change_the_markup() {
        let html = palette()
            .id("cmdk")
            .hotkey(Hotkey::parse("ctrl+shift+p").expect("valid"))
            .dialog()
            .into_string();
        assert!(html.contains(r#"<dialog id="cmdk""#), "{html}");
        assert!(html.contains(r#"aria-controls="cmdk-list""#), "{html}");
        assert!(
            html.contains(r#"data-command-palette-hotkey="ctrl+shift+p""#),
            "{html}"
        );
    }

    #[test]
    #[should_panic(expected = "is not a valid palette id")]
    fn a_bad_id_panics() {
        let _ = palette().id("1 bad");
    }

    #[test]
    fn fragment_has_groups_options_and_an_active_first_option() {
        let palette = palette();
        let results = palette.search("", &[], None);
        let html = palette.fragment(&results, Some(&csrf())).into_string();
        assert!(
            html.contains(r#"role="group" aria-labelledby="command-palette-group-0""#),
            "{html}"
        );
        assert!(html.contains(r#"<div id="command-palette-group-0" class="autumn-command-palette-group-label">Go to</div>"#), "{html}");
        assert!(html.contains(r#"<a id="command-palette-option-0" class="autumn-command-palette-option" role="option" tabindex="-1" aria-selected="true" data-command-id="home" href="/""#), "{html}");
        assert!(
            html.contains(r#"id="command-palette-option-1""#)
                && html.contains(r#"aria-selected="false""#),
            "{html}"
        );
        assert!(html.contains(r#"<kbd class="autumn-kbd-combo"><kbd class="autumn-kbd">G</kbd>+<kbd class="autumn-kbd">D</kbd></kbd>"#), "{html}");
        assert!(html.contains(r#"<button id="command-palette-option-2" class="autumn-command-palette-option" role="option" tabindex="-1" aria-selected="false" data-command-id="posts.new" type="submit" form="command-palette-action" formaction="/posts/new" formmethod="post" aria-describedby="command-palette-option-2-desc">"#), "{html}");
        assert!(html.contains(r#"<form id="command-palette-action" method="post" hidden><input type="hidden" name="_csrf" value="tok123"></form>"#), "{html}");
        assert!(
            html.contains(r#"<span class="autumn-command-palette-hint">Write a draft</span>"#),
            "{html}"
        );
        assert!(
            html.contains(r#"<p id="command-palette-status" hx-swap-oob="innerHTML">"#),
            "{html}"
        );
        assert!(html.contains("3 results"), "{html}");
    }

    #[test]
    fn the_token_goes_only_to_this_origin() {
        let palette = CommandPalette::new(
            CommandRegistry::new()
                .add(Command::action("local", "Local", "/x"))
                .add(Command::action("far", "Far", "https://other.example/x"))
                .add(Command::action("sneaky", "Sneaky", "//other.example/x"))
                .add(Command::action("slash", "Slash", "/\\other.example/x")),
        );
        let results = palette.search("", &[], None);
        let html = palette.fragment(&results, Some(&csrf())).into_string();
        assert_eq!(html.matches("tok123").count(), 1, "{html}");
        assert!(
            html.contains(r#"data-command-id="local" type="submit" form="command-palette-action""#),
            "{html}"
        );
        for id in ["far", "sneaky", "slash"] {
            assert!(
                html.contains(&format!(
                    r#"data-command-id="{id}" type="submit" form="command-palette-action-external""#
                )),
                "{id}: {html}"
            );
        }
        assert!(
            html.contains(
                r#"<form id="command-palette-action-external" method="post" hidden></form>"#
            ),
            "{html}"
        );
        let page = palette.page_body(&results, Some(&csrf())).into_string();
        assert_eq!(page.matches("tok123").count(), 1, "{page}");
    }

    #[test]
    fn one_result_uses_the_singular_text() {
        let palette = palette();
        let html = palette
            .fragment(&palette.search("new", &[], None), None)
            .into_string();
        assert!(html.contains(">1 result<"), "{html}");
    }

    #[test]
    fn the_dialog_has_a_close_button_and_an_error_text() {
        let html = palette().dialog().into_string();
        assert!(html.contains(r#"aria-label="Close""#), "{html}");
        assert!(
            html.contains(r#"data-command-palette-error="The search failed. Try again.""#),
            "{html}"
        );
        assert!(
            html.contains(r#"hx-trigger="input delay:150ms, command-palette-open""#),
            "{html}"
        );
    }

    #[test]
    fn fragment_without_actions_or_csrf_has_no_token_field() {
        let palette = palette();
        let html = palette
            .fragment(&palette.search("posts", &[], None), None)
            .into_string();
        assert!(
            !html.contains("command-palette-action\" method"),
            "no action, no form: {html}"
        );
        let html = palette
            .fragment(&palette.search("new", &[], None), None)
            .into_string();
        assert!(
            html.contains(r#"<form id="command-palette-action" method="post" hidden></form>"#),
            "{html}"
        );
    }

    #[test]
    fn empty_results_show_the_empty_text() {
        let palette = palette();
        let html = palette
            .fragment(&palette.search("zebra", &[], None), None)
            .into_string();
        assert!(!html.contains(r#"role="option""#), "{html}");
        assert!(html.contains("No commands match."), "{html}");
    }

    #[test]
    fn titles_are_escaped() {
        let palette = CommandPalette::new(CommandRegistry::new().add(Command::route(
            "x",
            "<script>alert(1)</script>",
            "/x",
        )));
        let results = palette.search("", &[], None);
        let html = palette.fragment(&results, None).into_string();
        assert!(!html.contains("<script>"), "{html}");
        assert!(html.contains("&lt;script&gt;"), "{html}");
        let html = palette.page_body(&results, None).into_string();
        assert!(!html.contains("<script>"), "{html}");
    }

    #[test]
    fn a_filtered_command_needs_a_request() {
        let palette = CommandPalette::new(
            CommandRegistry::new().add(Command::route("x", "Secret", "/x").visible_when(|_| true)),
        );
        assert!(palette.search("", &[], None).is_empty());
    }

    #[test]
    fn page_body_is_a_plain_form_and_lists() {
        let palette = palette();
        let results = palette.search("post", &[], None);
        let html = palette.page_body(&results, Some(&csrf())).into_string();
        for needle in [
            r#"<h1 class="autumn-command-palette-page-title">Search commands</h1>"#,
            r#"<form class="autumn-command-palette-page-search" role="search" method="get" action="/_palette">"#,
            r#"<input id="command-palette-page-q" class="autumn-command-palette-page-input" type="search" name="q" value="post">"#,
            r#"<button class="autumn-command-palette-page-submit" type="submit">Search</button>"#,
            r#"<a class="autumn-command-palette-page-link" href="/posts">Posts</a>"#,
            r#"<form class="autumn-command-palette-page-action" method="post" action="/posts/new"><input type="hidden" name="_csrf" value="tok123">"#,
        ] {
            assert!(html.contains(needle), "missing {needle}: {html}");
        }
        let html = palette
            .page_body(&palette.search("zebra", &[], None), None)
            .into_string();
        assert!(html.contains("No commands match."), "{html}");
    }

    #[test]
    fn page_is_a_document_or_uses_the_layout() {
        let palette = palette();
        let results = palette.search("", &[], None);
        let html = palette.page(&results, None).into_string();
        assert!(html.starts_with("<!DOCTYPE html>"), "{html}");
        assert!(html.contains("<title>Search commands</title>"), "{html}");
        assert!(html.contains(autumn_web::ui::WIDGETS_CSS_PATH), "{html}");
        assert!(
            html.contains(
                r#"<link rel="stylesheet" href="/static/_plugins/command-palette/command-palette."#
            ),
            "{html}"
        );
        let html = palette
            .clone()
            .page_layout(|title, body| maud::html! { section data-title=(title) { (body) } })
            .page(&results, None)
            .into_string();
        assert!(
            html.starts_with(r#"<section data-title="Search commands">"#),
            "{html}"
        );
    }

    #[test]
    #[allow(
        clippy::field_reassign_with_default,
        reason = "`Labels` is non_exhaustive: an app sets fields this way"
    )]
    fn labels_translate_the_texts() {
        let mut labels = Labels::default();
        labels.invoker = "Suchen".into();
        labels.count = "{count} Treffer".into();
        let palette = palette().labels(labels);
        assert!(palette.invoker().into_string().contains("Suchen"));
        let html = palette
            .fragment(&palette.search("", &[], None), None)
            .into_string();
        assert!(html.contains("3 Treffer"), "{html}");
        assert_eq!(palette.current_labels().invoker, "Suchen");
    }

    #[test]
    fn limits_and_getters() {
        let palette = palette().limits(Limits {
            results: 1,
            recent: 0,
        });
        assert_eq!(palette.search("", &["home"], None).len(), 1);
        assert_eq!(palette.palette_id(), "command-palette");
        assert_eq!(palette.registry().len(), 3);
        assert_eq!(palette.current_hotkey(), &Hotkey::default());
        assert!(format!("{palette:?}").contains("page_layout: false"));
    }

    #[test]
    fn render_writes_invoker_and_dialog() {
        let html = palette().render().into_string();
        assert!(
            html.contains("autumn-command-palette-invoker") && html.contains("<dialog"),
            "{html}"
        );
    }

    #[test]
    fn preview_is_an_open_panel() {
        let html = palette().preview("").into_string();
        assert!(!html.contains("<dialog"), "{html}");
        assert!(
            html.contains(r#"role="combobox""#) && html.contains(r#"aria-expanded="true""#),
            "{html}"
        );
        assert!(
            html.contains(r#"aria-activedescendant="command-palette-option-0""#),
            "{html}"
        );
    }

    #[test]
    fn the_pack_passes_static_conformance() {
        let report = run_widget_conformance(&pack());
        assert!(report.passed(), "{}", report.to_text_report());
    }

    #[test]
    fn the_pack_has_three_stories() {
        let pack = pack();
        let slugs: Vec<String> = pack
            .stories()
            .iter()
            .map(|story| story.slug().to_string())
            .collect();
        assert_eq!(
            slugs,
            [
                "command-palette-dialog",
                "command-palette-results",
                "command-palette-search-page"
            ]
        );
        assert_eq!(pack.name(), PACK_NAME);
    }

    #[test]
    fn demo_palette_has_routes_and_actions() {
        let palette = demo_palette("demo");
        assert_eq!(palette.palette_id(), "demo");
        assert!(
            palette
                .registry()
                .commands()
                .iter()
                .any(|c| matches!(c.target(), crate::CommandTarget::Action(_)))
        );
        assert!(
            palette
                .registry()
                .commands()
                .iter()
                .any(|c| matches!(c.target(), crate::CommandTarget::Route(_)))
        );
    }

    #[test]
    fn valid_dom_ids() {
        for good in ["a", "cmd-k", "Palette_2"] {
            assert!(valid_dom_id(good), "{good}");
        }
        for bad in ["", "1a", "-a", "a b", "a.b", "é"] {
            assert!(!valid_dom_id(bad), "{bad}");
        }
    }

    #[test]
    fn the_plugin_has_a_contract() {
        let palette = palette();
        assert_eq!(palette.name(), PLUGIN_NAME);
        assert!(palette.contract().is_some());
    }
}
