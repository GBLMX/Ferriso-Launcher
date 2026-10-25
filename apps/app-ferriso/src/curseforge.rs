//! Official CurseForge API client.
//!
//! Only `https://api.curseforge.com` is used in production. The base URL is
//! injectable for tests so fixtures can exercise parsing and authentication
//! without a live key.

use std::time::Duration;

use serde::Deserialize;

const API_BASE_URL: &str = "https://api.curseforge.com";
const MINECRAFT_GAME_ID: u32 = 432;

#[derive(Clone)]
pub struct CurseForgeClient {
    client: reqwest::Client,
    api_key: Option<String>,
    base_url: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Project {
    pub id: u32,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub slug: String,
    #[serde(default)]
    pub summary: String,
    #[serde(rename = "classId", default)]
    pub class_id: u32,
    #[serde(rename = "downloadCount", default)]
    pub downloads: u64,
    #[serde(default)]
    pub logo: Option<Asset>,
    #[serde(default)]
    pub screenshots: Vec<Screenshot>,
    #[serde(default)]
    pub authors: Vec<Author>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Asset {
    #[serde(rename = "thumbnailUrl", default)]
    pub thumbnail_url: Option<String>,
    #[serde(default)]
    pub url: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Screenshot {
    #[serde(default)]
    pub url: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Author {
    #[serde(default)]
    pub name: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct File {
    pub id: u32,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
    #[serde(rename = "downloadCount", default)]
    pub downloads: u64,
    #[serde(rename = "gameVersions", default)]
    pub game_versions: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Response<T> {
    data: T,
}

#[derive(Debug, Deserialize)]
struct SearchResponse {
    #[serde(default)]
    data: Vec<Project>,
}

#[derive(Clone, Copy, Debug)]
pub enum Sort {
    Relevance,
    Downloads,
    Updated,
}

impl Sort {
    fn field(self) -> u32 {
        match self {
            Self::Relevance => 2,
            Self::Downloads => 6,
            Self::Updated => 3,
        }
    }
}

impl CurseForgeClient {
    pub fn new(api_key: Option<String>) -> Self {
        Self::with_base_url(api_key, API_BASE_URL)
    }

    fn with_base_url(api_key: Option<String>, base_url: &str) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(Duration::from_secs(12))
                .build()
                .expect("CurseForge HTTP client configuration is valid"),
            api_key: api_key
                .map(|key| key.trim().to_string())
                .filter(|key| !key.is_empty()),
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    pub fn available(&self) -> bool {
        self.api_key.is_some()
    }

    pub fn missing_key_message() -> &'static str {
        "CurseForge is disabled: set curseforge.api_key in ~/.config/ferriso-launcher/config.toml or FERRISO_LAUNCHER__CURSEFORGE__API_KEY"
    }

    #[cfg(test)]
    pub fn with_test_base_url(api_key: Option<String>, base_url: &str) -> Self {
        Self::with_base_url(api_key, base_url)
    }

    async fn get<T: for<'de> Deserialize<'de>>(
        &self,
        path: &str,
        query: &[(&str, String)],
    ) -> Result<T, String> {
        let key = self
            .api_key
            .as_deref()
            .ok_or_else(|| Self::missing_key_message().to_string())?;
        let response = self
            .client
            .get(format!("{}{path}", self.base_url))
            .header("Accept", "application/json")
            .header("x-api-key", key)
            .query(query)
            .send()
            .await
            .map_err(|error| format!("CurseForge request failed: {error}"))?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!(
                "CurseForge API returned HTTP {}",
                status.as_u16()
            ));
        }
        response.json::<T>().await.map_err(|error| {
            format!("CurseForge response was invalid: {error}")
        })
    }

    pub async fn search(
        &self,
        query: &str,
        class_id: u32,
        sort: Sort,
    ) -> Result<Vec<Project>, String> {
        let mut params: Vec<(&str, String)> = vec![
            ("gameId", MINECRAFT_GAME_ID.to_string()),
            ("classId", class_id.to_string()),
            ("index", "0".to_string()),
            ("pageSize", "20".to_string()),
            ("sortField", sort.field().to_string()),
            ("sortOrder", "desc".to_string()),
        ];
        if !query.trim().is_empty() {
            params.push(("searchFilter", query.trim().into()));
        }
        let response: SearchResponse =
            self.get("/v1/mods/search", &params).await?;
        Ok(response.data)
    }

    pub async fn project(&self, id: u32) -> Result<Project, String> {
        let response: Response<Project> =
            self.get(&format!("/v1/mods/{id}"), &[]).await?;
        Ok(response.data)
    }

    pub async fn description(&self, id: u32) -> Result<String, String> {
        let response: Response<String> =
            self.get(&format!("/v1/mods/{id}/description"), &[]).await?;
        Ok(response.data)
    }

    pub async fn files(&self, id: u32) -> Result<Vec<File>, String> {
        let response: Response<Vec<File>> = self
            .get(
                &format!("/v1/mods/{id}/files"),
                &[("index", "0".into()), ("pageSize", "50".into())],
            )
            .await?;
        Ok(response.data)
    }
}

/// CurseForge class IDs for the content types Ferriso exposes.
pub const fn class_id(kind: super::app::ProjectKind) -> u32 {
    match kind {
        super::app::ProjectKind::Modpack => 4471,
        super::app::ProjectKind::Mod => 6,
        super::app::ProjectKind::ResourcePack => 12,
        super::app::ProjectKind::Shader => 6552,
    }
}

/// Project-page path segment used by curseforge.com.
pub const fn web_segment(kind: super::app::ProjectKind) -> &'static str {
    match kind {
        super::app::ProjectKind::Modpack => "modpacks",
        super::app::ProjectKind::Mod => "mc-mods",
        super::app::ProjectKind::ResourcePack => "texture-packs",
        super::app::ProjectKind::Shader => "shaders",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_key_is_a_configuration_error_not_a_request() {
        let client = CurseForgeClient::new(None);
        assert!(!client.available());
    }

    #[tokio::test]
    async fn missing_key_fails_before_any_network_request() {
        let client = CurseForgeClient::new(None);
        let error = client
            .search("example", 6, Sort::Relevance)
            .await
            .expect_err("missing key must fail");
        assert!(error.contains("CurseForge is disabled"));
    }

    #[test]
    fn parses_project_and_file_fixtures() {
        let project: Project = serde_json::from_str(
            r#"{"id":42,"name":"Example","slug":"example","summary":"Summary","classId":6,"downloadCount":7,"logo":{"thumbnailUrl":"https://example.invalid/icon.png"},"authors":[{"name":"Author"}],"screenshots":[{"url":"https://example.invalid/shot.png","title":"Shot"}]}"#,
        )
        .unwrap();
        assert_eq!(project.id, 42);
        assert_eq!(project.authors[0].name, "Author");
        assert_eq!(project.screenshots[0].title.as_deref(), Some("Shot"));

        let file: File = serde_json::from_str(
            r#"{"id":99,"displayName":"Example 1.0","downloadCount":3,"gameVersions":["1.21.1","Fabric"]}"#,
        )
        .unwrap();
        assert_eq!(file.id, 99);
        assert_eq!(file.game_versions, ["1.21.1", "Fabric"]);
    }

    #[tokio::test]
    async fn search_sends_the_official_key_and_parses_projects() {
        use tokio::{
            io::{AsyncReadExt, AsyncWriteExt},
            net::TcpListener,
        };

        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = vec![0_u8; 4096];
            let size = socket.read(&mut request).await.unwrap();
            let request = String::from_utf8_lossy(&request[..size]);
            assert!(request.starts_with("GET /v1/mods/search?"));
            assert!(request.contains("x-api-key: test-key"));
            let body = r#"{"data":[{"id":42,"name":"Example","slug":"example","summary":"Summary","classId":6,"downloadCount":7,"logo":{"thumbnailUrl":"https://example.invalid/icon.png"}}]}"#;
            let response = format!(
                "HTTP/1.1 200 OK\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{}",
                body.len(),
                body
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });

        let client = CurseForgeClient::with_test_base_url(
            Some("test-key".into()),
            &format!("http://{address}"),
        );
        let projects =
            client.search("example", 6, Sort::Downloads).await.unwrap();
        assert_eq!(projects.len(), 1);
        assert_eq!(projects[0].id, 42);
        assert_eq!(projects[0].name, "Example");
        server.await.unwrap();
    }
}
