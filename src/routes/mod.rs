use axum::{routing::get, Router};

mod health_check;
mod search;
mod sources;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health_check", get(health_check::health_check_handler))
        .merge(search::routes())
        .merge(sources::routes())
}
