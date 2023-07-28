use rust_bert::{
    pipelines::sentence_embeddings::{
        SentenceEmbeddingsBuilder, SentenceEmbeddingsModel, SentenceEmbeddingsModelType,
    },
    RustBertError,
};
use std::sync::Arc;
use tokio::{sync::Mutex, time::Instant};

#[derive(Clone)]
pub struct Embeddings {
    model: Arc<Mutex<SentenceEmbeddingsModel>>,
}

impl Embeddings {
    pub async fn new() -> Result<Self, RustBertError> {
        let instant = Instant::now();
        tracing::info!("Loading remote model 'AllMiniLmL12V2'");

        let blocking_task = tokio::task::spawn_blocking(|| {
            SentenceEmbeddingsBuilder::remote(SentenceEmbeddingsModelType::AllMiniLmL12V2)
                .create_model()
        });
        let model = blocking_task.await.unwrap()?;

        // let model = SentenceEmbeddingsBuilder::local("model")
        //     .with_device(tch::Device::cuda_if_available())
        //     .create_model()?;
        tracing::info!("Loaded remote model, elapsed {:?}", instant.elapsed());
        Ok(Self {
            model: Arc::new(Mutex::new(model)),
        })
    }

    pub async fn encode(&self, sentences: &[String]) -> Result<Vec<Vec<f32>>, RustBertError> {
        self.model.lock().await.encode(sentences)
    }
}
