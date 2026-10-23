//! Actor-owned model for Ferriso Launcher.
//!
//! This file contains no iced types. The renderer receives immutable snapshots
//! and sends semantic messages; all launcher work happens in command futures
//! owned by the Ferriso actor.

use std::{
    collections::VecDeque,
    future::Future,
    path::PathBuf,
    pin::Pin,
    sync::{Arc, Mutex, OnceLock},
    time::Duration,
};

use crate::skin3d;
use crate::update::{self, UpdateInfo, VERSION};
use base64::Engine as _;
use bytes::Bytes;
use either::Either;
use ferriso_core::{App, Command};
use image::{
    ExtendedColorType, ImageEncoder, Rgba, RgbaImage, codecs::png::PngEncoder,
};
use serde_json::Value;
use theseus::{
    State, cache,
    data::{
        ContentType, Credentials, EditInstance, InstanceLaunchOverridesPatch,
        InstanceLink, InstanceMetadata, MemorySettings, ModLoader,
        OnboardingChecklist, ProcessMetadata, ResolutionPreferences, Settings,
        Theme,
    },
    friends,
    install::{self, InstallJobSnapshot, InstallJobStatus},
    instance::{self, InstallProjectWithDependenciesRequest, QuickPlayType},
    jre, logs, metadata, minecraft_auth,
    minecraft_skins::{self, Skin},
    mr_auth, onboarding_checklist,
    pack::{import::ImportLauncherType, install_from::CreatePackLocation},
    process, settings as settings_api, users, worlds,
};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use uuid::Uuid;

/// The same identifier used by the Tauri app, so the native frontend reads the
/// existing database, instances, and account store.
pub const APP_IDENTIFIER: &str = "ModrinthApp";

/// Backend events are pushed by theseus and drained on the animation tick, so
/// progress bars and process state do not depend on one-second polling alone.
static EVENT_QUEUE: OnceLock<Mutex<VecDeque<theseus::AppEvent>>> =
    OnceLock::new();
static EVENT_LISTENER: OnceLock<()> = OnceLock::new();

