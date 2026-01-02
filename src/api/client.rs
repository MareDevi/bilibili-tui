use reqwest::Client;


pub struct ApiClient {
    client: Client,
    base_url: String,
}

pub struct ApiResponse {
    code : i32,
    message: String,
    ttl: i32,
    data: Option<serde_json::Value>,
}

impl ApiClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
        }
    }
}
