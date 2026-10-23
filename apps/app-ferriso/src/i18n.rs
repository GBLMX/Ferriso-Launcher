//! Presentation-layer localization for the native frontend.
//!
//! The launcher backend persists the selected locale in `Settings.locale`
//! (the same setting the original app uses); this module turns that code into
//! UI copy. English is the fallback for every unknown string.

use std::cell::Cell;

thread_local! {
    static ZH_CN: Cell<bool> = const { Cell::new(false) };
}

/// Select the language dictionary for the current view build.
pub fn set_locale(locale: &str) {
    let locale = locale.to_ascii_lowercase();
    ZH_CN.with(|zh| zh.set(locale.starts_with("zh")));
}

/// Translate a UI string. Unknown strings pass through unchanged so metadata
/// names, versions, paths, and backend errors are never rewritten.
pub fn t(value: &str) -> &str {
    if !ZH_CN.with(Cell::get) {
        return value;
    }

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
        "Working…" => "处理中…",
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
    }
}