fn start_event_listener() {
    EVENT_LISTENER.get_or_init(|| {
        let mut receiver = theseus::EventState::get().subscribe();
        tokio::spawn(async move {
            loop {
                match receiver.recv().await {
                    Ok(event) => {
                        let queue = EVENT_QUEUE
                            .get_or_init(|| Mutex::new(VecDeque::new()));
                        if let Ok(mut queue) = queue.lock() {
                            if queue.len() >= 2048 {
                                queue.pop_front();
                            }
                            queue.push_back(event);
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(
                        skipped,
                    )) => {
                        eprintln!(
                            "native event queue lagged; dropped {skipped} events"
                        );
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        break;
                    }
                }
            }
        });
    });
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Home,
    Browse,
    Project,
    Library,
    Instance,
    Account,
    Skins,
    Settings,
}

impl Page {
    pub const NAV: [Self; 6] = [
        Self::Home,
        Self::Browse,
        Self::Library,
        Self::Account,
        Self::Skins,
        Self::Settings,
    ];

    pub fn from_arg(value: &str) -> Option<Self> {
        match value {
            "home" => Some(Self::Home),
            "browse" => Some(Self::Browse),
            "project" => Some(Self::Project),
            "library" => Some(Self::Library),
            "account" => Some(Self::Account),
            "skins" => Some(Self::Skins),
            "settings" => Some(Self::Settings),
            _ => None,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Home => "Home",
            Self::Browse => "Browse",
            Self::Project => "Project",
            Self::Library => "Library",
            Self::Instance => "Instance",
            Self::Account => "Account",
            Self::Skins => "Skins",
            Self::Settings => "Settings",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectKind {
    Modpack,
    Mod,
    ResourcePack,
    Shader,
}

impl ProjectKind {
    pub const ALL: [Self; 4] =
        [Self::Modpack, Self::Mod, Self::ResourcePack, Self::Shader];

    pub fn label(self) -> &'static str {
        match self {
            Self::Modpack => "Modpacks",
            Self::Mod => "Mods",
            Self::ResourcePack => "Resource packs",
            Self::Shader => "Shaders",
        }
    }

    pub fn facet(self) -> &'static str {
        match self {
            Self::Modpack => "modpack",
            Self::Mod => "mod",
            Self::ResourcePack => "resourcepack",
            Self::Shader => "shader",
        }
    }

    fn from_hit(value: &Value) -> Option<Self> {
        let types = value.get("project_types")?.as_array()?;
        types.iter().find_map(|kind| match kind.as_str()? {
            "modpack" => Some(Self::Modpack),
            "mod" => Some(Self::Mod),
            "resourcepack" => Some(Self::ResourcePack),
            "shader" => Some(Self::Shader),
            _ => None,
        })
    }

    fn from_type(value: &str) -> Option<Self> {
        match value {
            "modpack" => Some(Self::Modpack),
            "mod" => Some(Self::Mod),
            "resourcepack" => Some(Self::ResourcePack),
            "shader" => Some(Self::Shader),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstanceView {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon_path: Option<PathBuf>,
    pub last_played: Option<String>,
    pub playtime_minutes: u64,
    pub game_version: String,
    pub loader: String,
    pub loader_version: Option<String>,
    pub memory_mb: Option<u32>,
    pub java_path: Option<String>,
    pub launch_args: Vec<String>,
    pub group_ids: Vec<String>,
    pub shared: Option<SharedLinkView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedLinkView {
    pub id: String,
    pub role: String,
    pub latest_version: Option<i32>,
    pub applied_version: Option<i32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedUserView {
    pub id: String,
    pub join_type: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedInviteView {
    pub id: String,
    pub expiration: String,
    pub max_uses: i32,
    pub uses: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedView {
    pub loading: bool,
    pub error: Option<String>,
    pub users: Vec<SharedUserView>,
    pub invites: Vec<SharedInviteView>,
    pub new_invite: Option<String>,
    pub user_input: String,
    pub busy: bool,
}

#[derive(Clone, Debug)]
pub struct SharedData {
    pub users: Vec<SharedUserView>,
    pub invites: Vec<SharedInviteView>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AccountView {
    pub id: String,
    pub name: String,
    pub active: bool,
}

#[derive(Clone, Debug)]
pub struct ModrinthAccountView {
    pub user_id: String,
    pub username: Option<String>,
}

#[derive(Clone, Debug)]
pub struct FriendView {
    pub user_id: String,
    pub accepted: bool,
}

#[derive(Clone, Debug)]
pub struct ProjectView {
    pub id: String,
    pub slug: String,
    pub title: String,
    pub summary: String,
    pub kind: ProjectKind,
    pub icon_url: Option<String>,
    pub icon: Option<Bytes>,
    pub downloads: u64,
    pub author: String,
    pub version_id: Option<String>,
}

impl ProjectView {
    fn from_hit(value: &Value) -> Option<Self> {
        let id = value.get("project_id")?.as_str()?.to_string();
        let slug = value
            .get("slug")
            .and_then(Value::as_str)
            .unwrap_or(&id)
            .to_string();
        let title = value
            .get("name")
            .or_else(|| value.get("title"))
            .and_then(Value::as_str)?
            .to_string();
        let summary = value
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        let kind = ProjectKind::from_hit(value)?;
        let icon_url = value
            .get("icon_url")
            .and_then(Value::as_str)
            .map(str::to_string);
        let downloads =
            value.get("downloads").and_then(Value::as_u64).unwrap_or(0);
        let author = value
            .get("author")
            .and_then(Value::as_str)
            .unwrap_or("Unknown")
            .to_string();
        let version_id = value
            .get("version_id")
            .and_then(Value::as_str)
            .map(str::to_string);

        Some(Self {
            id,
            slug,
            title,
            summary,
            kind,
            icon_url,
            icon: None,
            downloads,
            author,
            version_id,
        })
    }
}

#[derive(Clone, Debug)]
pub struct GalleryView {
    pub title: Option<String>,
    pub description: Option<String>,
    pub image: Option<Bytes>,
}

#[derive(Clone, Debug)]
pub struct ProjectPage {
    pub id: String,
    pub loading: bool,
    pub error: Option<String>,
    pub project: Option<ProjectView>,
    pub body: String,
    pub gallery: Vec<GalleryView>,
    pub versions: Vec<VersionChoice>,
    pub selected_version: Option<usize>,
    pub selected_instance: Option<String>,
    pub installing: bool,
    pub install_error: Option<String>,
    pub install_result: Option<String>,
}

impl ProjectPage {
    fn loading(id: String, instance: Option<String>) -> Self {
        Self {
            id,
            loading: true,
            error: None,
            project: None,
            body: String::new(),
            gallery: Vec::new(),
            versions: Vec::new(),
            selected_version: None,
            selected_instance: instance,
            installing: false,
            install_error: None,
            install_result: None,
        }
    }

    pub fn selected_version(&self) -> Option<&VersionChoice> {
        self.selected_version
            .and_then(|index| self.versions.get(index))
    }

    pub fn selected_project(&self) -> Option<&ProjectView> {
        self.project.as_ref()
    }
}

#[derive(Clone, Debug)]
pub struct BrowseView {
    pub query: String,
    pub kind: ProjectKind,
    pub results: Vec<ProjectView>,
    pub loading: bool,
    pub loaded: bool,
    pub error: Option<String>,
}

impl Default for BrowseView {
    fn default() -> Self {
        Self {
            query: String::new(),
            kind: ProjectKind::Modpack,
            results: Vec::new(),
            loading: false,
            loaded: false,
            error: None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct CreateView {
    pub open: bool,
    pub name: String,
    pub game_version: String,
    pub loader: usize,
    pub busy: bool,
    pub error: Option<String>,
}

impl CreateView {
    const LOADERS: [ModLoader; 5] = [
        ModLoader::Vanilla,
        ModLoader::Fabric,
        ModLoader::Forge,
        ModLoader::NeoForge,
        ModLoader::Quilt,
    ];

    fn loader(&self) -> ModLoader {
        Self::LOADERS[self.loader.min(Self::LOADERS.len() - 1)]
    }
}

impl Default for CreateView {
    fn default() -> Self {
        Self {
            open: false,
            name: String::new(),
            game_version: "1.21.10".into(),
            loader: 0,
            busy: false,
            error: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DetailTab {
    Overview,
    Content,
    Worlds,
    Logs,
}

impl DetailTab {
    pub const ALL: [Self; 4] =
        [Self::Overview, Self::Content, Self::Worlds, Self::Logs];

    pub fn label(self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Content => "Content",
            Self::Worlds => "Worlds",
            Self::Logs => "Logs",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContentRow {
    pub title: String,
    pub file_name: String,
    pub file_path: String,
    pub project_id: Option<String>,
    pub version_id: Option<String>,
    pub version: Option<String>,
    pub enabled: bool,
    pub locked: bool,
    pub has_update: bool,
    pub update_version_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldArt {
    Path(PathBuf),
    Bytes(Bytes),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionChoice {
    pub id: String,
    pub number: String,
    pub game_versions: Vec<String>,
    pub loaders: Vec<String>,
    pub downloads: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VersionPicker {
    pub path: String,
    pub title: String,
    pub loading: bool,
    pub error: Option<String>,
    pub versions: Vec<VersionChoice>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldRow {
    pub name: String,
    pub kind: String,
    pub detail: String,
    pub last_played: Option<String>,
    pub icon: Option<WorldArt>,
    pub icon_key: String,
    pub server_index: Option<usize>,
    pub pack_status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum WorldFormKind {
    AddServer,
    EditServer { index: usize },
    RenameWorld { path: String },
    DeleteWorld { path: String, name: String },
    DeleteServer { index: usize, name: String },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorldForm {
    pub kind: WorldFormKind,
    pub name: String,
    pub address: String,
    pub pack_status: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogRow {
    pub filename: String,
    pub kind: String,
    pub age_seconds: u64,
    pub crash: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogView {
    pub filename: String,
    pub loading: bool,
    pub content: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GroupForm {
    pub id: Option<String>,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LibraryForm {
    Import {
        base_path: String,
        folder: String,
        launcher: ImportLauncherType,
    },
    Export {
        path: String,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JreChoice {
    pub path: String,
    pub version: String,
    pub parsed_version: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InstanceEditForm {
    pub name: String,
    pub memory: String,
    pub java_path: String,
    pub args: String,
    pub jres: Vec<JreChoice>,
    pub jres_loading: bool,
    pub jres_error: Option<String>,
    pub required_java: Option<u32>,
    pub required_java_loading: bool,
    pub java_installing: bool,
    pub icon_path: String,
    pub group_ids: Vec<String>,
}

impl InstanceEditForm {
    fn from_instance(instance: &InstanceView) -> Self {
        Self {
            name: instance.name.clone(),
            memory: instance
                .memory_mb
                .map(|value| value.to_string())
                .unwrap_or_default(),
            java_path: instance.java_path.clone().unwrap_or_default(),
            args: instance.launch_args.join(" "),
            jres: Vec::new(),
            jres_loading: false,
            jres_error: None,
            required_java: None,
            required_java_loading: true,
            java_installing: false,
            icon_path: String::new(),
            group_ids: instance.group_ids.clone(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct DetailData {
    pub content: Vec<ContentRow>,
    pub worlds: Vec<WorldRow>,
    pub logs: Vec<LogRow>,
}

#[derive(Clone, Debug)]
pub struct DetailView {
    pub id: String,
    pub loading: bool,
    pub tab: DetailTab,
    pub error: Option<String>,
    pub content: Vec<ContentRow>,
    pub worlds: Vec<WorldRow>,
    pub logs: Vec<LogRow>,
    pub versions: Option<VersionPicker>,
    pub log_view: Option<LogView>,
    pub world_form: Option<WorldForm>,
}

#[derive(Clone, Debug)]
pub struct JobView {
    pub id: String,
    pub kind: String,
    pub status: String,
    pub phase: String,
    pub progress: Option<f32>,
    pub error: Option<String>,
    pub active: bool,
    pub can_cancel: bool,
    pub can_retry: bool,
    pub target: String,
    pub created: String,
    pub finished: Option<String>,
    pub display: Option<String>,
    pub rollback_error: Option<String>,
}

impl From<InstallJobSnapshot> for JobView {
    fn from(job: InstallJobSnapshot) -> Self {
        let progress = job.progress.as_ref().and_then(|progress| {
            (progress.total > 0)
                .then(|| progress.current as f32 / progress.total as f32)
        });

        let can_cancel = job.status == InstallJobStatus::Queued;
        let can_retry = matches!(
            job.status,
            InstallJobStatus::Failed | InstallJobStatus::Interrupted
        );

        Self {
            id: job.job_id,
            kind: format!("{:?}", job.kind),
            status: job.status.as_str().to_string(),
            phase: format!("{:?}", job.phase),
            progress,
            error: job.error.map(|error| error.message),
            active: !job.status.is_finished(),
            can_cancel,
            can_retry,
            target: format!("{:?}", job.target),
            created: job.created,
            finished: job.finished,
            display: job.display.map(|display| format!("{:?}", display)),
            rollback_error: job.rollback_error.map(|error| error.message),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProcessView {
    pub uuid: String,
    pub instance_id: String,
    pub instance_name: String,
    pub started_at: String,
}

impl From<ProcessMetadata> for ProcessView {
    fn from(process: ProcessMetadata) -> Self {
        Self {
            uuid: process.uuid.to_string(),
            instance_id: process.instance_id,
            instance_name: process.instance_name,
            started_at: process.start_time.format("%Y-%m-%d %H:%M").to_string(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct SkinView {
    pub index: usize,
    pub name: String,
    pub variant: String,
    pub source: String,
    pub equipped: bool,
    pub avatar: Option<Bytes>,
    pub cape_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CapeView {
    pub index: usize,
    pub name: String,
    pub equipped: bool,
    pub preview: Option<Bytes>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SkinTab {
    Saved,
    Defaults,
}

#[derive(Clone, Debug)]
pub struct SkinAsset {
    pub skin: Skin,
    pub texture: Option<Bytes>,
    pub avatar: Option<Bytes>,
    pub cape_name: Option<String>,
}

#[derive(Clone, Debug)]
pub struct CapeAsset {
    pub cape: minecraft_skins::Cape,
    pub texture: Option<Bytes>,
    pub preview: Option<Bytes>,
}

#[derive(Clone, Debug)]
pub struct SkinsData {
    pub skins: Vec<SkinAsset>,
    pub capes: Vec<CapeAsset>,
}

#[derive(Clone, Copy, Debug)]
pub enum SettingChange {
    Downloads(i32),
    Writes(i32),
    Memory(i32),
    SetTheme(Theme),
    SetLocale(&'static str),
    ToggleTelemetry,
    ToggleDiscordRpc,
    ToggleAdvancedRendering,
    ToggleNativeDecorations,
    ToggleForceFullscreen,
    ToggleHideOnProcessStart,
    ToggleSidebar,
}

#[derive(Clone, Debug)]
pub(crate) struct BootData {
    pub instances: Vec<InstanceView>,
    pub accounts: Vec<AccountView>,
    pub settings: Settings,
    pub jobs: Vec<JobView>,
    pub processes: Vec<ProcessView>,
    pub groups: Vec<instance::InstanceGroup>,
    pub modrinth_account: Option<ModrinthAccountView>,
    pub friends: Vec<FriendView>,
    pub checklist: OnboardingChecklist,
    pub latest_release: Option<String>,
}

type FinishLogin = Box<
    dyn FnOnce(
            String,
        )
            -> Pin<Box<dyn Future<Output = Result<String, String>> + Send>>
        + Send
        + 'static,
>;

#[derive(Clone)]
pub(crate) struct LoginStarted {
    url: String,
    opened: bool,
    finish: Arc<Mutex<Option<FinishLogin>>>,
}

#[derive(Clone, Debug)]
pub struct ViewModel {
    pub page: Page,
    pub booted: bool,
    pub error: Option<String>,
    pub status: String,
    pub instances: Vec<InstanceView>,
    pub selected: Option<String>,
    pub accounts: Vec<AccountView>,
    pub player_head: Option<Bytes>,
    pub player_skin: Option<Bytes>,
    pub settings: Option<Settings>,
    pub login_url: Option<String>,
    pub login_code: String,
    pub login_opened: bool,
    pub launching: Option<String>,
    pub browse: BrowseView,
    pub project: Option<ProjectPage>,
    pub create: CreateView,
    pub detail: Option<DetailView>,
    pub jobs: Vec<JobView>,
    pub skins: Vec<SkinView>,
    pub capes: Vec<CapeView>,
    pub skins_loaded: bool,
    pub system_light: bool,
    pub skin_tab: SkinTab,
    pub selected_skin: Option<usize>,
    pub selected_cape: Option<usize>,
    pub skin_preview: Option<Bytes>,
    pub skin_preview_rgba: Option<Bytes>,
    pub skin_preview_pending: bool,
    pub update: Option<UpdateInfo>,
    pub version: &'static str,
    pub latest_release: Option<String>,
    pub phase: f32,
    pub processes: Vec<ProcessView>,
    pub groups: Vec<instance::InstanceGroup>,
    pub selected_group: Option<String>,
    pub groups_open: bool,
    pub group_form: Option<GroupForm>,
    pub modrinth_account: Option<ModrinthAccountView>,
    pub modrinth_login: bool,
    pub friends: Vec<FriendView>,
    pub checklist: Option<OnboardingChecklist>,
    pub friend_input: String,
    pub shared_open: bool,
    pub shared: Option<SharedView>,
    pub join_invite: String,
    pub join_preview: Option<instance::SharedInstanceInviteInstallPreview>,
    pub join_loading: bool,
    pub expanded_job: Option<String>,
    pub library_form: Option<LibraryForm>,
    pub edit: Option<InstanceEditForm>,
    pub delete_confirm: bool,
}

#[derive(Clone)]
pub enum Message {
    Booted(Result<BootData, String>),
    Refresh,
    Refreshed(Result<BootData, String>),
    Navigate(Page),
    Back,
    SelectInstance(String),
    Launch(String),
    Launched(Result<ProcessView, String>),
    BeginLogin,
    LoginStarted(Result<LoginStarted, String>),
    OpenLoginUrl,
    LoginCodeChanged(String),
    FinishLogin,
    LoggedIn(Result<String, String>),
    CancelLogin,
    SetAccount(String),
    AccountSet(Result<(), String>),
    RemoveAccount(String),
    AccountRemoved(Result<(), String>),
    BrowseQueryChanged(String),
    BrowseKind(ProjectKind),
    BrowseSearch,
    BrowseResults(Result<Vec<ProjectView>, String>),
    OpenProject(String),
    ProjectLoaded(String, Result<ProjectPage, String>),
    SelectProjectVersion(usize),
    SelectProjectInstance(String),
    InstallProjectVersion,
    ProjectVersionInstalled(Result<String, String>),
    InstallProject(ProjectView),
    InstallStarted(Result<InstallJobSnapshot, String>),
    ModInstalled(Result<String, String>),
    OpenUrl(String),
    ToggleCreate,
    OpenCreate,
    CreateNameChanged(String),
    CreateVersionChanged(String),
    CreateLoaderChanged(usize),
    CreateInstance,
    CreateStarted(Result<InstallJobSnapshot, String>),
    DetailTab(DetailTab),
    DetailLoaded(String, Result<DetailData, String>),
    ToggleContent {
        path: String,
        enabled: bool,
    },
    ContentToggled(Result<(), String>),
    RemoveContent(String),
    ContentRemoved(Result<(), String>),
    PollJobs,
    JobsLoaded(Result<Vec<JobView>, String>),
    LoadSkins,
    SkinsLoaded(Result<SkinsData, String>),
    SystemThemeChanged(bool),
    SkinTab(SkinTab),
    SkinDragStart,
    SkinDragMove(f32, f32),
    SkinDragEnd,
    SkinResetView,
    SelectSkin(usize),
    SelectCape(Option<usize>),
    ApplySelectedSkin,
    ResetSkinSelection,
    SkinEquipped(Result<(), String>),
    AccountAppearanceLoaded(Option<PlayerAppearance>),
    SettingChanged(SettingChange),
    SettingsSaved(Result<(), String>),
    CancelJob(String),
    RetryJob(String),
    DismissJob(String),
    JobAction(Result<(), String>),
    BeginRename,
    RenameChanged(String),
    EditMemory(String),
    EditJava(String),
    EditArgs(String),
    EditIconPath(String),
    RemoveInstanceIcon,
    IconUpdated(Result<(), String>),
    DetectJava,
    JavaDetected(Result<Vec<JreChoice>, String>),
    UseJava(String),
    RequiredJavaLoaded(Result<Option<u32>, String>),
    InstallJava(u32),
    JavaInstalled(Result<String, String>),
    CancelRename,
    SaveRename,
    Renamed(Result<(), String>),
    AskDelete,
    CancelDelete,
    DeleteInstance,
    Deleted(Result<(), String>),
    UpdateAllContent,
    UpdateContent(String),
    ContentUpdated(Result<String, String>),
    ToggleLock {
        path: String,
        locked: bool,
    },
    LockChanged(Result<(), String>),
    ShowVersions {
        path: String,
        project_id: String,
    },
    VersionsLoaded {
        path: String,
        result: Result<Vec<VersionChoice>, String>,
    },
    SwitchVersion {
        path: String,
        version_id: String,
    },
    VersionSwitched(Result<String, String>),
    CloseVersions,
    ShowLog {
        filename: String,
        crash: bool,
    },
    LogLoaded {
        filename: String,
        result: Result<String, String>,
    },
    CloseLog,
    DeleteLog {
        filename: String,
        crash: bool,
    },
    DeleteAllLogs,
    LogDeleted(Result<(), String>),
    AddServer,
    EditServer(usize),
    RenameWorld(String),
    DeleteWorld {
        path: String,
        name: String,
    },
    DeleteServer {
        index: usize,
        name: String,
    },
    WorldFormName(String),
    WorldFormAddress(String),
    CyclePackStatus,
    CancelWorldForm,
    SaveWorldForm,
    WorldSaved(Result<(), String>),
    BackupWorld {
        path: String,
        name: String,
    },
    WorldBackupDone(Result<u64, String>),
    PollProcesses,
    ProcessesLoaded(Result<Vec<ProcessView>, String>),
    KillProcess(String),
    ProcessKilled(Result<(), String>),
    ToggleJobDetails(String),
    BeginImport,
    BeginExport,
    ImportBasePath(String),
    ImportFolder(String),
    CycleImportLauncher,
    ExportPath(String),
    CancelLibraryForm,
    StartImport,
    StartExport,
    ImportStarted(Result<InstallJobSnapshot, String>),
    ExportDone(Result<String, String>),
    DuplicateInstance(String),
    ToggleGroups,
    BeginGroupCreate,
    BeginGroupRename(String),
    GroupNameChanged(String),
    SaveGroup,
    CancelGroupForm,
    GroupSaved(Result<(), String>),
    DeleteGroup(String),
    GroupDeleted(Result<String, String>),
    FilterGroup(Option<String>),
    ToggleGroupMembership(String),
    BeginModrinthLogin,
    ModrinthLoginFinished(Result<ModrinthAccountView, String>),
    LogoutModrinth,
    ModrinthLoggedOut(Result<(), String>),
    FriendIdChanged(String),
    AddFriend,
    FriendAdded(Result<(), String>),
    RemoveFriend(String),
    FriendRemoved(Result<(), String>),
    ToggleShared,
    SharedLoaded(Result<SharedData, String>),
    PublishShared,
    SharedPublished(Result<(), String>),
    UnpublishShared,
    SharedUnpublished(Result<(), String>),
    UnlinkShared,
    SharedUnlinked(Result<(), String>),
    SharedUserInputChanged(String),
    InviteSharedUsers,
    SharedUsersChanged(Result<(), String>),
    RemoveSharedUser(String),
    CreateSharedInvite,
    SharedInviteCreated(Result<String, String>),
    RevokeSharedInvite(String),
    SharedInviteRevoked(Result<(), String>),
    UpdateShared,
    SharedUpdated(Result<InstallJobSnapshot, String>),
    JoinInviteChanged(String),
    PreviewJoinInvite,
    JoinPreviewLoaded(
        Result<instance::SharedInstanceInviteInstallPreview, String>,
    ),
    InstallSharedInstance,
    SharedInstalled(Result<InstallJobSnapshot, String>),
    CheckUpdates,
    UpdateChecked(Result<Option<UpdateInfo>, String>),
    DismissUpdate,
    AnimationTick,
}

pub struct FerrisoApp {
    page: Page,
    booted: bool,
    error: Option<String>,
    status: String,
    instances: Vec<InstanceView>,
    selected: Option<String>,
    accounts: Vec<AccountView>,
    player_head: Option<Bytes>,
    player_skin: Option<Bytes>,
    settings: Option<Settings>,
    login_url: Option<String>,
    login_code: String,
    login_opened: bool,
    finish_login: Option<FinishLogin>,
    launching: Option<String>,
    browse: BrowseView,
    project: Option<ProjectPage>,
    create: CreateView,
    detail: Option<DetailView>,
    jobs: Vec<JobView>,
    skins: Vec<SkinAsset>,
    capes: Vec<CapeAsset>,
    skins_loaded: bool,
    system_light: bool,
    skin_tab: SkinTab,
    selected_skin: Option<usize>,
    selected_cape: Option<usize>,
    skin_preview: Option<Bytes>,
    skin_preview_rgba: Option<Bytes>,
    skin_preview_pending: bool,
    update: Option<UpdateInfo>,
    dismissed_update: Option<String>,
    skin_yaw: f32,
    skin_pitch: f32,
    skin_dragging: bool,
    skin_drag_last: Option<(f32, f32)>,
    latest_release: Option<String>,
    phase: f32,
    processes: Vec<ProcessView>,
    groups: Vec<instance::InstanceGroup>,
    selected_group: Option<String>,
    groups_open: bool,
    group_form: Option<GroupForm>,
    modrinth_account: Option<ModrinthAccountView>,
    modrinth_login: bool,
    friends: Vec<FriendView>,
    checklist: Option<OnboardingChecklist>,
    friend_input: String,
    shared_open: bool,
    shared: Option<SharedView>,
    join_invite: String,
    join_preview: Option<instance::SharedInstanceInviteInstallPreview>,
    join_loading: bool,
    expanded_job: Option<String>,
    library_form: Option<LibraryForm>,
    edit: Option<InstanceEditForm>,
    delete_confirm: bool,
}

impl FerrisoApp {
    pub fn new() -> Self {
        Self {
            page: Page::Home,
            booted: false,
            error: None,
            status: "Starting Ferriso Launcher…".into(),
            instances: Vec::new(),
            selected: None,
            accounts: Vec::new(),
            player_head: None,
            player_skin: None,
            settings: None,
            login_url: None,
            login_code: String::new(),
            login_opened: false,
            finish_login: None,
            launching: None,
            browse: BrowseView::default(),
            project: None,
            create: CreateView::default(),
            detail: None,
            jobs: Vec::new(),
            skins: Vec::new(),
            capes: Vec::new(),
            skins_loaded: false,
            system_light: false,
            skin_tab: SkinTab::Saved,
            selected_skin: None,
            selected_cape: None,
            skin_preview: None,
            skin_preview_rgba: None,
            skin_preview_pending: false,
            update: None,
            dismissed_update: None,
            skin_yaw: 0.5,
            skin_pitch: 0.12,
            skin_dragging: false,
            skin_drag_last: None,
            latest_release: None,
            phase: 0.0,
            processes: Vec::new(),
            groups: Vec::new(),
            selected_group: None,
            groups_open: false,
            group_form: None,
            modrinth_account: None,
            modrinth_login: false,
            friends: Vec::new(),
            checklist: None,
            friend_input: String::new(),
            shared_open: false,
            shared: None,
            join_invite: String::new(),
            join_preview: None,
            join_loading: false,
            expanded_job: None,
            library_form: None,
            edit: None,
            delete_confirm: false,
        }
    }

    pub fn with_page(page: Page) -> Self {
        let mut app = Self::new();
        app.page = page;
        app
    }

    pub fn with_browse_query(query: impl Into<String>) -> Self {
        let mut app = Self::new();
        app.page = Page::Browse;
        app.browse.query = query.into();
        app
    }

    pub fn with_project(identifier: impl Into<String>) -> Self {
        let mut app = Self::with_page(Page::Project);
        let id = identifier.into();
        app.project = Some(ProjectPage::loading(id.clone(), None));
        app.status = "Loading project…".into();
        app
    }

    pub fn with_instance(identifier: impl Into<String>) -> Self {
        let mut app = Self::with_page(Page::Instance);
        app.selected = Some(identifier.into());
        app
    }

    fn apply_snapshot(&mut self, data: BootData) {
        self.instances = data.instances;
        self.accounts = data.accounts;
        self.settings = Some(data.settings);
        self.jobs = data.jobs;
        self.processes = data.processes;
        self.groups = data.groups;
        if let Some(mut account) = data.modrinth_account {
            if account.username.is_none() {
                account.username = self
                    .modrinth_account
                    .as_ref()
                    .filter(|current| current.user_id == account.user_id)
                    .and_then(|current| current.username.clone());
            }
            self.modrinth_account = Some(account);
        } else {
            self.modrinth_account = None;
        }
        self.friends = data.friends;
        self.checklist = Some(data.checklist);
        if data.latest_release.is_some() {
            self.latest_release = data.latest_release.clone();
        }
        if let (Some(release), create) =
            (&self.latest_release, &mut self.create)
            && create.game_version.is_empty()
        {
            create.game_version = release.clone();
        }
        self.booted = true;
        self.error = None;
        if self.selected.is_none() {
            self.selected =
                self.instances.first().map(|instance| instance.id.clone());
        }
    }

    fn view_model(&self) -> ViewModel {
        ViewModel {
            page: self.page,
            booted: self.booted,
            error: self.error.clone(),
            status: self.status.clone(),
            instances: self.instances.clone(),
            selected: self.selected.clone(),
            accounts: self.accounts.clone(),
            player_head: self.player_head.clone(),
            player_skin: self.player_skin.clone(),
            settings: self.settings.clone(),
            login_url: self.login_url.clone(),
            login_code: self.login_code.clone(),
            login_opened: self.login_opened,
            launching: self.launching.clone(),
            browse: self.browse.clone(),
            project: self.project.clone(),
            create: self.create.clone(),
            detail: self.detail.clone(),
            jobs: self.jobs.clone(),
            skins: self.skins_view(),
            capes: self.capes_view(),
            skins_loaded: self.skins_loaded,
            system_light: self.system_light,
            skin_tab: self.skin_tab,
            selected_skin: self.selected_skin,
            selected_cape: self.selected_cape,
            skin_preview: self.skin_preview.clone(),
            skin_preview_rgba: self.skin_preview_rgba.clone(),
            skin_preview_pending: self.skin_preview_pending,
            update: self.update.clone(),
            version: VERSION,
            latest_release: self.latest_release.clone(),
            phase: self.phase,
            processes: self.processes.clone(),
            groups: self.groups.clone(),
            selected_group: self.selected_group.clone(),
            groups_open: self.groups_open,
            group_form: self.group_form.clone(),
            modrinth_account: self.modrinth_account.clone(),
            modrinth_login: self.modrinth_login,
            friends: self.friends.clone(),
            checklist: self.checklist.clone(),
            friend_input: self.friend_input.clone(),
            shared_open: self.shared_open,
            shared: self.shared.clone(),
            join_invite: self.join_invite.clone(),
            join_preview: self.join_preview.clone(),
            join_loading: self.join_loading,
            expanded_job: self.expanded_job.clone(),
            library_form: self.library_form.clone(),
            edit: self.edit.clone(),
            delete_confirm: self.delete_confirm,
        }
    }

    fn reset_skin_selection(&mut self) {
        self.selected_skin = self
            .skins
            .iter()
            .position(|asset| asset.skin.is_equipped)
            .or_else(|| (!self.skins.is_empty()).then_some(0));
        self.selected_cape = self
            .selected_skin
            .and_then(|index| self.skins.get(index))
            .and_then(|asset| asset.skin.cape_id)
            .and_then(|id| {
                self.capes.iter().position(|cape| cape.cape.id == id)
            });
        self.refresh_skin_preview();
    }

    fn refresh_skin_preview(&mut self) {
        let skin_index = self.selected_skin.or_else(|| {
            self.skins.iter().position(|asset| asset.skin.is_equipped)
        });
        let Some(asset) = skin_index.and_then(|index| self.skins.get(index))
        else {
            self.skin_preview = None;
            self.skin_preview_rgba = None;
            self.skin_preview_pending = false;
            return;
        };
        let cape_texture = self
            .selected_cape
            .and_then(|index| self.capes.get(index))
            .and_then(|cape| cape.texture.as_deref());
        let slim = matches!(
            asset.skin.variant,
            minecraft_skins::MinecraftSkinVariant::Slim
        );
        self.skin_preview_rgba = asset.texture.as_deref().and_then(|texture| {
            skin3d::render_skin(
                texture,
                cape_texture,
                slim,
                self.skin_yaw,
                self.skin_pitch,
            )
            .map(|image| Bytes::from(image.into_raw()))
        });
        self.skin_preview = if self.skin_preview_rgba.is_none() {
            asset.texture.as_deref().and_then(|texture| {
                render_avatar_png(texture, cape_texture, slim)
            })
        } else {
            None
        };
        let equipped_cape = self
            .skins
            .iter()
            .find(|asset| asset.skin.is_equipped)
            .and_then(|asset| asset.skin.cape_id);
        let selected_cape = self
            .selected_cape
            .and_then(|index| self.capes.get(index))
            .map(|cape| cape.cape.id);
        self.skin_preview_pending = skin_index
            != self.skins.iter().position(|asset| asset.skin.is_equipped)
            || selected_cape != equipped_cape;
    }

    fn skins_view(&self) -> Vec<SkinView> {
        self.skins
            .iter()
            .enumerate()
            .map(|(index, asset)| SkinView {
                index,
                name: asset
                    .skin
                    .name
                    .as_deref()
                    .map(str::to_string)
                    .unwrap_or_else(|| format!("Saved skin {}", index + 1)),
                variant: format!("{:?}", asset.skin.variant),
                source: format!("{:?}", asset.skin.source),
                equipped: asset.skin.is_equipped,
                avatar: asset.avatar.clone(),
                cape_name: asset.cape_name.clone(),
            })
            .collect()
    }

    fn capes_view(&self) -> Vec<CapeView> {
        self.capes
            .iter()
            .enumerate()
            .map(|(index, asset)| CapeView {
                index,
                name: asset.cape.name.to_string(),
                equipped: asset.cape.is_equipped,
                preview: asset.preview.clone(),
            })
            .collect()
    }

    fn refresh_shared(&self) -> Command<Message> {
        if self.shared.is_none() {
            return self.reload_after_mutation();
        }
        let Some(instance_id) = self.selected.clone() else {
            return self.reload_after_mutation();
        };
        let id = instance_id.clone();
        Command::batch([
            Command::perform(load_snapshot(), Message::Refreshed),
            Command::perform(load_detail(instance_id.clone()), move |result| {
                Message::DetailLoaded(id, result)
            }),
            Command::perform(load_shared(instance_id), Message::SharedLoaded),
        ])
    }

    fn reload_detail(&self) -> Command<Message> {
        let Some(detail) = &self.detail else {
            return Command::none();
        };
        let id = detail.id.clone();
        let message_id = id.clone();
        Command::perform(load_detail(id), move |result| {
            Message::DetailLoaded(message_id, result)
        })
    }

    fn reload_after_mutation(&self) -> Command<Message> {
        Command::batch([
            Command::perform(load_snapshot(), Message::Refreshed),
            self.reload_detail(),
        ])
    }

    fn load_browse(&mut self) -> Command<Message> {
        self.browse.loading = true;
        self.browse.error = None;
        Command::perform(
            search_projects(self.browse.query.clone(), self.browse.kind),
            Message::BrowseResults,
        )
    }
}

impl Default for FerrisoApp {
    fn default() -> Self {
        Self::new()
    }
}

impl App for FerrisoApp {
    type Message = Message;
    type View = ViewModel;

    fn init(&mut self) -> Command<Self::Message> {
        Command::perform(boot(), Message::Booted)
    }

    fn update(&mut self, message: Self::Message) -> Command<Self::Message> {
        match message {
            Message::Booted(result) => {
                match result {
                    Ok(data) => {
                        self.apply_snapshot(data);
                        self.status = self
                            .instances
                            .first()
                            .map(|_| "Ready".to_string())
                            .unwrap_or_else(|| "No instances yet".to_string());
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Failed to initialize".into();
                        return Command::none();
                    }
                }
                let mut commands = Vec::new();
                if self.accounts.iter().any(|account| account.active) {
                    commands.push(Command::perform(
                        load_account_appearance(),
                        Message::AccountAppearanceLoaded,
                    ));
                }
                if self.page == Page::Browse && !self.browse.loaded {
                    commands.push(self.load_browse());
                }
                if self.page == Page::Skins && !self.skins_loaded {
                    commands.push(Command::perform(
                        load_skins(),
                        Message::SkinsLoaded,
                    ));
                }
                commands.push(Command::perform(
                    update::check(),
                    Message::UpdateChecked,
                ));
                if self.page == Page::Project
                    && let Some(page) = &self.project
                {
                    let id = page.id.clone();
                    let instance = self.selected.clone();
                    let message_id = id.clone();
                    commands.push(Command::perform(
                        load_project(id, instance),
                        move |result| {
                            Message::ProjectLoaded(message_id, result)
                        },
                    ));
                }
                if self.page == Page::Instance
                    && let Some(id) = self.selected.clone()
                {
                    let message_id = id.clone();
                    self.detail = Some(DetailView {
                        id: id.clone(),
                        loading: true,
                        tab: DetailTab::Overview,
                        error: None,
                        content: Vec::new(),
                        worlds: Vec::new(),
                        logs: Vec::new(),
                        versions: None,
                        log_view: None,
                        world_form: None,
                    });
                    commands.push(Command::perform(
                        load_detail(id),
                        move |result| Message::DetailLoaded(message_id, result),
                    ));
                }
                Command::batch(commands)
            }
            Message::Refresh => {
                self.status = "Refreshing…".into();
                let snapshot =
                    Command::perform(load_snapshot(), Message::Refreshed);
                let detail = self.reload_detail();
                Command::batch([snapshot, detail])
            }
            Message::Refreshed(result) => {
                match result {
                    Ok(data) => {
                        self.apply_snapshot(data);
                        self.status = "Ready".into();
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Refresh failed".into();
                    }
                }
                Command::none()
            }
            Message::Navigate(page) => {
                self.edit = None;
                self.delete_confirm = false;
                self.library_form = None;
                self.page = page;
                let mut commands = Vec::new();
                if page == Page::Browse
                    && !self.browse.loaded
                    && !self.browse.loading
                {
                    commands.push(self.load_browse());
                }
                if page == Page::Skins && !self.skins_loaded {
                    commands.push(Command::perform(
                        load_skins(),
                        Message::SkinsLoaded,
                    ));
                }
                Command::batch(commands)
            }
            Message::Back => {
                self.edit = None;
                self.delete_confirm = false;
                self.library_form = None;
                self.page = if self.page == Page::Project {
                    Page::Browse
                } else {
                    Page::Library
                };
                Command::none()
            }
            Message::SelectInstance(id) => {
                self.edit = None;
                self.delete_confirm = false;
                self.library_form = None;
                self.selected = Some(id.clone());
                self.page = Page::Instance;
                self.detail = Some(DetailView {
                    id: id.clone(),
                    loading: true,
                    tab: DetailTab::Overview,
                    error: None,
                    content: Vec::new(),
                    worlds: Vec::new(),
                    logs: Vec::new(),
                    versions: None,
                    log_view: None,
                    world_form: None,
                });
                let message_id = id.clone();
                Command::perform(load_detail(id), move |result| {
                    Message::DetailLoaded(message_id, result)
                })
            }
            Message::Launch(id) => {
                if self.launching.is_some() {
                    return Command::none();
                }
                self.launching = Some(id.clone());
                self.error = None;
                self.status = "Launching…".into();
                Command::perform(run_instance(id), Message::Launched)
            }
            Message::Launched(result) => {
                self.launching = None;
                match result {
                    Ok(process) => {
                        self.status =
                            format!("Launched {}", process.instance_name);
                        self.processes
                            .retain(|known| known.uuid != process.uuid);
                        self.processes.push(process);
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Launch failed".into();
                    }
                }
                Command::none()
            }
            Message::BeginLogin => {
                self.error = None;
                self.status = "Opening Microsoft sign-in…".into();
                Command::perform(begin_login(), Message::LoginStarted)
            }
            Message::LoginStarted(result) => {
                match result {
                    Ok(started) => {
                        self.login_url = Some(started.url);
                        self.login_opened = started.opened;
                        self.finish_login =
                            started.finish.lock().expect("login slot").take();
                        self.status = if started.opened {
                            "Sign in in your browser, then paste the final URL here"
                                .into()
                        } else {
                            "Open the sign-in URL, then paste the final URL here"
                                .into()
                        };
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Could not start sign-in".into();
                    }
                }
                Command::none()
            }
            Message::OpenLoginUrl => {
                if let Some(url) = &self.login_url {
                    self.login_opened = webbrowser::open(url).is_ok();
                    if !self.login_opened {
                        self.error =
                            Some("Could not open the default browser".into());
                    }
                }
                Command::none()
            }
            Message::LoginCodeChanged(code) => {
                self.login_code = code;
                Command::none()
            }
            Message::FinishLogin => {
                let Some(finish) = self.finish_login.take() else {
                    return Command::none();
                };
                let code = extract_login_code(&self.login_code);
                if code.is_empty() {
                    self.finish_login = Some(finish);
                    self.error = Some(
                        "Paste the sign-in URL or authorization code first"
                            .into(),
                    );
                    return Command::none();
                }
                self.status = "Completing Microsoft sign-in…".into();
                Command::perform(
                    async move { finish(code).await },
                    Message::LoggedIn,
                )
            }
            Message::LoggedIn(result) => match result {
                Ok(name) => {
                    self.login_url = None;
                    self.login_code.clear();
                    self.login_opened = false;
                    self.status = format!("Signed in as {name}");
                    Command::batch([
                        Command::perform(load_snapshot(), Message::Refreshed),
                        Command::perform(load_skins(), Message::SkinsLoaded),
                        Command::perform(
                            load_account_appearance(),
                            Message::AccountAppearanceLoaded,
                        ),
                    ])
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Sign-in failed".into();
                    Command::none()
                }
            },
            Message::CancelLogin => {
                self.login_url = None;
                self.login_code.clear();
                self.login_opened = false;
                self.finish_login = None;
                self.status = "Sign-in cancelled".into();
                Command::none()
            }
            Message::SetAccount(id) => {
                self.status = "Switching account…".into();
                Command::perform(set_default_account(id), Message::AccountSet)
            }
            Message::AccountSet(result) => match result {
                Ok(()) => {
                    self.status = "Account switched".into();
                    Command::batch([
                        Command::perform(load_snapshot(), Message::Refreshed),
                        Command::perform(load_skins(), Message::SkinsLoaded),
                        Command::perform(
                            load_account_appearance(),
                            Message::AccountAppearanceLoaded,
                        ),
                    ])
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not switch account".into();
                    Command::none()
                }
            },
            Message::RemoveAccount(id) => {
                self.status = "Removing account…".into();
                Command::perform(remove_account(id), Message::AccountRemoved)
            }
            Message::AccountRemoved(result) => match result {
                Ok(()) => {
                    self.status = "Account removed".into();
                    Command::batch([
                        Command::perform(load_snapshot(), Message::Refreshed),
                        Command::perform(load_skins(), Message::SkinsLoaded),
                        Command::perform(
                            load_account_appearance(),
                            Message::AccountAppearanceLoaded,
                        ),
                    ])
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not remove account".into();
                    Command::none()
                }
            },
            Message::BrowseQueryChanged(query) => {
                self.browse.query = query;
                Command::none()
            }
            Message::BrowseKind(kind) => {
                self.browse.kind = kind;
                self.browse.loaded = false;
                self.load_browse()
            }
            Message::BrowseSearch => self.load_browse(),
            Message::BrowseResults(result) => {
                self.browse.loading = false;
                self.browse.loaded = true;
                match result {
                    Ok(results) => {
                        self.browse.results = results;
                        self.browse.error = None;
                    }
                    Err(error) => {
                        self.browse.results.clear();
                        self.browse.error = Some(error);
                    }
                }
                Command::none()
            }
            Message::OpenProject(id) => {
                self.page = Page::Project;
                self.project = Some(ProjectPage::loading(
                    id.clone(),
                    self.selected.clone(),
                ));
                self.status = "Loading project…".into();
                let message_id = id.clone();
                Command::perform(
                    load_project(id, self.selected.clone()),
                    move |result| Message::ProjectLoaded(message_id, result),
                )
            }
            Message::ProjectLoaded(id, result) => {
                if self.project.as_ref().map(|page| page.id.as_str())
                    != Some(id.as_str())
                {
                    return Command::none();
                }
                match result {
                    Ok(mut page) => {
                        page.selected_instance =
                            self.selected.clone().or(page.selected_instance);
                        self.project = Some(page);
                        self.status = "Ready".into();
                    }
                    Err(error) => {
                        if let Some(page) = self.project.as_mut() {
                            page.loading = false;
                            page.error = Some(error);
                        }
                        self.status = "Could not load project".into();
                    }
                }
                Command::none()
            }
            Message::SelectProjectVersion(index) => {
                if let Some(page) = self.project.as_mut() {
                    page.selected_version =
                        (index < page.versions.len()).then_some(index);
                    page.install_error = None;
                    page.install_result = None;
                }
                Command::none()
            }
            Message::SelectProjectInstance(instance_id) => {
                if let Some(page) = self.project.as_mut() {
                    page.selected_instance = Some(instance_id);
                    page.install_error = None;
                    page.install_result = None;
                }
                Command::none()
            }
            Message::InstallProjectVersion => {
                let Some(page) = self.project.as_mut() else {
                    return Command::none();
                };
                let Some(project) = page.project.clone() else {
                    return Command::none();
                };
                let Some(version) = page.selected_version().cloned() else {
                    page.install_error =
                        Some("Select a version to install".into());
                    return Command::none();
                };
                if project.kind != ProjectKind::Modpack
                    && page.selected_instance.is_none()
                {
                    page.install_error =
                        Some("Select an instance first".into());
                    return Command::none();
                }
                page.installing = true;
                page.install_error = None;
                page.install_result = None;
                self.status = format!("Installing {}…", project.title);
                let target = page.selected_instance.clone();
                Command::perform(
                    install_project_version(project, version.id, target),
                    Message::ProjectVersionInstalled,
                )
            }
            Message::ProjectVersionInstalled(result) => {
                if let Some(page) = self.project.as_mut() {
                    page.installing = false;
                    match &result {
                        Ok(message) => {
                            page.install_result = Some(message.clone());
                            page.install_error = None;
                        }
                        Err(error) => {
                            page.install_error = Some(error.clone());
                            page.install_result = None;
                        }
                    }
                }
                match result {
                    Ok(message) => {
                        self.status = message;
                        Command::batch([
                            self.reload_detail(),
                            Command::perform(load_jobs(), Message::JobsLoaded),
                        ])
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Install failed".into();
                        Command::none()
                    }
                }
            }
            Message::InstallProject(project) => {
                if project.kind == ProjectKind::Modpack {
                    self.status = format!("Installing {}…", project.title);
                    Command::perform(
                        install_modpack(project),
                        Message::InstallStarted,
                    )
                } else {
                    let Some(instance_id) = self.selected.clone() else {
                        self.error = Some(
                            "Select an instance before installing content"
                                .into(),
                        );
                        return Command::none();
                    };
                    self.status = format!("Installing {}…", project.title);
                    Command::perform(
                        install_to_instance(instance_id, project),
                        Message::ModInstalled,
                    )
                }
            }
            Message::InstallStarted(result) => match result {
                Ok(job) => {
                    self.status = "Install queued".into();
                    self.jobs.insert(0, JobView::from(job));
                    Command::perform(load_jobs(), Message::JobsLoaded)
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not queue install".into();
                    Command::none()
                }
            },
            Message::ModInstalled(result) => match result {
                Ok(path) => {
                    self.status = format!("Installed {path}");
                    self.reload_detail()
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Content install failed".into();
                    Command::none()
                }
            },
            Message::OpenUrl(url) => {
                if webbrowser::open(&url).is_err() {
                    self.error =
                        Some("Could not open the default browser".into());
                }
                Command::none()
            }
            Message::ToggleCreate => {
                self.create.open = !self.create.open;
                Command::none()
            }
            Message::OpenCreate => {
                self.page = Page::Library;
                self.create.open = true;
                Command::none()
            }
            Message::CreateNameChanged(name) => {
                self.create.name = name;
                Command::none()
            }
            Message::CreateVersionChanged(version) => {
                self.create.game_version = version;
                Command::none()
            }
            Message::CreateLoaderChanged(index) => {
                self.create.loader = index.min(CreateView::LOADERS.len() - 1);
                Command::none()
            }
            Message::CreateInstance => {
                if self.create.name.trim().is_empty() {
                    self.create.error = Some("Enter an instance name".into());
                    return Command::none();
                }
                if self.create.game_version.trim().is_empty() {
                    self.create.error =
                        Some("Enter a Minecraft version".into());
                    return Command::none();
                }
                self.create.busy = true;
                self.create.error = None;
                self.status = "Creating instance…".into();
                Command::perform(
                    create_instance(
                        self.create.name.clone(),
                        self.create.game_version.clone(),
                        self.create.loader(),
                    ),
                    Message::CreateStarted,
                )
            }
            Message::CreateStarted(result) => {
                self.create.busy = false;
                match result {
                    Ok(job) => {
                        self.create.open = false;
                        self.status = "Instance install queued".into();
                        self.jobs.insert(0, JobView::from(job));
                        Command::perform(load_jobs(), Message::JobsLoaded)
                    }
                    Err(error) => {
                        self.create.error = Some(error);
                        self.status = "Could not create instance".into();
                        Command::none()
                    }
                }
            }
            Message::DetailTab(tab) => {
                if let Some(detail) = &mut self.detail {
                    detail.tab = tab;
                }
                Command::none()
            }
            Message::DetailLoaded(id, result) => {
                if self.selected.as_deref() != Some(id.as_str()) {
                    return Command::none();
                }
                let tab = self
                    .detail
                    .as_ref()
                    .map(|detail| detail.tab)
                    .unwrap_or(DetailTab::Overview);
                match result {
                    Ok(data) => {
                        self.detail = Some(DetailView {
                            id,
                            loading: false,
                            tab,
                            error: None,
                            content: data.content,
                            worlds: data.worlds,
                            logs: data.logs,
                            versions: None,
                            log_view: None,
                            world_form: None,
                        });
                    }
                    Err(error) => {
                        self.detail = Some(DetailView {
                            id,
                            loading: false,
                            tab,
                            error: Some(error),
                            content: Vec::new(),
                            worlds: Vec::new(),
                            logs: Vec::new(),
                            versions: None,
                            log_view: None,
                            world_form: None,
                        });
                    }
                }
                Command::none()
            }
            Message::ToggleContent { path, enabled } => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                Command::perform(
                    set_content_enabled(instance_id, path, enabled),
                    Message::ContentToggled,
                )
            }
            Message::ContentToggled(result) => match result {
                Ok(()) => self.reload_detail(),
                Err(error) => {
                    self.error = Some(error);
                    Command::none()
                }
            },
            Message::RemoveContent(path) => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                Command::perform(
                    remove_content(instance_id, path),
                    Message::ContentRemoved,
                )
            }
            Message::ContentRemoved(result) => match result {
                Ok(()) => self.reload_detail(),
                Err(error) => {
                    self.error = Some(error);
                    Command::none()
                }
            },
            Message::CancelJob(id) => {
                Command::perform(cancel_job(id), Message::JobAction)
            }
            Message::RetryJob(id) => {
                Command::perform(retry_job(id), Message::JobAction)
            }
            Message::DismissJob(id) => {
                Command::perform(dismiss_job(id), Message::JobAction)
            }
            Message::JobAction(result) => match result {
                Ok(()) => Command::perform(load_jobs(), Message::JobsLoaded),
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not update install job".into();
                    Command::none()
                }
            },
            Message::BeginRename => {
                let edit = self
                    .selected
                    .as_ref()
                    .and_then(|id| {
                        self.instances
                            .iter()
                            .find(|instance| &instance.id == id)
                    })
                    .map(InstanceEditForm::from_instance);
                self.edit = edit;
                match self.selected.clone() {
                    Some(instance_id) => Command::perform(
                        load_required_java(instance_id),
                        Message::RequiredJavaLoaded,
                    ),
                    None => Command::none(),
                }
            }
            Message::RenameChanged(name) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.name = name;
                }
                Command::none()
            }
            Message::EditMemory(value) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.memory = value;
                }
                Command::none()
            }
            Message::EditJava(value) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.java_path = value;
                }
                Command::none()
            }
            Message::EditArgs(value) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.args = value;
                }
                Command::none()
            }
            Message::EditIconPath(value) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.icon_path = value;
                }
                Command::none()
            }
            Message::RemoveInstanceIcon => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                Command::perform(
                    clear_instance_icon(instance_id),
                    Message::IconUpdated,
                )
            }
            Message::IconUpdated(result) => match result {
                Ok(()) => {
                    self.status = "Instance icon updated".into();
                    self.reload_after_mutation()
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not update instance icon".into();
                    Command::none()
                }
            },
            Message::DetectJava => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.jres_loading = true;
                    edit.jres_error = None;
                }
                Command::perform(detect_java(), Message::JavaDetected)
            }
            Message::JavaDetected(result) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.jres_loading = false;
                    match result {
                        Ok(jres) => edit.jres = jres,
                        Err(error) => edit.jres_error = Some(error),
                    }
                }
                Command::none()
            }
            Message::UseJava(path) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.java_path = path;
                }
                Command::none()
            }
            Message::RequiredJavaLoaded(result) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.required_java_loading = false;
                    match result {
                        Ok(required) => edit.required_java = required,
                        Err(error) => edit.jres_error = Some(error),
                    }
                }
                Command::none()
            }
            Message::InstallJava(version) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.java_installing = true;
                    edit.jres_error = None;
                }
                self.status = format!("Installing Java {version}…");
                Command::perform(install_java(version), Message::JavaInstalled)
            }
            Message::JavaInstalled(result) => {
                if let Some(edit) = self.edit.as_mut() {
                    edit.java_installing = false;
                }
                match result {
                    Ok(path) => {
                        if let Some(edit) = self.edit.as_mut() {
                            edit.java_path = path;
                            edit.jres_loading = true;
                        }
                        self.status = "Java installed".into();
                        Command::perform(detect_java(), Message::JavaDetected)
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Could not install Java".into();
                        Command::none()
                    }
                }
            }
            Message::CancelRename => {
                self.edit = None;
                Command::none()
            }
            Message::SaveRename => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                let Some(edit) = self.edit.clone() else {
                    return Command::none();
                };
                let name = edit.name.trim().to_string();
                if name.is_empty() {
                    self.error = Some("Instance name cannot be empty".into());
                    return Command::none();
                }
                let memory = match edit.memory.trim() {
                    "" => Some(None),
                    value => match value.parse::<u32>() {
                        Ok(value) if (256..=65_536).contains(&value) => {
                            Some(Some(value))
                        }
                        _ => {
                            self.error = Some(
                                "Memory must be 256–65536 MB, or empty to inherit"
                                    .into(),
                            );
                            return Command::none();
                        }
                    },
                };
                let java_path = Some(match edit.java_path.trim() {
                    "" => None,
                    value => Some(value.to_string()),
                });
                let args = edit
                    .args
                    .split_whitespace()
                    .map(str::to_string)
                    .collect::<Vec<_>>();
                let args = Some((!args.is_empty()).then_some(args));
                self.status = "Saving instance settings…".into();
                let icon_path = (!edit.icon_path.trim().is_empty())
                    .then(|| edit.icon_path.trim().to_string());
                Command::perform(
                    save_instance(
                        instance_id,
                        name,
                        memory,
                        java_path,
                        args,
                        icon_path,
                        edit.group_ids.clone(),
                    ),
                    Message::Renamed,
                )
            }
            Message::Renamed(result) => match result {
                Ok(()) => {
                    self.edit = None;
                    self.status = "Instance settings saved".into();
                    self.reload_after_mutation()
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not save instance settings".into();
                    Command::none()
                }
            },
            Message::AskDelete => {
                self.delete_confirm = true;
                Command::none()
            }
            Message::CancelDelete => {
                self.delete_confirm = false;
                Command::none()
            }
            Message::DeleteInstance => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                self.delete_confirm = false;
                self.status = "Deleting instance…".into();
                Command::perform(delete_instance(instance_id), Message::Deleted)
            }
            Message::Deleted(result) => match result {
                Ok(()) => {
                    self.selected = None;
                    self.detail = None;
                    self.page = Page::Library;
                    self.status = "Instance deleted".into();
                    Command::perform(load_snapshot(), Message::Refreshed)
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not delete instance".into();
                    Command::none()
                }
            },
            Message::UpdateAllContent => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                self.status = "Updating content…".into();
                Command::perform(
                    update_all_content(instance_id),
                    Message::ContentUpdated,
                )
            }
            Message::UpdateContent(path) => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                self.status = "Updating content…".into();
                Command::perform(
                    update_content(instance_id, path),
                    Message::ContentUpdated,
                )
            }
            Message::ContentUpdated(result) => match result {
                Ok(message) => {
                    self.status = message;
                    self.reload_detail()
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not update content".into();
                    Command::none()
                }
            },
            Message::ToggleLock { path, locked } => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                Command::perform(
                    set_content_lock(instance_id, path, locked),
                    Message::LockChanged,
                )
            }
            Message::LockChanged(result) => match result {
                Ok(()) => self.reload_detail(),
                Err(error) => {
                    self.error = Some(error);
                    Command::none()
                }
            },
            Message::ShowVersions { path, project_id } => {
                let Some(detail) = self.detail.as_mut() else {
                    return Command::none();
                };
                let title = detail
                    .content
                    .iter()
                    .find(|row| row.file_path == path)
                    .map(|row| row.title.clone())
                    .unwrap_or_default();
                detail.versions = Some(VersionPicker {
                    path: path.clone(),
                    title,
                    loading: true,
                    error: None,
                    versions: Vec::new(),
                });
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                Command::perform(
                    load_versions(instance_id, project_id),
                    move |result| Message::VersionsLoaded { path, result },
                )
            }
            Message::VersionsLoaded { path, result } => {
                if let Some(picker) = self
                    .detail
                    .as_mut()
                    .and_then(|detail| detail.versions.as_mut())
                    .filter(|picker| picker.path == path)
                {
                    picker.loading = false;
                    match result {
                        Ok(versions) => picker.versions = versions,
                        Err(error) => picker.error = Some(error),
                    }
                }
                Command::none()
            }
            Message::CloseVersions => {
                if let Some(detail) = self.detail.as_mut() {
                    detail.versions = None;
                }
                Command::none()
            }
            Message::ShowLog { filename, crash } => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                if let Some(detail) = self.detail.as_mut() {
                    detail.log_view = Some(LogView {
                        filename: filename.clone(),
                        loading: true,
                        content: None,
                        error: None,
                    });
                }
                Command::perform(
                    load_log(instance_id, filename.clone(), crash),
                    move |result| Message::LogLoaded { filename, result },
                )
            }
            Message::LogLoaded { filename, result } => {
                if let Some(view) = self
                    .detail
                    .as_mut()
                    .and_then(|detail| detail.log_view.as_mut())
                    .filter(|view| view.filename == filename)
                {
                    view.loading = false;
                    match result {
                        Ok(content) => view.content = Some(content),
                        Err(error) => view.error = Some(error),
                    }
                }
                Command::none()
            }
            Message::CloseLog => {
                if let Some(detail) = self.detail.as_mut() {
                    detail.log_view = None;
                }
                Command::none()
            }
            Message::DeleteLog { filename, crash } => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                Command::perform(
                    delete_log(instance_id, filename, crash),
                    Message::LogDeleted,
                )
            }
            Message::DeleteAllLogs => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                Command::perform(
                    delete_all_logs(instance_id),
                    Message::LogDeleted,
                )
            }
            Message::LogDeleted(result) => match result {
                Ok(()) => {
                    if let Some(detail) = self.detail.as_mut() {
                        detail.log_view = None;
                    }
                    self.reload_detail()
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not delete log".into();
                    Command::none()
                }
            },
            Message::SwitchVersion { path, version_id } => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                self.status = "Switching content version…".into();
                Command::perform(
                    switch_content_version(instance_id, path, version_id),
                    Message::VersionSwitched,
                )
            }
            Message::VersionSwitched(result) => match result {
                Ok(message) => {
                    self.status = message;
                    if let Some(detail) = self.detail.as_mut() {
                        detail.versions = None;
                    }
                    self.reload_detail()
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not switch content version".into();
                    Command::none()
                }
            },
            Message::AddServer => {
                if let Some(detail) = self.detail.as_mut() {
                    detail.world_form = Some(WorldForm {
                        kind: WorldFormKind::AddServer,
                        name: String::new(),
                        address: String::new(),
                        pack_status: "prompt".into(),
                    });
                }
                Command::none()
            }
            Message::EditServer(index) => {
                let Some(detail) = self.detail.as_mut() else {
                    return Command::none();
                };
                let Some(world) = detail
                    .worlds
                    .iter()
                    .find(|world| world.server_index == Some(index))
                else {
                    return Command::none();
                };
                detail.world_form = Some(WorldForm {
                    kind: WorldFormKind::EditServer { index },
                    name: world.name.clone(),
                    address: world.detail.clone(),
                    pack_status: world.pack_status.clone(),
                });
                Command::none()
            }
            Message::RenameWorld(path) => {
                let Some(detail) = self.detail.as_mut() else {
                    return Command::none();
                };
                let Some(world) =
                    detail.worlds.iter().find(|world| world.detail == path)
                else {
                    return Command::none();
                };
                detail.world_form = Some(WorldForm {
                    kind: WorldFormKind::RenameWorld { path },
                    name: world.name.clone(),
                    address: String::new(),
                    pack_status: String::new(),
                });
                Command::none()
            }
            Message::DeleteWorld { path, name } => {
                if let Some(detail) = self.detail.as_mut() {
                    detail.world_form = Some(WorldForm {
                        kind: WorldFormKind::DeleteWorld { path, name },
                        name: String::new(),
                        address: String::new(),
                        pack_status: String::new(),
                    });
                }
                Command::none()
            }
            Message::DeleteServer { index, name } => {
                if let Some(detail) = self.detail.as_mut() {
                    detail.world_form = Some(WorldForm {
                        kind: WorldFormKind::DeleteServer { index, name },
                        name: String::new(),
                        address: String::new(),
                        pack_status: String::new(),
                    });
                }
                Command::none()
            }
            Message::WorldFormName(value) => {
                if let Some(form) = self
                    .detail
                    .as_mut()
                    .and_then(|detail| detail.world_form.as_mut())
                {
                    form.name = value;
                }
                Command::none()
            }
            Message::WorldFormAddress(value) => {
                if let Some(form) = self
                    .detail
                    .as_mut()
                    .and_then(|detail| detail.world_form.as_mut())
                {
                    form.address = value;
                }
                Command::none()
            }
            Message::CyclePackStatus => {
                if let Some(form) = self
                    .detail
                    .as_mut()
                    .and_then(|detail| detail.world_form.as_mut())
                {
                    form.pack_status = match form.pack_status.as_str() {
                        "prompt" => "enabled".into(),
                        "enabled" => "disabled".into(),
                        _ => "prompt".into(),
                    };
                }
                Command::none()
            }
            Message::CancelWorldForm => {
                if let Some(detail) = self.detail.as_mut() {
                    detail.world_form = None;
                }
                Command::none()
            }
            Message::SaveWorldForm => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                let Some(form) = self
                    .detail
                    .as_ref()
                    .and_then(|detail| detail.world_form.clone())
                else {
                    return Command::none();
                };
                self.status = "Saving world changes…".into();
                Command::perform(
                    save_world_form(instance_id, form),
                    Message::WorldSaved,
                )
            }
            Message::WorldSaved(result) => match result {
                Ok(()) => {
                    if let Some(detail) = self.detail.as_mut() {
                        detail.world_form = None;
                    }
                    self.status = "World changes saved".into();
                    self.reload_detail()
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not save world changes".into();
                    Command::none()
                }
            },
            Message::BackupWorld { path, name } => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                self.status = format!("Backing up {name}…");
                Command::perform(
                    backup_world(instance_id, path),
                    Message::WorldBackupDone,
                )
            }
            Message::WorldBackupDone(result) => {
                match result {
                    Ok(bytes) => {
                        self.status = format!(
                            "Backup saved ({:.1} MB)",
                            bytes as f64 / 1_048_576.0
                        );
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Could not back up world".into();
                    }
                }
                Command::none()
            }
            Message::PollProcesses => {
                if !self.booted {
                    return Command::none();
                }
                Command::perform(load_processes(), Message::ProcessesLoaded)
            }
            Message::ProcessesLoaded(result) => {
                match result {
                    Ok(processes) => self.processes = processes,
                    Err(error) => {
                        self.status =
                            format!("Could not read processes: {error}");
                    }
                }
                Command::none()
            }
            Message::KillProcess(uuid) => {
                Command::perform(kill_process(uuid), Message::ProcessKilled)
            }
            Message::ProcessKilled(result) => {
                match result {
                    Ok(()) => self.status = "Process stopped".into(),
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Could not stop process".into();
                    }
                }
                Command::batch([
                    Command::perform(
                        load_processes(),
                        Message::ProcessesLoaded,
                    ),
                    Command::perform(load_snapshot(), Message::Refreshed),
                ])
            }
            Message::ToggleGroups => {
                self.groups_open = !self.groups_open;
                Command::none()
            }
            Message::BeginGroupCreate => {
                self.group_form = Some(GroupForm {
                    id: None,
                    name: String::new(),
                });
                self.groups_open = true;
                Command::none()
            }
            Message::BeginGroupRename(id) => {
                let name = self
                    .groups
                    .iter()
                    .find(|group| group.id == id)
                    .map(|group| group.name.clone())
                    .unwrap_or_default();
                self.group_form = Some(GroupForm { id: Some(id), name });
                self.groups_open = true;
                Command::none()
            }
            Message::GroupNameChanged(name) => {
                if let Some(form) = self.group_form.as_mut() {
                    form.name = name;
                }
                Command::none()
            }
            Message::CancelGroupForm => {
                self.group_form = None;
                Command::none()
            }
            Message::SaveGroup => {
                let Some(form) = self.group_form.clone() else {
                    return Command::none();
                };
                if form.name.trim().is_empty() {
                    self.error = Some("Group name cannot be empty".into());
                    return Command::none();
                }
                self.status = "Saving group…".into();
                Command::perform(save_group(form), Message::GroupSaved)
            }
            Message::GroupSaved(result) => match result {
                Ok(()) => {
                    self.group_form = None;
                    self.status = "Group saved".into();
                    Command::perform(load_snapshot(), Message::Refreshed)
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not save group".into();
                    Command::none()
                }
            },
            Message::DeleteGroup(id) => {
                self.status = "Deleting group…".into();
                Command::perform(delete_group(id), Message::GroupDeleted)
            }
            Message::GroupDeleted(result) => match result {
                Ok(id) => {
                    if self.selected_group.as_deref() == Some(id.as_str()) {
                        self.selected_group = None;
                    }
                    self.status = "Group deleted".into();
                    Command::perform(load_snapshot(), Message::Refreshed)
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not delete group".into();
                    Command::none()
                }
            },
            Message::FilterGroup(id) => {
                self.selected_group = id;
                Command::none()
            }
            Message::ToggleGroupMembership(id) => {
                if let Some(edit) = self.edit.as_mut() {
                    if let Some(index) =
                        edit.group_ids.iter().position(|group| group == &id)
                    {
                        edit.group_ids.remove(index);
                    } else {
                        edit.group_ids.push(id);
                    }
                }
                Command::none()
            }
            Message::BeginModrinthLogin => {
                self.modrinth_login = true;
                self.status = "Waiting for Modrinth sign-in…".into();
                Command::perform(
                    modrinth_login(),
                    Message::ModrinthLoginFinished,
                )
            }
            Message::ModrinthLoginFinished(result) => {
                self.modrinth_login = false;
                match result {
                    Ok(account) => {
                        self.status = format!(
                            "Signed in to Modrinth as {}",
                            account
                                .username
                                .as_deref()
                                .unwrap_or(&account.user_id)
                        );
                        Command::perform(load_snapshot(), Message::Refreshed)
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Modrinth sign-in failed".into();
                        Command::none()
                    }
                }
            }
            Message::LogoutModrinth => {
                Command::perform(logout_modrinth(), Message::ModrinthLoggedOut)
            }
            Message::ModrinthLoggedOut(result) => match result {
                Ok(()) => {
                    self.modrinth_account = None;
                    self.friends.clear();
                    self.status = "Signed out of Modrinth".into();
                    Command::perform(load_snapshot(), Message::Refreshed)
                }
                Err(error) => {
                    self.error = Some(error);
                    Command::none()
                }
            },
            Message::FriendIdChanged(value) => {
                self.friend_input = value;
                Command::none()
            }
            Message::AddFriend => {
                let user_id = self.friend_input.trim().to_string();
                if user_id.is_empty() {
                    self.error = Some("Enter a Modrinth user ID".into());
                    return Command::none();
                }
                self.status = "Adding friend…".into();
                Command::perform(add_friend(user_id), Message::FriendAdded)
            }
            Message::FriendAdded(result) => match result {
                Ok(()) => {
                    self.friend_input.clear();
                    self.status = "Friend added".into();
                    Command::perform(load_snapshot(), Message::Refreshed)
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not add friend".into();
                    Command::none()
                }
            },
            Message::RemoveFriend(user_id) => {
                Command::perform(remove_friend(user_id), Message::FriendRemoved)
            }
            Message::FriendRemoved(result) => match result {
                Ok(()) => {
                    self.status = "Friend removed".into();
                    Command::perform(load_snapshot(), Message::Refreshed)
                }
                Err(error) => {
                    self.error = Some(error);
                    Command::none()
                }
            },
            Message::ToggleShared => {
                if self.shared_open {
                    self.shared_open = false;
                    self.shared = None;
                    return Command::none();
                }
                let Some(instance_id) = self.selected.clone() else {
                    self.error = Some("Select an instance first".into());
                    return Command::none();
                };
                self.shared_open = true;
                self.shared = Some(SharedView {
                    loading: true,
                    error: None,
                    users: Vec::new(),
                    invites: Vec::new(),
                    new_invite: None,
                    user_input: String::new(),
                    busy: false,
                });
                Command::perform(
                    load_shared(instance_id),
                    Message::SharedLoaded,
                )
            }
            Message::SharedLoaded(result) => match result {
                Ok(data) => {
                    if let Some(shared) = self.shared.as_mut() {
                        shared.loading = false;
                        shared.busy = false;
                        shared.error = None;
                        shared.users = data.users;
                        shared.invites = data.invites;
                    }
                    Command::none()
                }
                Err(error) => {
                    if let Some(shared) = self.shared.as_mut() {
                        shared.loading = false;
                        shared.busy = false;
                        shared.error = Some(error);
                    }
                    Command::none()
                }
            },
            Message::PublishShared => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                if let Some(shared) = self.shared.as_mut() {
                    shared.busy = true;
                }
                self.status = "Publishing shared instance…".into();
                Command::perform(
                    publish_shared(instance_id),
                    Message::SharedPublished,
                )
            }
            Message::SharedPublished(result) => match result {
                Ok(()) => {
                    self.status = "Shared instance published".into();
                    self.refresh_shared()
                }
                Err(error) => {
                    if let Some(shared) = self.shared.as_mut() {
                        shared.busy = false;
                        shared.error = Some(error);
                    }
                    self.status = "Could not publish shared instance".into();
                    Command::none()
                }
            },
            Message::UnpublishShared => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                if let Some(shared) = self.shared.as_mut() {
                    shared.busy = true;
                }
                Command::perform(
                    unpublish_shared(instance_id),
                    Message::SharedUnpublished,
                )
            }
            Message::SharedUnpublished(result) => match result {
                Ok(()) => {
                    self.status = "Shared instance unpublished".into();
                    self.shared_open = false;
                    self.shared = None;
                    self.reload_after_mutation()
                }
                Err(error) => {
                    if let Some(shared) = self.shared.as_mut() {
                        shared.busy = false;
                        shared.error = Some(error);
                    }
                    Command::none()
                }
            },
            Message::UnlinkShared => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                if let Some(shared) = self.shared.as_mut() {
                    shared.busy = true;
                }
                Command::perform(
                    unlink_shared(instance_id),
                    Message::SharedUnlinked,
                )
            }
            Message::SharedUnlinked(result) => match result {
                Ok(()) => {
                    self.status = "Left the shared instance".into();
                    self.shared_open = false;
                    self.shared = None;
                    self.reload_after_mutation()
                }
                Err(error) => {
                    if let Some(shared) = self.shared.as_mut() {
                        shared.busy = false;
                        shared.error = Some(error);
                    }
                    Command::none()
                }
            },
            Message::SharedUserInputChanged(value) => {
                if let Some(shared) = self.shared.as_mut() {
                    shared.user_input = value;
                }
                Command::none()
            }
            Message::InviteSharedUsers => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                let Some(shared) = self.shared.as_ref() else {
                    return Command::none();
                };
                let ids: Vec<String> = shared
                    .user_input
                    .split([' ', ',', '\n'])
                    .map(str::trim)
                    .filter(|id| !id.is_empty())
                    .map(str::to_string)
                    .collect();
                if ids.is_empty() {
                    self.error =
                        Some("Enter at least one Modrinth user ID".into());
                    return Command::none();
                }
                if let Some(shared) = self.shared.as_mut() {
                    shared.busy = true;
                }
                Command::perform(
                    invite_shared_users(instance_id, ids),
                    Message::SharedUsersChanged,
                )
            }
            Message::SharedUsersChanged(result) => match result {
                Ok(()) => {
                    self.status = "Shared instance members updated".into();
                    if let Some(shared) = self.shared.as_mut() {
                        shared.user_input.clear();
                    }
                    self.refresh_shared()
                }
                Err(error) => {
                    if let Some(shared) = self.shared.as_mut() {
                        shared.busy = false;
                        shared.error = Some(error);
                    }
                    Command::none()
                }
            },
            Message::RemoveSharedUser(user_id) => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                if let Some(shared) = self.shared.as_mut() {
                    shared.busy = true;
                }
                Command::perform(
                    remove_shared_user(instance_id, user_id),
                    Message::SharedUsersChanged,
                )
            }
            Message::CreateSharedInvite => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                if let Some(shared) = self.shared.as_mut() {
                    shared.busy = true;
                }
                Command::perform(
                    create_shared_invite(instance_id),
                    Message::SharedInviteCreated,
                )
            }
            Message::SharedInviteCreated(result) => match result {
                Ok(invite_id) => {
                    if let Some(shared) = self.shared.as_mut() {
                        shared.busy = false;
                        shared.new_invite = Some(invite_id);
                    }
                    self.status = "Invite link created".into();
                    self.refresh_shared()
                }
                Err(error) => {
                    if let Some(shared) = self.shared.as_mut() {
                        shared.busy = false;
                        shared.error = Some(error);
                    }
                    Command::none()
                }
            },
            Message::RevokeSharedInvite(invite_id) => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                if let Some(shared) = self.shared.as_mut() {
                    shared.busy = true;
                }
                Command::perform(
                    revoke_shared_invite(instance_id, invite_id),
                    Message::SharedInviteRevoked,
                )
            }
            Message::SharedInviteRevoked(result) => match result {
                Ok(()) => {
                    self.status = "Invite revoked".into();
                    self.refresh_shared()
                }
                Err(error) => {
                    if let Some(shared) = self.shared.as_mut() {
                        shared.busy = false;
                        shared.error = Some(error);
                    }
                    Command::none()
                }
            },
            Message::UpdateShared => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                if let Some(shared) = self.shared.as_mut() {
                    shared.busy = true;
                }
                Command::perform(
                    update_shared(instance_id),
                    Message::SharedUpdated,
                )
            }
            Message::SharedUpdated(result) => {
                if let Some(shared) = self.shared.as_mut() {
                    shared.busy = false;
                }
                match result {
                    Ok(_) => {
                        self.status = "Updating from shared instance…".into();
                        self.reload_after_mutation()
                    }
                    Err(error) => {
                        if let Some(shared) = self.shared.as_mut() {
                            shared.error = Some(error);
                        }
                        Command::none()
                    }
                }
            }
            Message::JoinInviteChanged(value) => {
                self.join_invite = value;
                self.join_preview = None;
                self.join_loading = false;
                Command::none()
            }
            Message::PreviewJoinInvite => {
                let invite = extract_invite_id(&self.join_invite);
                if invite.is_empty() {
                    self.error =
                        Some("Enter a shared instance invite ID".into());
                    return Command::none();
                }
                self.join_loading = true;
                self.join_preview = None;
                Command::perform(
                    preview_join(invite),
                    Message::JoinPreviewLoaded,
                )
            }
            Message::JoinPreviewLoaded(result) => {
                self.join_loading = false;
                match result {
                    Ok(preview) => {
                        self.join_preview = Some(preview);
                        Command::none()
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.join_preview = None;
                        Command::none()
                    }
                }
            }
            Message::InstallSharedInstance => {
                let Some(preview) = self.join_preview.clone() else {
                    return Command::none();
                };
                self.join_loading = true;
                Command::perform(
                    install_shared(preview),
                    Message::SharedInstalled,
                )
            }
            Message::SharedInstalled(result) => {
                self.join_loading = false;
                match result {
                    Ok(_) => {
                        self.join_preview = None;
                        self.join_invite.clear();
                        self.status = "Installing shared instance…".into();
                        self.reload_after_mutation()
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status =
                            "Could not install shared instance".into();
                        Command::none()
                    }
                }
            }
            Message::BeginImport => {
                self.library_form = Some(LibraryForm::Import {
                    base_path: String::new(),
                    folder: String::new(),
                    launcher: ImportLauncherType::PrismLauncher,
                });
                Command::none()
            }
            Message::BeginExport => {
                let name = self
                    .selected
                    .as_ref()
                    .and_then(|id| {
                        self.instances
                            .iter()
                            .find(|instance| &instance.id == id)
                    })
                    .map(|instance| instance.name.replace(['/', '\\'], "_"))
                    .unwrap_or_else(|| "instance".into());
                self.library_form = Some(LibraryForm::Export {
                    path: format!("{name}.mrpack"),
                });
                Command::none()
            }
            Message::ImportBasePath(value) => {
                if let Some(LibraryForm::Import { base_path, .. }) =
                    self.library_form.as_mut()
                {
                    *base_path = value;
                }
                Command::none()
            }
            Message::ImportFolder(value) => {
                if let Some(LibraryForm::Import { folder, .. }) =
                    self.library_form.as_mut()
                {
                    *folder = value;
                }
                Command::none()
            }
            Message::ExportPath(value) => {
                if let Some(LibraryForm::Export { path }) =
                    self.library_form.as_mut()
                {
                    *path = value;
                }
                Command::none()
            }
            Message::CycleImportLauncher => {
                if let Some(LibraryForm::Import { launcher, .. }) =
                    self.library_form.as_mut()
                {
                    *launcher = match launcher {
                        ImportLauncherType::PrismLauncher => {
                            ImportLauncherType::MultiMC
                        }
                        ImportLauncherType::MultiMC => {
                            ImportLauncherType::ATLauncher
                        }
                        ImportLauncherType::ATLauncher => {
                            ImportLauncherType::GDLauncher
                        }
                        ImportLauncherType::GDLauncher => {
                            ImportLauncherType::Curseforge
                        }
                        ImportLauncherType::Curseforge
                        | ImportLauncherType::Unknown => {
                            ImportLauncherType::PrismLauncher
                        }
                    };
                }
                Command::none()
            }
            Message::CancelLibraryForm => {
                self.library_form = None;
                Command::none()
            }
            Message::StartImport => {
                let Some(LibraryForm::Import {
                    base_path,
                    folder,
                    launcher,
                }) = self.library_form.clone()
                else {
                    return Command::none();
                };
                if base_path.trim().is_empty() || folder.trim().is_empty() {
                    self.error = Some(
                        "Source path and instance folder are required".into(),
                    );
                    return Command::none();
                }
                self.status = "Importing instance…".into();
                Command::perform(
                    import_instance(base_path, folder, launcher),
                    Message::ImportStarted,
                )
            }
            Message::ImportStarted(result) => match result {
                Ok(job) => {
                    self.library_form = None;
                    self.status = "Instance import queued".into();
                    self.jobs.insert(0, JobView::from(job));
                    Command::perform(load_jobs(), Message::JobsLoaded)
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not import instance".into();
                    Command::none()
                }
            },
            Message::StartExport => {
                let Some(instance_id) = self.selected.clone() else {
                    return Command::none();
                };
                let Some(LibraryForm::Export { path }) =
                    self.library_form.clone()
                else {
                    return Command::none();
                };
                if path.trim().is_empty() {
                    self.error = Some("Export path is required".into());
                    return Command::none();
                }
                self.status = "Exporting instance…".into();
                Command::perform(
                    export_instance(instance_id, path),
                    Message::ExportDone,
                )
            }
            Message::ExportDone(result) => {
                self.library_form = None;
                match result {
                    Ok(path) => self.status = format!("Exported to {path}"),
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Could not export instance".into();
                    }
                }
                Command::none()
            }
            Message::DuplicateInstance(id) => {
                self.status = "Duplicating instance…".into();
                Command::perform(install::duplicate_instance(id), |result| {
                    Message::InstallStarted(result.map_err(describe))
                })
            }
            Message::ToggleJobDetails(id) => {
                self.expanded_job =
                    if self.expanded_job.as_deref() == Some(id.as_str()) {
                        None
                    } else {
                        Some(id)
                    };
                Command::none()
            }
            Message::PollJobs => {
                if !self.booted {
                    return Command::none();
                }
                Command::perform(load_jobs(), Message::JobsLoaded)
            }
            Message::JobsLoaded(result) => {
                let had_active = self.jobs.iter().any(|job| job.active);
                match result {
                    Ok(jobs) => {
                        let now_active = jobs.iter().any(|job| job.active);
                        self.jobs = jobs;
                        if had_active && !now_active {
                            self.status = "Install finished".into();
                            return Command::perform(
                                load_snapshot(),
                                Message::Refreshed,
                            );
                        }
                    }
                    Err(error) => {
                        self.status =
                            format!("Could not read install jobs: {error}");
                    }
                }
                Command::none()
            }
            Message::LoadSkins => {
                Command::perform(load_skins(), Message::SkinsLoaded)
            }
            Message::SkinsLoaded(result) => {
                self.skins_loaded = true;
                match result {
                    Ok(data) => {
                        self.skins = data.skins;
                        self.capes = data.capes;
                        if let Some(asset) = self
                            .skins
                            .iter()
                            .find(|asset| asset.skin.is_equipped)
                        {
                            self.player_head = asset
                                .texture
                                .as_deref()
                                .and_then(render_head_png);
                            self.player_skin = asset.avatar.clone();
                        }
                        if !self.skins.iter().any(|skin| {
                            matches!(
                                skin.skin.source,
                                minecraft_skins::SkinSource::Custom
                            )
                        }) {
                            self.skin_tab = SkinTab::Defaults;
                        }
                        self.reset_skin_selection();
                    }
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Could not load skins".into();
                    }
                }
                Command::none()
            }
            Message::SystemThemeChanged(light) => {
                self.system_light = light;
                Command::none()
            }
            Message::SkinTab(tab) => {
                self.skin_tab = tab;
                Command::none()
            }
            Message::SkinDragStart => {
                self.skin_dragging = true;
                self.skin_drag_last = None;
                Command::none()
            }
            Message::SkinDragMove(x, y) => {
                if !self.skin_dragging {
                    return Command::none();
                }
                if let Some((last_x, last_y)) = self.skin_drag_last {
                    let dx = x - last_x;
                    let dy = y - last_y;
                    if dx.abs() + dy.abs() > 2.0 {
                        self.skin_yaw += dx * 0.012;
                        self.skin_pitch =
                            (self.skin_pitch + dy * 0.012).clamp(-1.1, 1.1);
                        self.skin_drag_last = Some((x, y));
                        self.refresh_skin_preview();
                    }
                } else {
                    self.skin_drag_last = Some((x, y));
                }
                Command::none()
            }
            Message::SkinDragEnd => {
                self.skin_dragging = false;
                self.skin_drag_last = None;
                Command::none()
            }
            Message::SkinResetView => {
                self.skin_yaw = 0.5;
                self.skin_pitch = 0.12;
                self.refresh_skin_preview();
                Command::none()
            }
            Message::SelectSkin(index) => {
                if self.skins.get(index).is_some() {
                    self.selected_skin = Some(index);
                    self.selected_cape =
                        self.skins[index].skin.cape_id.and_then(|id| {
                            self.capes
                                .iter()
                                .position(|cape| cape.cape.id == id)
                        });
                    self.refresh_skin_preview();
                }
                Command::none()
            }
            Message::SelectCape(index) => {
                if index.is_none_or(|index| self.capes.get(index).is_some()) {
                    self.selected_cape = index;
                    self.refresh_skin_preview();
                }
                Command::none()
            }
            Message::ResetSkinSelection => {
                self.reset_skin_selection();
                Command::none()
            }
            Message::ApplySelectedSkin => {
                let Some(index) = self.selected_skin.or_else(|| {
                    self.skins.iter().position(|asset| asset.skin.is_equipped)
                }) else {
                    return Command::none();
                };
                let Some(asset) = self.skins.get(index) else {
                    return Command::none();
                };
                let mut skin = asset.skin.clone();
                skin.cape_id = self.selected_cape.and_then(|index| {
                    self.capes.get(index).map(|cape| cape.cape.id)
                });
                self.status = "Applying skin and cape…".into();
                Command::perform(equip_skin(skin), Message::SkinEquipped)
            }
            Message::SkinEquipped(result) => match result {
                Ok(()) => {
                    self.status = "Skin updated".into();
                    Command::batch([
                        Command::perform(load_skins(), Message::SkinsLoaded),
                        Command::perform(
                            load_account_appearance(),
                            Message::AccountAppearanceLoaded,
                        ),
                    ])
                }
                Err(error) => {
                    self.error = Some(error);
                    self.status = "Could not update skin".into();
                    Command::none()
                }
            },
            Message::AccountAppearanceLoaded(appearance) => {
                if let Some(appearance) = appearance {
                    self.player_head = Some(appearance.head);
                    self.player_skin = Some(appearance.skin);
                }
                Command::none()
            }
            Message::SettingChanged(change) => {
                let Some(settings) = self.settings.clone() else {
                    return Command::none();
                };
                let Some(next) = apply_setting(settings, change) else {
                    return Command::none();
                };
                self.settings = Some(next.clone());
                Command::perform(save_settings(next), Message::SettingsSaved)
            }
            Message::SettingsSaved(result) => {
                match result {
                    Ok(()) => self.status = "Settings saved".into(),
                    Err(error) => {
                        self.error = Some(error);
                        self.status = "Could not save settings".into();
                    }
                }
                Command::none()
            }
            Message::CheckUpdates => {
                if self.update.is_some() {
                    return Command::none();
                }
                Command::perform(update::check(), Message::UpdateChecked)
            }
            Message::UpdateChecked(result) => {
                match result {
                    Ok(Some(info)) => {
                        if self.dismissed_update.as_deref()
                            != Some(info.version.as_str())
                        {
                            self.status = format!(
                                "Ferriso Launcher {} is available",
                                info.version
                            );
                            self.update = Some(info);
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        eprintln!("native update check failed: {error}");
                    }
                }
                Command::none()
            }
            Message::DismissUpdate => {
                self.dismissed_update =
                    self.update.as_ref().map(|info| info.version.clone());
                self.update = None;
                Command::none()
            }
            Message::AnimationTick => {
                self.phase = (self.phase + 0.07) % std::f32::consts::TAU;
                let events: Vec<theseus::AppEvent> = EVENT_QUEUE
                    .get_or_init(|| Mutex::new(VecDeque::new()))
                    .lock()
                    .map(|mut queue| queue.drain(..).collect())
                    .unwrap_or_default();
                let mut reload = false;
                let mut refresh_processes = false;
                let mut refresh_jobs = false;
                for event in events {
                    match event {
                        theseus::AppEvent::InstallJob(job) => {
                            let view = JobView::from((*job).clone());
                            if let Some(existing) = self
                                .jobs
                                .iter_mut()
                                .find(|known| known.id == view.id)
                            {
                                *existing = view;
                            } else {
                                self.jobs.insert(0, view);
                            }
                            refresh_jobs = true;
                        }
                        theseus::AppEvent::Process(_) => {
                            refresh_processes = true;
                        }
                        theseus::AppEvent::Instance(_)
                        | theseus::AppEvent::InstanceGroupsChanged(_) => {
                            reload = true;
                        }
                        theseus::AppEvent::Warning(payload) => {
                            self.status = payload.message;
                            self.error = Some(self.status.clone());
                        }
                        theseus::AppEvent::Notification(payload) => {
                            self.status = payload;
                        }
                        theseus::AppEvent::OnboardingChecklist(checklist) => {
                            self.checklist = Some(checklist);
                        }
                        _ => {}
                    }
                }
                let mut commands = Vec::new();
                if reload {
                    commands.push(Command::perform(
                        load_snapshot(),
                        Message::Refreshed,
                    ));
                    commands.push(self.reload_detail());
                }
                if refresh_processes {
                    commands.push(Command::perform(
                        load_processes(),
                        Message::ProcessesLoaded,
                    ));
                }
                if refresh_jobs {
                    commands.push(Command::perform(
                        load_jobs(),
                        Message::JobsLoaded,
                    ));
                }
                Command::batch(commands)
            }
        }
    }

    fn view(&self) -> Self::View {
        self.view_model()
    }
}

