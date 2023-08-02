use axum::{routing::get, Router};

mod api;
mod dashboard;
mod health_check;

use crate::AppState;

pub fn router() -> Router<AppState> {
    Router::new()
        .route("/health_check", get(health_check::health_check_handler))
        .merge(api::routes())
        .merge(dashboard::routes())
}
