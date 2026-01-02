use anyhow::Result;
use reqwest::Client;
use serde::Deserialize;

pub enum BilibiliApiDomain {
    Main,
    Passport,
    Vc,
    Live,
}

impl BilibiliApiDomain {
    fn as_str(&self) -> &'static str {
        match self {
            BilibiliApiDomain::Main => "https://api.bilibili.com",
            BilibiliApiDomain::Passport => "https://passport.bilibili.com",
            BilibiliApiDomain::Vc => "https://vc.bilibili.com",
            BilibiliApiDomain::Live => "https://live.bilibili.com",
        }
    }
}

#[derive(Debug, Deserialize)]
struct ApiResponse<T> {
    code: i32,
    message: String,
    ttl: Option<i32>,
    data: Option<T>,
}
pub struct ApiClient {
    client: Client,
}

#[derive(Debug, Deserialize)]
pub struct QrcodeData {
    pub url: String,
    pub qrcode_key: String,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::new(),
        }
    }

    fn build_url(&self, domain: BilibiliApiDomain, endpoint: &str) -> String {
        format!("{}{}", domain.as_str(), endpoint)
    }

    pub async fn get_qrcode_data(&self) -> Result<QrcodeData> {
        let url = self.build_url(
            BilibiliApiDomain::Passport,
            "/x/passport-login/web/qrcode/generate",
        );
        let resp = self.client.get(&url).send().await?;
        let api_resp: ApiResponse<QrcodeData> = resp.json().await?;
        match (api_resp.code, api_resp.message, api_resp.data) {
            (0, _, Some(data)) => Ok(data),
            (code, message, _) if code != 0 => {
                Err(anyhow::anyhow!("API Error {}: {}", code, message))
            }
            _ => Err(anyhow::anyhow!("No data in API response")),
        }
    }
}
