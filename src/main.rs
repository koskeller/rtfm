use octocrab::Octocrab;
use server::{setup_tracing, Configuration, Db, OpenAI};

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

    let gh = Octocrab::builder()
        .personal_token(cfg.github_token.clone())
        .build()
        .expect("Failed to build Octocrab");

    let open_ai = OpenAI::new();

    // Spin up our server.
    tracing::info!("Starting server on {}...", cfg.listen_address);
    server::run(cfg, db, gh, open_ai).await
}
