pub mod appcheck;
pub mod custom_token;
pub mod id_token;
pub mod jwt;
pub mod oauth;
pub mod service_account;
pub mod users;

use std::sync::Arc;

use reqwest::Client;

#[derive(Clone)]
pub struct HttpClient(pub Arc<Client>);

impl HttpClient {
    pub fn new() -> Self {
        let client = Client::builder()
            .user_agent(concat!(
                env!("CARGO_PKG_NAME"),
                "/",
                env!("CARGO_PKG_VERSION")
            ))
            .build()
            .expect("reqwest client build");
        Self(Arc::new(client))
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}