async fn boot() -> Result<BootData, String> {
    theseus::EventState::init().await.map_err(describe)?;
    start_event_listener();
    State::init(APP_IDENTIFIER.to_owned())
        .await
        .map_err(describe)?;
    let mut data = load_snapshot().await?;
    if let Some(account) = data.modrinth_account.as_mut() {
        account.username = tokio::time::timeout(
            Duration::from_secs(4),
            users::get_user_profile(&account.user_id),
        )
        .await
        .ok()
        .and_then(Result::ok)
        .and_then(|profile| {
            profile
                .get("username")
                .and_then(Value::as_str)
                .map(str::to_string)
        });
    }
    data.latest_release = tokio::time::timeout(
        Duration::from_secs(4),
        metadata::get_minecraft_versions(),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .map(|manifest| manifest.latest.release);
    Ok(data)
}

async fn load_snapshot() -> Result<BootData, String> {
    let caches = State::get()
        .await
        .map_err(describe)?
        .directories
        .caches_dir();
    let mut instances = instance::list().await.map_err(describe)?;
    let mut accounts = minecraft_auth::users().await.map_err(describe)?;
    let settings = settings_api::get().await.map_err(describe)?;
    let jobs = install::list_jobs(true)
        .await
        .unwrap_or_default()
        .into_iter()
        .map(JobView::from)
        .take(12)
        .collect();
    let processes = process::get_all()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(ProcessView::from)
        .collect();
    let groups = instance::list_groups().await.unwrap_or_default();
    let modrinth_account =
        mr_auth::get_credentials()
            .await
            .ok()
            .flatten()
            .map(|credentials| ModrinthAccountView {
                user_id: credentials.user_id,
                username: None,
            });
    let checklist = onboarding_checklist::get().await.map_err(describe)?;
    let friends = if modrinth_account.is_some() {
        friends::friends()
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|friend| FriendView {
                user_id: friend.friend_id,
                accepted: friend.accepted,
            })
            .collect()
    } else {
        Vec::new()
    };

    instances.sort_by(|left, right| {
        right
            .instance
            .last_played
            .cmp(&left.instance.last_played)
            .then_with(|| left.instance.name.cmp(&right.instance.name))
    });
    accounts.sort_by_key(|account| !account.active);

    let instances = instances
        .into_iter()
        .map(|metadata| {
            let mut view = InstanceView::from(metadata);
            view.icon_path = view.icon_path.map(|path| {
                if path.is_absolute() {
                    path
                } else {
                    caches.join(path)
                }
            });
            view
        })
        .collect();

    Ok(BootData {
        instances,
        accounts: accounts.into_iter().map(AccountView::from).collect(),
        settings,
        jobs,
        processes,
        groups,
        modrinth_account,
        friends,
        checklist,
        latest_release: None,
    })
}

