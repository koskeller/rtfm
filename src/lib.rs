use async_openai::{config::OpenAIConfig, Client};
use axum::{routing::IntoMakeService, Router, Server};
use hyper::server::conn::AddrIncoming;
use octocrab::Octocrab;
use std::{sync::Arc, time::Duration};
use tower_http::{
    cors::{AllowHeaders, Any, CorsLayer},
    timeout::TimeoutLayer,
};

mod cfg;
pub use cfg::*;
mod telemetry;
pub use telemetry::*;
mod middleware;
pub use middleware::*;
mod db;
pub use db::*;
mod errors;
mod parser;
pub use parser::*;
mod routes;
mod types;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub github: Octocrab,
    pub open_ai: Client<OpenAIConfig>,
    pub cfg: Arc<Configuration>,
}

pub fn run(
    cfg: Config,
    db: Db,
    github: Octocrab,
    open_ai: Client<OpenAIConfig>,
) -> Server<AddrIncoming, IntoMakeService<Router>> {
    let addr = cfg.listen_address.clone();

    let app_state = AppState {
        db,
        github,
        cfg,
        open_ai,
    };

    let trace_layer = telemetry::trace_layer();
    let (req_headers_layer, resp_headers_layer) = telemetry::sensitive_headers_layers();

    let request_id_layer = middleware::request_id_layer();
    let propagate_request_id_layer = middleware::propagate_request_id_layer();

    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(AllowHeaders::mirror_request())
        .max_age(Duration::from_secs(600));

    let timeout_layer = TimeoutLayer::new(Duration::from_secs(10));

    let app = Router::new()
        .merge(routes::router())
        .layer(cors_layer)
        .layer(timeout_layer)
        .layer(resp_headers_layer)
        .layer(propagate_request_id_layer)
        .layer(trace_layer)
        .layer(req_headers_layer)
        .layer(request_id_layer)
        .with_state(app_state);

    axum::Server::bind(&addr).serve(app.into_make_service())
}
