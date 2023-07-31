use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Collection {
    pub id: i64,
    pub name: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Source {
    pub id: i64,
    pub collection_id: i64,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub allowed_ext: HashSet<String>,
    pub allowed_dirs: HashSet<String>,
    pub ignored_dirs: HashSet<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Document {
    pub id: i64,
    pub source_id: i64,
    pub collection_id: i64,
    pub path: String,
    pub checksum: u32,
    pub tokens_len: usize,
    pub data: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct Chunk {
    pub id: i64,
    pub document_id: i64,
    pub source_id: i64,
    pub collection_id: i64,
    pub chunk_index: usize,
    pub context: String,
    pub data: String,
    pub vector: Vec<f32>,
}
