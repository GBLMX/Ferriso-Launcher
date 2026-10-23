//! AccessKit bridge between Ferriso's immutable view model and winit.
//!
//! The tree is intentionally renderer-neutral: it exposes navigation,
//! settings and status controls, not pixel geometry. Actions are queued here
//! and drained by the iced subscription, so the platform adapter never needs
//! to know about application messages.

use std::{
    collections::VecDeque,
    sync::{Mutex, OnceLock},
};

pub use accesskit::{
    Action, ActionRequest, Live, Node, NodeId, Role, TreeId, TreeInfo,
    TreeUpdate,
};
use accesskit_winit::Adapter as PlatformAdapter;
use winit::{event_loop::ActiveEventLoop, window::Window};

struct State {
    tree: Option<TreeUpdate>,
    revision: u64,
    actions: VecDeque<ActionRequest>,
}

fn state() -> &'static Mutex<State> {
    static STATE: OnceLock<Mutex<State>> = OnceLock::new();
    STATE.get_or_init(|| {
        Mutex::new(State {
            tree: None,
            revision: 0,
            actions: VecDeque::new(),
        })
    })
}

/// Publish a full tree for the current view. Unchanged trees are ignored so
/// animation ticks do not spam the platform adapter.
pub fn set_tree(tree: TreeUpdate) {
    let mut state = state().lock().expect("accessibility state poisoned");
    if state.tree.as_ref() == Some(&tree) {
        return;
    }
    state.tree = Some(tree);
    state.revision = state.revision.wrapping_add(1);
}

/// Revision number of the last published tree.
pub fn revision() -> u64 {
    state()
        .lock()
        .expect("accessibility state poisoned")
        .revision
}

/// Full tree to send to the platform adapter.
pub fn tree() -> TreeUpdate {
    state()
        .lock()
        .expect("accessibility state poisoned")
        .tree
        .clone()
        .unwrap_or_else(placeholder_tree)
}

/// Remove all queued actions, newest last.
pub fn drain_actions() -> Vec<ActionRequest> {
    state()
        .lock()
        .expect("accessibility state poisoned")
        .actions
        .drain(..)
        .collect()
}

/// Create a platform adapter for an invisible winit window.
pub fn adapter(
    event_loop: &ActiveEventLoop,
    window: &Window,
) -> PlatformAdapter {
    PlatformAdapter::with_direct_handlers(
        event_loop,
        window,
        Activation,
        Actions,
        Deactivation,
    )
}

struct Activation;

impl accesskit::ActivationHandler for Activation {
    fn request_initial_tree(&mut self) -> Option<TreeUpdate> {
        state()
            .lock()
            .expect("accessibility state poisoned")
            .tree
            .clone()
    }
}

struct Actions;

impl accesskit::ActionHandler for Actions {
    fn do_action(&mut self, request: ActionRequest) {
        state()
            .lock()
            .expect("accessibility state poisoned")
            .actions
            .push_back(request);
    }
}

struct Deactivation;

impl accesskit::DeactivationHandler for Deactivation {
    fn deactivate_accessibility(&mut self) {}
}

fn placeholder_tree() -> TreeUpdate {
    let root = NodeId(1);
    let mut node = Node::new(Role::Window);
    node.set_label("Ferriso Launcher");
    TreeUpdate {
        nodes: vec![(root, node)],
        tree: Some(TreeInfo::new(root)),
        tree_id: TreeId::ROOT,
        focus: root,
    }
}

// Keeps the adapter event type reachable for the vendored iced_winit shim
// without depending on accesskit_winit directly there.
pub use accesskit_winit::Adapter;
