//! Layered native-frontend configuration.
//!
//! The model is intentionally separate from the Vue app's persisted settings:
//! this file controls the native window and future Ferriso-specific options,
//! while `theseus::settings` remains the launcher's source of truth.

use serde::Deserialize;

use crate::update::Channel;

/// App name used by `ferriso-config` for files and environment variables.
pub const APP: &str = "ferriso-launcher";

/// Native frontend configuration.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AppConfig {
    pub gui: GuiConfig,
    pub update: UpdateConfig,
    pub curseforge: CurseForgeConfig,
}

impl AppConfig {
    /// Load `/etc/ferriso-launcher/config.toml`, the user config,
    /// `./ferriso-launcher.toml`, `$FERRISO_LAUNCHER_CONFIG`, and
    /// `FERRISO_LAUNCHER__GUI__*` environment variables.
    pub fn load() -> Result<Self, ferriso_config::ConfigError> {
        ferriso_config::Loader::new(APP).load::<Self>()
    }

    /// Clamp values that would make the window unusable.
    #[must_use]
    pub fn sanitized(mut self) -> Self {
        self.gui.width = self.gui.width.clamp(640.0, 3840.0);
        self.gui.height = self.gui.height.clamp(480.0, 2160.0);
        self
    }
}

/// iced window options.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct GuiConfig {
    pub title: String,
    pub width: f32,
    pub height: f32,
    pub application_id: String,
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            title: "Ferriso Launcher".into(),
            width: 1280.0,
            height: 820.0,
            application_id: "com.ferriso.FerrisoLauncher".into(),
        }
    }
}

impl GuiConfig {
    pub fn settings(&self) -> ferriso_gui::Settings {
        ferriso_gui::Settings {
            title: self.title.clone(),
            size: (self.width, self.height),
            exit_on_close_request: true,
            application_id: self.application_id.clone(),
        }
    }
}

/// Official CurseForge API settings.
///
/// The key is read from the local layered config or
/// `FERRISO_LAUNCHER__CURSEFORGE__API_KEY`; it is never built into the binary.
#[derive(Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct CurseForgeConfig {
    pub api_key: Option<String>,
}

impl std::fmt::Debug for CurseForgeConfig {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CurseForgeConfig")
            .field("api_key", &self.api_key().map(|_| "<configured>"))
            .finish()
    }
}

impl CurseForgeConfig {
    pub fn api_key(&self) -> Option<&str> {
        self.api_key
            .as_deref()
            .map(str::trim)
            .filter(|key| !key.is_empty())
    }
}

/// Native update channel selection.
#[derive(Clone, Debug, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct UpdateConfig {
    pub channel: Channel,
}

impl Default for UpdateConfig {
    fn default() -> Self {
        Self {
            channel: Channel::Stable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_output_never_contains_the_curseforge_key() {
        let config = CurseForgeConfig {
            api_key: Some("super-secret".into()),
        };
        let rendered = format!("{config:?}");
        assert!(!rendered.contains("super-secret"));
        assert!(rendered.contains("<configured>"));
    }
}
