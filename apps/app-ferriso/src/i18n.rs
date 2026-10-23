//! Presentation-layer localization for the native frontend.
//!
//! The launcher backend persists the selected locale in `Settings.locale`
//! (the same setting the original app uses); this module turns that code into
//! UI copy. English is the fallback for every unknown string.

use std::{
    cell::Cell,
    collections::{BTreeMap, HashMap},
    fmt,
    sync::OnceLock,
};

thread_local! {
    static LOCALE: Cell<&'static str> = const { Cell::new("en-US") };
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Locale {
    pub code: &'static str,
    pub name: &'static str,
}

impl fmt::Display for Locale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.name)
    }
}

pub const LOCALES: [Locale; 33] = [
    Locale {
        code: "ar-SA",
        name: "العربية",
    },
    Locale {
        code: "cs-CZ",
        name: "Čeština",
    },
    Locale {
        code: "da-DK",
        name: "Dansk",
    },
    Locale {
        code: "de-CH",
        name: "Deutsch (Schweiz)",
    },
    Locale {
        code: "de-DE",
        name: "Deutsch",
    },
    Locale {
        code: "en-US",
        name: "English",
    },
    Locale {
        code: "es-419",
        name: "Español (Latinoamérica)",
    },
    Locale {
        code: "es-ES",
        name: "Español",
    },
    Locale {
        code: "fi-FI",
        name: "Suomi",
    },
    Locale {
        code: "fil-PH",
        name: "Filipino",
    },
    Locale {
        code: "fr-FR",
        name: "Français",
    },
    Locale {
        code: "he-IL",
        name: "עברית",
    },
    Locale {
        code: "hu-HU",
        name: "Magyar",
    },
    Locale {
        code: "id-ID",
        name: "Bahasa Indonesia",
    },
    Locale {
        code: "it-IT",
        name: "Italiano",
    },
    Locale {
        code: "ja-JP",
        name: "日本語",
    },
    Locale {
        code: "ko-KR",
        name: "한국어",
    },
    Locale {
        code: "ms-MY",
        name: "Bahasa Melayu",
    },
    Locale {
        code: "nl-NL",
        name: "Nederlands",
    },
    Locale {
        code: "no-NO",
        name: "Norsk",
    },
    Locale {
        code: "pl-PL",
        name: "Polski",
    },
    Locale {
        code: "pt-BR",
        name: "Português (Brasil)",
    },
    Locale {
        code: "pt-PT",
        name: "Português (Portugal)",
    },
    Locale {
        code: "ro-RO",
        name: "Română",
    },
    Locale {
        code: "ru-RU",
        name: "Русский",
    },
    Locale {
        code: "sr-CS",
        name: "Srpski",
    },
    Locale {
        code: "sv-SE",
        name: "Svenska",
    },
    Locale {
        code: "th-TH",
        name: "ไทย",
    },
    Locale {
        code: "tr-TR",
        name: "Türkçe",
    },
    Locale {
        code: "uk-UA",
        name: "Українська",
    },
    Locale {
        code: "vi-VN",
        name: "Tiếng Việt",
    },
    Locale {
        code: "zh-CN",
        name: "简体中文",
    },
    Locale {
        code: "zh-TW",
        name: "繁體中文",
    },
];

/// Select the language dictionary for the current view build.
pub fn set_locale(locale: &str) {
    let locale = locale.to_ascii_lowercase();
    let selected = LOCALES
        .iter()
        .find(|entry| entry.code.eq_ignore_ascii_case(&locale))
        .or_else(|| {
            let language = locale.split('-').next().unwrap_or_default();
            LOCALES.iter().find(|entry| {
                entry
                    .code
                    .split('-')
                    .next()
                    .is_some_and(|code| code.eq_ignore_ascii_case(language))
            })
        })
        .map(|entry| entry.code)
        .unwrap_or("en-US");
    LOCALE.with(|current| current.set(selected));
}

