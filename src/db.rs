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
        INSERT OR REPLACE INTO documents (source_id, path, checksum, tokens, blob, created_at, updated_at)
        VALUES (?, ?, ?, ?, ?, ?, ?)
        "#,
            data.source_id,
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

    pub async fn select_document(
        &self,
        source_id: &str,
        path: &str,
    ) -> Result<Document, sqlx::Error> {
        let row = sqlx::query!(
            r#"
            SELECT * FROM documents WHERE source_id = ? AND path = ?"#,
            source_id,
            path
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(Document {
            source_id: row.source_id,
            path: row.path,
            checksum: row.checksum as u32,
            tokens: row.tokens as usize,
            blob: row.blob,
            created_at: row.created_at.parse().unwrap_or_default(),
            updated_at: row.updated_at.parse().unwrap_or_default(),
        })
    }

    pub async fn insert_documents(&self, docs: &[Document]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for data in docs {
            let tokens = data.tokens as u32;
            sqlx::query!(r#"
                INSERT OR REPLACE INTO documents (source_id, path, checksum, tokens, blob, created_at, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?)
                "#,
                data.source_id,
                data.path,
                data.checksum,
                tokens,
                data.blob,
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
        source_id: &str,
    ) -> Result<Vec<Document>, sqlx::Error> {
        let mut docs = Vec::new();
        let recs = sqlx::query!(r#"SELECT * FROM documents WHERE source_id = ?"#, source_id)
            .fetch_all(&self.pool)
            .await?;

        for rec in recs {
            let doc = Document {
                source_id: rec.source_id,
                path: rec.path,
                checksum: rec.checksum as u32,
                tokens: rec.tokens as usize,
                blob: rec.blob,
                created_at: rec.created_at.parse().unwrap_or_default(),
                updated_at: rec.updated_at.parse().unwrap_or_default(),
            };
            docs.push(doc);
        }

        Ok(docs)
    }

    pub async fn insert_embedding(&self, data: &Embedding) -> Result<(), sqlx::Error> {
        let vector = bincode::serialize(&data.vector).expect("Failed to serialize vector");
        let chunk = data.chunk as u32;
        sqlx::query!(
            r#"
        INSERT OR REPLACE INTO embeddings (source_id, doc_path, chunk, blob, vector)
        VALUES (?, ?, ?, ?, ?)
        "#,
            data.source_id,
            data.doc_path,
            chunk,
            data.blob,
            vector,
        )
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn insert_embeddings(&self, embeddings: &[Embedding]) -> Result<(), sqlx::Error> {
        let mut tx = self.pool.begin().await?;
        for data in embeddings {
            let vector = bincode::serialize(&data.vector).expect("Failed to serialize vector");
            let chunk = data.chunk as u32;
            sqlx::query!(
                r#"
        INSERT OR REPLACE INTO embeddings (source_id, doc_path, chunk, blob, vector)
        VALUES (?, ?, ?, ?, ?)
        "#,
                data.source_id,
                data.doc_path,
                chunk,
                data.blob,
                vector,
            )
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;
        Ok(())
    }

    pub async fn query_embeddings_by_source(
        &self,
        source_id: &str,
    ) -> Result<Vec<Embedding>, sqlx::Error> {
        let mut embeddings = Vec::new();
        let rows = sqlx::query!(
            r#" SELECT * FROM embeddings WHERE source_id = ?"#,
            source_id
        )
        .fetch_all(&self.pool)
        .await?;

        for row in rows {
            let vector: Vec<f32> =
                bincode::deserialize(&row.vector).expect("Failed to deserialize vector");
            embeddings.push(Embedding {
                source_id: row.source_id,
                doc_path: row.doc_path,
                chunk: row.chunk as usize,
                blob: row.blob,
                vector,
            });
        }

        Ok(embeddings)
    }
}

#[cfg(test)]
mod tests {
    use chrono::Utc;
    use std::collections::HashSet;

    use super::*;

    #[tokio::test]
    async fn test_insert_source() {
        let db = Db::new_in_memory().await.expect("Failed to create db");

        let t = Utc::now();
        let source = Source {
            id: "1".to_string(),
            owner: "foo".to_string(),
            repo: "bar".to_string(),
            branch: "master".to_string(),
            allowed_ext: HashSet::from(["mdx".to_string()]),
            allowed_dirs: HashSet::from(["docs".to_string()]),
            ignored_dirs: HashSet::from(["docs/src".to_string()]),
            created_at: t,
            updated_at: t,
        };
        db.insert_source(&source)
            .await
            .expect("Failed to insert source");

        let row = sqlx::query!("SELECT * FROM sources WHERE id = 1")
            .fetch_one(&db.pool)
            .await
            .expect("Failed to query source");
        assert_eq!(row.id, "1");
        assert_eq!(row.owner, "foo");
        assert_eq!(row.repo, "bar");
        assert_eq!(row.branch, "master");
        assert_eq!(row.allowed_ext, "mdx");
        assert_eq!(row.allowed_dirs, "docs");
        assert_eq!(row.ignored_dirs, "docs/src");
        assert_eq!(row.created_at, t.to_rfc3339());
        assert_eq!(row.updated_at, t.to_rfc3339());

        let selected_source = db
            .select_source("1")
            .await
            .expect("Failed to select source");
        assert_eq!(selected_source, source);
    }

    #[tokio::test]
    async fn test_insert_document() {
        let db = Db::new_in_memory().await.expect("Failed to create db");

        let t = Utc::now();
        let document = Document {
            source_id: "1".to_string(),
            path: "path/to/doc".to_string(),
            checksum: 123,
            tokens: 456,
            blob: "blob".to_string(),
            created_at: t,
            updated_at: t,
        };
        db.insert_document(&document)
            .await
            .expect("Failed to insert document");

        let row =
            sqlx::query!("SELECT * FROM documents WHERE source_id = 1 AND path = 'path/to/doc'")
                .fetch_one(&db.pool)
                .await
                .expect("Failed to query document");
        assert_eq!(row.source_id, "1");
        assert_eq!(row.path, "path/to/doc");
        assert_eq!(row.checksum, 123);
        assert_eq!(row.tokens, 456);
        assert_eq!(row.blob, "blob");
        assert_eq!(row.created_at, t.to_rfc3339());
        assert_eq!(row.updated_at, t.to_rfc3339());

        let selected_doc = db
            .select_document("1", "path/to/doc")
            .await
            .expect("Failed to select document");
        assert_eq!(selected_doc, document);
    }

    #[tokio::test]
    async fn test_insert_documents() {
        let db = Db::new_in_memory().await.expect("Failed to create db");

        let documents = vec![
            Document {
                source_id: "1".to_string(),
                path: "path/to/doc1".to_string(),
                checksum: 123,
                tokens: 456,
                blob: "blob1".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Document {
                source_id: "2".to_string(),
                path: "path/to/doc2".to_string(),
                checksum: 789,
                tokens: 101112,
                blob: "blob2".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];
        db.insert_documents(&documents)
            .await
            .expect("Failed to insert documents");

        let rows = sqlx::query!("SELECT * FROM documents")
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query documents");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].source_id, "1");
        assert_eq!(rows[1].source_id, "2");
    }

    #[tokio::test]
    async fn test_query_docs_by_source() {
        let db = Db::new_in_memory().await.expect("Failed to create db");

        let documents = vec![
            Document {
                source_id: "1".to_string(),
                path: "path/to/doc1".to_string(),
                checksum: 123,
                tokens: 456,
                blob: "blob1".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
            Document {
                source_id: "2".to_string(),
                path: "path/to/doc2".to_string(),
                checksum: 789,
                tokens: 101112,
                blob: "blob2".to_string(),
                created_at: Utc::now(),
                updated_at: Utc::now(),
            },
        ];
        db.insert_documents(&documents)
            .await
            .expect("Failed to insert documents");

        let queried_docs = db
            .query_documents_by_source("1")
            .await
            .expect("Failed to query documents by source");
        assert_eq!(queried_docs.len(), 1);
        assert_eq!(queried_docs[0].source_id, "1");
    }

    #[tokio::test]
    async fn test_insert_embedding() {
        let db = Db::new_in_memory().await.expect("Failed to create db");

        let embedding = Embedding {
            source_id: "1".to_string(),
            doc_path: "2".to_string(),
            chunk: 0,
            blob: "blob".to_string(),
            vector: vec![1.0, 2.0, 3.0],
        };
        db.insert_embedding(&embedding)
            .await
            .expect("Failed to insert embedding");

        let row = sqlx::query!("SELECT * FROM embeddings WHERE source_id = '1' AND doc_path = '2'")
            .fetch_one(&db.pool)
            .await
            .expect("Failed to query embedding");
        assert_eq!(row.source_id, "1");
        assert_eq!(row.doc_path, "2");
        let vector: Vec<f32> =
            bincode::deserialize(&row.vector).expect("Failed to deserialize vector");
        assert_eq!(vector, vec![1.0, 2.0, 3.0]);
    }

    #[tokio::test]
    async fn test_insert_embeddings() {
        let db = Db::new_in_memory().await.expect("Failed to create db");

        let embeddings = vec![
            Embedding {
                source_id: "1".to_string(),
                doc_path: "2".to_string(),
                chunk: 0,
                blob: "blob".to_string(),
                vector: vec![1.0, 2.0, 3.0],
            },
            Embedding {
                source_id: "2".to_string(),
                doc_path: "2".to_string(),
                chunk: 1,
                blob: "blob".to_string(),
                vector: vec![4.0, 5.0, 6.0],
            },
        ];
        db.insert_embeddings(&embeddings)
            .await
            .expect("Failed to insert embeddings");

        let rows = sqlx::query!("SELECT * FROM embeddings")
            .fetch_all(&db.pool)
            .await
            .expect("Failed to query embeddings");
        assert_eq!(rows.len(), 2);
        assert_eq!(rows[0].source_id, "1");
        assert_eq!(rows[1].source_id, "2");
    }

    #[tokio::test]
    async fn test_query_embeddings_by_source() {
        let db = Db::new_in_memory().await.expect("Failed to create db");

        let embeddings = vec![
            Embedding {
                source_id: "1".to_string(),
                doc_path: "2".to_string(),
                chunk: 0,
                blob: "blob".to_string(),
                vector: vec![1.0, 2.0, 3.0],
            },
            Embedding {
                source_id: "2".to_string(),
                doc_path: "2".to_string(),
                chunk: 1,
                blob: "blob".to_string(),
                vector: vec![4.0, 5.0, 6.0],
            },
        ];
        db.insert_embeddings(&embeddings)
            .await
            .expect("Failed to insert embeddings");

        let queried_embeddings = db
            .query_embeddings_by_source("1")
            .await
            .expect("Failed to query embeddings by source");
        assert_eq!(queried_embeddings.len(), 1);
        assert_eq!(queried_embeddings[0].source_id, "1");
        assert_eq!(queried_embeddings[0].doc_path, "2");
    }
}
