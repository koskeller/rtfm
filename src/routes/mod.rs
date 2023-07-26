use axum::{
    routing::{get, put},
    Router,
};

mod health_check;
pub(crate) use health_check::health_check_handler;

mod sources;
pub(crate) use sources::create_source;
pub(crate) use sources::fetch_source_content;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health_check", get(health_check_handler))
        .route("/sources/create", put(create_source))
        .route("/sources/:id/fetch_content", get(fetch_source_content))
}
