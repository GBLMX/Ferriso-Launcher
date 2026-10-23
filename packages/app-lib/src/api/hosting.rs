//! Modrinth hosting (Archon) API.
//!
//! These endpoints require a signed-in Modrinth account. They are exposed to
//! the native frontends so it can list, start, and stop hosted servers without
//! embedding a web view.

use std::sync::LazyLock;

use reqwest::{
    Method,
    header::{HeaderMap, HeaderValue},
};
use serde::{Deserialize, Serialize};

use crate::{
    ErrorKind, State, state::ModrinthCredentials,
    util::fetch::fetch_advanced_with_client,
};

/// Archon requires the panel API version header. Keep it on one shared client
/// so every hosting request sends it, including requests made through the
/// shared fetch helper.
static ARCHON_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    let mut headers = HeaderMap::new();
    headers.insert("X-Panel-Version", HeaderValue::from_static("1"));
    headers.insert(
        "User-Agent",
        HeaderValue::from_str(&crate::launcher_user_agent())
            .unwrap_or_else(|_| HeaderValue::from_static("modrinth/theseus")),
    );
    reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .unwrap_or_default()
});

fn archon_base_url() -> String {
    std::env::var("MODRINTH_ARCHON_BASE_URL")
        .ok()
        .or_else(|| option_env!("MODRINTH_ARCHON_BASE_URL").map(str::to_owned))
        .unwrap_or_else(|| "https://archon.modrinth.com".to_string())
        .trim_end_matches('/')
        .to_string()
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostedServerNet {
    #[serde(default)]
    pub ip: Option<String>,
    #[serde(default)]
    pub port: u16,
    #[serde(default)]
    pub domain: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostedServerUpstream {
    #[serde(default)]
    pub kind: String,
    #[serde(default)]
    pub version_id: String,
    #[serde(default)]
    pub project_id: String,
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub struct HostedServer {
    pub server_id: String,
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub owner_id: String,
    #[serde(default = "default_net")]
    pub net: HostedServerNet,
    #[serde(default)]
    pub game: String,
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub suspension_reason: Option<String>,
    #[serde(default)]
    pub loader: Option<String>,
    #[serde(default)]
    pub loader_version: Option<String>,
    #[serde(default)]
    pub mc_version: Option<String>,
    #[serde(default)]
    pub upstream: Option<HostedServerUpstream>,
    #[serde(default)]
    pub datacenter: String,
    #[serde(default)]
    pub is_medal: bool,
    #[serde(default)]
    pub current_user_permissions: u32,
}

fn default_net() -> HostedServerNet {
    HostedServerNet {
        ip: None,
        port: 0,
        domain: String::new(),
    }
}

impl HostedServer {
    /// Player-facing address, preferring the Modrinth subdomain.
    pub fn address(&self) -> String {
        if !self.net.domain.is_empty() {
            if self.net.port == 0 || self.net.port == 25565 {
                return self.net.domain.clone();
            }
            return format!("{}:{}", self.net.domain, self.net.port);
        }
        match (&self.net.ip, self.net.port) {
            (Some(ip), 25565) => ip.clone(),
            (Some(ip), port) => format!("{ip}:{port}"),
            (None, _) => String::new(),
        }
    }

    pub fn is_operational(&self) -> bool {
        matches!(self.status.as_str(), "available" | "broken")
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum HostingPowerAction {
    Start,
    Stop,
    Restart,
    Kill,
}

impl HostingPowerAction {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Start => "Start",
            Self::Stop => "Stop",
            Self::Restart => "Restart",
            Self::Kill => "Kill",
        }
    }
}

#[derive(Deserialize)]
struct ServerListResponse {
    #[serde(default)]
    servers: Vec<HostedServer>,
}

async fn auth_header() -> crate::Result<String> {
    let state = State::get().await?;
    let credentials =
        ModrinthCredentials::get_and_refresh(&state.pool, &state.api_semaphore)
            .await?
            .ok_or_else(|| ErrorKind::NoCredentialsError.as_error())?;
    Ok(format!("Bearer {}", credentials.session))
}

#[tracing::instrument]
pub async fn list_hosted_servers(
    limit: u32,
) -> crate::Result<Vec<HostedServer>> {
    let state = State::get().await?;
    let auth = auth_header().await?;
    let url = format!(
        "{}/modrinth/v0/servers?limit={}",
        archon_base_url(),
        limit.clamp(1, 100)
    );
    let bytes = fetch_advanced_with_client(
        Method::GET,
        &url,
        None,
        None,
        Some(("Authorization", auth.as_str())),
        None,
        None,
        Some("/modrinth/v0/servers"),
        &state.api_semaphore,
        &state.pool,
        &ARCHON_CLIENT,
    )
    .await?;
    let response: ServerListResponse = serde_json::from_slice(&bytes)?;
    Ok(response.servers)
}

fn validate_server_name(name: &str) -> crate::Result<&str> {
    let name = name.trim();
    if name.is_empty() || name.chars().count() > 64 {
        return Err(ErrorKind::OtherError(
            "Server name must be between 1 and 64 characters".to_string(),
        )
        .into());
    }
    Ok(name)
}

#[tracing::instrument]
pub async fn rename_hosted_server(
    server_id: &str,
    name: &str,
) -> crate::Result<()> {
    let name = validate_server_name(name)?;
    let state = State::get().await?;
    let auth = auth_header().await?;
    let url = format!(
        "{}/modrinth/v0/servers/{}/name",
        archon_base_url(),
        server_id
    );
    fetch_advanced_with_client(
        Method::POST,
        &url,
        None,
        Some(serde_json::json!({ "name": name })),
        Some(("Authorization", auth.as_str())),
        None,
        None,
        Some("/modrinth/v0/servers/:id/name"),
        &state.api_semaphore,
        &state.pool,
        &ARCHON_CLIENT,
    )
    .await?;
    Ok(())
}

#[tracing::instrument]
pub async fn set_hosted_server_power(
    server_id: &str,
    action: HostingPowerAction,
) -> crate::Result<()> {
    let state = State::get().await?;
    let auth = auth_header().await?;
    let url = format!(
        "{}/modrinth/v0/servers/{}/power",
        archon_base_url(),
        server_id
    );
    fetch_advanced_with_client(
        Method::POST,
        &url,
        None,
        Some(serde_json::json!({ "action": action.as_str() })),
        Some(("Authorization", auth.as_str())),
        None,
        None,
        Some("/modrinth/v0/servers/:id/power"),
        &state.api_semaphore,
        &state.pool,
        &ARCHON_CLIENT,
    )
    .await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{HostedServer, HostedServerNet};

    fn server(domain: &str, ip: Option<&str>, port: u16) -> HostedServer {
        HostedServer {
            server_id: "abc".into(),
            name: "Test".into(),
            owner_id: "owner".into(),
            net: HostedServerNet {
                ip: ip.map(str::to_string),
                port,
                domain: domain.into(),
            },
            game: "Minecraft".into(),
            status: "available".into(),
            suspension_reason: None,
            loader: Some("Fabric".into()),
            loader_version: Some("0.16".into()),
            mc_version: Some("1.21.1".into()),
            upstream: None,
            datacenter: "eu".into(),
            is_medal: false,
            current_user_permissions: 0,
        }
    }

    #[test]
    fn validates_server_names() {
        assert_eq!(
            super::validate_server_name(" My Server ").unwrap(),
            "My Server"
        );
        assert!(super::validate_server_name("").is_err());
        assert!(super::validate_server_name("   ").is_err());
        assert!(super::validate_server_name(&"x".repeat(65)).is_err());
    }

    #[test]
    fn prefers_the_public_subdomain() {
        assert_eq!(
            server("play.example.com", Some("10.0.0.1"), 25565).address(),
            "play.example.com"
        );
        assert_eq!(
            server("play.example.com", Some("10.0.0.1"), 25566).address(),
            "play.example.com:25566"
        );
        assert_eq!(server("", Some("10.0.0.1"), 25565).address(), "10.0.0.1");
    }

    #[test]
    fn parses_a_server_list_response() {
        let json = r#"{
            "servers": [{
                "server_id": "abc",
                "name": "Test",
                "owner_id": "owner",
                "net": {"ip": "10.0.0.1", "port": 25565, "domain": "play.example.com"},
                "game": "Minecraft",
                "status": "available",
                "loader": "Fabric",
                "loader_version": "0.16",
                "mc_version": "1.21.1",
                "datacenter": "eu",
                "is_medal": false,
                "current_user_permissions": 7
            }]
        }"#;
        let parsed: super::ServerListResponse =
            serde_json::from_str(json).unwrap();
        assert_eq!(parsed.servers.len(), 1);
        assert_eq!(parsed.servers[0].address(), "play.example.com");
        assert!(parsed.servers[0].is_operational());
    }
}
