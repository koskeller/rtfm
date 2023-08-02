use octocrab::Octocrab;
use server::{setup_tracing, Configuration, Db, Embeddings, Tiny, Tinyvector};
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    // Loads the .env file located in the environment's current directory or its parents in sequence.
    // .env used only for development, so we discard error in all other cases.
    dotenv::dotenv().ok();

    // Tries to load tracing config from environment (RUST_LOG) or uses "debug" by default.
    setup_tracing();

    tracing::debug!("Initializing configuration");
    let cfg = Configuration::new();

    tracing::debug!("Initializing db");
    let db = Db::new(&cfg.db_dsn).await.expect("Failed to setup db");

    tracing::debug!("Running migrations");
    let _ = db.migrate().await.expect("Failed to run migrations");

    tracing::debug!("Initializing GitHub client");
    let gh = Octocrab::builder()
        .personal_token(cfg.github_token.clone())
        .build()
        .expect("Failed to build GitHub client");

    tracing::debug!("Initializing embeddings model");
    let embeddings = Embeddings::new().expect("Failed to load embeddings model");

    tracing::debug!("Initializing vector db");
    let tiny = Tiny::new().extension();
    load_tinyvector(&db, tiny.clone()).await;

    tracing::info!("Starting server on {}...", cfg.listen_address);
    server::run(cfg, db, gh, embeddings, tiny).await
}

async fn load_tinyvector(db: &Db, tiny: Tinyvector) {
    let instant = Instant::now();
    let chunks = db
        .query_chunks_by_collection(1)
        .await
        .expect("Failed to query chunks");
    if chunks.is_empty() {
        tracing::info!("No chunks to load");
        return;
    }

    tiny.clone()
        .write_owned()
        .await
        .create_collection("default".to_string())
        .expect("Failed to create tinyvector collection");

    for chunk in chunks {
        let _ = tiny.write().await.insert_into_collection(
            "default",
            format!("{}", chunk.document_id),
            chunk.vector,
            chunk.data,
        );
    }
    tracing::info!("Loaded tinyvector, elapsed {:?}", instant.elapsed());
}
