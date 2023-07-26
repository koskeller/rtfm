use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashSet;

use crate::GitHub;

#[derive(Serialize, Deserialize)]
pub struct Source {
    pub id: String,
    pub owner: String,
    pub repo: String,
    pub branch: String,
    pub allowed_ext: HashSet<String>,
    pub allowed_dirs: HashSet<String>,
    pub ignored_dirs: HashSet<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct Document {
    pub id: String,
    pub path: String,
    pub checksum: u32,
    pub tokens: usize,
    pub blob: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Serialize, Deserialize)]
pub struct Embedding {
    pub id: u32,
    pub doc_id: String,
    pub vector: Vec<f32>,
}

#[derive(Serialize_repr, Deserialize_repr)]
#[repr(u8)]
pub enum SourceType {
    GitHub,
    Web,
}
