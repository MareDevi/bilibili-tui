//! Bilibili API Client with cookie management and WBI signing

use super::wbi;
use crate::storage::Credentials;
use anyhow::Result;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER, USER_AGENT};
use reqwest::Client;
use serde::Deserialize;
use std::sync::RwLock;

const UA: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36";

pub enum BilibiliApiDomain {
    Main,
    Passport,
}

impl BilibiliApiDomain {
    fn as_str(&self) -> &'static str {
        match self {
            BilibiliApiDomain::Main => "https://api.bilibili.com",
            BilibiliApiDomain::Passport => "https://passport.bilibili.com",
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct ApiResponse<T> {
    pub code: i32,
    pub message: String,
    #[allow(dead_code)]
    pub ttl: Option<i32>,
    pub data: Option<T>,
}

/// WBI keys for signing requests
#[derive(Debug, Clone)]
pub struct WbiKeys {
    pub img_key: String,
    pub sub_key: String,
}

pub struct ApiClient {
    client: Client,
    cookies: RwLock<Option<String>>,
    wbi_keys: RwLock<Option<WbiKeys>>,
}

impl ApiClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .default_headers(Self::default_headers())
                .build()
                .expect("Failed to create HTTP client"),
            cookies: RwLock::new(None),
            wbi_keys: RwLock::new(None),
        }
    }

    pub fn with_cookies(credentials: &Credentials) -> Self {
        let client = Self::new();
        client.set_credentials(credentials);
        client
    }

    fn default_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.insert(USER_AGENT, HeaderValue::from_static(UA));
        headers.insert(REFERER, HeaderValue::from_static("https://www.bilibili.com/"));
        headers
    }

    pub fn set_credentials(&self, credentials: &Credentials) {
        let cookie_str = format!(
            "SESSDATA={}; bili_jct={}; DedeUserID={}",
            credentials.sessdata, credentials.bili_jct, credentials.dede_user_id
        );
        *self.cookies.write().unwrap() = Some(cookie_str);
    }

    fn build_url(&self, domain: BilibiliApiDomain, endpoint: &str) -> String {
        format!("{}{}", domain.as_str(), endpoint)
    }

    /// Make a GET request
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<ApiResponse<T>> {
        let mut req = self.client.get(url);
        if let Some(ref cookies) = *self.cookies.read().unwrap() {
            req = req.header(COOKIE, cookies.as_str());
        }
        let resp = req.send().await?;
        let api_resp: ApiResponse<T> = resp.json().await?;
        Ok(api_resp)
    }

    /// Make a WBI-signed GET request
    pub async fn get_with_wbi<T: for<'de> Deserialize<'de>>(
        &self,
        base_url: &str,
        params: Vec<(&str, String)>,
    ) -> Result<ApiResponse<T>> {
        // Ensure we have WBI keys
        self.ensure_wbi_keys().await?;

        let keys = self.wbi_keys.read().unwrap();
        let keys = keys.as_ref().unwrap();

        let query = wbi::encode_wbi(params, &keys.img_key, &keys.sub_key);
        let url = format!("{}?{}", base_url, query);

        self.get(&url).await
    }

    /// Fetch WBI keys from nav API
    async fn ensure_wbi_keys(&self) -> Result<()> {
        if self.wbi_keys.read().unwrap().is_some() {
            return Ok(());
        }

        #[derive(Deserialize)]
        struct WbiImg {
            img_url: String,
            sub_url: String,
        }

        #[derive(Deserialize)]
        struct NavData {
            wbi_img: WbiImg,
        }

        let url = self.build_url(BilibiliApiDomain::Main, "/x/web-interface/nav");
        let resp: ApiResponse<NavData> = self.get(&url).await?;

        if let Some(data) = resp.data {
            let img_key = wbi::extract_key_from_url(&data.wbi_img.img_url)
                .ok_or_else(|| anyhow::anyhow!("Failed to extract img_key"))?;
            let sub_key = wbi::extract_key_from_url(&data.wbi_img.sub_url)
                .ok_or_else(|| anyhow::anyhow!("Failed to extract sub_key"))?;

            *self.wbi_keys.write().unwrap() = Some(WbiKeys { img_key, sub_key });
        }

        Ok(())
    }

    // Auth APIs
    pub async fn get_qrcode_data(&self) -> Result<super::auth::QrcodeData> {
        let url = self.build_url(
            BilibiliApiDomain::Passport,
            "/x/passport-login/web/qrcode/generate",
        );
        let resp: ApiResponse<super::auth::QrcodeData> = self.get(&url).await?;
        resp.data
            .ok_or_else(|| anyhow::anyhow!("No data in QR code response"))
    }

    pub async fn poll_qrcode(&self, qrcode_key: &str) -> Result<super::auth::QrcodePollResult> {
        let url = format!(
            "{}/x/passport-login/web/qrcode/poll?qrcode_key={}",
            BilibiliApiDomain::Passport.as_str(),
            qrcode_key
        );

        let mut req = self.client.get(&url);
        if let Some(ref cookies) = *self.cookies.read().unwrap() {
            req = req.header(COOKIE, cookies.as_str());
        }

        let resp = req.send().await?;

        // Extract cookies from response headers
        let mut new_cookies = Vec::new();
        for cookie in resp.cookies() {
            new_cookies.push((cookie.name().to_string(), cookie.value().to_string()));
        }

        let api_resp: ApiResponse<super::auth::QrcodePollData> = resp.json().await?;

        Ok(super::auth::QrcodePollResult {
            data: api_resp.data,
            cookies: new_cookies,
        })
    }

    // Recommendation API
    pub async fn get_recommendations(&self) -> Result<Vec<super::recommend::VideoItem>> {
        let url = self.build_url(
            BilibiliApiDomain::Main,
            "/x/web-interface/wbi/index/top/feed/rcmd",
        );

        let params = vec![
            ("fresh_type", "4".to_string()),
            ("ps", "20".to_string()),
            ("fresh_idx", "1".to_string()),
            ("fresh_idx_1h", "1".to_string()),
        ];

        let resp: ApiResponse<super::recommend::RecommendData> =
            self.get_with_wbi(&url, params).await?;

        Ok(resp
            .data
            .map(|d| d.item.into_iter().filter(|v| v.bvid.is_some()).collect())
            .unwrap_or_default())
    }

    // Video API
    pub async fn get_video_info(&self, bvid: &str) -> Result<super::video::VideoInfo> {
        let url = format!(
            "{}/x/web-interface/view?bvid={}",
            BilibiliApiDomain::Main.as_str(),
            bvid
        );
        let resp: ApiResponse<super::video::VideoInfo> = self.get(&url).await?;
        resp.data
            .ok_or_else(|| anyhow::anyhow!("No data in video info response"))
    }

    // Search API
    pub async fn search_videos(&self, keyword: &str, page: i32) -> Result<super::search::SearchData> {
        let url = self.build_url(
            BilibiliApiDomain::Main,
            "/x/web-interface/wbi/search/type",
        );

        let params = vec![
            ("search_type", "video".to_string()),
            ("keyword", keyword.to_string()),
            ("page", page.to_string()),
            ("order", "totalrank".to_string()),
        ];

        let resp: ApiResponse<super::search::SearchData> = self.get_with_wbi(&url, params).await?;
        Ok(resp.data.unwrap_or(super::search::SearchData {
            result: None,
            num_results: Some(0),
            page: Some(page),
            pagesize: Some(20),
        }))
    }

    // Dynamic Feed API
    pub async fn get_dynamic_feed(&self, offset: Option<&str>) -> Result<super::dynamic::DynamicFeedData> {
        let mut url = format!(
            "{}/x/polymer/web-dynamic/v1/feed/all?type=video",
            BilibiliApiDomain::Main.as_str()
        );
        
        if let Some(off) = offset {
            url.push_str(&format!("&offset={}", off));
        }

        let resp: ApiResponse<super::dynamic::DynamicFeedData> = self.get(&url).await?;
        Ok(resp.data.unwrap_or(super::dynamic::DynamicFeedData {
            items: None,
            offset: None,
            has_more: Some(false),
            update_num: Some(0),
        }))
    }

    // Comments API
    pub async fn get_comments(&self, oid: i64, pn: i32) -> Result<super::comment::CommentData> {
        let url = format!(
            "{}/x/v2/reply?type=1&oid={}&sort=1&ps=20&pn={}",
            BilibiliApiDomain::Main.as_str(),
            oid,
            pn
        );

        let resp: ApiResponse<super::comment::CommentData> = self.get(&url).await?;
        Ok(resp.data.unwrap_or(super::comment::CommentData {
            page: None,
            replies: None,
            hots: None,
        }))
    }

    // Related Videos API
    pub async fn get_related_videos(&self, bvid: &str) -> Result<Vec<super::video::RelatedVideoItem>> {
        let url = format!(
            "{}/x/web-interface/archive/related?bvid={}",
            BilibiliApiDomain::Main.as_str(),
            bvid
        );

        let resp: ApiResponse<Vec<super::video::RelatedVideoItem>> = self.get(&url).await?;
        Ok(resp.data.unwrap_or_default())
    }

    // Extended Recommendations API with pagination
    pub async fn get_recommendations_paged(&self, fresh_idx: i32) -> Result<Vec<super::recommend::VideoItem>> {
        let url = self.build_url(
            BilibiliApiDomain::Main,
            "/x/web-interface/wbi/index/top/feed/rcmd",
        );

        let params = vec![
            ("fresh_type", "4".to_string()),
            ("ps", "20".to_string()),
            ("fresh_idx", fresh_idx.to_string()),
            ("fresh_idx_1h", fresh_idx.to_string()),
        ];

        let resp: ApiResponse<super::recommend::RecommendData> =
            self.get_with_wbi(&url, params).await?;

        Ok(resp
            .data
            .map(|d| d.item.into_iter().filter(|v| v.bvid.is_some()).collect())
            .unwrap_or_default())
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}
