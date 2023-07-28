use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::{
    collections::{BinaryHeap, HashMap},
    sync::Arc,
};
use tokio::sync::RwLock;

#[allow(clippy::module_name_repetitions)]
pub type Tinyvector = Arc<RwLock<Tiny>>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Collection already exists")]
    UniqueViolation,

    #[error("Collection doesn't exist")]
    NotFound,

    #[error("The dimension of the vector doesn't match the dimension of the collection")]
    DimensionMismatch,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimilarityResult {
    pub score: f32,
    pub embedding: Embedding,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Collection {
    /// Dimension of the vectors in the collection
    pub dimension: usize,
    /// Distance metric used for querying
    pub distance: Distance,
    /// Embeddings in the collection
    #[serde(default)]
    pub embeddings: Vec<Embedding>,
}

impl Collection {
    pub fn get_similarity(&self, query: &[f32], k: usize) -> Vec<SimilarityResult> {
        let memo_attr = get_cache_attr(self.distance, query);
        let distance_fn = get_distance_fn(self.distance);

        let scores = self
            .embeddings
            .par_iter()
            .enumerate()
            .map(|(index, embedding)| {
                let score = distance_fn(&embedding.vector, query, memo_attr);
                ScoreIndex { score, index }
            })
            .collect::<Vec<_>>();

        let mut heap = BinaryHeap::new();
        for score_index in scores {
            if heap.len() < k || score_index < *heap.peek().unwrap() {
                heap.push(score_index);

                if heap.len() > k {
                    heap.pop();
                }
            }
        }

        heap.into_sorted_vec()
            .into_iter()
            .map(|ScoreIndex { score, index }| SimilarityResult {
                score,
                embedding: self.embeddings[index].clone(),
            })
            .collect()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Embedding {
    pub id: String,
    vector: Vec<f32>,
    pub blob: String,
}

impl Embedding {
    pub fn new(id: String, vector: Vec<f32>, blob: String) -> Self {
        Self { id, vector, blob }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Tiny {
    pub collections: HashMap<String, Collection>,
}

impl Tiny {
    pub fn new() -> Self {
        Self {
            collections: HashMap::new(),
        }
    }

    pub fn extension(self) -> Tinyvector {
        Arc::new(RwLock::new(self))
    }

    pub fn create_collection(&mut self, name: String) -> Result<Collection, Error> {
        if self.collections.contains_key(&name) {
            return Err(Error::UniqueViolation);
        }
        let dimension = 384;
        let distance = Distance::Cosine;
        let collection = Collection {
            dimension,
            distance,
            embeddings: Vec::new(),
        };
        self.collections.insert(name, collection.clone());
        Ok(collection)
    }

    pub fn delete_collection(&mut self, name: &str) -> Result<(), Error> {
        if !self.collections.contains_key(name) {
            return Err(Error::NotFound);
        }
        self.collections.remove(name);
        Ok(())
    }

    pub fn insert_into_collection(
        &mut self,
        collection_name: &str,
        id: String,
        mut vector: Vec<f32>,
        blob: String,
    ) -> Result<(), Error> {
        let collection = self
            .collections
            .get_mut(collection_name)
            .ok_or(Error::NotFound)?;

        if collection.embeddings.iter().any(|e| e.id == id) {
            return Err(Error::UniqueViolation);
        }

        if vector.len() != collection.dimension {
            return Err(Error::DimensionMismatch);
        }

        // Normalize the vector if the distance metric is cosine, so we can use dot product later
        if collection.distance == Distance::Cosine {
            vector = normalize(&vector);
        }

        collection.embeddings.push(Embedding { id, vector, blob });

        Ok(())
    }

    pub fn get_collection(&self, name: &str) -> Option<&Collection> {
        self.collections.get(name)
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Distance {
    #[serde(rename = "euclidean")]
    Euclidean,
    #[serde(rename = "cosine")]
    Cosine,
    #[serde(rename = "dot")]
    DotProduct,
}

pub fn get_cache_attr(metric: Distance, vec: &[f32]) -> f32 {
    match metric {
        // Dot product doesn't allow any caching
        Distance::DotProduct | Distance::Euclidean => 0.0,
        // Precompute the magnitude of the vector
        Distance::Cosine => vec.iter().map(|&x| x.powi(2)).sum::<f32>().sqrt(),
    }
}

pub fn get_distance_fn(metric: Distance) -> impl Fn(&[f32], &[f32], f32) -> f32 {
    match metric {
        Distance::Euclidean => euclidian_distance,
        // We use dot product for cosine because we've normalized the vectors on insertion
        Distance::Cosine | Distance::DotProduct => dot_product,
    }
}

fn euclidian_distance(a: &[f32], b: &[f32], a_sum_squares: f32) -> f32 {
    let mut cross_terms = 0.0;
    let mut b_sum_squares = 0.0;

    for (i, j) in a.iter().zip(b) {
        cross_terms += i * j;
        b_sum_squares += j.powi(2);
    }

    2.0f32
        .mul_add(-cross_terms, a_sum_squares + b_sum_squares)
        .max(0.0)
        .sqrt()
}

fn dot_product(a: &[f32], b: &[f32], _: f32) -> f32 {
    a.iter().zip(b).fold(0.0, |acc, (x, y)| acc + x * y)
}

pub fn normalize(vec: &[f32]) -> Vec<f32> {
    let magnitude = (vec.iter().fold(0.0, |acc, &val| val.mul_add(val, acc))).sqrt();

    if magnitude > std::f32::EPSILON {
        vec.iter().map(|&val| val / magnitude).collect()
    } else {
        vec.to_vec()
    }
}

pub struct ScoreIndex {
    pub score: f32,
    pub index: usize,
}

impl PartialEq for ScoreIndex {
    fn eq(&self, other: &Self) -> bool {
        self.score.eq(&other.score)
    }
}

impl Eq for ScoreIndex {}

impl PartialOrd for ScoreIndex {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // The comparison is intentionally reversed here to make the heap a min-heap
        other.score.partial_cmp(&self.score)
    }
}

impl Ord for ScoreIndex {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap_or(Ordering::Equal)
    }
}
