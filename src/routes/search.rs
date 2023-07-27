use anyhow::Context;
use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::{errors::ServerError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new().route("/search", get(search))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: String,
}

#[derive(Serialize)]
pub struct SearchResp {
    pub score: f32,
    pub path: String,
    pub text: String,
}

pub async fn search(
    params: Query<SearchQuery>,
    State(state): State<AppState>,
) -> Result<Json<Vec<SearchResp>>, ServerError> {
    let instant = Instant::now();
    let query = state
        .embeddings
        .encode(&[params.query.clone()])
        .await
        .context("Failed to create embedding")
        .map_err(|err| ServerError::Embeddings(err))?;
    tracing::info!("Encoded embedding, elapsed {:?}", instant.elapsed());

    let instant = Instant::now();
    let vectors = state
        .tinyvector
        .read()
        .await
        .get_collection("default")
        .context("Failed to get Tinyvector collection")
        .map_err(|err| ServerError::Embeddings(err))?
        .get_similarity(&query[0], 10);
    tracing::info!("Search completed, elapsed {:?}", instant.elapsed());

    let mut result = Vec::with_capacity(vectors.len());
    for n in vectors {
        result.push(SearchResp {
            score: n.score,
            path: n.embedding.id,
            text: n.embedding.blob,
        })
    }

    Ok(Json(result))
}