async fn load_jobs() -> Result<Vec<JobView>, String> {
    let mut jobs = install::list_jobs(true)
        .await
        .map_err(describe)?
        .into_iter()
        .map(JobView::from)
        .collect::<Vec<_>>();
    jobs.sort_by(|left, right| right.id.cmp(&left.id));
    jobs.truncate(12);
    Ok(jobs)
}

async fn search_projects(
    query: String,
    kind: ProjectKind,
) -> Result<Vec<ProjectView>, String> {
    let facets = format!("[[\"project_types:{}\"]]", kind.facet());
    let key = format!(
        "?query={}&limit=20&facets={}",
        urlencoding::encode(&query),
        urlencoding::encode(&facets)
    );
    let results = cache::get_search_results_v3(&key, None)
        .await
        .map_err(describe)?
        .ok_or_else(|| "Modrinth search returned no results".to_string())?;

    let projects: Vec<ProjectView> = results
        .result
        .hits
        .iter()
        .filter_map(ProjectView::from_hit)
        .collect();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(4))
        .build()
        .ok();

    Ok(
        futures::future::join_all(projects.into_iter().map(|mut project| {
            let client = client.clone();
            async move {
                if let (Some(client), Some(url)) =
                    (client, project.icon_url.as_deref())
                {
                    project.icon = fetch_project_icon(&client, url).await;
                }
                project
            }
        }))
        .await,
    )
}

