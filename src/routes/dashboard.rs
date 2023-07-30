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
            .route("/sources/create", get(create_source))
            // .route("/sources/:source_id/edit", get(source_form))
            .route("/sources/:source_id/embeddings", get(get_embeddings))
            .route("/sources/:source_id/docs", get(get_docs)),
    )
}

#[derive(TemplateOnce)]
#[template(path = "sources.html")]
struct SourcesPage {
    data: Vec<Source>,
}

struct Source {
    id: String,
    url: String,
    allowed_ext: String,
    allowed_dirs: String,
    ignored_dirs: String,
    docs_url: String,
    embeddings_url: String,
    parse_worker_url: String,
    embeddings_worker_url: String,
    delete_docs_url: String,
    delete_embeddings_url: String,
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
            id: x.id.clone(),
            url: format!("https://github.com/{}/{}", x.owner, x.repo),
            allowed_ext: x.allowed_ext.into_iter().collect::<Vec<_>>().join(", "),
            allowed_dirs: x.allowed_dirs.into_iter().collect::<Vec<_>>().join(", "),
            ignored_dirs: x.ignored_dirs.into_iter().collect::<Vec<_>>().join(", "),
            docs_url: format!("/dashboard/sources/{}/docs", &x.id),
            embeddings_url: format!("/dashboard/sources/{}/embeddings", &x.id),
            parse_worker_url: format!("/sources/{}/worker/parse", &x.id),
            embeddings_worker_url: format!("/sources/{}/worker/embeddings", &x.id),
            delete_docs_url: format!("/sources/{}/docs", &x.id),
            delete_embeddings_url: format!("/sources/{}/embeddings", &x.id),
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
#[template(path = "source_form.html")]
struct CreateSourcePage {}

pub async fn create_source() -> Result<Html<String>, ServerError> {
    let page = CreateSourcePage {};
    let html = page
        .render_once()
        .context("Failed to render source form")
        .map_err(|err| ServerError::Embeddings(err))?;
    Ok(Html(html))
}

#[derive(TemplateOnce)]
#[template(path = "embeddings.html")]
struct EmbeddingsPage {
    data: Vec<Embedding>,
}

struct Embedding {
    id: String,
    html: String,
}

pub async fn get_embeddings(
    Path(source_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, ServerError> {
    let data = state
        .db
        .query_embeddings_by_source(&source_id)
        .await
        .context("Failed to query embeddings")
        .map_err(|err| ServerError::DbError(err))?;
    let data = data
        .into_iter()
        .map(|x| Embedding {
            id: format!("{}:{}", x.doc_path, x.chunk_index),
            html: markdown::to_html(&x.blob),
        })
        .collect();
    let page = EmbeddingsPage { data };
    let html = page
        .render_once()
        .context("Failed to render embeddings")
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
    Path(source_id): Path<String>,
    State(state): State<AppState>,
) -> Result<Html<String>, ServerError> {
    let data = state
        .db
        .query_documents_by_source(&source_id)
        .await
        .context("Failed to query embeddings")
        .map_err(|err| ServerError::DbError(err))?;
    let data = data
        .into_iter()
        .map(|x| Doc {
            id: x.path,
            html: markdown::to_html(&x.blob),
        })
        .collect();
    let page = DocsPage { data };
    let html = page
        .render_once()
        .context("Failed to render embeddings")
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