use anyhow::{anyhow, Context};
use axum::{
    extract::{Path, State},
    routing::{post, put},
    Json, Router,
};
use chrono::Utc;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};
use tokio::time::Instant;

use crate::{
    encoder,
    errors::ServerError,
    parser,
    types::{Embedding, Source},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new().nest(
        "/sources",
        Router::new()
            .route("/create", put(create_source))
            .route("/:source_id/worker/parse", post(parse_source))
            .route("/:source_id/worker/embeddings", post(create_embeddings)),
    )
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSourceReq {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub allowed_ext: Vec<String>,
    pub allowed_dirs: Vec<String>,
    pub ignored_dirs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSourceResp {
    pub id: String,
}

pub async fn create_source(
    State(state): State<AppState>,
    Json(payload): Json<CreateSourceReq>,
) -> Result<(StatusCode, Json<CreateSourceResp>), ServerError> {
    tracing::info!(
        ?payload,
        "Creating source {}:{}:{}",
        payload.owner,
        payload.repo,
        payload.branch
    );

    let source: Source = payload.into();
    let response = CreateSourceResp {
        id: source.id.clone(),
    };
    // TODO check collection uniquiness
    let _ = state
        .db
        .insert_source(&source)
        .await
        .context("Failed to insert source")
        .map_err(|err| ServerError::DbError(err))?;

    Ok((StatusCode::CREATED, Json(response)))
}

impl From<CreateSourceReq> for Source {
    fn from(value: CreateSourceReq) -> Self {
        let id = format!("github.com:{}:{}:{}", value.owner, value.repo, value.branch);
        Self {
            id,
            owner: value.owner,
            repo: value.repo,
            branch: value.branch,
            allowed_ext: value.allowed_ext.into_iter().collect(),
            allowed_dirs: value.allowed_dirs.into_iter().collect(),
            ignored_dirs: value.ignored_dirs.into_iter().collect(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}

pub async fn parse_source(
    Path(source_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, ServerError> {
    let source = state
        .db
        .select_source(&source_id)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ServerError::NoContent(anyhow!("Source does not exist")),
            _ => ServerError::DbError(anyhow!("Failed to select source: {}", err)),
        })?;

    let tokenizer = tiktoken_rs::cl100k_base()
        .context("Failed to instantiate tokenizer")
        .map_err(|err| ServerError::EncodingError(err))?;
    let parser = parser::GitHubParser::new(&source, &state.github, &tokenizer);

    let documents = parser
        .get_documents()
        .await
        .context("Failed to parse github documents")
        .map_err(|err| ServerError::GitHubAPIError(err))?;

    let _ = state
        .db
        .insert_documents(&documents)
        .await
        .context("Failed to insert documents")
        .map_err(|err| ServerError::DbError(err))?;

    Ok(StatusCode::OK)
}

pub async fn create_embeddings(
    Path(source_id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, ServerError> {
    let documents = state
        .db
        .query_documents_by_source(&source_id)
        .await
        .context("Failed to query documents")
        .map_err(|err| ServerError::DbError(err))?;
    tracing::info!("Got {} documents", documents.len());

    for doc in documents {
        let chunks = encoder::split_to_chunks(&doc.blob)
            .context("Failed to split document to chunks")
            .map_err(|err| ServerError::EncodingError(err))?;
        tracing::info!("Document {} has {} chunks", doc.path, chunks.len());

        let instant = Instant::now();
        let embeddings = state
            .embeddings
            .encode(&chunks)
            .await
            .context("Failed to create embeddings")
            .map_err(|err| ServerError::Embeddings(err))?;
        tracing::info!("Created embeddings, elapsed {:?}", instant.elapsed());

        if chunks.len() != embeddings.len() {
            return Err(ServerError::EncodingError(anyhow!(
                "Chunks and embeddings len does not match: chunks {}, embeddings: {}",
                chunks.len(),
                embeddings.len()
            )));
        }

        let embeddings = chunks
            .into_iter()
            .zip(embeddings)
            .enumerate()
            .map(|(index, (chunk, embedding))| Embedding {
                source_id: doc.source_id.clone(),
                doc_path: doc.path.clone(),
                chunk: index,
                blob: chunk,
                vector: embedding,
            })
            .collect::<Vec<Embedding>>();

        let instant = Instant::now();
        let _ = state
            .db
            .insert_embeddings(&embeddings)
            .await
            .context("Failed to inserts embeddings")
            .map_err(|err| ServerError::DbError(err))?;
        tracing::info!("Saved embeddings, elapsed {:?}", instant.elapsed());
    }

    Ok(StatusCode::OK)
}
