use anyhow::{anyhow, Context};
use axum::{
    extract::{Path, State},
    routing::{delete, post, put},
    Json, Router,
};
use chrono::Utc;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    encoder,
    errors::ServerError,
    parser,
    types::{Chunk, Source},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/sources", put(create_source))
        .route("/sources/:source_id/parse", post(parse_source))
        .route("/sources/:source_id/encode", post(encode_source))
        .route("/sources/:source_id/chunks", delete(delete_chunks))
        .route("/sources/:source_id/docs", delete(delete_documents))
}

pub async fn parse_source(
    Path(source_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<StatusCode, ServerError> {
    let source = state
        .db
        .select_source(source_id)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ServerError::NoContent(anyhow!("Source does not exist")),
            _ => ServerError::DbError(anyhow!("Failed to select source: {}", err)),
        })?;

    let _ = tokio::spawn(async move {
        let tokenizer = tiktoken_rs::cl100k_base()
            .context("Failed to instantiate tokenizer")
            .unwrap();
        let parser =
            parser::GitHubParser::new(source.collection_id, &source, &state.github, &tokenizer);

        let documents = parser
            .get_documents()
            .await
            .context("Failed to parse github documents")
            .unwrap();

        let _ = state
            .db
            .insert_documents(&documents)
            .await
            .context("Failed to insert documents")
            .unwrap();
    });

    Ok(StatusCode::OK)
}

pub async fn encode_source(
    Path(source_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<StatusCode, ServerError> {
    let documents = state
        .db
        .query_documents_by_source(source_id)
        .await
        .context("Failed to query documents")
        .map_err(|err| ServerError::DbError(err))?;
    tracing::info!("Got {} documents", documents.len());

    let _ = tokio::spawn(async move {
        for doc in documents {
            let chunks = encoder::split_to_chunks(&doc.data)
                .context("Failed to split document to chunks")
                .unwrap();
            if chunks.len() == 0 {
                continue;
            }

            for (chunk_index, data) in chunks.into_iter().enumerate() {
                let embeddings = state
                    .embeddings
                    .encode(&vec![data.clone()])
                    .await
                    .context("Failed to create embeddings")
                    .unwrap();

                let chunk = Chunk {
                    id: 0,
                    document_id: doc.id,
                    source_id,
                    collection_id: doc.collection_id,
                    chunk_index,
                    context: "".to_string(), // TODO
                    data,
                    vector: embeddings[0].clone(),
                };

                let _ = state
                    .db
                    .insert_chunk(&chunk)
                    .await
                    .context("Failed to inserts chunks")
                    .unwrap();
            }
        }
        tracing::info!("Inserted all documents");
    });

    Ok(StatusCode::OK)
}

#[allow(unused)]
pub async fn delete_chunks(
    Path(source_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<StatusCode, ServerError> {
    let _ = state
        .db
        .delete_chunks_by_source(source_id)
        .await
        .context("Failed to delete chunks")
        .map_err(|err| ServerError::DbError(err))?;
    Ok(StatusCode::OK)
}

#[allow(unused)]
pub async fn delete_documents(
    Path(source_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<StatusCode, ServerError> {
    let _ = state
        .db
        .delete_documents_by_source(source_id)
        .await
        .context("Failed to delete documents")
        .map_err(|err| ServerError::DbError(err))?;
    Ok(StatusCode::OK)
}
#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSourceReq {
    pub collection_id: i64,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub allowed_ext: Vec<String>,
    pub allowed_dirs: Vec<String>,
    pub ignored_dirs: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateSourceResp {
    pub id: i64,
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
    let response = CreateSourceResp { id: source.id };
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
        Self {
            id: 0,
            collection_id: value.collection_id,
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
