//! Semantic tree and action bridge for screen readers.
//!
//! iced does not expose widget semantics, so this tree mirrors the stable
//! structure of the launcher: navigation, page status, refresh, and the
//! settings that can be toggled without a pointer. Node ids are stable and
//! actions are translated back into the same messages the widgets send.

use ferriso_access::{
    Action, ActionRequest, Node, NodeId, Role, TreeId, TreeInfo, TreeUpdate,
};

use crate::{
    app::{Message, Page, SettingChange, ViewModel},
    i18n::t,
};

const ROOT: u64 = 1;
const NAV: u64 = 2;
const MAIN: u64 = 3;
const REFRESH: u64 = 4;
const STATUS: u64 = 5;
const DETAIL: u64 = 6;

const NAV_BASE: u64 = 100;
const SETTING_BASE: u64 = 200;

const SETTING_COUNT: usize = 8;

pub fn publish(view: &ViewModel) {
    ferriso_access::set_tree(tree(view));
}

pub fn tree(view: &ViewModel) -> TreeUpdate {
    let page = view.page;
    let mut nodes = Vec::new();

    let mut nav = Node::new(Role::Navigation);
    nav.set_label(t("Navigation"));
    let nav_children = Page::NAV
        .iter()
        .enumerate()
        .map(|(index, item)| {
            let id = NodeId(NAV_BASE + index as u64);
            let mut node = Node::new(Role::Button);
            node.set_label(t(item.label()));
            node.set_selected(*item == page);
            node.add_action(Action::Click);
            nodes.push((id, node));
            id
        })
        .collect::<Vec<_>>();
    nav.set_children(nav_children.clone());
    nodes.push((NodeId(NAV), nav));

    let mut main = Node::new(Role::Main);
    main.set_label(t(page.label()));
    let mut main_children = Vec::new();

    let mut detail = Node::new(Role::Label);
    detail.set_value(detail_text(view));
    nodes.push((NodeId(DETAIL), detail));
    main_children.push(NodeId(DETAIL));

    if !view.status.is_empty() {
        let mut status = Node::new(Role::Status);
        status.set_label(&view.status);
        status.set_live(ferriso_access::Live::Polite);
        nodes.push((NodeId(STATUS), status));
        main_children.push(NodeId(STATUS));
    }

    if page == Page::Settings
        && let Some(settings) = view.settings.as_ref()
    {
        for (index, (label, toggled)) in
            setting_states(settings).into_iter().enumerate()
        {
            let id = NodeId(SETTING_BASE + index as u64);
            let mut node = Node::new(Role::CheckBox);
            node.set_label(t(label));
            node.set_toggled(toggled.into());
            node.add_action(Action::Click);
            nodes.push((id, node));
            main_children.push(id);
        }

        let mut language = Node::new(Role::Label);
        language.set_value(format!("{}: {}", t("Language"), settings.locale));
        let language_id = NodeId(SETTING_BASE + SETTING_COUNT as u64);
        nodes.push((language_id, language));
        main_children.push(language_id);
    }

    main.set_children(main_children.clone());
    nodes.push((NodeId(MAIN), main));

    let mut refresh = Node::new(Role::Button);
    refresh.set_label(t("Refresh"));
    refresh.add_action(Action::Click);
    nodes.push((NodeId(REFRESH), refresh));

    let mut root = Node::new(Role::Window);
    root.set_label("Ferriso Launcher");
    root.set_children([NodeId(NAV), NodeId(MAIN), NodeId(REFRESH)]);
    nodes.push((NodeId(ROOT), root));

    TreeUpdate {
        nodes,
        tree: Some(TreeInfo::new(NodeId(ROOT))),
        tree_id: TreeId::ROOT,
        focus: NodeId(ROOT),
    }
}

/// Translate queued platform actions into core messages.
pub fn drain_messages() -> Vec<Message> {
    ferriso_access::drain_actions()
        .into_iter()
        .filter_map(message_for)
        .collect()
}

