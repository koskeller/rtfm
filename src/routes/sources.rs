use anyhow::{anyhow, Context};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use hyper::StatusCode;
use serde::{Deserialize, Serialize};

use crate::{errors::ServerError, parser, types::Source, AppState};

#[derive(Serialize, Deserialize)]
pub struct CreateSourceReq {
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub allowed_ext: Vec<String>,
    pub allowed_dirs: Vec<String>,
    pub ignored_dirs: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CreateSourceResp {
    pub id: String,
}

pub async fn create_source(
    State(state): State<AppState>,
    Json(payload): Json<CreateSourceReq>,
) -> Result<(StatusCode, Json<CreateSourceResp>), ServerError> {
    let source: Source = payload.into();
    let response = CreateSourceResp {
        id: source.id.clone(),
    };
    let _ = state
        .db
        .insert_source(&source)
        .await
        .context("Failed to insert source")
        .map_err(|err| ServerError::DbError(err))?;
    Ok((StatusCode::OK, Json(response)))
}

impl From<CreateSourceReq> for Source {
    fn from(value: CreateSourceReq) -> Self {
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

pub async fn parse_source(
    Path(id): Path<String>,
    State(state): State<AppState>,
) -> Result<StatusCode, ServerError> {
    let source = state.db.select_source(&id).await.map_err(|err| match err {
        sqlx::Error::RowNotFound => ServerError::NoContent(anyhow!("Source does not exist")),
        _ => ServerError::DbError(anyhow!("Failed to select source: {}", err)),
    })?;

    let parser = parser::GitHub::new(&state.github, &source.owner, &source.repo, &source.branch)
        .allowed_ext(source.allowed_ext.clone())
        .allowed_dirs(source.allowed_dirs.clone())
        .ignored_dirs(source.ignored_dirs.clone())
        .build();

    let documents = parser
        .get_documents()
        .await
        .context("Failed to get github documents")
        .map_err(|err| ServerError::GitHubAPIError(err))?;

    Ok(StatusCode::OK)
}
