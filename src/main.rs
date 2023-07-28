use octocrab::Octocrab;
use server::{setup_tracing, Configuration, Db, Embeddings, Tiny, Tinyvector};
use tokio::time::Instant;

#[tokio::main]
async fn main() -> Result<(), hyper::Error> {
    // Loads the .env file located in the environment's current directory or its parents in sequence.
    // .env used only for development, so we discard error in all other cases.
    dotenv::dotenv().ok();

    // Tries to load tracing config from environment (RUST_LOG) or uses "debug".
    setup_tracing();

    // Parse configuration from the environment.
    tracing::debug!("Initializing configuration");
    let cfg = Configuration::new();

    // Initialize db and run migrations.
    tracing::debug!("Initializing db pool");
    let db = Db::new(&cfg.db_dsn).await.expect("Failed to setup db");

    // Initialize GitHub client.
    let gh = Octocrab::builder()
        .personal_token(cfg.github_token.clone())
        .build()
        .expect("Failed to build Octocrab");

    // Initialize Embeddings model.
    let embeddings = Embeddings::new()
        .await
        .expect("Failed to load embeddings model");

    // Initialize Tinyvector.
    let tiny = Tiny::new().extension();
    load_tinyvector(&db, tiny.clone()).await;

    // Spin up our server.
    tracing::info!("Starting server on {}...", cfg.listen_address);
    server::run(cfg, db, gh, embeddings, tiny).await
}

async fn load_tinyvector(db: &Db, tiny: Tinyvector) {
    let instant = Instant::now();
    let embeddings = db
        .query_embeddings_by_source("github.com:vercel:next.js:canary")
        .await
        .expect("Failed to query embeddings");
    if embeddings.is_empty() {
        tracing::info!("No embeddings to load");
        return;
    }

    tiny.clone()
        .write_owned()
        .await
        .create_collection("default".to_string())
        .expect("Failed to create tinyvector collection");

    for embedding in embeddings {
        let _ = tiny.write().await.insert_into_collection(
            "default",
            embedding.doc_path,
            embedding.vector,
            embedding.blob,
        );
    }
    tracing::info!("Loaded tinyvector, elapsed {:?}", instant.elapsed());
}
