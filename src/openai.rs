use async_openai::{
    config::OpenAIConfig,
    error::OpenAIError,
    types::{CreateEmbeddingRequestArgs, Embedding},
    Client,
};

#[derive(Clone)]
pub struct OpenAI {
    client: Client<OpenAIConfig>,
}

impl OpenAI {
    pub fn new() -> Self {
        let client = async_openai::Client::new();
        Self { client }
    }

    pub async fn create_embeddings(
        &self,
        chunks: &Vec<String>,
    ) -> Result<Vec<Embedding>, OpenAIError> {
        let req = CreateEmbeddingRequestArgs::default()
            .model("text-embedding-ada-002")
            .input(chunks)
            .build()?;
        let emb = self.client.embeddings().create(req).await?;
        Ok(emb.data)
    }
}
