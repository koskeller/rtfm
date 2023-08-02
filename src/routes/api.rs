use anyhow::{anyhow, Context};
use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post, put},
    Json, Router,
};
use chrono::Utc;
use futures::stream::StreamExt;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{
    encoder,
    errors::ServerError,
    parser,
    types::{Chunk, Document, Source},
    AppState,
};

pub fn routes() -> Router<AppState> {
    Router::new().nest(
        "/api",
        Router::new()
            .route("/search", get(search))
            .route("/sources", put(create_source))
            .route("/sources/:source_id/parse", post(parse))
            .route("/sources/:source_id/encode", post(encode_source))
            .route("/sources/:source_id/chunks", delete(delete_chunks))
            .route("/sources/:source_id/docs", delete(delete_documents)),
    )
}

pub async fn parse(
    Path(source_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<StatusCode, ServerError> {
    tracing::info!("Got request to parse source #{}", source_id);
    let source = state
        .db
        .select_source(source_id)
        .await
        .map_err(|err| match err {
            sqlx::Error::RowNotFound => ServerError::NoContent(anyhow!("Source does not exist")),
            _ => ServerError::DbError(anyhow!("Failed to select source: {}", err)),
        })?;
    let collection_id = source.collection_id;

    tracing::info!(
        "Parsing source #{} from collection #{}",
        source_id,
        collection_id
    );

    let parser = parser::GitHubParser::new(source, state.github);
    let paths = parser
        .get_paths()
        .await
        .context("Failed to get repo paths")
        .map_err(|err| ServerError::GitHubAPIError(err))?;

    let _ = futures::stream::iter(paths)
        .map(|path| {
            let parser = &parser;
            let db = &state.db;
            async move {
                tracing::info!("Gettings path '{}'", &path);
                let data = parser
                    .get_content(&path)
                    .await
                    .context("Failed to get github path content")
                    .unwrap();

                let document = Document {
                    id: 0,
                    source_id,
                    collection_id,
                    path,
                    checksum: crc32fast::hash(data.as_bytes()),
                    tokens_len: 0, // TODO
                    data,
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                };

                let _ = db
                    .insert_document(&document)
                    .await
                    .context("Failed to insert document")
                    .unwrap();
            }
        })
        .buffer_unordered(20)
        .collect::<Vec<_>>()
        .await;

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
            let head = encoder::extract_head(&doc.data).unwrap_or_default();
            let head = encoder::extract_head_values(&head);
            let context = format!("{} {}", head.title, head.desc);

            let data = encoder::remove_head(doc.data);

            let chunks = encoder::split_by_headings(&data)
                .context("Failed to split document to chunks")
                .unwrap();
            if chunks.len() == 0 {
                continue;
            }

            for (chunk_index, data) in chunks.into_iter().enumerate() {
                let payload = format!("{}\n{}", &context, &data);
                let sequences = vec![payload.clone()];
                let vector = state
                    .embeddings
                    .encode(&sequences)
                    .await
                    .context("Failed to create embeddings")
                    .unwrap()
                    .first()
                    .unwrap()
                    .to_vec();

                let chunk = Chunk {
                    id: 0,
                    document_id: doc.id,
                    source_id,
                    collection_id: doc.collection_id,
                    chunk_index,
                    context: context.clone(),
                    data,
                    vector,
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
    tracing::info!("Searching '{}'", params.query);
    let query = state
        .embeddings
        .encode(&[params.query.clone()])
        .await
        .context("Failed to create embedding")
        .map_err(|err| ServerError::Embeddings(err))?;

    let vectors = state
        .tinyvector
        .read()
        .await
        .get_collection("default")
        .context("Failed to get Tinyvector collection")
        .map_err(|err| ServerError::Embeddings(err))?
        .get_similarity(&query[0], 10);

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