fn message_for(request: ActionRequest) -> Option<Message> {
    if request.action != Action::Click {
        return None;
    }

    let id = request.target_node.0;

    if let Some(index) = id.checked_sub(NAV_BASE)
        && let Some(page) = Page::NAV.get(index as usize)
    {
        return Some(Message::Navigate(*page));
    }

    if id == REFRESH {
        return Some(Message::Refresh);
    }

    if let Some(index) = id.checked_sub(SETTING_BASE) {
        return setting_change(index as usize).map(Message::SettingChanged);
    }

    None
}

fn setting_states(
    settings: &theseus::data::Settings,
) -> [(&'static str, bool); SETTING_COUNT] {
    [
        ("Telemetry", settings.telemetry),
        ("Discord presence", settings.discord_rpc),
        ("Advanced rendering", settings.advanced_rendering),
        ("Native decorations", settings.native_decorations),
        ("Force fullscreen", settings.force_fullscreen),
        ("Hide on game start", settings.hide_on_process_start),
        (
            "Reduce motion",
            settings
                .feature_flags
                .get(&theseus::data::FeatureFlag::ReduceMotion)
                .copied()
                .unwrap_or(false),
        ),
        ("Toggle sidebar", settings.toggle_sidebar),
    ]
}

fn setting_change(index: usize) -> Option<SettingChange> {
    match index {
        0 => Some(SettingChange::ToggleTelemetry),
        1 => Some(SettingChange::ToggleDiscordRpc),
        2 => Some(SettingChange::ToggleAdvancedRendering),
        3 => Some(SettingChange::ToggleNativeDecorations),
        4 => Some(SettingChange::ToggleForceFullscreen),
        5 => Some(SettingChange::ToggleHideOnProcessStart),
        6 => Some(SettingChange::ToggleReduceMotion),
        7 => Some(SettingChange::ToggleSidebar),
        _ => None,
    }
}

fn detail_text(view: &ViewModel) -> String {
    match view.page {
        Page::Settings => t("Preferences").to_string(),
        Page::Home => view
            .instances
            .first()
            .map(|instance| instance.name.clone())
            .unwrap_or_else(|| t("No instances yet").to_string()),
        Page::Library => {
            format!("{}: {}", t("Instances"), view.instances.len())
        }
        Page::Browse => t("Browse").to_string(),
        Page::Project => t("Project").to_string(),
        Page::Instance => view
            .selected
            .clone()
            .unwrap_or_else(|| t("Instance").to_string()),
        Page::Account => t("Account").to_string(),
        Page::Skins => t("Skins").to_string(),
        Page::Hosting => t("Hosting").to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn click(node: u64) -> ActionRequest {
        ActionRequest {
            action: Action::Click,
            target_tree: TreeId::ROOT,
            target_node: NodeId(node),
            data: None,
        }
    }

    #[test]
    fn navigation_actions_map_to_pages() {
        for (index, page) in Page::NAV.iter().enumerate() {
            let message = message_for(click(NAV_BASE + index as u64)).unwrap();
            assert!(
                matches!(message, Message::Navigate(target) if target == *page)
            );
        }
    }

    #[test]
    fn settings_actions_are_unique_and_toggle_all_rows() {
        let mut pending = Vec::new();
        for index in 0..SETTING_COUNT {
            let change = setting_change(index).unwrap();
            pending.push(std::mem::discriminant(&change));
            let message = message_for(click(SETTING_BASE + index as u64));
            assert!(matches!(message, Some(Message::SettingChanged(_))));
        }

        for (index, discriminant) in pending.iter().enumerate() {
            assert!(!pending[..index].contains(discriminant));
        }
    }

    #[test]
    fn refresh_and_unknown_actions_are_handled() {
        assert!(matches!(
            message_for(click(REFRESH)),
            Some(Message::Refresh)
        ));
        assert!(message_for(click(9_999)).is_none());
    }
}
