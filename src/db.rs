use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

use crate::types::{Document, Embedding, Source};

#[derive(Clone)]
pub struct Db {
    pool: SqlitePool,
}

impl Db {
    pub async fn new(url: &str) -> Result<Self, sqlx::Error> {
        let options = SqliteConnectOptions::from_str(url)?;
        let pool = SqlitePoolOptions::new().connect_with(options).await?;
        sqlx::migrate!("./migrations").run(&pool).await?;
        Ok(Self { pool })
    }

    pub async fn insert_source(&self, data: &Source) -> Result<(), sqlx::Error> {
        let allowed_ext = data
            .allowed_ext
            .clone()
            .into_iter()
            .collect::<Vec<_>>()
            .join(";");
        let allowed_dirs = data
            .allowed_dirs
            .clone()
            .into_iter()
            .collect::<Vec<_>>()
            .join(";");
        let ignored_dirs = data
            .ignored_dirs
            .clone()
            .into_iter()
            .collect::<Vec<_>>()
            .join(";");

        sqlx::query!(
            r#"
        INSERT OR REPLACE INTO sources (id, owner, repo, branch, allowed_ext, allowed_dirs, ignored_dirs, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
            data.id,
            data.owner,
            data.repo,
            data.branch,
            allowed_ext,
            allowed_dirs,
            ignored_dirs,
            data.created_at,
            data.updated_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn select_source(&self, id: &str) -> Result<Source, sqlx::Error> {
        let row = sqlx::query!(r#"SELECT * FROM sources WHERE id = ?"#, id)
            .fetch_one(&self.pool)
            .await?;
        Ok(Source {
            id: row.id,
            owner: row.owner,
            repo: row.repo,
            branch: row.branch,
            allowed_ext: row.allowed_ext.split(';').map(|x| x.to_string()).collect(),
            allowed_dirs: row.allowed_dirs.split(';').map(|x| x.to_string()).collect(),
            ignored_dirs: row.ignored_dirs.split(';').map(|x| x.to_string()).collect(),
            created_at: row.created_at.parse().unwrap_or_default(),
            updated_at: row.updated_at.parse().unwrap_or_default(),
        })
    }

    pub async fn insert_document(&self, data: &Document) -> Result<(), sqlx::Error> {
        let tokens = data.tokens as u32;
        sqlx::query!(
            r#"
        INSERT OR REPLACE INTO documents (id, path, checksum, tokens, blob, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
            data.id,
            data.path,
            data.checksum,
            tokens,
            data.blob,
            data.created_at,
            data.updated_at,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn insert_embedding(&self, data: &Embedding) -> Result<(), sqlx::Error> {
        let vector = bincode::serialize(&data.vector).expect("Failed to serialize vector");
        sqlx::query!(
            r#"
        INSERT OR REPLACE INTO embeddings (id, doc_id, vector)
        VALUES (?, ?, ?)
        "#,
            data.id,
            data.doc_id,
            vector,
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