async fn fetch_project_icon(
    client: &reqwest::Client,
    url: &str,
) -> Option<Bytes> {
    const MAX_ICON_BYTES: usize = 2 * 1024 * 1024;

    let response = client.get(url).send().await.ok()?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|size| size > MAX_ICON_BYTES as u64)
    {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    (bytes.len() <= MAX_ICON_BYTES).then_some(bytes)
}

async fn load_project(
    id: String,
    selected_instance: Option<String>,
) -> Result<ProjectPage, String> {
    let project = cache::get_project(&id, None)
        .await
        .map_err(describe)?
        .ok_or_else(|| "Project not found".to_string())?;
    let kind =
        ProjectKind::from_type(&project.project_type).ok_or_else(|| {
            format!("Unsupported project type: {}", project.project_type)
        })?;
    let versions = cache::get_project_versions(&id, None)
        .await
        .map_err(describe)?
        .unwrap_or_default();

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(6))
        .build()
        .map_err(|error| format!("Could not create image client: {error}"))?;
    let icon = match project.icon_url.as_deref() {
        Some(url) => fetch_project_icon(&client, url).await,
        None => None,
    };
    let gallery =
        futures::future::join_all(project.gallery.iter().take(8).map(|item| {
            let client = client.clone();
            async move {
                GalleryView {
                    title: item.title.clone(),
                    description: item.description.clone(),
                    image: fetch_project_icon(&client, &item.url).await,
                }
            }
        }))
        .await;

    let author = cache::get_team(&project.team, None)
        .await
        .ok()
        .flatten()
        .and_then(|members| {
            members
                .iter()
                .find(|member| member.is_owner)
                .or_else(|| members.first())
                .map(|member| member.user.username.clone())
        })
        .unwrap_or_else(|| {
            project
                .organization
                .clone()
                .unwrap_or_else(|| "Modrinth".into())
        });

    let mut choices: Vec<VersionChoice> = versions
        .into_iter()
        .take(40)
        .map(|version| VersionChoice {
            id: version.id,
            number: version.version_number,
            game_versions: version.game_versions,
            loaders: version.loaders,
            downloads: version.downloads,
        })
        .collect();

    let compatibility = if let Some(instance_id) = selected_instance.as_deref()
    {
        instance::get(instance_id)
            .await
            .ok()
            .flatten()
            .map(|metadata| {
                (
                    metadata.applied_content_set.game_version,
                    metadata.applied_content_set.loader.as_str().to_string(),
                )
            })
    } else {
        None
    };
    let selected_version = compatibility
        .as_ref()
        .and_then(|(game, loader)| {
            choices.iter().position(|version| {
                version.game_versions.iter().any(|value| value == game)
                    && (kind != ProjectKind::Mod
                        || version.loaders.iter().any(|value| value == loader))
            })
        })
        .or_else(|| (!choices.is_empty()).then_some(0));

    let view = ProjectView {
        id: project.id,
        slug: project.slug.unwrap_or_default(),
        title: project.title,
        summary: project.description,
        kind,
        icon_url: project.icon_url,
        icon,
        downloads: project.downloads as u64,
        author,
        version_id: None,
    };

    Ok(ProjectPage {
        id,
        loading: false,
        error: None,
        project: Some(view),
        body: project.body,
        gallery,
        versions: std::mem::take(&mut choices),
        selected_version,
        selected_instance,
        installing: false,
        install_error: None,
        install_result: None,
    })
}

