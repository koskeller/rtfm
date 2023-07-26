use std::{
    env::var,
    net::{Ipv6Addr, SocketAddr},
    sync::Arc,
};

pub type Config = Arc<Configuration>;

#[derive(serde::Deserialize)]
pub struct Configuration {
    /// The address to listen on.
    pub listen_address: SocketAddr,
    // The port to listen on.
    pub app_port: u16,

    pub db_dsn: String,
    pub github_token: String,
    pub open_ai_key: String,
}

impl Configuration {
    pub fn new() -> Config {
        let app_port = var("PORT")
            .expect("Missing PORT environment variable")
            .parse::<u16>()
            .expect("Unable to parse the value of the PORT environment variable. Please make sure it is a valid unsigned 16-bit integer");

        let db_dsn = var("DATABASE_URL").expect("Missing DATABASE_URL environment variable");
        let github_token = var("GITHUB_TOKEN").expect("Missing GITHUB_TOKEN environment variablw");
        let open_ai_key =
            var("OPENAI_API_KEY").expect("Missing OPENAI_API_KEY environment variablw");

        let listen_address = SocketAddr::from((Ipv6Addr::UNSPECIFIED, app_port));

        Arc::new(Configuration {
            listen_address,
            app_port,
            db_dsn,
            github_token,
            open_ai_key,
        })
    }

    pub fn set_dsn(&mut self, db_dsn: String) {
        self.db_dsn = db_dsn
    }
}