/// Translate a UI string. Unknown strings pass through unchanged so metadata
/// names, versions, paths, and backend errors are never rewritten.
pub fn t(value: &str) -> &str {
    let locale = LOCALE.with(Cell::get);
    if locale == "en-US" {
        return value;
    }
    if locale == "zh-CN" {
        let translated = zh_cn(value);
        if !std::ptr::eq(translated, value) {
            return translated;
        }
    }
    if let Some(translated) =
        catalogs().get(locale).and_then(|map| map.get(value))
    {
        return translated;
    }
    value
}

/// Hand-written Simplified Chinese copy. The generated catalog from the
/// existing Modrinth locale files fills in labels that are shared with the
/// web app; this dictionary covers launcher-specific strings.
fn zh_cn(value: &str) -> &str {
    match value {
        "live" => "实时",
        "attention" => "注意",
        "ready" => "就绪",
        "Not signed in" => "未登录",
        "↻  Refresh" => "↻  刷新",
        "↻ Refresh" => "↻ 刷新",
        "Group name" => "分组名称",
        "Instance name" => "实例名称",
        "Minecraft version (e.g. 1.21.10)" => "Minecraft 版本（例如 1.21.10）",
        "Search Modrinth…" => "搜索 Modrinth…",
        "Modrinth user ID(s), comma separated" => {
            "Modrinth 用户 ID（用逗号分隔）"
        }
        "Invite ID or share link" => "邀请 ID 或分享链接",
        "Memory MB (empty = inherit)" => "内存 MB（留空 = 继承）",
        "Java path (empty = auto)" => "Java 路径（留空 = 自动）",
        "Extra JVM args (space separated)" => "额外 JVM 参数（空格分隔）",
        "New icon image path (saved on Save)" => "新图标路径（保存后生效）",
        "Launcher base path" => "启动器基础路径",
        "Instance folder" => "实例文件夹",
        "Export path" => "导出路径",
        "Name" => "名称",
        "Server address" => "服务器地址",
        "Paste the final redirect URL or code" => "粘贴最终重定向 URL 或代码",
        "Friend's Modrinth user ID" => "好友的 Modrinth 用户 ID",
        "Modrinth user " => "Modrinth 用户 ",
        "Jump back into" => "继续游玩",
        "minutes played" => "分钟游戏时间",
        "min" => "分钟",
        "instances" => "个实例",
        "installs running" => "个安装任务进行中",
        "shared with the Modrinth data directory" => "与 Modrinth 数据目录共享",
        "Create a Minecraft instance, or install a modpack from Browse." => {
            "创建一个 Minecraft 实例，或从浏览页安装整合包。"
        }
        "Continue playing" => "继续游玩",
        "Sorted by your latest session" => "按最近游玩时间排序",
        "INSTANCES" => "实例",
        "ACCOUNTS" => "账户",
        "ACTIVE JOBS" => "活动任务",
        "total" => "共",
        "No skin selected" => "未选择皮肤",
        "Custom" => "自定义",
        "CustomExternal" => "外部",
        "Classic" => "经典",
        "Slim" => "纤细",
        "Default" => "默认",
        "cape:" => "披风：",
        "Never played" => "从未游玩",
        "Install modpack" => "安装整合包",
        "Add to instance" => "添加到实例",
        "Modpacks" => "整合包",
        "Mods" => "模组",
        "Resource packs" => "资源包",
        "Shaders" => "光影",
        "by " => "作者 ",
        " downloads" => " 次下载",
        "Latest release: " => "最新版本：",
        "Latest version " => "最新版本 ",
        "minutes" => "分钟",
        "files" => "个文件",
        "log files" => "个日志文件",
        "worlds" => "个世界",
        "entries" => "个条目",
        "s old" => "秒前",
        "no game versions" => "无游戏版本",
        "no loader" => "无加载器",
        "unknown version" => "未知版本",
        "Launcher: " => "启动器：",
        "Delete " => "删除 ",
        "Pack: " => "整合包：",
        "Target: " => "目标：",
        "Display: " => "显示：",
        "Rollback error: " => "回滚错误：",
        "Instance ID" => "实例 ID",
        "Loader version" => "加载器版本",
        "Last played" => "最近游玩",
        "Playtime" => "游戏时长",
        "Runtime" => "运行时间",
        "Memory" => "内存",
        "Java" => "Java",
        "Never" => "从未",
        "Stopped" => "已停止",
        "Global default" => "全局默认",
        "Auto-detected" => "自动检测",
        "Running since" => "运行自",
        "Overview" => "概览",
        "Add server" => "添加服务器",
        "Edit server" => "编辑服务器",
        "Rename world" => "重命名世界",
        "This removes the world data from disk." => "这会从磁盘删除世界数据。",
        "This removes the server from this instance." => {
            "这会从实例中移除服务器。"
        }
        "member" => "成员",
        "Not published" => "未发布",
        "No published version" => "没有已发布版本",
        "Enter a Minecraft version" => "输入 Minecraft 版本",
        "Applied " => "已应用 ",
        "uses" => "次使用",
        "expires" => "过期于",
        "Invite ID: " => "邀请 ID：",
        "mods" => "个模组",
        "external files" => "个外部文件",
        "Shared instance " => "共享实例 ",
        "active jobs" => "个活动任务",
        "working" => "处理中",
        "Created: " => "创建：",
        "Finished: " => "完成：",
        "Concurrent downloads" => "同时下载数",
        "Concurrent writes" => "同时写入数",
        "Maximum memory" => "最大内存",
        "Telemetry" => "遥测",
        "Discord presence" => "Discord 状态",
        "Advanced rendering" => "高级渲染",
        "Reduce motion" => "减少动态效果",
        "Native decorations" => "原生窗口装饰",
        "Force fullscreen" => "强制全屏",
        "Hide on game start" => "游戏启动时隐藏",
        "Toggle sidebar" => "切换侧边栏",
        "Data directory" => "数据目录",
        "Default Modrinth location" => "默认 Modrinth 位置",
        "Game resolution" => "游戏分辨率",
        "dark" => "深色",
        "light" => "浅色",
        "oled" => "OLED",
        "retro" => "复古",
        "violet" => "紫罗兰",
        "rose" => "玫瑰",
        "teal" => "青碧",
        "crimson" => "绯红",
        "emerald" => "翡翠",
        "system" => "跟随系统",
        "Sync with system" => "跟随系统",
        "PROFILE" => "资料",
        "WARDROBE" => "衣橱",
        "FERRISO" => "FERRISO",
        "Update" => "更新",
        "Download" => "下载",
        "Version" => "版本",
        "Unofficial Modrinth fork" => "非官方 Modrinth 分支",
        "About Ferriso Launcher" => "关于 Ferriso Launcher",
        "Ferriso Launcher is an unofficial fork of Modrinth App. It uses Modrinth services for authentication, project metadata, and content delivery, but it is not affiliated with or endorsed by Modrinth." => {
            "Ferriso Launcher 是 Modrinth App 的非官方分支。它使用 Modrinth 的账户、项目元数据与内容分发服务，但与 Modrinth 官方无隶属关系，也未获其背书。"
        }
        "Modrinth source" => "Modrinth 源码",
        "License: GPL-3.0-only" => "许可证：GPL-3.0-only",
        "LAUNCHER · GLASS" => "启动器 · 玻璃",
        "Starting Ferriso Launcher" => "正在启动 Ferriso Launcher",
        "Starting Ferriso Launcher…" => "正在启动 Ferriso Launcher…",
        "Welcome to Ferriso Launcher" => "欢迎使用 Ferriso Launcher",
        "GET STARTED" => "开始使用",
        "Finish setting up Ferriso Launcher" => "完成 Ferriso Launcher 设置",
        "Create your first instance" => "创建第一个实例",
        "Sign in with Microsoft" => "使用 Microsoft 登录",
        "Sign in to Modrinth" => "登录 Modrinth",
        "Create" => "创建",
        "Account" => "账户",
        "Navigation" => "导航",
        "Instances" => "实例",
        "Home" => "主页",
        "Browse" => "浏览",
        "Project" => "项目",
        "← Back" => "← 返回",
        "View on Modrinth" => "在 Modrinth 查看",
        "About this project" => "项目简介",
        "No description provided." => "暂无简介。",
        "Install" => "安装",
        "Select a version and an instance." => "选择版本和实例。",
        "Loading project…" => "正在加载项目…",
        "Project not found." => "未找到项目。",
        "Select a version to install" => "请选择要安装的版本",
        "Selected version is no longer available" => "所选版本已不可用",
        "Library" => "资料库",
        "Skins" => "皮肤",
        "Settings" => "设置",
        "Instance" => "实例",
        "Saved skin" => "已保存皮肤",
        "Reset view" => "重置视角",
        "Drag to rotate" => "拖动旋转",
        "Skins and capes are applied to the active Minecraft account through Mojang." => {
            "皮肤和披风通过 Mojang 应用到当前 Minecraft 账户。"
        }
        "Refreshing…" => "刷新中…",
        "Refresh failed" => "刷新失败",
        "Skin updated" => "皮肤已更新",
        "Applying skin and cape…" => "正在应用皮肤和披风…",
        "Could not load skins" => "无法加载皮肤",
        "Could not update skin" => "无法更新皮肤",
        "Sign-in failed" => "登录失败",
        "Sign-in cancelled" => "已取消登录",
        "Switching account…" => "正在切换账户…",
        "Account switched" => "账户已切换",
        "Could not switch account" => "无法切换账户",
        "Removing account…" => "正在移除账户…",
        "Account removed" => "账户已移除",
        "Failed to initialize" => "初始化失败",
        "Starting Modrinth…" => "正在启动 Modrinth…",
        "No instances yet" => "还没有实例",
        "Saved" => "已保存",
        "Capes" => "披风",
        "Current skin" => "当前皮肤",
        "CURRENT SKIN" => "当前皮肤",
        "This is the skin Mojang currently reports for your profile." => {
            "这是 Mojang 当前为你的账户报告的皮肤。"
        }
        "No cape" => "无披风",
        "Equipped" => "已装备",
        "Previewing" => "预览中",
        "EQUIPPED" => "已装备",
        "PREVIEWING" => "预览中",
        "Language" => "语言",
        "High contrast" => "高对比度",
        "English" => "English",
        "简体中文" => "简体中文",
        "Theme" => "主题",
        "Appearance" => "外观",
        "Preferences" => "偏好设置",
        "PREFERENCES" => "偏好设置",
        "Workspace" => "工作区",
        "WORKSPACE" => "工作区",
        "Discover" => "发现",
        "DISCOVER" => "发现",
        "Your library" => "你的资料库",
        "YOUR LIBRARY" => "你的资料库",
        "Your next adventure" => "下一段旅程",
        "YOUR NEXT ADVENTURE" => "下一段旅程",
        "Ready" => "就绪",
        "Settings saved" => "设置已保存",
        "Could not save settings" => "无法保存设置",
        "Active" => "活跃",
        "Running" => "运行中",
        "Disabled" => "已禁用",
        "Enabled" => "已启用",
        "On" => "开",
        "Off" => "关",
        "Add friend" => "添加好友",
        "Add Microsoft account" => "添加 Microsoft 账户",
        "All instances" => "所有实例",
        "Applied together with the selected skin." => "将与所选皮肤一起应用。",
        "Apply skin and cape" => "应用皮肤和披风",
        "Assign instances to the group from their Edit card." => {
            "在实例的“编辑”卡片中将实例分配到该分组。"
        }
        "Backup" => "备份",
        "Browse modpacks" => "浏览整合包",
        "Cancel" => "取消",
        "Checking invite…" => "正在检查邀请…",
        "Close" => "关闭",
        "Content" => "内容",
        "Create instance" => "创建实例",
        "Create invite link" => "创建邀请链接",
        "Creating…" => "创建中…",
        "Delete" => "删除",
        "Files" => "文件",
        "Loading files…" => "正在加载文件…",
        "folder" => "文件夹",
        "This folder is empty." => "此文件夹为空。",
        "truncated" => "已截断",
        "Binary files cannot be previewed" => "无法预览二进制文件",
        "Not a file" => "不是文件",
        "Invalid instance path" => "无效的实例路径",
        "Path escapes the instance folder" => "路径超出实例目录",
        "Refresh" => "刷新",
        "New folder" => "新建文件夹",
        "New folder name" => "新文件夹名称",
        "Invalid file name" => "无效的文件名",

        "A file with that name already exists" => "同名文件已存在",
        "The instance root cannot be deleted" => "不能删除实例根目录",
        "The instance root cannot be renamed" => "不能重命名实例根目录",
        "Hosting" => "托管",
        "Search servers…" => "搜索服务器…",
        "servers" => "台服务器",
        "No servers match your search." => "没有匹配的服务器。",
        "Server name" => "服务器名称",
        "Server renamed" => "服务器已重命名",
        "Hosting dashboard" => "托管面板",
        "Sign in to manage your hosted servers" => "登录后管理你的托管服务器",
        "Modrinth Hosting servers are tied to your Modrinth account." => {
            "Modrinth 托管服务器与你的 Modrinth 账号绑定。"
        }
        "Sign in" => "登录",
        "Loading servers…" => "正在加载服务器…",
        "No hosted servers yet" => "还没有托管服务器",
        "Get a Modrinth-hosted server and it will appear here." => {
            "获取一台 Modrinth 托管服务器后会显示在这里。"
        }
        "Explore hosting" => "了解托管服务",
        "Needs attention" => "需要处理",
        "Suspended" => "已暂停",
        "Medal" => "奖章",
        "Manage" => "管理",
        "Working…" => "处理中…",
        "Hosting action sent" => "托管操作已发送",
        "Start" => "启动",
        "Restart" => "重启",
        "Stop" => "停止",
        "Game closed" => "游戏已退出",
        "Game crashed" => "游戏崩溃",
        "Analyzing the latest logs…" => "正在分析最新日志…",
        "What likely happened" => "可能的原因",
        "Log excerpt" => "日志摘录",
        "No crash report found" => "未找到崩溃报告",
        "Out of memory" => "内存不足",
        "Increase the instance memory or reduce render distance and mods." => {
            "提高实例内存，或降低渲染距离并减少模组。"
        }
        "Wrong Java version" => "Java 版本不匹配",
        "Install the Java version this Minecraft version expects." => {
            "安装该 Minecraft 版本所需的 Java。"
        }
        "Java could not start" => "Java 无法启动",
        "Check the selected Java path and JVM arguments." => {
            "检查所选 Java 路径和 JVM 参数。"
        }
        "Graphics or window failure" => "图形或窗口失败",
        "Try updating GPU drivers or disable advanced rendering." => {
            "尝试更新显卡驱动，或关闭高级渲染。"
        }
        "Mod dependency conflict" => "模组依赖冲突",
        "Check that all required mods and versions are installed." => {
            "检查是否安装了全部必需模组及对应版本。"
        }
        "Duplicate mods" => "重复模组",
        "Remove duplicate copies of the same mod." => {
            "删除同一模组的重复文件。"
        }
        "Mod incompatibility" => "模组不兼容",
        "Disable recently added mods and retry." => {
            "禁用最近添加的模组后重试。"
        }
        "Network failure" => "网络故障",
        "Check your connection and retry." => "检查网络连接后重试。",
        "Login or session failure" => "登录或会话失效",
        "Sign out and sign back in to your account." => "退出账号后重新登录。",
        "Minecraft crash report" => "Minecraft 崩溃报告",
        "Open the latest crash report for the full stack trace." => {
            "打开最新崩溃报告查看完整堆栈。"
        }
        "Unexpected exit" => "异常退出",
        "Review the log excerpt below and the latest crash report." => {
            "查看下方日志摘录和最新崩溃报告。"
        }
        "Delete all" => "全部删除",
        "Delete permanently" => "永久删除",
        "Detect installed Java" => "检测已安装的 Java",
        "Dismiss" => "忽略",
        "Downloads" => "下载",
        "Duplicate" => "复制",
        "Edit" => "编辑",
        "Export" => "导出",
        "Export instance" => "导出实例",
        "Filter the library and assign instances from Edit." => {
            "筛选资料库，并在“编辑”中分配实例。"
        }
        "Find your next project" => "发现你的下一个项目",
        "Finish sign-in" => "完成登录",
        "Friends" => "好友",
        "Groups" => "分组",
        "Groups (applied on Save)" => "分组（保存后应用）",
        "Import" => "导入",
        "Import instance" => "导入实例",
        "Installing Java…" => "正在安装 Java…",
        "Installs" => "安装",
        "INSTANCE" => "实例",
        "Instance settings" => "实例设置",
        "Invite" => "邀请",
        "Invite links" => "邀请链接",
        "Join a friend's instance with an invite ID. You must be signed in to Modrinth." => {
            "使用邀请 ID 加入好友的实例。你需要登录 Modrinth。"
        }
        "Kill" => "终止",
        "Launch" => "启动",
        "Quarantined" => "已隔离",
        "Linked modpack" => "关联整合包",
        "Installed version" => "已安装版本",
        "Update modpack" => "更新整合包",
        "Repair files" => "修复文件",
        "Installing" => "安装中",
        "Installing…" => "安装中…",
        "Blocked" => "已阻止",
        "Beta channel" => "测试通道",
        "This instance is quarantined and cannot be launched" => {
            "此实例已被隔离，无法启动"
        }
        "This instance has not finished installing" => "此实例尚未安装完成",
        "Launch blocked" => "启动已阻止",
        "Launching…" => "启动中…",
        "Launch now" => "立即启动",
        "Leave shared instance" => "离开共享实例",
        "Loader" => "加载器",
        "Loading instance…" => "正在加载实例…",
        "Loading log…" => "正在加载日志…",
        "Loading shared instance…" => "正在加载共享实例…",
        "Loading versions…" => "正在加载版本…",
        "Load skins" => "加载皮肤",
        "Lock" => "锁定",
        "Locked" => "已锁定",
        "Logs" => "日志",
        "Live console" => "实时控制台",
        "running" => "运行中",
        "Clear" => "清空",
        "Members" => "成员",
        "Microsoft sign-in" => "Microsoft 登录",
        "Modrinth account" => "Modrinth 账户",
        "New instance" => "新建实例",
        "No capes are available for this account." => "此账户没有可用披风。",
        "No friends yet." => "还没有好友。",
        "No groups yet." => "还没有分组。",
        "No groups yet. Create one from the Library." => {
            "还没有分组。请在资料库中创建一个。"
        }
        "No install jobs yet." => "还没有安装任务。",
        "No instances in this group" => "该分组中没有实例",
        "No logs or crash reports found." => "未找到日志或崩溃报告。",
        "No managed content in this instance." => "此实例中没有受管理的内容。",
        "No Minecraft account signed in." => "未登录 Minecraft 账户。",
        "No projects found." => "未找到项目。",
        "No skins in this tab." => "此分栏中没有皮肤。",
        "No versions available." => "没有可用版本。",
        "No worlds or servers found." => "未找到世界或服务器。",
        "Open browser" => "打开浏览器",
        "Open library" => "打开资料库",
        "Open page" => "打开页面",
        "Open share page" => "打开分享页面",
        "Overrides apply only to this instance. Launch changes take effect next run." => {
            "覆盖项仅对此实例生效。启动项更改将在下次运行时生效。"
        }
        "owner" => "所有者",
        "Paths" => "路径",
        "Preview" => "预览",
        "Publish shared instance" => "发布共享实例",
        "Publish this instance to invite friends. Instances stay local; only the content set is shared." => {
            "发布此实例以邀请好友。实例本身仍保存在本地，仅共享内容集合。"
        }
        "Publish update" => "发布更新",
        "READY WHEN YOU ARE" => "准备好了吗",
        "Remove" => "移除",
        "Remove icon" => "移除图标",
        "Rename" => "重命名",
        "Reset" => "重置",
        "Resolving required Java…" => "正在解析所需 Java…",
        "Retry" => "重试",
        "Revoke" => "撤销",
        "Save" => "保存",
        "Scanning Java…" => "正在扫描 Java…",
        "Search" => "搜索",
        "Searching Modrinth…" => "正在搜索 Modrinth…",
        "Search Modrinth and install it in one click." => {
            "搜索 Modrinth，一键安装。"
        }
        "Select an instance in the Library." => "请在资料库中选择一个实例。",
        "Select instance first" => "请先选择实例",
        "Share" => "分享",
        "Shared instance" => "共享实例",
        "Shared instances" => "共享实例",
        "Sign in to use friends and shared instances." => {
            "登录后可使用好友和共享实例。"
        }
        "Sign in with Modrinth from the Account page to share this instance." => {
            "在账户页面登录 Modrinth 以共享此实例。"
        }
        "Sign out" => "退出登录",
        "Something went wrong" => "出现了一些问题",
        "Start import" => "开始导入",
        "Starting Modrinth" => "正在启动 Modrinth",
        "The browser did not open automatically; open the URL manually." => {
            "浏览器未自动打开，请手动打开链接。"
        }
        "This instance is no longer available." => "此实例已不可用。",
        "This removes the instance folder and its content." => {
            "这将删除实例文件夹及其内容。"
        }
        "Unlock" => "解锁",
        "Unpublish shared instance" => "取消发布共享实例",
        "Update all" => "全部更新",
        "Update from shared" => "从共享更新",
        "Use" => "使用",
        "Versions" => "版本",
        "View" => "查看",
        "Waiting for launcher settings…" => "正在等待启动器设置…",
        "Welcome to Modrinth" => "欢迎使用 Modrinth",

        "Worlds" => "世界",
        "Writes a .mrpack containing the instance content and overrides." => {
            "写入包含实例内容和覆盖项的 .mrpack 文件。"
        }
        "You are a member of this shared instance." => "你是此共享实例的成员。",
        "Your instances" => "你的实例",
        "Player profile" => "玩家资料",
        "Wardrobe" => "衣橱",
        "All" => "全部",
        "None" => "无",
        "Unknown" => "未知",
        "Yes" => "是",
        "No" => "否",
        "Back" => "返回",
        "Next" => "下一步",
        "Copy" => "复制",
        "Copied" => "已复制",
        "Search skins" => "搜索皮肤",
        "Search projects" => "搜索项目",
        _ => value,
    }
}