async fn install_modpack(
    project: ProjectView,
) -> Result<InstallJobSnapshot, String> {
    let version_id = project
        .version_id
        .ok_or_else(|| "This project has no installable version".to_string())?;
    let location = CreatePackLocation::FromVersionId {
        project_id: project.id,
        version_id,
        title: project.title,
        icon_url: project.icon_url,
    };
    install::create_modpack_instance(location, None)
        .await
        .map_err(describe)
}

async fn install_to_instance(
    instance_id: String,
    project: ProjectView,
) -> Result<String, String> {
    install_to_instance_version(instance_id, project, None).await
}

async fn install_to_instance_version(
    instance_id: String,
    project: ProjectView,
    pinned_version: Option<String>,
) -> Result<String, String> {
    let metadata = instance::get(&instance_id)
        .await
        .map_err(describe)?
        .ok_or_else(|| "Unknown instance".to_string())?;
    let content_set = &metadata.applied_content_set;
    let versions = cache::get_project_versions(&project.id, None)
        .await
        .map_err(describe)?
        .ok_or_else(|| "This project has no published versions".to_string())?;

    let version = match pinned_version {
        Some(version_id) => versions
            .iter()
            .find(|version| version.id == version_id)
            .ok_or_else(|| {
                "Selected version is no longer available".to_string()
            })?,
        None => versions
            .iter()
            .find(|version| {
                let game_ok = version
                    .game_versions
                    .iter()
                    .any(|version| version == &content_set.game_version);
                let loader_ok = project.kind != ProjectKind::Mod
                    || version
                        .loaders
                        .iter()
                        .any(|loader| loader == content_set.loader.as_str());
                game_ok && loader_ok
            })
            .ok_or_else(|| {
                format!(
                    "No compatible version for Minecraft {} / {}",
                    content_set.game_version,
                    content_set.loader.as_str()
                )
            })?,
    };
    let version_id = version.id.clone();
    let content_type = match project.kind {
        ProjectKind::Mod => ContentType::Mod,
        ProjectKind::ResourcePack => ContentType::ResourcePack,
        ProjectKind::Shader => ContentType::Shader,
        ProjectKind::Modpack => ContentType::ModPack,
    };
    let title = project.title.clone();

    let plan = instance::install_project_with_dependencies(
        &instance_id,
        InstallProjectWithDependenciesRequest {
            project_id: project.id.clone(),
            version_id: Some(version_id),
            content_type,
            selected: ResolutionPreferences::default(),
        },
    )
    .await
    .map_err(describe)?;

    // The content-management API applies the plan in a spawned task. Wait for
    // its database row so the detail view does not refresh against a half
    // installed set.
    let primary = plan.primary.project_id.clone();
    for _ in 0..120 {
        let installed = instance::get_content_items(&instance_id, None)
            .await
            .unwrap_or_default()
            .iter()
            .any(|item| {
                item.project
                    .as_ref()
                    .is_some_and(|project| project.id == primary)
            });
        if installed {
            return Ok(format!("Installed {title}"));
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }

    Err(format!("Install of {title} did not complete"))
}

async fn install_project_version(
    project: ProjectView,
    version_id: String,
    instance_id: Option<String>,
) -> Result<String, String> {
    if project.kind == ProjectKind::Modpack {
        let location = CreatePackLocation::FromVersionId {
            project_id: project.id,
            version_id,
            title: project.title,
            icon_url: project.icon_url,
        };
        install::create_modpack_instance(location, None)
            .await
            .map_err(describe)?;
        return Ok("Modpack installation queued".to_string());
    }
    let instance_id =
        instance_id.ok_or_else(|| "Select an instance first".to_string())?;
    install_to_instance_version(instance_id, project, Some(version_id)).await
}

async fn load_detail(instance_id: String) -> Result<DetailData, String> {
    let content = instance::get_content_items(&instance_id, None)
        .await
        .map_err(describe)?;
    let worlds = worlds::get_instance_worlds(&instance_id)
        .await
        .unwrap_or_default();
    let logs = logs::get_logs(&instance_id, Some(true))
        .await
        .unwrap_or_default();

    let content = content
        .into_iter()
        .map(|item| ContentRow {
            title: item
                .project
                .as_ref()
                .map(|project| project.title.clone())
                .or_else(|| {
                    item.embedded_metadata
                        .as_ref()
                        .and_then(|metadata| metadata.name.clone())
                })
                .unwrap_or_else(|| item.file_name.clone()),
            project_id: item.project.as_ref().map(|project| project.id.clone()),
            version_id: item.version.as_ref().map(|version| version.id.clone()),
            file_name: item.file_name,
            file_path: item.file_path,
            version: item.version.map(|version| version.version_number),
            enabled: item.enabled,
            locked: item.locked,
            has_update: item.has_update,
            update_version_id: item.update_version_id,
        })
        .collect();

    let worlds = worlds
        .into_iter()
        .map(|world| {
            let icon_key = world.world_id().to_string();
            let icon = match world.icon.as_ref() {
                Some(Either::Left(path)) => Some(WorldArt::Path(path.clone())),
                Some(Either::Right(url)) => {
                    decode_data_icon(url.as_str()).map(WorldArt::Bytes)
                }
                None => None,
            };
            let (server_index, pack_status) = match &world.details {
                worlds::WorldDetails::Server {
                    index, pack_status, ..
                } => {
                    (Some(*index), pack_status_string(*pack_status).to_string())
                }
                worlds::WorldDetails::Singleplayer { .. } => {
                    (None, String::new())
                }
            };
            WorldRow {
                name: world.name.clone(),
                kind: world.world_type().as_str().to_string(),
                detail: world.world_id().to_string(),
                last_played: world
                    .last_played
                    .map(|date| date.format("%Y-%m-%d %H:%M").to_string()),
                icon,
                icon_key,
                server_index,
                pack_status,
            }
        })
        .collect();

    let logs = logs
        .into_iter()
        .map(|log| LogRow {
            crash: log.log_type == logs::LogType::CrashReport,
            filename: log.filename,
            kind: format!("{:?}", log.log_type),
            age_seconds: log.age,
        })
        .collect();

    Ok(DetailData {
        content,
        worlds,
        logs,
    })
}

fn decode_data_icon(url: &str) -> Option<Bytes> {
    let (metadata, payload) = url.split_once(',')?;
    if !metadata.starts_with("data:image/") {
        return None;
    }
    base64::engine::general_purpose::STANDARD
        .decode(payload)
        .ok()
        .map(Bytes::from)
}

async fn set_content_enabled(
    instance_id: String,
    path: String,
    enabled: bool,
) -> Result<(), String> {
    instance::toggle_disable_project(&instance_id, &path, Some(enabled))
        .await
        .map(|_| ())
        .map_err(describe)
}

async fn remove_content(
    instance_id: String,
    path: String,
) -> Result<(), String> {
    instance::remove_project(&instance_id, &path)
        .await
        .map_err(describe)
}

async fn cancel_job(id: String) -> Result<(), String> {
    let id = Uuid::parse_str(&id).map_err(describe)?;
    install::cancel_job(id).await.map(|_| ()).map_err(describe)
}

async fn retry_job(id: String) -> Result<(), String> {
    let id = Uuid::parse_str(&id).map_err(describe)?;
    install::retry_job(id).await.map(|_| ()).map_err(describe)
}

async fn dismiss_job(id: String) -> Result<(), String> {
    let id = Uuid::parse_str(&id).map_err(describe)?;
    install::dismiss_job(id).await.map_err(describe)
}

async fn save_instance(
    instance_id: String,
    name: String,
    memory: Option<Option<u32>>,
    java_path: Option<Option<String>>,
    extra_launch_args: Option<Option<Vec<String>>>,
    icon_path: Option<String>,
    group_ids: Vec<String>,
) -> Result<(), String> {
    let launch_overrides = Some(InstanceLaunchOverridesPatch {
        memory: memory
            .map(|maximum| maximum.map(|maximum| MemorySettings { maximum })),
        java_path,
        extra_launch_args,
        ..InstanceLaunchOverridesPatch::default()
    });
    instance::edit(
        &instance_id,
        EditInstance {
            name: Some(name),
            launch_overrides,
            ..EditInstance::default()
        },
    )
    .await
    .map_err(describe)?;
    if let Some(icon_path) = icon_path {
        instance::edit_icon(
            &instance_id,
            Some(std::path::Path::new(&icon_path)),
        )
        .await
        .map_err(describe)?;
    }
    instance::set_group_memberships(vec![
        instance::InstanceGroupMembershipUpdate {
            instance_id,
            group_ids,
        },
    ])
    .await
    .map_err(describe)
}

async fn load_shared(instance_id: String) -> Result<SharedData, String> {
    let metadata = instance::get(&instance_id)
        .await
        .map_err(describe)?
        .ok_or_else(|| "Unknown instance".to_string())?;
    let Some(attachment) = metadata.shared_instance else {
        return Ok(SharedData {
            users: Vec::new(),
            invites: Vec::new(),
        });
    };
    let users = instance::get_shared_instance_users(&instance_id)
        .await
        .ok()
        .map(|users| users.users)
        .unwrap_or_default()
        .into_iter()
        .map(|user| SharedUserView {
            id: user.id,
            join_type: match user.join_type {
                instance::SharedInstanceJoinType::Owner => "owner",
                instance::SharedInstanceJoinType::Invite => "invite",
                instance::SharedInstanceJoinType::Link => "link",
            }
            .to_string(),
        })
        .collect();
    let invites = if attachment.role.is_member() {
        Vec::new()
    } else {
        instance::get_shared_instance_invites(&instance_id)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|invite| SharedInviteView {
                id: invite.id,
                expiration: invite
                    .expiration
                    .format("%Y-%m-%d %H:%M")
                    .to_string(),
                max_uses: invite.max_uses,
                uses: invite.uses,
            })
            .collect()
    };
    Ok(SharedData { users, invites })
}

