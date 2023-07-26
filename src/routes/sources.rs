use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{errors::HTTPError, types::Source, AppState, GitHub};

#[derive(Serialize, Deserialize)]
pub struct CreateSourcePayload {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub allowed_ext: Vec<String>,
    pub allowed_dirs: Vec<String>,
    pub ignored_dirs: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateSourceResponse {
    pub id: String,
}

pub async fn create_source(
    State(state): State<AppState>,
    Json(payload): Json<CreateSourcePayload>,
) -> Result<(StatusCode, Json<CreateSourceResponse>), HTTPError> {
    let source: Source = payload.into();
    let response = CreateSourceResponse {
        id: source.id.clone(),
    };
    let result = state.db.insert_source(&source).await;
    match result {
        Ok(_) => Ok((StatusCode::OK, Json(response))),
        Err(e) => {
            tracing::error!("Failed to execute query: {}", e);
            Err(HTTPError::new("Internal error").with_status(StatusCode::INTERNAL_SERVER_ERROR))
        }
    }
}

impl From<CreateSourcePayload> for Source {
    fn from(value: CreateSourcePayload) -> Self {
        let id = format!("github.com/{}/{}", value.owner, value.repo);
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

pub async fn fetch_source_content(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, HTTPError> {
    let source = &state.db.select_source(&id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => HTTPError::no_content("Unknown id"),
        _ => {
            tracing::error!("Failed to select source: {:?}", err);
            HTTPError::internal_error()
        }
    })?;

    let parser = GitHub::new(&state.github, &source.owner, &source.repo, &source.branch)
        .allowed_ext(source.allowed_ext.clone())
        .allowed_dirs(source.allowed_dirs.clone())
        .ignored_dirs(source.ignored_dirs.clone())
        .build();

    let documents = parser.get_documents().await.map_err(|err| {
        tracing::error!("Failed to get documents: {:?}", err);
        HTTPError::internal_error()
    })?;

    Ok(StatusCode::OK)
}
