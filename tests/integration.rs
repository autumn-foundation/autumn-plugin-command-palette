//! The plugin in a `TestApp`: the route, the assets, the stories and the
//! CSRF token.

use autumn_command_palette_plugin::{Command, CommandPalette, CommandRegistry, PALETTE_PATH};
use autumn_web::config::AutumnConfig;
use autumn_web::test::{TestApp, TestClient};

fn palette() -> CommandPalette {
    CommandPalette::new(
        CommandRegistry::new()
            .add(Command::route("home", "Dashboard", "/"))
            .add(Command::route("posts", "Posts", "/posts").keywords(["articles"]))
            .add(Command::action("posts.new", "New post", "/posts/new"))
            .add(
                Command::route("admin", "Admin", "/admin").visible_when(|parts| {
                    parts.headers.get("x-role").is_some_and(|v| v == "admin")
                }),
            ),
    )
}

fn client() -> TestClient {
    TestApp::new().plugin(palette()).build()
}

#[tokio::test]
async fn an_htmx_request_gets_the_listbox_fragment() {
    let response = client()
        .get("/_palette?q=post")
        .header("HX-Request", "true")
        .send()
        .await;
    response
        .assert_ok()
        .assert_header("vary", "HX-Request")
        .assert_header("cache-control", "private, no-store");
    let body = response.text();
    assert!(!body.contains("<html"), "{body}");
    assert!(body.contains(r#"data-command-id="posts""#), "{body}");
    assert!(body.contains(r#"data-command-id="posts.new""#), "{body}");
    assert!(!body.contains(r#"data-command-id="home""#), "{body}");
}

#[tokio::test]
async fn a_plain_request_gets_the_search_page() {
    let response = client().get("/_palette?q=articles").send().await;
    response.assert_ok().assert_header("vary", "HX-Request");
    let body = response.text();
    assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
    assert!(body.contains(r#"href="/posts""#), "{body}");
    assert!(body.contains(r#"value="articles""#), "{body}");
}

#[tokio::test]
async fn a_history_restore_gets_the_page() {
    let body = client()
        .get("/_palette")
        .header("HX-Request", "true")
        .header("HX-History-Restore-Request", "true")
        .send()
        .await
        .text();
    assert!(body.starts_with("<!DOCTYPE html>"), "{body}");
}

#[tokio::test]
async fn recent_ids_show_first_for_an_empty_query() {
    let body = client()
        .get("/_palette?q=&recent=posts.new,nope,home")
        .header("HX-Request", "true")
        .send()
        .await
        .text();
    let recent = body.find(">Recent<").expect("recent group");
    let new_post = body
        .find(r#"data-command-id="posts.new""#)
        .expect("new post");
    let posts = body.find(r#"data-command-id="posts""#).expect("posts");
    assert!(recent < new_post && new_post < posts, "{body}");
    assert!(
        !body.contains("nope"),
        "an unknown id does not show: {body}"
    );
}

#[tokio::test]
async fn visible_when_reads_the_request() {
    let hidden = client()
        .get("/_palette?q=admin")
        .header("HX-Request", "true")
        .send()
        .await
        .text();
    assert!(!hidden.contains(r#"data-command-id="admin""#), "{hidden}");
    let shown = client()
        .get("/_palette?q=admin")
        .header("HX-Request", "true")
        .header("x-role", "admin")
        .send()
        .await
        .text();
    assert!(shown.contains(r#"data-command-id="admin""#), "{shown}");
}

#[tokio::test]
async fn actions_carry_the_csrf_token() {
    let mut config = AutumnConfig::default();
    config.security.csrf.enabled = true;
    let client = TestApp::new().config(config).plugin(palette()).build();
    let body = client
        .get("/_palette?q=new")
        .header("HX-Request", "true")
        .send()
        .await
        .text();
    let field = r#"<input type="hidden" name="_csrf" value=""#;
    let at = body
        .find(field)
        .unwrap_or_else(|| panic!("no CSRF field: {body}"));
    let token = &body[at + field.len()..];
    assert!(!token.starts_with('"'), "the token is not empty: {body}");
    let page = client.get("/_palette?q=new").send().await.text();
    assert!(page.contains(r#"name="_csrf""#), "{page}");
}

#[tokio::test]
async fn the_plugin_serves_the_pack_assets() {
    let client = client();
    for file in ["command-palette.css", "command-palette.js"] {
        client
            .get(&format!("/static/_plugins/command-palette/{file}"))
            .send()
            .await
            .assert_ok();
    }
}

/// `Plugin::build` installs the route and the pack through the seam.
/// `TestApp::plugin` does not carry the story gallery over, so this test
/// reads the `AppBuilder` route list. The browser conformance opens each
/// story in the gallery.
#[test]
fn the_plugin_mounts_the_route_and_the_pack() {
    let builder = autumn_web::app().plugin(palette());
    let routes = builder.plugin_route_infos().expect("routes");
    for path in [
        PALETTE_PATH,
        "/static/_plugins/command-palette/command-palette.css",
        "/static/_plugins/command-palette/command-palette.js",
    ] {
        assert!(
            routes.iter().any(|r| r.path == path),
            "{path} is not mounted: {routes:?}"
        );
    }
    let pack = autumn_command_palette_plugin::pack();
    let slugs: Vec<&str> = pack
        .stories()
        .iter()
        .map(autumn_web::stories::Story::slug)
        .collect();
    assert_eq!(
        slugs,
        [
            "command-palette-dialog",
            "command-palette-results",
            "command-palette-search-page"
        ]
    );
}

#[tokio::test]
async fn the_route_needs_the_plugin() {
    let client = TestApp::new()
        .routes(autumn_web::routes![
            autumn_command_palette_plugin::route::command_palette_search
        ])
        .build();
    client.get(PALETTE_PATH).send().await.assert_status(404);
}

#[test]
fn the_path_constant_is_the_route_path() {
    assert_eq!(PALETTE_PATH, "/_palette");
}
