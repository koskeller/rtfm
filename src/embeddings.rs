use rust_bert::{
    pipelines::sentence_embeddings::{SentenceEmbeddingsBuilder, SentenceEmbeddingsModel},
    RustBertError,
};
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct Embeddings {
    model: Arc<Mutex<SentenceEmbeddingsModel>>,
}

impl Embeddings {
    pub fn new() -> Result<Self, RustBertError> {
        tracing::info!("Loading local model 'AllMiniLmL12V2' from disk");
        let model = SentenceEmbeddingsBuilder::local("model")
            .with_device(tch::Device::cuda_if_available())
            .create_model()?;
        Ok(Self {
            model: Arc::new(Mutex::new(model)),
        })
    }

    pub async fn encode(&self, sentences: &[String]) -> Result<Vec<Vec<f32>>, RustBertError> {
        self.model.lock().await.encode(sentences)
    }
}
