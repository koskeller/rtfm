use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;

use crate::types::{Chunk, Document, Source};

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

    pub async fn new_in_memory() -> Result<Self, sqlx::Error> {
        Db::new("sqlite::memory:").await
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
        INSERT INTO source (collection_id, owner, repo, branch, allowed_ext, allowed_dirs, ignored_dirs, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
        "#,
            data.collection_id,
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

    pub async fn select_source(&self, id: i64) -> Result<Source, sqlx::Error> {
        let row = sqlx::query!(r#"SELECT * FROM source WHERE id = ?"#, id)
            .fetch_one(&self.pool)
            .await?;
        Ok(Source {
            id: row.id,
            collection_id: row.collection_id,
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

    pub async fn query_sources(&self) -> Result<Vec<Source>, sqlx::Error> {
        let mut data = Vec::new();
        let rows = sqlx::query!(r#" SELECT * FROM source"#)
            .fetch_all(&self.pool)
            .await?;

        for row in rows {
            data.push(Source {
                id: row.id,
                collection_id: row.collection_id,
                owner: row.owner,
                repo: row.repo,
                branch: row.branch,
                allowed_ext: row.allowed_ext.split(';').map(|x| x.to_string()).collect(),
                allowed_dirs: row.allowed_dirs.split(';').map(|x| x.to_string()).collect(),
                ignored_dirs: row.ignored_dirs.split(';').map(|x| x.to_string()).collect(),
                created_at: row.created_at.parse().unwrap_or_default(),
                updated_at: row.updated_at.parse().unwrap_or_default(),
            });
        }

        Ok(data)
    }

    pub async fn insert_document(&self, data: &Document) -> Result<(), sqlx::Error> {
        let tokens_len = data.tokens_len as u32;
        sqlx::query!(
            r#"
        INSERT INTO document (source_id, collection_id, path, checksum, tokens_len, data, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?, ?)
        "#,
            data.source_id,
            data.collection_id,
            data.path,
            data.checksum,
            tokens_len,
            data.data,
            data.created_at,
            data.updated_at,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn select_document(
        &self,
        source_id: i64,
        path: &str,
    ) -> Result<Document, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT * FROM document WHERE source_id = ? AND path = ?"#,
            source_id,
            path
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Document {
            id: row.id,
            source_id: row.source_id,
            collection_id: row.collection_id,
            path: row.path,
            checksum: row.checksum as u32,
            tokens_len: row.tokens_len as usize,
            data: row.data,
            created_at: row.created_at.parse().unwrap_or_default(),
            updated_at: row.updated_at.parse().unwrap_or_default(),
        })
    }

    pub async fn insert_documents(&self, docs: &[Document]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for data in docs {
            let tokens = data.tokens_len as u32;
            sqlx::query!(r#"
                INSERT INTO document (source_id, collection_id, path, checksum, tokens_len, data, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
                data.source_id,
                data.collection_id,
                data.path,
                data.checksum,
                tokens,
                data.data,
                data.created_at,
                data.updated_at,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn query_documents_by_source(
        &self,
        source_id: i64,
    ) -> Result<Vec<Document>, sqlx::Error> {
        let mut docs = Vec::new();
        let rows = sqlx::query!(r#"SELECT * FROM document WHERE source_id = ?"#, source_id)
            .fetch_all(&self.pool)
            .await?;

        for row in rows {
            let doc = Document {
                id: row.id,
                source_id: row.source_id,
                collection_id: row.collection_id,
                path: row.path,
                checksum: row.checksum as u32,
                tokens_len: row.tokens_len as usize,
                data: row.data,
                created_at: row.created_at.parse().unwrap_or_default(),
                updated_at: row.updated_at.parse().unwrap_or_default(),
            };
            docs.push(doc);
        }

        Ok(docs)
    }

    pub async fn delete_documents_by_source(&self, source_id: i64) -> Result<(), sqlx::Error> {
        let _ = sqlx::query!(r#"DELETE FROM document WHERE source_id = ?"#, source_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn insert_chunk(&self, data: &Chunk) -> Result<(), sqlx::Error> {
        let vector = bincode::serialize(&data.vector).expect("Failed to serialize vector");
        let chunk_index = data.chunk_index as u32;
        sqlx::query!(
            r#"
        INSERT INTO chunk (document_id, source_id, collection_id, chunk_index, context, data, vector)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
            data.document_id,
            data.source_id,
            data.collection_id,
            chunk_index,
            data.context,
            data.data,
            vector,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn query_chunks_by_source(&self, source_id: i64) -> Result<Vec<Chunk>, sqlx::Error> {
        let mut chunks = Vec::new();
        let rows = sqlx::query!(r#" SELECT * FROM chunk WHERE source_id = ?"#, source_id)
            .fetch_all(&self.pool)
            .await?;
        for row in rows {
            let vector: Vec<f32> =
                bincode::deserialize(&row.vector).expect("Failed to deserialize vector");
            chunks.push(Chunk {
                id: row.id,
                document_id: row.document_id,
                source_id: row.source_id,
                collection_id: row.collection_id,
                chunk_index: row.chunk_index as usize,
                context: row.context,
                data: row.data,
                vector,
            });
        }
        Ok(chunks)
    }

    pub async fn query_chunks_by_collection(
        &self,
        collection_id: i64,
    ) -> Result<Vec<Chunk>, sqlx::Error> {
        let mut chunks = Vec::new();
        let rows = sqlx::query!(
            r#" SELECT * FROM chunk WHERE collection_id = ?"#,
            collection_id
        )
        .fetch_all(&self.pool)
        .await?;
        for row in rows {
            let vector: Vec<f32> =
                bincode::deserialize(&row.vector).expect("Failed to deserialize vector");
            chunks.push(Chunk {
                id: row.id,
                document_id: row.document_id,
                source_id: row.source_id,
                collection_id: row.collection_id,
                chunk_index: row.chunk_index as usize,
                context: row.context,
                data: row.data,
                vector,
            });
        }
        Ok(chunks)
    }

    pub async fn delete_chunks_by_source(&self, source_id: i64) -> Result<(), sqlx::Error> {
        let _ = sqlx::query!(r#"DELETE FROM chunk WHERE source_id = ?"#, source_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}
