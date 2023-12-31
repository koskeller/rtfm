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
mod encoder;
mod errors;
mod openai;
pub use openai::*;
mod embeddings;
pub use embeddings::*;
mod parser;
mod routes;
mod tinyvector;
pub use tinyvector::*;
mod types;

#[derive(Clone)]
pub struct AppState {
    pub db: Db,
    pub github: Octocrab,
    pub embeddings: Embeddings,
    pub tinyvector: Tinyvector,
    pub cfg: Arc<Configuration>,
}

pub fn run(
    cfg: Config,
    db: Db,
    github: Octocrab,
    embeddings: Embeddings,
    tinyvector: Tinyvector,
) -> Server<AddrIncoming, IntoMakeService<Router>> {
    let addr = cfg.listen_address.clone();

    let app_state = AppState {
        db,
        github,
        embeddings,
        tinyvector,
        cfg,
    };

    // Adds high level tracing.
    let trace_layer = telemetry::trace_layer();

    // Mark headers as sensitive.
    let (req_headers_layer, resp_headers_layer) = telemetry::sensitive_headers_layers();

    // Sets and propagates request ids.
    let request_id_layer = middleware::request_id_layer();
    let propagate_request_id_layer = middleware::propagate_request_id_layer();

    // Adds headers for CORS.
    let cors_layer = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(AllowHeaders::mirror_request())
        .max_age(Duration::from_secs(600));

    // Applies a timeout to requests.
    // If the request does not complete within the specified timeout
    // it will be aborted and a 408 Request Timeout response will be sent.
    let timeout_layer = TimeoutLayer::new(Duration::from_secs(15));

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