async fn publish_shared(instance_id: String) -> Result<(), String> {
    instance::publish_shared_instance(&instance_id, Vec::new())
        .await
        .map(|_| ())
        .map_err(describe)
}

async fn unpublish_shared(instance_id: String) -> Result<(), String> {
    instance::unpublish_shared_instance(&instance_id)
        .await
        .map_err(describe)
}

async fn unlink_shared(instance_id: String) -> Result<(), String> {
    instance::unlink_shared_instance(&instance_id)
        .await
        .map_err(describe)
}

async fn invite_shared_users(
    instance_id: String,
    user_ids: Vec<String>,
) -> Result<(), String> {
    instance::invite_shared_instance_users(&instance_id, user_ids)
        .await
        .map(|_| ())
        .map_err(describe)
}

async fn remove_shared_user(
    instance_id: String,
    user_id: String,
) -> Result<(), String> {
    instance::remove_shared_instance_users(&instance_id, vec![user_id], false)
        .await
        .map(|_| ())
        .map_err(describe)
}

async fn create_shared_invite(instance_id: String) -> Result<String, String> {
    instance::create_shared_instance_invite_link(&instance_id, None, None, None)
        .await
        .map(|link| link.invite_id)
        .map_err(describe)
}

async fn revoke_shared_invite(
    instance_id: String,
    invite_id: String,
) -> Result<(), String> {
    instance::revoke_shared_instance_invite(&instance_id, invite_id)
        .await
        .map_err(describe)
}

async fn update_shared(
    instance_id: String,
) -> Result<InstallJobSnapshot, String> {
    instance::update_shared_instance(&instance_id)
        .await
        .map_err(describe)
}

fn extract_invite_id(input: &str) -> String {
    let trimmed = input.trim().trim_end_matches('/');
    if trimmed.is_empty() {
        return String::new();
    }
    let tail = trimmed.rsplit('/').next().unwrap_or(trimmed);
    tail.split(['?', '#']).next().unwrap_or(tail).to_string()
}

async fn preview_join(
    invite_id: String,
) -> Result<instance::SharedInstanceInviteInstallPreview, String> {
    instance::accept_shared_instance_invite_for_install(&invite_id)
        .await
        .map_err(describe)
}

async fn install_shared(
    preview: instance::SharedInstanceInviteInstallPreview,
) -> Result<InstallJobSnapshot, String> {
    instance::install_shared_instance(
        &preview.shared_instance_id,
        preview.preview.name.clone(),
        preview.manager_id.clone(),
        preview.server_manager_name.clone(),
        preview.server_manager_icon_url.clone(),
        preview.instance_icon_url.clone(),
    )
    .await
    .map_err(describe)
}

async fn modrinth_login() -> Result<ModrinthAccountView, String> {
    let listener =
        theseus::prelude::tcp_listen_any_loopback()
            .await
            .map_err(|error| {
                format!("Could not start the Modrinth callback: {error}")
            })?;
    let address = listener.local_addr().map_err(|error| {
        format!("Could not read the callback address: {error}")
    })?;
    let ipver = if address.is_ipv4() { 4 } else { 6 };
    let url = format!(
        "{}?launcher=true&ipver={ipver}&port={}",
        mr_auth::authenticate_begin_flow(mr_auth::ModrinthAuthFlow::SignIn),
        address.port()
    );
    webbrowser::open(&url).map_err(|error| {
        format!("Could not open the Modrinth sign-in page: {error}")
    })?;

    let code = tokio::time::timeout(
        Duration::from_secs(300),
        accept_auth_code(listener),
    )
    .await
    .map_err(|_| "Timed out waiting for Modrinth sign-in".to_string())??;

    mr_auth::authenticate_finish_flow(&code)
        .await
        .map_err(describe)?;

    let username = tokio::time::timeout(
        Duration::from_secs(5),
        users::get_user_profile(
            &mr_auth::get_credentials()
                .await
                .map_err(describe)?
                .ok_or_else(|| "Modrinth sign-in did not persist".to_string())?
                .user_id,
        ),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .and_then(|profile| {
        profile
            .get("username")
            .and_then(Value::as_str)
            .map(str::to_string)
    });

    let user_id = mr_auth::get_credentials()
        .await
        .map_err(describe)?
        .ok_or_else(|| "Modrinth sign-in did not persist".to_string())?
        .user_id;
    Ok(ModrinthAccountView { user_id, username })
}

async fn accept_auth_code(
    listener: tokio::net::TcpListener,
) -> Result<String, String> {
    loop {
        let (mut socket, _) = listener
            .accept()
            .await
            .map_err(|error| format!("Modrinth callback failed: {error}"))?;
        let mut buffer = [0_u8; 8192];
        let read = socket.read(&mut buffer).await.map_err(|error| {
            format!("Modrinth callback read failed: {error}")
        })?;
        let request = String::from_utf8_lossy(&buffer[..read]);
        let path = request
            .lines()
            .next()
            .and_then(|line| line.split_whitespace().nth(1))
            .unwrap_or_default();
        if let Some(code) = auth_code_from_path(path) {
            let _ = socket
                .write_all(
                    b"HTTP/1.1 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n<!doctype html><meta charset=\"utf-8\"><title>Signed in</title><body style=\"font-family:sans-serif;background:#0b1118;color:#ecf2f8;padding:3rem\"><h1>Signed in to Modrinth</h1><p>You can close this tab and return to the launcher.</p>",
                )
                .await;
            return Ok(code);
        }
        let _ = socket
            .write_all(
                b"HTTP/1.1 404 Not Found\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\nWaiting for Modrinth sign-in.",
            )
            .await;
    }
}

fn auth_code_from_path(path: &str) -> Option<String> {
    let query = path.split_once('?')?.1;
    for pair in query.split('&') {
        let (key, value) = pair.split_once('=')?;
        if key == "code" {
            let value = urlencoding::decode(value).ok()?;
            let value = value.trim();
            if !value.is_empty() && value.len() <= 4096 {
                return Some(value.to_string());
            }
        }
    }
    None
}

async fn logout_modrinth() -> Result<(), String> {
    mr_auth::logout().await.map_err(describe)
}

async fn add_friend(user_id: String) -> Result<(), String> {
    friends::add_friend(&user_id).await.map_err(describe)
}

async fn remove_friend(user_id: String) -> Result<(), String> {
    friends::remove_friend(&user_id).await.map_err(describe)
}

async fn save_group(form: GroupForm) -> Result<(), String> {
    let name = form.name.trim().to_string();
    match form.id {
        Some(id) => instance::rename_group(id, name).await.map(|_| ()),
        None => instance::create_group(name).await.map(|_| ()),
    }
    .map_err(describe)
}

async fn delete_group(id: String) -> Result<String, String> {
    instance::delete_group(id.clone()).await.map_err(describe)?;
    Ok(id)
}

async fn clear_instance_icon(instance_id: String) -> Result<(), String> {
    instance::edit_icon(&instance_id, None)
        .await
        .map_err(describe)
}

async fn delete_instance(instance_id: String) -> Result<(), String> {
    instance::remove(&instance_id).await.map_err(describe)
}

async fn import_instance(
    base_path: String,
    folder: String,
    launcher: ImportLauncherType,
) -> Result<InstallJobSnapshot, String> {
    install::import_instance(
        launcher,
        PathBuf::from(base_path.trim()),
        folder.trim().to_string(),
    )
    .await
    .map_err(describe)
}

async fn export_instance(
    instance_id: String,
    path: String,
) -> Result<String, String> {
    let candidates = instance::get_pack_export_candidates(&instance_id)
        .await
        .map_err(describe)?;
    let included = candidates
        .iter()
        .map(|candidate| candidate.path.as_str().to_string())
        .collect();
    instance::export_mrpack(
        &instance_id,
        PathBuf::from(path.trim()),
        included,
        Vec::new(),
        None,
        None,
        None,
    )
    .await
    .map_err(describe)?;
    Ok(path)
}

async fn save_world_form(
    instance_id: String,
    form: WorldForm,
) -> Result<(), String> {
    let instance_path = instance::get_full_path(&instance_id)
        .await
        .map_err(describe)?;
    match form.kind {
        WorldFormKind::AddServer => {
            let name = form.name.trim();
            let address = form.address.trim();
            if name.is_empty() || address.is_empty() {
                return Err("Server name and address are required".into());
            }
            worlds::add_server_to_instance(
                &instance_id,
                name.to_string(),
                address.to_string(),
                parse_pack_status(&form.pack_status),
                None,
                None,
            )
            .await
            .map(|_| ())
            .map_err(describe)
        }
        WorldFormKind::EditServer { index } => {
            let name = form.name.trim();
            let address = form.address.trim();
            if name.is_empty() || address.is_empty() {
                return Err("Server name and address are required".into());
            }
            worlds::edit_server_in_instance(
                &instance_id,
                index,
                name.to_string(),
                address.to_string(),
                parse_pack_status(&form.pack_status),
            )
            .await
            .map_err(describe)
        }
        WorldFormKind::RenameWorld { path } => {
            let name = form.name.trim();
            if name.is_empty() {
                return Err("World name cannot be empty".into());
            }
            worlds::rename_world(&instance_path, &path, name)
                .await
                .map_err(describe)
        }
        WorldFormKind::DeleteWorld { path, .. } => {
            worlds::delete_world(&instance_path, &path)
                .await
                .map_err(describe)
        }
        WorldFormKind::DeleteServer { index, .. } => {
            worlds::remove_server_from_instance(&instance_id, index)
                .await
                .map_err(describe)
        }
    }
}

async fn backup_world(
    instance_id: String,
    path: String,
) -> Result<u64, String> {
    let instance_path = instance::get_full_path(&instance_id)
        .await
        .map_err(describe)?;
    worlds::backup_world(&instance_path, &path)
        .await
        .map_err(describe)
}

fn parse_pack_status(value: &str) -> worlds::ServerPackStatus {
    match value {
        "enabled" => worlds::ServerPackStatus::Enabled,
        "disabled" => worlds::ServerPackStatus::Disabled,
        _ => worlds::ServerPackStatus::Prompt,
    }
}

async fn load_required_java(
    instance_id: String,
) -> Result<Option<u32>, String> {
    instance::get_required_java_major(&instance_id)
        .await
        .map_err(describe)
}

async fn install_java(version: u32) -> Result<String, String> {
    jre::auto_install_java(version)
        .await
        .map(|path| path.to_string_lossy().to_string())
        .map_err(describe)
}

async fn detect_java() -> Result<Vec<JreChoice>, String> {
    jre::find_filtered_jres(None)
        .await
        .map(|jres| {
            jres.into_iter()
                .map(|jre| JreChoice {
                    path: jre.path,
                    version: jre.version,
                    parsed_version: jre.parsed_version,
                })
                .collect()
        })
        .map_err(describe)
}

async fn load_processes() -> Result<Vec<ProcessView>, String> {
    process::get_all()
        .await
        .map(|processes| processes.into_iter().map(ProcessView::from).collect())
        .map_err(describe)
}

async fn kill_process(uuid: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&uuid).map_err(describe)?;
    process::kill(uuid).await.map_err(describe)
}

async fn load_log(
    instance_id: String,
    filename: String,
    crash: bool,
) -> Result<String, String> {
    let log_type = if crash {
        logs::LogType::CrashReport
    } else {
        logs::LogType::InfoLog
    };
    logs::get_output_by_filename(&instance_id, log_type, &filename)
        .await
        .map(logs::CensoredString::into_inner)
        .map_err(describe)
}

async fn delete_log(
    instance_id: String,
    filename: String,
    crash: bool,
) -> Result<(), String> {
    let log_type = if crash {
        logs::LogType::CrashReport
    } else {
        logs::LogType::InfoLog
    };
    logs::delete_logs_by_filename(&instance_id, log_type, &filename)
        .await
        .map_err(describe)
}

async fn delete_all_logs(instance_id: String) -> Result<(), String> {
    logs::delete_logs(&instance_id).await.map_err(describe)
}

async fn update_all_content(instance_id: String) -> Result<String, String> {
    let updated = instance::update_all_projects(&instance_id)
        .await
        .map_err(describe)?;
    Ok(format!("Updated {} content item(s)", updated.len()))
}

async fn update_content(
    instance_id: String,
    path: String,
) -> Result<String, String> {
    instance::update_project(&instance_id, &path, None)
        .await
        .map_err(describe)?;
    Ok("Content updated".into())
}

async fn set_content_lock(
    instance_id: String,
    path: String,
    locked: bool,
) -> Result<(), String> {
    instance::set_project_locked(&instance_id, &path, locked)
        .await
        .map_err(describe)
}

async fn load_versions(
    instance_id: String,
    project_id: String,
) -> Result<Vec<VersionChoice>, String> {
    let game_version = instance::get(&instance_id)
        .await
        .map_err(describe)?
        .map(|metadata| metadata.applied_content_set.game_version);
    let versions = cache::get_project_versions(&project_id, None)
        .await
        .map_err(describe)?
        .ok_or_else(|| "This project has no published versions".to_string())?;
    Ok(versions
        .into_iter()
        .filter(|version| {
            game_version.as_ref().is_none_or(|game_version| {
                version
                    .game_versions
                    .iter()
                    .any(|value| value == game_version)
            })
        })
        .map(|version| VersionChoice {
            id: version.id,
            number: version.version_number,
            game_versions: version.game_versions,
            loaders: version.loaders,
            downloads: version.downloads,
        })
        .collect())
}

async fn switch_content_version(
    instance_id: String,
    path: String,
    version_id: String,
) -> Result<String, String> {
    instance::switch_project_version_with_dependencies(
        &instance_id,
        &path,
        &version_id,
    )
    .await
    .map(|_| "Content version switched".into())
    .map_err(describe)
}

async fn create_instance(
    name: String,
    game_version: String,
    loader: ModLoader,
) -> Result<InstallJobSnapshot, String> {
    install::create_instance(
        name,
        game_version,
        loader,
        None,
        None,
        None,
        InstanceLink::Unmanaged,
    )
    .await
    .map_err(describe)
}

async fn run_instance(id: String) -> Result<ProcessView, String> {
    instance::run(&id, QuickPlayType::None)
        .await
        .map(ProcessView::from)
        .map_err(describe)
}

async fn begin_login() -> Result<LoginStarted, String> {
    let flow = minecraft_auth::begin_login().await.map_err(describe)?;
    let url = flow.auth_request_uri.clone();
    let opened = webbrowser::open(&url).is_ok();
    let finish: FinishLogin = Box::new(move |code| {
        Box::pin(async move {
            minecraft_auth::finish_login(&code, flow)
                .await
                .map(|credentials| credentials.offline_profile.name)
                .map_err(describe)
        })
    });

    Ok(LoginStarted {
        url,
        opened,
        finish: Arc::new(Mutex::new(Some(finish))),
    })
}

async fn set_default_account(id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(describe)?;
    minecraft_auth::set_default_user(uuid)
        .await
        .map_err(describe)
}

async fn remove_account(id: String) -> Result<(), String> {
    let uuid = Uuid::parse_str(&id).map_err(describe)?;
    minecraft_auth::remove_user(uuid).await.map_err(describe)
}

#[derive(Clone, Debug)]
pub struct PlayerAppearance {
    pub head: Bytes,
    pub skin: Bytes,
}

async fn load_account_appearance() -> Option<PlayerAppearance> {
    let skins = minecraft_skins::get_available_skins().await.ok()?;
    let equipped = skins.into_iter().find(|skin| skin.is_equipped)?;
    let capes = minecraft_skins::get_available_capes()
        .await
        .unwrap_or_default();
    let cape = capes.into_iter().find(|cape| cape.is_equipped);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .ok()?;
    let texture = fetch_texture(&client, equipped.texture.as_str()).await?;
    let cape_texture = match cape {
        Some(cape) => fetch_texture(&client, cape.texture.as_str()).await,
        None => None,
    };
    let slim = matches!(
        equipped.variant,
        minecraft_skins::MinecraftSkinVariant::Slim
    );
    Some(PlayerAppearance {
        head: render_head_png(&texture)?,
        skin: render_avatar_png(&texture, cape_texture.as_deref(), slim)?,
    })
}

