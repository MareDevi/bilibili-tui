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
    pub fn as_str(&self) -> &'static str {
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
        headers.insert(
            REFERER,
            HeaderValue::from_static("https://www.bilibili.com/"),
        );
        headers
    }

    pub fn set_credentials(&self, credentials: &Credentials) {
        let cookie_str = format!(
            "SESSDATA={}; bili_jct={}; DedeUserID={}",
            credentials.sessdata, credentials.bili_jct, credentials.dede_user_id
        );
        *self.cookies.write().expect("cookies lock poisoned") = Some(cookie_str);
    }

    fn build_url(&self, domain: BilibiliApiDomain, endpoint: &str) -> String {
        format!("{}{}", domain.as_str(), endpoint)
    }

    /// Make a GET request
    pub async fn get<T: for<'de> Deserialize<'de>>(&self, url: &str) -> Result<ApiResponse<T>> {
        let mut req = self.client.get(url);
        if let Some(ref cookies) = *self.cookies.read().expect("cookies lock poisoned") {
            req = req.header(COOKIE, cookies.as_str());
        }
        let resp = req.send().await?;
        let api_resp: ApiResponse<T> = resp.json().await?;
        Ok(api_resp)
    }

    /// Make a POST request with form data
    pub async fn post<T: for<'de> Deserialize<'de>>(
        &self,
        url: &str,
        form_data: Vec<(&str, String)>,
    ) -> Result<ApiResponse<T>> {
        let mut req = self.client.post(url);

        // 使用块作用域确保锁在 await 之前释放
        let params = {
            let cookies = self.cookies.read().expect("cookies lock poisoned");
            if let Some(ref cookie_str) = *cookies {
                req = req.header(COOKIE, cookie_str.as_str());
            }

            let has_csrf = cookies
                .as_ref()
                .map(|c| c.contains("bili_jct"))
                .unwrap_or(false);

            let mut params: Vec<(String, String)> = form_data
                .into_iter()
                .map(|(k, v)| (k.to_string(), v))
                .collect();

            if has_csrf && !params.iter().any(|(k, _)| k == "csrf") {
                if let Some(cookie_str) = cookies.as_ref() {
                    if let Some(csrf) = cookie_str.split(';').find_map(|part| {
                        let part = part.trim();
                        part.split_once('=')
                            .filter(|(name, _)| *name == "bili_jct")
                            .map(|(_, value)| value.to_string())
                    }) {
                        params.push(("csrf".to_string(), csrf));
                    }
                }
            }
            params
        }; // 锁在此处释放

        req = req.form(&params);
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

        let query = {
            let keys = self.wbi_keys.read().expect("wbi_keys lock poisoned");
            let keys = keys
                .as_ref()
                .expect("WBI keys should be set after ensure_wbi_keys");
            wbi::encode_wbi(params, &keys.img_key, &keys.sub_key)
        };
        let url = format!("{}?{}", base_url, query);

        self.get(&url).await
    }

    /// Fetch WBI keys from nav API
    async fn ensure_wbi_keys(&self) -> Result<()> {
        if self
            .wbi_keys
            .read()
            .expect("wbi_keys lock poisoned")
            .is_some()
        {
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

            *self.wbi_keys.write().expect("wbi_keys lock poisoned") =
                Some(WbiKeys { img_key, sub_key });
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
    pub async fn search_videos(
        &self,
        keyword: &str,
        page: i32,
    ) -> Result<super::search::SearchData> {
        let url = self.build_url(BilibiliApiDomain::Main, "/x/web-interface/wbi/search/type");

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
    pub async fn get_dynamic_feed(
        &self,
        offset: Option<&str>,
        feed_type: Option<&str>,
        host_mid: Option<i64>,
    ) -> Result<super::dynamic::DynamicFeedData> {
        let mut url = format!(
            "{}/x/polymer/web-dynamic/v1/feed/all",
            BilibiliApiDomain::Main.as_str()
        );

        let mut params = Vec::new();

        if let Some(ft) = feed_type {
            params.push(format!("type={}", ft));
        }

        if let Some(off) = offset {
            params.push(format!("offset={}", off));
        }

        if let Some(mid) = host_mid {
            params.push(format!("host_mid={}", mid));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let resp: ApiResponse<super::dynamic::DynamicFeedData> = self.get(&url).await?;
        Ok(resp.data.unwrap_or(super::dynamic::DynamicFeedData {
            items: None,
            offset: None,
            has_more: Some(false),
            update_num: Some(0),
        }))
    }

    // Dynamic Detail API
    pub async fn get_dynamic_detail(
        &self,
        dynamic_id: &str,
    ) -> Result<super::dynamic::DynamicItem> {
        let url = format!(
            "{}/x/polymer/web-dynamic/v1/detail?id={}",
            BilibiliApiDomain::Main.as_str(),
            dynamic_id
        );

        #[derive(Deserialize)]
        struct DynamicDetailData {
            item: super::dynamic::DynamicItem,
        }

        let resp: ApiResponse<DynamicDetailData> = self.get(&url).await?;
        resp.data
            .map(|d| d.item)
            .ok_or_else(|| anyhow::anyhow!("No data in dynamic detail response"))
    }

    // Get following users (关注列表)
    pub async fn get_followings(
        &self,
        vmid: i64,
        ps: i32,
        pn: i32,
    ) -> Result<super::dynamic::FollowingsData> {
        let url = format!(
            "{}/x/relation/followings?vmid={}&ps={}&pn={}",
            BilibiliApiDomain::Main.as_str(),
            vmid,
            ps,
            pn
        );

        let resp: ApiResponse<super::dynamic::FollowingsData> = self.get(&url).await?;
        Ok(resp.data.unwrap_or(super::dynamic::FollowingsData {
            list: None,
            total: Some(0),
        }))
    }

    /// Get dynamic portal with frequently watched UP masters (常看UP主)
    pub async fn get_dynamic_portal(&self) -> Result<super::dynamic::PortalData> {
        let url = format!(
            "{}/x/polymer/web-dynamic/v1/portal",
            BilibiliApiDomain::Main.as_str()
        );

        let resp: ApiResponse<super::dynamic::PortalData> = self.get(&url).await?;
        Ok(resp
            .data
            .unwrap_or(super::dynamic::PortalData { up_list: None }))
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

    // Dynamic Comments API
    // type=11: 相簿（图片动态） - image/photo albums
    // type=17: 动态（纯文字动态&分享） - text dynamics and shares
    pub async fn get_dynamic_comments(
        &self,
        oid: i64,
        comment_type: i32,
        pn: i32,
    ) -> Result<super::comment::CommentData> {
        let url = format!(
            "{}/x/v2/reply?type={}&oid={}&sort=1&ps=20&pn={}",
            BilibiliApiDomain::Main.as_str(),
            comment_type,
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

    // Comment replies API
    pub async fn get_comment_replies(
        &self,
        oid: i64,
        root: i64,
        pn: i32,
    ) -> Result<super::comment::CommentData> {
        let url = format!(
            "{}/x/v2/reply/reply?type=1&oid={}&root={}&ps=20&pn={}",
            BilibiliApiDomain::Main.as_str(),
            oid,
            root,
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
    pub async fn get_related_videos(
        &self,
        bvid: &str,
    ) -> Result<Vec<super::video::RelatedVideoItem>> {
        let url = format!(
            "{}/x/web-interface/archive/related?bvid={}",
            BilibiliApiDomain::Main.as_str(),
            bvid
        );

        let resp: ApiResponse<Vec<super::video::RelatedVideoItem>> = self.get(&url).await?;
        Ok(resp.data.unwrap_or_default())
    }

    // Extended Recommendations API with pagination
    pub async fn get_recommendations_paged(
        &self,
        fresh_idx: i32,
    ) -> Result<Vec<super::recommend::VideoItem>> {
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

    pub async fn get_history(
        &self,
        max: Option<i64>,
        view_at: Option<i64>,
        business: Option<&str>,
    ) -> Result<super::history::HistoryData> {
        let mut url = format!(
            "{}/x/web-interface/history/cursor",
            BilibiliApiDomain::Main.as_str()
        );

        let mut params = Vec::new();
        params.push("ps=20".to_string());

        if let Some(m) = max {
            params.push(format!("max={}", m));
        }
        if let Some(v) = view_at {
            params.push(format!("view_at={}", v));
        }
        if let Some(b) = business {
            params.push(format!("business={}", b));
        }

        if !params.is_empty() {
            url.push('?');
            url.push_str(&params.join("&"));
        }

        let resp: ApiResponse<super::history::HistoryData> = self.get(&url).await?;
        resp.data
            .ok_or_else(|| anyhow::anyhow!("No data in history response"))
    }

    // ========== Comment Action APIs ==========

    /// Add a comment (发表评论)
    /// - `oid`: Target ID (e.g., video aid)
    /// - `comment_type`: Comment area type (1=video, 17=dynamic, etc.)
    /// - `message`: Comment content
    /// - `root`: Root comment rpid for reply (None for top-level comment)
    /// - `parent`: Parent comment rpid for reply (None for top-level comment)
    pub async fn add_comment(
        &self,
        oid: i64,
        comment_type: i32,
        message: &str,
        root: Option<i64>,
        parent: Option<i64>,
    ) -> Result<super::comment::AddCommentResponse> {
        let url = self.build_url(BilibiliApiDomain::Main, "/x/v2/reply/add");

        let mut form_data = vec![
            ("type", comment_type.to_string()),
            ("oid", oid.to_string()),
            ("message", message.to_string()),
            ("plat", "1".to_string()), // Web platform
        ];

        if let Some(r) = root {
            form_data.push(("root", r.to_string()));
        }
        if let Some(p) = parent {
            form_data.push(("parent", p.to_string()));
        }

        let resp: ApiResponse<super::comment::AddCommentResponse> =
            self.post(&url, form_data).await?;

        if resp.code != 0 {
            return Err(anyhow::anyhow!("Failed to add comment: {}", resp.message));
        }

        resp.data
            .ok_or_else(|| anyhow::anyhow!("No data in add comment response"))
    }

    /// Like or unlike a comment (点赞/取消点赞评论)
    /// - `action`: true = like, false = unlike
    pub async fn like_comment(
        &self,
        oid: i64,
        rpid: i64,
        comment_type: i32,
        action: bool,
    ) -> Result<()> {
        let url = self.build_url(BilibiliApiDomain::Main, "/x/v2/reply/action");

        let form_data = vec![
            ("type", comment_type.to_string()),
            ("oid", oid.to_string()),
            ("rpid", rpid.to_string()),
            ("action", if action { "1" } else { "0" }.to_string()),
        ];

        let resp: ApiResponse<serde_json::Value> = self.post(&url, form_data).await?;

        if resp.code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to {} comment: {}",
                if action { "like" } else { "unlike" },
                resp.message
            ));
        }

        Ok(())
    }

    /// Dislike or un-dislike a comment (点踩/取消点踩评论)
    /// - `action`: true = dislike, false = un-dislike
    pub async fn dislike_comment(
        &self,
        oid: i64,
        rpid: i64,
        comment_type: i32,
        action: bool,
    ) -> Result<()> {
        let url = self.build_url(BilibiliApiDomain::Main, "/x/v2/reply/hate");

        let form_data = vec![
            ("type", comment_type.to_string()),
            ("oid", oid.to_string()),
            ("rpid", rpid.to_string()),
            ("action", if action { "1" } else { "0" }.to_string()),
        ];

        let resp: ApiResponse<serde_json::Value> = self.post(&url, form_data).await?;

        if resp.code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to {} comment: {}",
                if action { "dislike" } else { "un-dislike" },
                resp.message
            ));
        }

        Ok(())
    }

    /// Delete a comment (删除评论)
    /// Only own comments can be deleted
    pub async fn delete_comment(&self, oid: i64, rpid: i64, comment_type: i32) -> Result<()> {
        let url = self.build_url(BilibiliApiDomain::Main, "/x/v2/reply/del");

        let form_data = vec![
            ("type", comment_type.to_string()),
            ("oid", oid.to_string()),
            ("rpid", rpid.to_string()),
        ];

        let resp: ApiResponse<serde_json::Value> = self.post(&url, form_data).await?;

        if resp.code != 0 {
            return Err(anyhow::anyhow!(
                "Failed to delete comment: {}",
                resp.message
            ));
        }

        Ok(())
    }
}

impl Default for ApiClient {
    fn default() -> Self {
        Self::new()
    }
}