fn load_entries(json: &'static str) -> BTreeMap<&'static str, &'static str> {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(json) else {
        return BTreeMap::new();
    };
    let Some(entries) = value.as_object() else {
        return BTreeMap::new();
    };
    let mut result = BTreeMap::new();
    for (key, entry) in entries {
        let Some(message) =
            entry.get("message").and_then(|value| value.as_str())
        else {
            continue;
        };
        let key: &'static str = Box::leak(key.clone().into_boxed_str());
        let message: &'static str =
            Box::leak(message.to_string().into_boxed_str());
        result.insert(key, message);
    }
    result
}

static CATALOGS: OnceLock<
    HashMap<&'static str, HashMap<&'static str, &'static str>>,
> = OnceLock::new();

fn catalogs()
-> &'static HashMap<&'static str, HashMap<&'static str, &'static str>> {
    CATALOGS.get_or_init(|| {
        let english = load_entries(include_str!(
            "../../app-frontend/src/locales/en-US/index.json"
        ));
        let mut catalogs = HashMap::new();
        for (code, json) in LOCALE_FILES {
            let entries = load_entries(json);
            let mut translations = HashMap::new();
            for (key, translated) in entries {
                if let Some(source) = english.get(key) {
                    let prefer = translations.get(source).is_none_or(
                        |current: &&str| translated.len() < current.len(),
                    );
                    if prefer {
                        translations.insert(*source, translated);
                    }
                }
            }
            catalogs.insert(code, translations);
        }
        catalogs
    })
}