async fn load_skins() -> Result<SkinsData, String> {
    let skins = minecraft_skins::get_available_skins()
        .await
        .map_err(describe)?;
    let capes = minecraft_skins::get_available_capes()
        .await
        .unwrap_or_default();
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(8))
        .build()
        .map_err(describe)?;

    let cape_textures = futures::future::join_all(capes.iter().map(|cape| {
        let client = client.clone();
        async move {
            let texture = fetch_texture(&client, cape.texture.as_str()).await;
            (cape.id, texture)
        }
    }))
    .await;
    let cape_textures: std::collections::HashMap<_, _> = cape_textures
        .into_iter()
        .filter_map(|(id, texture)| texture.map(|texture| (id, texture)))
        .collect();

    let mut assets = Vec::with_capacity(skins.len());
    for skin in skins {
        let texture = fetch_texture(&client, skin.texture.as_str()).await;
        let cape_texture = skin
            .cape_id
            .and_then(|id| cape_textures.get(&id))
            .map(Bytes::as_ref);
        let slim =
            matches!(skin.variant, minecraft_skins::MinecraftSkinVariant::Slim);
        let avatar = texture
            .as_deref()
            .and_then(|texture| render_avatar_png(texture, cape_texture, slim));
        let cape_name = skin.cape_id.and_then(|id| {
            capes
                .iter()
                .find(|cape| cape.id == id)
                .map(|cape| cape.name.to_string())
        });
        assets.push(SkinAsset {
            skin,
            texture,
            avatar,
            cape_name,
        });
    }

    let capes = capes
        .into_iter()
        .map(|cape| {
            let texture = cape_textures.get(&cape.id).cloned();
            let preview = texture.as_deref().and_then(render_cape_png);
            CapeAsset {
                cape,
                texture,
                preview,
            }
        })
        .collect();

    Ok(SkinsData {
        skins: assets,
        capes,
    })
}

async fn fetch_texture(
    client: &reqwest::Client,
    source: &str,
) -> Option<Bytes> {
    const MAX_TEXTURE_BYTES: usize = 2 * 1024 * 1024;
    if let Some(data) = source.strip_prefix("data:") {
        let (_, payload) = data.split_once(',')?;
        let decoded = base64::engine::general_purpose::STANDARD
            .decode(payload)
            .or_else(|_| {
                base64::engine::general_purpose::STANDARD_NO_PAD.decode(payload)
            })
            .ok()?;
        return (decoded.len() <= MAX_TEXTURE_BYTES)
            .then(|| Bytes::from(decoded));
    }
    let response = client.get(source).send().await.ok()?;
    if !response.status().is_success()
        || response
            .content_length()
            .is_some_and(|size| size > MAX_TEXTURE_BYTES as u64)
    {
        return None;
    }
    let bytes = response.bytes().await.ok()?;
    (bytes.len() <= MAX_TEXTURE_BYTES).then_some(bytes)
}

fn render_head_png(skin_bytes: &[u8]) -> Option<Bytes> {
    let skin = image::load_from_memory(skin_bytes).ok()?.to_rgba8();
    let mut head = RgbaImage::from_pixel(8, 8, Rgba([0, 0, 0, 0]));
    blit_region(&skin, (8, 8, 8, 8), &mut head, 0, 0);
    blit_region(&skin, (40, 8, 8, 8), &mut head, 0, 0);
    encode_png(&head)
}

fn render_avatar_png(
    skin_bytes: &[u8],
    cape_bytes: Option<&[u8]>,
    slim: bool,
) -> Option<Bytes> {
    let skin = image::load_from_memory(skin_bytes).ok()?.to_rgba8();
    let cape = cape_bytes
        .and_then(|bytes| image::load_from_memory(bytes).ok())
        .map(|image| image.to_rgba8());
    let avatar = compose_avatar(&skin, cape.as_ref(), slim);
    encode_png(&avatar)
}

fn render_cape_png(cape_bytes: &[u8]) -> Option<Bytes> {
    let cape = image::load_from_memory(cape_bytes).ok()?.to_rgba8();
    let mut preview = RgbaImage::from_pixel(10, 16, Rgba([0, 0, 0, 0]));
    blit_region(&cape, (1, 1, 10, 16), &mut preview, 0, 0);
    encode_png(&preview)
}

fn encode_png(image: &RgbaImage) -> Option<Bytes> {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            image.as_raw(),
            image.width(),
            image.height(),
            ExtendedColorType::Rgba8,
        )
        .ok()?;
    Some(Bytes::from(bytes))
}

fn compose_avatar(
    skin: &RgbaImage,
    cape: Option<&RgbaImage>,
    slim: bool,
) -> RgbaImage {
    const WIDTH: u32 = 16;
    const HEIGHT: u32 = 32;
    let mut output = RgbaImage::from_pixel(WIDTH, HEIGHT, Rgba([0, 0, 0, 0]));

    if let Some(cape) = cape {
        blit_scaled(cape, (1, 1, 10, 16), &mut output, 2, 8, 12, 22);
    }

    let leg_width = 4;
    let arm_width = if slim { 3 } else { 4 };
    // Cape is behind the body. Legs, torso, arms, then head match the
    // standard front-facing skin layout.
    if !blit_region(skin, (4, 20, leg_width, 12), &mut output, 4, 20) {
        blit_region(skin, (20, 52, leg_width, 12), &mut output, 4, 20);
    }
    if !blit_region(skin, (20, 52, leg_width, 12), &mut output, 8, 20) {
        blit_region(skin, (4, 20, leg_width, 12), &mut output, 8, 20);
    }
    blit_region(skin, (20, 20, 8, 12), &mut output, 4, 8);
    let right_x = if slim { 1 } else { 0 };
    let left_x = 12;
    if !blit_region(skin, (44, 20, arm_width, 12), &mut output, right_x, 8) {
        blit_region(skin, (44, 20, 4, 12), &mut output, 0, 8);
    }
    if !blit_region(skin, (36, 52, arm_width, 12), &mut output, left_x, 8) {
        blit_region(skin, (44, 20, 4, 12), &mut output, 12, 8);
    }
    blit_region(skin, (8, 8, 8, 8), &mut output, 4, 0);

    // Hat, jacket, sleeve and leg overlays are optional transparent layers.
    blit_region(skin, (40, 8, 8, 8), &mut output, 4, 0);
    blit_region(skin, (20, 36, 8, 12), &mut output, 4, 8);
    blit_region(skin, (44, 36, arm_width, 12), &mut output, right_x, 8);
    blit_region(skin, (36, 52, arm_width, 12), &mut output, left_x, 8);
    blit_region(skin, (4, 36, leg_width, 12), &mut output, 4, 20);
    blit_region(skin, (20, 52, leg_width, 12), &mut output, 8, 20);

    output
}

fn blit_region(
    source: &RgbaImage,
    source_rect: (u32, u32, u32, u32),
    destination: &mut RgbaImage,
    dest_x: u32,
    dest_y: u32,
) -> bool {
    let (sx, sy, width, height) = source_rect;
    if sx + width > source.width()
        || sy + height > source.height()
        || dest_x + width > destination.width()
        || dest_y + height > destination.height()
    {
        return false;
    }
    for y in 0..height {
        for x in 0..width {
            blend_pixel(
                destination.get_pixel_mut(dest_x + x, dest_y + y),
                *source.get_pixel(sx + x, sy + y),
            );
        }
    }
    true
}

fn blit_scaled(
    source: &RgbaImage,
    source_rect: (u32, u32, u32, u32),
    destination: &mut RgbaImage,
    dest_x: u32,
    dest_y: u32,
    width: u32,
    height: u32,
) -> bool {
    let (sx, sy, sw, sh) = source_rect;
    if sx + sw > source.width()
        || sy + sh > source.height()
        || dest_x + width > destination.width()
        || dest_y + height > destination.height()
    {
        return false;
    }
    for y in 0..height {
        for x in 0..width {
            let sample_x = sx + x * sw / width;
            let sample_y = sy + y * sh / height;
            blend_pixel(
                destination.get_pixel_mut(dest_x + x, dest_y + y),
                *source.get_pixel(sample_x, sample_y),
            );
        }
    }
    true
}

fn blend_pixel(destination: &mut Rgba<u8>, source: Rgba<u8>) {
    let alpha = u16::from(source[3]);
    if alpha == 0 {
        return;
    }
    if alpha == 255 {
        *destination = source;
        return;
    }
    let dest_alpha = u16::from(destination[3]);
    let out_alpha = alpha + dest_alpha * (255 - alpha) / 255;
    if out_alpha == 0 {
        *destination = Rgba([0, 0, 0, 0]);
        return;
    }
    let channel = |dst: u8, src: u8| {
        ((u16::from(src) * alpha
            + u16::from(dst) * dest_alpha * (255 - alpha) / 255)
            / out_alpha) as u8
    };
    *destination = Rgba([
        channel(destination[0], source[0]),
        channel(destination[1], source[1]),
        channel(destination[2], source[2]),
        out_alpha as u8,
    ]);
}

async fn equip_skin(skin: Skin) -> Result<(), String> {
    minecraft_skins::equip_skin(skin).await.map_err(describe)
}

async fn save_settings(settings: Settings) -> Result<(), String> {
    settings_api::set(settings).await.map_err(describe)
}

fn apply_setting(
    mut settings: Settings,
    change: SettingChange,
) -> Option<Settings> {
    match change {
        SettingChange::Downloads(delta) => {
            settings.max_concurrent_downloads =
                (settings.max_concurrent_downloads as i32 + delta).clamp(1, 64)
                    as usize;
        }
        SettingChange::Writes(delta) => {
            settings.max_concurrent_writes =
                (settings.max_concurrent_writes as i32 + delta).clamp(1, 64)
                    as usize;
        }
        SettingChange::Memory(delta) => {
            settings.memory.maximum = (settings.memory.maximum as i32 + delta)
                .clamp(512, 65_536)
                as u32;
        }
        SettingChange::SetTheme(theme) => {
            settings.theme = theme;
        }
        SettingChange::SetLocale(locale) => {
            settings.locale = locale.to_string();
        }
        SettingChange::ToggleTelemetry => {
            settings.telemetry = !settings.telemetry
        }
        SettingChange::ToggleDiscordRpc => {
            settings.discord_rpc = !settings.discord_rpc
        }
        SettingChange::ToggleAdvancedRendering => {
            settings.advanced_rendering = !settings.advanced_rendering;
        }
        SettingChange::ToggleNativeDecorations => {
            settings.native_decorations = !settings.native_decorations;
        }
        SettingChange::ToggleForceFullscreen => {
            settings.force_fullscreen = !settings.force_fullscreen;
        }
        SettingChange::ToggleHideOnProcessStart => {
            settings.hide_on_process_start = !settings.hide_on_process_start;
        }
        SettingChange::ToggleSidebar => {
            settings.toggle_sidebar = !settings.toggle_sidebar
        }
    }

    Some(settings)
}

impl From<InstanceMetadata> for InstanceView {
    fn from(metadata: InstanceMetadata) -> Self {
        let memory_mb = metadata
            .launch_overrides
            .memory
            .map(|memory| memory.maximum);
        let java_path = metadata.launch_overrides.java_path.clone();
        let group_ids = metadata.group_ids.clone();
        let shared =
            metadata
                .shared_instance
                .as_ref()
                .map(|shared| SharedLinkView {
                    id: shared.id.clone(),
                    role: shared.role.as_str().to_string(),
                    latest_version: shared.latest_version,
                    applied_version: shared.applied_version,
                });
        let launch_args = metadata
            .launch_overrides
            .extra_launch_args
            .clone()
            .unwrap_or_default();
        let instance = metadata.instance;
        let content_set = metadata.applied_content_set;
        Self {
            id: instance.id,
            name: instance.name,
            path: instance.path,
            icon_path: instance.icon_path.map(PathBuf::from),
            last_played: instance
                .last_played
                .map(|date| date.format("%Y-%m-%d %H:%M").to_string()),
            playtime_minutes: instance.submitted_time_played / 60,
            game_version: content_set.game_version,
            loader: content_set.loader.as_str().to_string(),
            loader_version: content_set.loader_version,
            memory_mb,
            java_path,
            launch_args,
            group_ids,
            shared,
        }
    }
}

impl From<Credentials> for AccountView {
    fn from(credentials: Credentials) -> Self {
        Self {
            id: credentials.offline_profile.id.to_string(),
            name: credentials.offline_profile.name,
            active: credentials.active,
        }
    }
}

fn pack_status_string(status: worlds::ServerPackStatus) -> &'static str {
    match status {
        worlds::ServerPackStatus::Enabled => "enabled",
        worlds::ServerPackStatus::Disabled => "disabled",
        worlds::ServerPackStatus::Prompt => "prompt",
    }
}

fn describe(error: impl std::fmt::Display) -> String {
    error.to_string()
}

/// Accept either a raw OAuth code or the final redirect URL pasted by the user.
pub fn extract_login_code(input: &str) -> String {
    let input = input.trim();
    if input.is_empty() {
        return String::new();
    }

    for pair in input.split(['?', '&']) {
        if let Some(value) = pair.strip_prefix("code=") {
            return value
                .split(['&', '#'])
                .next()
                .unwrap_or_default()
                .to_string();
        }
    }

    input.to_string()
}

#[cfg(test)]
mod tests {
    use base64::Engine as _;

    use super::{
        ProjectKind, ProjectView, auth_code_from_path, compose_avatar,
        decode_data_icon, extract_invite_id, extract_login_code,
        render_avatar_png,
    };
    use image::{Rgba, RgbaImage};

    #[test]
    fn parses_a_v3_search_hit() {
        let hit = serde_json::json!({
            "project_id": "AANobbMI",
            "slug": "sodium",
            "name": "Sodium",
            "summary": "Modern rendering engine",
            "project_types": ["mod"],
            "icon_url": "https://cdn.modrinth.com/data/AANobbMI/icon.png",
            "downloads": 12_345_678,
            "author": "jellysquid3",
            "version_id": "mc1.21.10-0.6.13"
        });

        let project = ProjectView::from_hit(&hit).expect("valid hit");
        assert_eq!(project.id, "AANobbMI");
        assert_eq!(project.slug, "sodium");
        assert_eq!(project.title, "Sodium");
        assert_eq!(project.kind, ProjectKind::Mod);
        assert_eq!(project.downloads, 12_345_678);
        assert_eq!(project.version_id.as_deref(), Some("mc1.21.10-0.6.13"));
    }

    #[test]
    fn decodes_a_base64_server_favicon() {
        let png = [0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a];
        let payload = base64::engine::general_purpose::STANDARD.encode(png);
        let url = format!("data:image/png;base64,{payload}");
        assert_eq!(decode_data_icon(&url).as_deref(), Some(&png[..]));
        assert!(decode_data_icon("data:text/plain;base64,aGk=").is_none());
        assert!(decode_data_icon("not-a-data-url").is_none());
    }

    #[test]
    fn rejects_hits_without_a_supported_project_type() {
        let hit = serde_json::json!({
            "project_id": "abc",
            "slug": "abc",
            "name": "Unknown",
            "project_types": ["plugin"]
        });
        assert!(ProjectView::from_hit(&hit).is_none());
    }

    #[test]
    fn extracts_code_from_redirect_url() {
        assert_eq!(
            extract_login_code(
                "https://login.live.com/oauth20_desktop.srf?code=M.R3_ABC&lc=1033"
            ),
            "M.R3_ABC"
        );
    }

    #[test]
    fn accepts_raw_code() {
        assert_eq!(extract_login_code("  M.R3_ABC  "), "M.R3_ABC");
    }

    #[test]
    fn ignores_empty_input() {
        assert!(extract_login_code("   ").is_empty());
    }

    #[test]
    fn extracts_invite_ids_from_share_links_and_raw_ids() {
        assert_eq!(
            extract_invite_id(
                "https://modrinth.com/share/abc123?from=launcher"
            ),
            "abc123"
        );
        assert_eq!(extract_invite_id("  abc123/  "), "abc123");
        assert!(extract_invite_id("   ").is_empty());
    }

    #[test]
    fn reads_the_oauth_code_from_a_loopback_callback() {
        assert_eq!(
            auth_code_from_path("/?launcher=true&code=token_123").as_deref(),
            Some("token_123")
        );
        assert_eq!(
            auth_code_from_path("/?code=token%5F123").as_deref(),
            Some("token_123")
        );
        assert!(auth_code_from_path("/favicon.ico").is_none());
        assert!(auth_code_from_path("/?error=denied").is_none());
    }

    #[test]
    fn renders_skin_and_cape_front_view() {
        let mut skin = RgbaImage::from_pixel(64, 64, Rgba([0, 0, 0, 0]));
        for (rect, color) in [
            ((8, 8, 8, 8), Rgba([220, 80, 80, 255])),
            ((20, 20, 8, 12), Rgba([80, 120, 220, 255])),
            ((4, 20, 4, 12), Rgba([80, 200, 120, 255])),
            ((20, 52, 4, 12), Rgba([200, 80, 200, 255])),
            ((44, 20, 4, 12), Rgba([230, 200, 70, 255])),
            ((36, 52, 4, 12), Rgba([70, 210, 220, 255])),
        ] {
            let (x, y, width, height) = rect;
            for py in y..y + height {
                for px in x..x + width {
                    skin.put_pixel(px, py, color);
                }
            }
        }
        let mut cape = RgbaImage::from_pixel(64, 32, Rgba([0, 0, 0, 0]));
        for py in 1..17 {
            for px in 1..11 {
                cape.put_pixel(px, py, Rgba([90, 60, 200, 255]));
            }
        }

        let avatar = compose_avatar(&skin, Some(&cape), false);
        assert_eq!(avatar.dimensions(), (16, 32));
        assert_eq!(avatar.get_pixel(8, 4), &Rgba([220, 80, 80, 255]));
        assert_eq!(avatar.get_pixel(8, 12), &Rgba([80, 120, 220, 255]));
        assert_eq!(avatar.get_pixel(6, 26), &Rgba([80, 200, 120, 255]));
        assert_eq!(avatar.get_pixel(2, 12), &Rgba([230, 200, 70, 255]));
        assert_eq!(avatar.get_pixel(2, 22), &Rgba([90, 60, 200, 255]));
    }

    #[test]
    fn encoded_avatar_is_a_usable_png() {
        let skin = RgbaImage::from_pixel(64, 64, Rgba([10, 20, 30, 255]));
        let mut bytes = Vec::new();
        image::DynamicImage::ImageRgba8(skin)
            .write_to(
                &mut std::io::Cursor::new(&mut bytes),
                image::ImageFormat::Png,
            )
            .unwrap();
        let rendered = render_avatar_png(&bytes, None, false).expect("png");
        let decoded = image::load_from_memory(&rendered).unwrap().to_rgba8();
        assert_eq!(decoded.dimensions(), (16, 32));
    }
}
