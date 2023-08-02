use anyhow::Context;
use axum::{
    extract::{Path, Query, State},
    response::Html,
    routing::get,
    Router,
};
use sailfish::TemplateOnce;
use serde::Deserialize;

use crate::{errors::ServerError, AppState};

pub fn routes() -> Router<AppState> {
    Router::new().nest(
        "/dashboard",
        Router::new()
            .route("/search", get(search))
            .route("/sources", get(get_sources))
            .route("/sources/:source_id/chunks", get(get_chunks))
            .route("/sources/:source_id/docs", get(get_docs)),
    )
}

#[derive(TemplateOnce)]
#[template(path = "sources.html")]
struct SourcesPage {
    data: Vec<Source>,
}

struct Source {
    id: i64,
    url: String,
    allowed_ext: String,
    allowed_dirs: String,
    ignored_dirs: String,
    docs_url: String,
    chunks_url: String,
}

pub async fn get_sources(State(state): State<AppState>) -> Result<Html<String>, ServerError> {
    let data = state
        .db
        .query_sources()
        .await
        .context("Failed to query sources")
        .map_err(|err| ServerError::DbError(err))?;
    let data = data
        .into_iter()
        .map(|x| Source {
            id: x.id,
            url: format!("https://github.com/{}/{}", x.owner, x.repo),
            allowed_ext: x.allowed_ext.into_iter().collect::<Vec<_>>().join(", "),
            allowed_dirs: x.allowed_dirs.into_iter().collect::<Vec<_>>().join(", "),
            ignored_dirs: x.ignored_dirs.into_iter().collect::<Vec<_>>().join(", "),
            docs_url: format!("/dashboard/sources/{}/docs", &x.id),
            chunks_url: format!("/dashboard/sources/{}/chunk", &x.id),
        })
        .collect();
    let page = SourcesPage { data };
    let html = page
        .render_once()
        .context("Failed to render sources")
        .map_err(|err| ServerError::Embeddings(err))?;
    Ok(Html(html))
}

#[derive(TemplateOnce)]
#[template(path = "chunks.html")]
struct ChunksPage {
    data: Vec<Chunk>,
}

struct Chunk {
    context: String,
    html: String,
}

pub async fn get_chunks(
    Path(source_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Html<String>, ServerError> {
    let data = state
        .db
        .query_chunks_by_source(source_id)
        .await
        .context("Failed to query chunks")
        .map_err(|err| ServerError::DbError(err))?;
    let data = data
        .into_iter()
        .map(|x| Chunk {
            context: x.context,
            html: markdown::to_html(&x.data),
        })
        .collect();
    let page = ChunksPage { data };
    let html = page
        .render_once()
        .context("Failed to render chunks")
        .map_err(|err| ServerError::Embeddings(err))?;
    Ok(Html(html))
}

#[derive(TemplateOnce)]
#[template(path = "docs.html")]
struct DocsPage {
    data: Vec<Doc>,
}

struct Doc {
    id: String,
    html: String,
}

pub async fn get_docs(
    Path(source_id): Path<i64>,
    State(state): State<AppState>,
) -> Result<Html<String>, ServerError> {
    let data = state
        .db
        .query_documents_by_source(source_id)
        .await
        .context("Failed to query documents")
        .map_err(|err| ServerError::DbError(err))?;
    let data = data
        .into_iter()
        .map(|x| Doc {
            id: x.path,
            html: markdown::to_html(&x.data),
        })
        .collect();
    let page = DocsPage { data };
    let html = page
        .render_once()
        .context("Failed to render documents")
        .map_err(|err| ServerError::Embeddings(err))?;
    Ok(Html(html))
}

#[derive(Deserialize)]
pub struct SearchQuery {
    pub query: Option<String>,
}

#[derive(TemplateOnce)]
#[template(path = "search.html")]
struct SearchPage {
    data: Vec<SearchResult>,
}

pub struct SearchResult {
    pub score: f32,
    pub path: String,
    pub html: String,
}

pub async fn search(
    params: Query<SearchQuery>,
    State(state): State<AppState>,
) -> Result<Html<String>, ServerError> {
    if let Some(q) = params.query.clone() {
        tracing::info!("Searching for '{}'", q);
        let query = state
            .embeddings
            .encode(&[q.clone()])
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

        let mut data = Vec::with_capacity(vectors.len());
        for n in vectors {
            data.push(SearchResult {
                score: n.score,
                path: n.embedding.id,
                html: markdown::to_html(&n.embedding.blob),
            })
        }

        let page = SearchPage { data };
        let html = page
            .render_once()
            .context("Failed to render search")
            .map_err(|err| ServerError::Embeddings(err))?;
        Ok(Html(html))
    } else {
        let page = SearchPage { data: Vec::new() };
        let html = page
            .render_once()
            .context("Failed to render search")
            .map_err(|err| ServerError::Embeddings(err))?;
        Ok(Html(html))
    }
}
