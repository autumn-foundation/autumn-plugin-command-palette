//! The search route, [`PALETTE_PATH`](crate::PALETTE_PATH).

use autumn_web::AppState;
use autumn_web::prelude::*;
use autumn_web::reexports::axum::extract::State;
use autumn_web::reexports::axum::response::{IntoResponse, Response};
use autumn_web::reexports::http::StatusCode;
use autumn_web::reexports::http::header::{CACHE_CONTROL, HeaderValue, VARY};
use autumn_web::reexports::http::request::Parts;
use autumn_web::security::{CsrfFormField, CsrfToken};

use crate::palette::{CommandPalette, Csrf};
use crate::search::parse_recent;

/// The query of the search route.
#[derive(Debug, Default, serde::Deserialize)]
pub struct SearchParams {
    /// The query.
    #[serde(default)]
    pub q: String,
    /// The recent ids, newest first, comma-separated.
    #[serde(default)]
    pub recent: String,
}

/// `GET /_palette`. An htmx request gets the listbox content. Other
/// requests get the search page. The response is `private, no-store` and
/// varies on `HX-Request`.
#[get("/_palette")]
pub async fn command_palette_search(
    State(state): State<AppState>,
    parts: Parts,
    Query(params): Query<SearchParams>,
) -> Response {
    let Some(palette) = state.extension::<CommandPalette>() else {
        return (
            StatusCode::NOT_FOUND,
            "the command palette plugin is not installed",
        )
            .into_response();
    };
    let recent = parse_recent(&params.recent);
    let results = palette.search(&params.q, &recent, Some(&parts));
    let csrf = csrf_of(&parts);
    let body = if wants_fragment(&parts) {
        palette.fragment(&results, csrf.as_ref())
    } else {
        palette.page(&results, csrf.as_ref())
    };
    let mut response = body.into_response();
    let headers = response.headers_mut();
    headers.insert(VARY, HeaderValue::from_static("HX-Request"));
    headers.insert(CACHE_CONTROL, HeaderValue::from_static("private, no-store"));
    response
}

/// `true` for an htmx request that is not a history restore. A history
/// restore needs the full page.
fn wants_fragment(parts: &Parts) -> bool {
    let is = |name: &str| {
        parts
            .headers
            .get(name)
            .is_some_and(|value| value.as_bytes().eq_ignore_ascii_case(b"true"))
    };
    is("hx-request") && !is("hx-history-restore-request")
}

/// The CSRF token of the request, when the CSRF layer gave one.
fn csrf_of(parts: &Parts) -> Option<Csrf> {
    let token = parts.extensions.get::<CsrfToken>()?.token();
    if token.is_empty() {
        return None;
    }
    let field = parts
        .extensions
        .get::<CsrfFormField>()
        .map_or("_csrf", |field| field.0.as_str());
    Some(Csrf {
        field: field.to_owned(),
        token: token.to_owned(),
    })
}