static LOCALE_FILES: [(&str, &str); 33] = [
    (
        "ar-SA",
        include_str!("../../app-frontend/src/locales/ar-SA/index.json"),
    ),
    (
        "cs-CZ",
        include_str!("../../app-frontend/src/locales/cs-CZ/index.json"),
    ),
    (
        "da-DK",
        include_str!("../../app-frontend/src/locales/da-DK/index.json"),
    ),
    (
        "de-CH",
        include_str!("../../app-frontend/src/locales/de-CH/index.json"),
    ),
    (
        "de-DE",
        include_str!("../../app-frontend/src/locales/de-DE/index.json"),
    ),
    (
        "en-US",
        include_str!("../../app-frontend/src/locales/en-US/index.json"),
    ),
    (
        "es-419",
        include_str!("../../app-frontend/src/locales/es-419/index.json"),
    ),
    (
        "es-ES",
        include_str!("../../app-frontend/src/locales/es-ES/index.json"),
    ),
    (
        "fi-FI",
        include_str!("../../app-frontend/src/locales/fi-FI/index.json"),
    ),
    (
        "fil-PH",
        include_str!("../../app-frontend/src/locales/fil-PH/index.json"),
    ),
    (
        "fr-FR",
        include_str!("../../app-frontend/src/locales/fr-FR/index.json"),
    ),
    (
        "he-IL",
        include_str!("../../app-frontend/src/locales/he-IL/index.json"),
    ),
    (
        "hu-HU",
        include_str!("../../app-frontend/src/locales/hu-HU/index.json"),
    ),
    (
        "id-ID",
        include_str!("../../app-frontend/src/locales/id-ID/index.json"),
    ),
    (
        "it-IT",
        include_str!("../../app-frontend/src/locales/it-IT/index.json"),
    ),
    (
        "ja-JP",
        include_str!("../../app-frontend/src/locales/ja-JP/index.json"),
    ),
    (
        "ko-KR",
        include_str!("../../app-frontend/src/locales/ko-KR/index.json"),
    ),
    (
        "ms-MY",
        include_str!("../../app-frontend/src/locales/ms-MY/index.json"),
    ),
    (
        "nl-NL",
        include_str!("../../app-frontend/src/locales/nl-NL/index.json"),
    ),
    (
        "no-NO",
        include_str!("../../app-frontend/src/locales/no-NO/index.json"),
    ),
    (
        "pl-PL",
        include_str!("../../app-frontend/src/locales/pl-PL/index.json"),
    ),
    (
        "pt-BR",
        include_str!("../../app-frontend/src/locales/pt-BR/index.json"),
    ),
    (
        "pt-PT",
        include_str!("../../app-frontend/src/locales/pt-PT/index.json"),
    ),
    (
        "ro-RO",
        include_str!("../../app-frontend/src/locales/ro-RO/index.json"),
    ),
    (
        "ru-RU",
        include_str!("../../app-frontend/src/locales/ru-RU/index.json"),
    ),
    (
        "sr-CS",
        include_str!("../../app-frontend/src/locales/sr-CS/index.json"),
    ),
    (
        "sv-SE",
        include_str!("../../app-frontend/src/locales/sv-SE/index.json"),
    ),
    (
        "th-TH",
        include_str!("../../app-frontend/src/locales/th-TH/index.json"),
    ),
    (
        "tr-TR",
        include_str!("../../app-frontend/src/locales/tr-TR/index.json"),
    ),
    (
        "uk-UA",
        include_str!("../../app-frontend/src/locales/uk-UA/index.json"),
    ),
    (
        "vi-VN",
        include_str!("../../app-frontend/src/locales/vi-VN/index.json"),
    ),
    (
        "zh-CN",
        include_str!("../../app-frontend/src/locales/zh-CN/index.json"),
    ),
    (
        "zh-TW",
        include_str!("../../app-frontend/src/locales/zh-TW/index.json"),
    ),
];

#[cfg(test)]
mod tests {
    use super::{set_locale, t};

    #[test]
    fn selects_the_persisted_locale_and_falls_back() {
        set_locale("zh-CN");
        assert_eq!(t("Home"), "主页");
        assert_eq!(t("Saved skin 3"), "Saved skin 3");

        set_locale("en-US");
        assert_eq!(t("Home"), "Home");

        set_locale("de-DE");
        assert_eq!(t("Home"), "Start");
        set_locale("fr");
        assert_eq!(t("Home"), "Accueil");
        set_locale("ja-JP");
        assert_eq!(t("Home"), "ホーム");
        set_locale("zh-TW");
        assert_eq!(t("Home"), "首頁");
    }
}
