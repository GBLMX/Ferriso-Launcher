//! iced renderer for Ferriso Launcher.
//!
//! The renderer owns only presentation and keyboard translation. It never
//! mutates launcher state; every action is a semantic message sent to the
//! Ferriso actor.

use std::time::Duration;

use bytes::Bytes;
use ferriso_gui::iced::{
    self, Alignment, Background, Border, Color, ContentFit, Element, Length,
    Radians, Shadow, Theme, Vector,
    gradient::Linear,
    keyboard, mouse, time,
    widget::{
        Space, button, column, container, image, mouse_area, progress_bar, row,
        scrollable, text, text_input,
    },
};
use ferriso_gui::{Message as GuiMessage, Renderer};
use ferriso_keymap::Keymap;
use theseus::data::Theme as AppTheme;
use theseus::instance::{FAVORITES_GROUP_ID, InstanceGroup};

use crate::app::{
    ContentRow, DetailTab, DetailView, GroupForm, InstanceEditForm,
    InstanceView, JobView, JreChoice, LibraryForm, LogView, Message, Page,
    ProcessView, ProjectKind, ProjectView, SettingChange, SkinTab,
    VersionPicker, ViewModel, WorldArt, WorldForm, WorldFormKind, WorldRow,
};
use crate::i18n::{self, t};

type Msg = GuiMessage<ViewModel, Message>;

const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Color {
    Color::from_rgba8(red, green, blue, alpha as f32 / 255.0)
}

#[derive(Clone, Copy)]
struct Palette {
    bg_top: Color,
    bg_mid: Color,
    bg_bottom: Color,
    border: Color,
    border_strong: Color,
    accent: Color,
    accent_cyan: Color,
    text: Color,
    muted: Color,
    error: Color,
    warning: Color,
    ink: Color,
    shadow: Color,
    light: bool,
    panel_a: Color,
    panel_b: Color,
    surface_a: Color,
    surface_b: Color,
    card_a: Color,
    card_b: Color,
    stat_a: Color,
    stat_b: Color,
    row_selected_a: Color,
    row_selected_b: Color,
    input_bg: Color,
    input_focus: Color,
    danger_a: Color,
    danger_b: Color,
    hero_a: Color,
    hero_b: Color,
    hero_c: Color,
}

const DARK: Palette = Palette {
    // bionickernel.com · CachyOS Nord dark
    bg_top: Color::from_rgb8(30, 34, 51),
    bg_mid: Color::from_rgb8(24, 28, 44),
    bg_bottom: Color::from_rgb8(22, 25, 37),
    border: rgba(102, 194, 242, 26),
    border_strong: rgba(102, 194, 242, 58),
    accent: Color::from_rgb8(71, 173, 214),
    accent_cyan: Color::from_rgb8(102, 194, 242),
    text: Color::from_rgb8(224, 240, 250),
    muted: Color::from_rgb8(129, 161, 193),
    error: Color::from_rgb8(255, 122, 138),
    warning: Color::from_rgb8(235, 203, 139),
    ink: Color::from_rgb8(10, 11, 12),
    shadow: rgba(0, 0, 0, 130),
    light: false,
    panel_a: rgba(22, 25, 37, 220),
    panel_b: rgba(16, 19, 29, 238),
    surface_a: rgba(26, 31, 45, 225),
    surface_b: rgba(18, 22, 33, 238),
    card_a: rgba(29, 35, 51, 210),
    card_b: rgba(19, 23, 35, 228),
    stat_a: rgba(33, 40, 58, 220),
    stat_b: rgba(20, 24, 37, 235),
    row_selected_a: rgba(39, 66, 86, 230),
    row_selected_b: rgba(25, 38, 53, 235),
    input_bg: rgba(102, 194, 242, 10),
    input_focus: rgba(28, 33, 48, 245),
    danger_a: rgba(88, 30, 40, 220),
    danger_b: rgba(39, 17, 24, 230),
    hero_a: rgba(24, 55, 78, 220),
    hero_b: rgba(24, 33, 50, 230),
    hero_c: rgba(17, 21, 33, 240),
};

const LIGHT: Palette = Palette {
    // bionickernel.com · Breeze Light
    bg_top: Color::from_rgb8(239, 240, 241),
    bg_mid: Color::from_rgb8(234, 241, 246),
    bg_bottom: Color::from_rgb8(225, 237, 245),
    border: rgba(35, 38, 41, 30),
    border_strong: rgba(35, 38, 41, 58),
    accent: Color::from_rgb8(61, 174, 233),
    accent_cyan: Color::from_rgb8(27, 106, 151),
    text: Color::from_rgb8(35, 38, 41),
    muted: Color::from_rgb8(92, 107, 120),
    error: Color::from_rgb8(194, 59, 75),
    warning: Color::from_rgb8(167, 107, 8),
    ink: Color::from_rgb8(11, 28, 38),
    shadow: rgba(35, 50, 65, 55),
    light: true,
    panel_a: rgba(255, 255, 255, 222),
    panel_b: rgba(241, 247, 251, 238),
    surface_a: rgba(255, 255, 255, 232),
    surface_b: rgba(238, 245, 250, 242),
    card_a: rgba(255, 255, 255, 214),
    card_b: rgba(244, 249, 252, 230),
    stat_a: rgba(255, 255, 255, 224),
    stat_b: rgba(238, 246, 251, 234),
    row_selected_a: rgba(205, 232, 247, 235),
    row_selected_b: rgba(226, 242, 251, 240),
    input_bg: rgba(255, 255, 255, 205),
    input_focus: rgba(255, 255, 255, 250),
    danger_a: rgba(255, 226, 228, 235),
    danger_b: rgba(255, 241, 242, 240),
    hero_a: rgba(207, 233, 247, 238),
    hero_b: rgba(246, 250, 252, 240),
    hero_c: rgba(227, 240, 248, 242),
};

const OLED: Palette = Palette {
    bg_top: Color::from_rgb8(0, 0, 0),
    bg_mid: Color::from_rgb8(0, 0, 0),
    bg_bottom: Color::from_rgb8(0, 0, 0),
    border: rgba(102, 194, 242, 22),
    border_strong: rgba(102, 194, 242, 56),
    accent: Color::from_rgb8(71, 173, 214),
    accent_cyan: Color::from_rgb8(102, 194, 242),
    text: Color::from_rgb8(236, 247, 255),
    muted: Color::from_rgb8(129, 161, 193),
    error: Color::from_rgb8(255, 122, 138),
    warning: Color::from_rgb8(235, 203, 139),
    ink: Color::from_rgb8(0, 8, 14),
    shadow: rgba(0, 0, 0, 200),
    light: false,
    panel_a: rgba(3, 6, 11, 225),
    panel_b: rgba(0, 0, 0, 238),
    surface_a: rgba(4, 8, 14, 228),
    surface_b: rgba(0, 0, 0, 240),
    card_a: rgba(5, 10, 17, 225),
    card_b: rgba(0, 0, 0, 235),
    stat_a: rgba(6, 12, 20, 228),
    stat_b: rgba(0, 0, 0, 240),
    row_selected_a: rgba(18, 43, 60, 225),
    row_selected_b: rgba(1, 10, 17, 232),
    input_bg: rgba(102, 194, 242, 9),
    input_focus: rgba(3, 9, 15, 245),
    danger_a: rgba(60, 12, 20, 230),
    danger_b: rgba(20, 0, 4, 240),
    hero_a: rgba(5, 29, 44, 232),
    hero_b: rgba(0, 8, 14, 240),
    hero_c: rgba(0, 0, 0, 244),
};

const RETRO: Palette = Palette {
    bg_top: Color::from_rgb8(25, 19, 12),
    bg_mid: Color::from_rgb8(34, 25, 14),
    bg_bottom: Color::from_rgb8(44, 30, 14),
    border: rgba(255, 224, 160, 30),
    border_strong: rgba(255, 224, 160, 60),
    accent: Color::from_rgb8(242, 184, 75),
    accent_cyan: Color::from_rgb8(122, 215, 200),
    text: Color::from_rgb8(246, 232, 205),
    muted: Color::from_rgb8(185, 163, 125),
    error: Color::from_rgb8(255, 138, 122),
    warning: Color::from_rgb8(255, 209, 102),
    ink: Color::from_rgb8(36, 23, 7),
    shadow: rgba(0, 0, 0, 120),
    light: false,
    panel_a: rgba(43, 31, 18, 220),
    panel_b: rgba(24, 17, 10, 235),
    surface_a: rgba(48, 35, 20, 225),
    surface_b: rgba(27, 19, 11, 238),
    card_a: rgba(52, 38, 21, 215),
    card_b: rgba(29, 21, 12, 230),
    stat_a: rgba(58, 42, 23, 220),
    stat_b: rgba(30, 22, 13, 235),
    row_selected_a: rgba(92, 67, 32, 225),
    row_selected_b: rgba(45, 32, 16, 235),
    input_bg: rgba(255, 224, 160, 12),
    input_focus: rgba(58, 43, 24, 245),
    danger_a: rgba(92, 38, 32, 225),
    danger_b: rgba(44, 18, 14, 236),
    hero_a: rgba(70, 52, 26, 230),
    hero_b: rgba(42, 30, 16, 238),
    hero_c: rgba(25, 18, 10, 242),
};

const VIOLET: Palette = Palette {
    bg_top: Color::from_rgb8(36, 31, 51),
    bg_mid: Color::from_rgb8(29, 25, 43),
    bg_bottom: Color::from_rgb8(23, 20, 32),
    border: rgba(177, 140, 255, 28),
    border_strong: rgba(177, 140, 255, 62),
    accent: Color::from_rgb8(177, 140, 255),
    accent_cyan: Color::from_rgb8(214, 184, 255),
    text: Color::from_rgb8(240, 234, 255),
    muted: Color::from_rgb8(169, 155, 191),
    error: Color::from_rgb8(255, 122, 138),
    warning: Color::from_rgb8(235, 203, 139),
    ink: Color::from_rgb8(18, 10, 30),
    shadow: rgba(0, 0, 0, 130),
    light: false,
    panel_a: rgba(30, 25, 44, 220),
    panel_b: rgba(20, 16, 31, 238),
    surface_a: rgba(35, 29, 51, 225),
    surface_b: rgba(23, 18, 35, 238),
    card_a: rgba(39, 32, 57, 210),
    card_b: rgba(25, 20, 38, 228),
    stat_a: rgba(43, 35, 63, 220),
    stat_b: rgba(27, 21, 42, 235),
    row_selected_a: rgba(65, 48, 94, 230),
    row_selected_b: rgba(40, 29, 60, 235),
    input_bg: rgba(177, 140, 255, 11),
    input_focus: rgba(35, 29, 51, 245),
    danger_a: rgba(88, 30, 40, 220),
    danger_b: rgba(39, 17, 24, 230),
    hero_a: rgba(62, 43, 91, 225),
    hero_b: rgba(33, 26, 49, 235),
    hero_c: rgba(23, 18, 35, 242),
};

const ROSE: Palette = Palette {
    bg_top: Color::from_rgb8(42, 31, 43),
    bg_mid: Color::from_rgb8(36, 26, 39),
    bg_bottom: Color::from_rgb8(29, 21, 32),
    border: rgba(255, 143, 179, 28),
    border_strong: rgba(255, 143, 179, 62),
    accent: Color::from_rgb8(255, 143, 179),
    accent_cyan: Color::from_rgb8(255, 194, 214),
    text: Color::from_rgb8(255, 234, 241),
    muted: Color::from_rgb8(195, 163, 177),
    error: Color::from_rgb8(255, 122, 138),
    warning: Color::from_rgb8(235, 203, 139),
    ink: Color::from_rgb8(36, 11, 21),
    shadow: rgba(0, 0, 0, 130),
    light: false,
    panel_a: rgba(35, 26, 38, 220),
    panel_b: rgba(24, 17, 27, 238),
    surface_a: rgba(41, 30, 44, 225),
    surface_b: rgba(27, 19, 31, 238),
    card_a: rgba(46, 34, 50, 210),
    card_b: rgba(29, 21, 33, 228),
    stat_a: rgba(51, 37, 55, 220),
    stat_b: rgba(31, 22, 36, 235),
    row_selected_a: rgba(94, 52, 72, 230),
    row_selected_b: rgba(58, 32, 46, 235),
    input_bg: rgba(255, 143, 179, 11),
    input_focus: rgba(41, 30, 44, 245),
    danger_a: rgba(88, 30, 40, 220),
    danger_b: rgba(39, 17, 24, 230),
    hero_a: rgba(94, 43, 65, 225),
    hero_b: rgba(45, 30, 42, 235),
    hero_c: rgba(29, 21, 33, 242),
};

const TEAL: Palette = Palette {
    bg_top: Color::from_rgb8(19, 38, 41),
    bg_mid: Color::from_rgb8(16, 34, 36),
    bg_bottom: Color::from_rgb8(12, 28, 30),
    border: rgba(79, 209, 197, 28),
    border_strong: rgba(79, 209, 197, 62),
    accent: Color::from_rgb8(79, 209, 197),
    accent_cyan: Color::from_rgb8(127, 240, 231),
    text: Color::from_rgb8(227, 251, 248),
    muted: Color::from_rgb8(143, 184, 180),
    error: Color::from_rgb8(255, 122, 138),
    warning: Color::from_rgb8(235, 203, 139),
    ink: Color::from_rgb8(3, 22, 21),
    shadow: rgba(0, 0, 0, 130),
    light: false,
    panel_a: rgba(15, 35, 37, 220),
    panel_b: rgba(9, 25, 26, 238),
    surface_a: rgba(18, 40, 42, 225),
    surface_b: rgba(11, 29, 30, 238),
    card_a: rgba(21, 44, 47, 210),
    card_b: rgba(13, 32, 34, 228),
    stat_a: rgba(24, 49, 52, 220),
    stat_b: rgba(14, 35, 37, 235),
    row_selected_a: rgba(29, 75, 75, 230),
    row_selected_b: rgba(18, 49, 51, 235),
    input_bg: rgba(79, 209, 197, 11),
    input_focus: rgba(18, 40, 42, 245),
    danger_a: rgba(88, 30, 40, 220),
    danger_b: rgba(39, 17, 24, 230),
    hero_a: rgba(23, 70, 74, 225),
    hero_b: rgba(17, 43, 46, 235),
    hero_c: rgba(11, 29, 30, 242),
};

const CRIMSON: Palette = Palette {
    bg_top: Color::from_rgb8(43, 26, 30),
    bg_mid: Color::from_rgb8(36, 22, 26),
    bg_bottom: Color::from_rgb8(29, 17, 20),
    border: rgba(255, 107, 122, 28),
    border_strong: rgba(255, 107, 122, 62),
    accent: Color::from_rgb8(255, 107, 122),
    accent_cyan: Color::from_rgb8(255, 179, 187),
    text: Color::from_rgb8(255, 236, 238),
    muted: Color::from_rgb8(198, 160, 165),
    error: Color::from_rgb8(255, 145, 155),
    warning: Color::from_rgb8(235, 203, 139),
    ink: Color::from_rgb8(34, 7, 11),
    shadow: rgba(0, 0, 0, 130),
    light: false,
    panel_a: rgba(35, 21, 25, 220),
    panel_b: rgba(24, 14, 17, 238),
    surface_a: rgba(41, 25, 29, 225),
    surface_b: rgba(27, 16, 19, 238),
    card_a: rgba(46, 29, 33, 210),
    card_b: rgba(29, 18, 21, 228),
    stat_a: rgba(51, 32, 37, 220),
    stat_b: rgba(31, 19, 23, 235),
    row_selected_a: rgba(94, 43, 51, 230),
    row_selected_b: rgba(58, 28, 34, 235),
    input_bg: rgba(255, 107, 122, 11),
    input_focus: rgba(41, 25, 29, 245),
    danger_a: rgba(105, 28, 36, 225),
    danger_b: rgba(47, 14, 19, 235),
    hero_a: rgba(92, 41, 48, 225),
    hero_b: rgba(45, 27, 31, 235),
    hero_c: rgba(29, 18, 21, 242),
};

const EMERALD: Palette = Palette {
    bg_top: Color::from_rgb8(18, 39, 33),
    bg_mid: Color::from_rgb8(15, 34, 29),
    bg_bottom: Color::from_rgb8(11, 28, 24),
    border: rgba(52, 211, 153, 28),
    border_strong: rgba(52, 211, 153, 62),
    accent: Color::from_rgb8(52, 211, 153),
    accent_cyan: Color::from_rgb8(110, 231, 183),
    text: Color::from_rgb8(228, 251, 242),
    muted: Color::from_rgb8(143, 184, 169),
    error: Color::from_rgb8(255, 122, 138),
    warning: Color::from_rgb8(235, 203, 139),
    ink: Color::from_rgb8(3, 23, 17),
    shadow: rgba(0, 0, 0, 130),
    light: false,
    panel_a: rgba(14, 34, 29, 220),
    panel_b: rgba(8, 25, 21, 238),
    surface_a: rgba(17, 39, 33, 225),
    surface_b: rgba(10, 29, 24, 238),
    card_a: rgba(20, 44, 37, 210),
    card_b: rgba(12, 32, 27, 228),
    stat_a: rgba(23, 49, 42, 220),
    stat_b: rgba(13, 35, 29, 235),
    row_selected_a: rgba(28, 74, 60, 230),
    row_selected_b: rgba(17, 49, 40, 235),
    input_bg: rgba(52, 211, 153, 11),
    input_focus: rgba(17, 39, 33, 245),
    danger_a: rgba(88, 30, 40, 220),
    danger_b: rgba(39, 17, 24, 230),
    hero_a: rgba(22, 70, 56, 225),
    hero_b: rgba(16, 43, 36, 235),
    hero_c: rgba(10, 29, 24, 242),
};

thread_local! {
    static ACTIVE_PALETTE: std::cell::Cell<Palette> =
        const { std::cell::Cell::new(DARK) };
}

/// Selects the color palette used by the native renderer. Called from `view`
/// before elements are built, so every style and explicit text color follows
/// the saved launcher theme immediately.
fn palette_for(theme: &str, system_light: bool) -> Palette {
    match theme {
        "light" => LIGHT,
        "oled" => OLED,
        "retro" => RETRO,
        "violet" => VIOLET,
        "rose" => ROSE,
        "teal" => TEAL,
        "crimson" => CRIMSON,
        "emerald" => EMERALD,
        "system" if system_light => LIGHT,
        _ => DARK,
    }
}

pub(crate) fn set_theme(theme: &str, system_light: bool) {
    ACTIVE_PALETTE.with(|active| active.set(palette_for(theme, system_light)));
}

fn palette() -> Palette {
    ACTIVE_PALETTE.with(std::cell::Cell::get)
}

fn bg_top() -> Color {
    palette().bg_top
}
fn bg_mid() -> Color {
    palette().bg_mid
}
fn bg_bottom() -> Color {
    palette().bg_bottom
}
fn border_c() -> Color {
    palette().border
}
fn border_strong_c() -> Color {
    palette().border_strong
}
fn accent_c() -> Color {
    palette().accent
}
fn accent_cyan_c() -> Color {
    palette().accent_cyan
}
fn fg() -> Color {
    palette().text
}
fn muted_c() -> Color {
    palette().muted
}
fn danger_c() -> Color {
    palette().error
}
fn warning_c() -> Color {
    palette().warning
}
fn ink_c() -> Color {
    palette().ink
}
fn shadow_c() -> Color {
    palette().shadow
}
fn overlay(alpha: u8) -> Color {
    if palette().light {
        rgba(0, 0, 0, alpha)
    } else {
        rgba(255, 255, 255, alpha)
    }
}
fn blend(from: Color, to: Color, amount: f32) -> Color {
    let amount = amount.clamp(0.0, 1.0);
    Color::from_rgba8(
        ((from.r * 255.0) * (1.0 - amount) + (to.r * 255.0) * amount) as u8,
        ((from.g * 255.0) * (1.0 - amount) + (to.g * 255.0) * amount) as u8,
        ((from.b * 255.0) * (1.0 - amount) + (to.b * 255.0) * amount) as u8,
        from.a,
    )
}

fn with_alpha(color: Color, alpha: u8) -> Color {
    Color::from_rgba8(
        (color.r * 255.0) as u8,
        (color.g * 255.0) as u8,
        (color.b * 255.0) as u8,
        alpha as f32 / 255.0,
    )
}
fn accent_hover() -> Color {
    let accent = accent_c();
    Color::from_rgb8(
        ((accent.r * 255.0) as u8).saturating_add(30),
        ((accent.g * 255.0) as u8).saturating_add(20),
        ((accent.b * 255.0) as u8).saturating_add(20),
    )
}
fn error_hover() -> Color {
    let error = danger_c();
    Color::from_rgb8(
        ((error.r * 255.0) as u8).saturating_add(20),
        ((error.g * 255.0) as u8).saturating_sub(15),
        ((error.b * 255.0) as u8).saturating_sub(8),
    )
}

const LOADER_LABELS: [&str; 5] =
    ["Vanilla", "Fabric", "Forge", "NeoForge", "Quilt"];

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Shortcut {
    Home,
    Browse,
    Library,
    Account,
    Skins,
    Settings,
    Refresh,
    Quit,
}

pub struct FerrisoGui {
    keymap: Keymap<Shortcut>,
    title: String,
    system_theme_probed: bool,
}

impl FerrisoGui {
    pub fn new() -> Self {
        Self::with_title("Ferriso Launcher")
    }

    pub fn with_title(title: impl Into<String>) -> Self {
        let title = title.into();
        let title = if title.trim().is_empty() {
            "Ferriso Launcher".into()
        } else {
            title
        };
        let mut keymap = Keymap::new();
        for (binding, shortcut) in [
            ("Ctrl+1", Shortcut::Home),
            ("Ctrl+2", Shortcut::Browse),
            ("Ctrl+3", Shortcut::Library),
            ("Ctrl+4", Shortcut::Account),
            ("Ctrl+5", Shortcut::Skins),
            ("Ctrl+6", Shortcut::Settings),
            ("Ctrl+r", Shortcut::Refresh),
            ("Ctrl+q", Shortcut::Quit),
        ] {
            keymap
                .bind_str(binding, shortcut)
                .expect("static native app binding is valid");
        }
        Self {
            keymap,
            title,
            system_theme_probed: false,
        }
    }
}

impl Default for FerrisoGui {
    fn default() -> Self {
        Self::new()
    }
}

impl Renderer<ViewModel, Message> for FerrisoGui {
    fn view<'a>(&'a self, view: &'a ViewModel) -> Element<'a, Msg> {
        i18n::set_locale(
            view.settings
                .as_ref()
                .map(|settings| settings.locale.as_str())
                .unwrap_or("en-US"),
        );
        set_theme(
            view.settings
                .as_ref()
                .map(|settings| settings.theme.as_str())
                .unwrap_or("dark"),
            view.system_light,
        );
        let sidebar = sidebar(view);
        let main = column![top_bar(view), content(view)]
            .spacing(14)
            .width(Length::Fill)
            .height(Length::Fill);
        let backdrop = ambient_background(view.phase);

        container(row![sidebar, main].spacing(14))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(14)
            .style(move |_| container::Style {
                background: Some(backdrop),
                text_color: Some(fg()),
                ..Default::default()
            })
            .into()
    }

    fn subscription(&self, _view: &ViewModel) -> iced::Subscription<Msg> {
        let keys = keyboard::listen().with(self.keymap.clone()).filter_map(
            |(keymap, event)| {
                let keyboard::Event::KeyPressed { key, modifiers, .. } = event
                else {
                    return None;
                };
                let chord = ferriso_gui::keymap::chord(&key, modifiers)?;
                match keymap.resolve(&chord).copied()? {
                    Shortcut::Home => {
                        Some(Msg::User(Message::Navigate(Page::Home)))
                    }
                    Shortcut::Browse => {
                        Some(Msg::User(Message::Navigate(Page::Browse)))
                    }
                    Shortcut::Library => {
                        Some(Msg::User(Message::Navigate(Page::Library)))
                    }
                    Shortcut::Account => {
                        Some(Msg::User(Message::Navigate(Page::Account)))
                    }
                    Shortcut::Skins => {
                        Some(Msg::User(Message::Navigate(Page::Skins)))
                    }
                    Shortcut::Settings => {
                        Some(Msg::User(Message::Navigate(Page::Settings)))
                    }
                    Shortcut::Refresh => Some(Msg::User(Message::Refresh)),
                    Shortcut::Quit => Some(Msg::Closed),
                }
            },
        );

        let jobs = time::every(Duration::from_secs(1))
            .map(|_| Msg::User(Message::PollJobs));
        let processes = time::every(Duration::from_secs(1))
            .map(|_| Msg::User(Message::PollProcesses));
        let ambience = time::every(Duration::from_millis(90))
            .map(|_| Msg::User(Message::AnimationTick));

        let updates = time::every(Duration::from_secs(300))
            .map(|_| Msg::User(Message::CheckUpdates));
        let system = iced::system::theme_changes().map(|mode| {
            Msg::User(Message::SystemThemeChanged(
                mode == iced::theme::Mode::Light,
            ))
        });

        iced::Subscription::batch([
            keys, jobs, processes, ambience, updates, system,
        ])
    }

    fn on_snapshot_task(&mut self, _view: &ViewModel) -> iced::Task<Msg> {
        if self.system_theme_probed {
            return iced::Task::none();
        }
        self.system_theme_probed = true;
        iced::system::theme().map(|mode| {
            Msg::User(Message::SystemThemeChanged(
                mode == iced::theme::Mode::Light,
            ))
        })
    }

    fn theme(&self, view: &ViewModel) -> Theme {
        let theme = view
            .settings
            .as_ref()
            .map(|settings| settings.theme.as_str())
            .unwrap_or("dark");
        if theme == "light" || (theme == "system" && view.system_light) {
            Theme::Light
        } else {
            Theme::Dark
        }
    }

    fn title(&self, view: &ViewModel) -> Option<String> {
        i18n::set_locale(
            view.settings
                .as_ref()
                .map(|settings| settings.locale.as_str())
                .unwrap_or("en-US"),
        );
        Some(format!("{} · {}", self.title, t(view.page.label())))
    }
}

fn sidebar(view: &ViewModel) -> Element<'_, Msg> {
    let active_page = match view.page {
        Page::Instance => Page::Library,
        Page::Project => Page::Browse,
        page => page,
    };
    let mut nav = column![].spacing(5);
    for page in Page::NAV {
        let selected = active_page == page;
        nav = nav.push(
            button(
                row![
                    text(if selected { "●" } else { "○" })
                        .size(9)
                        .color(if selected { accent_c() } else { muted_c() }),
                    text(t(page.label())).size(14).color(if selected {
                        fg()
                    } else {
                        muted_c()
                    }),
                    Space::new().width(Length::Fill),
                    text(if selected { "›" } else { "" })
                        .size(15)
                        .color(accent_c()),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding([10, 12])
            .on_press(Msg::User(Message::Navigate(page)))
            .style(nav_style(selected, view.phase)),
        );
    }

    let active_account = view
        .accounts
        .iter()
        .find(|account| account.active)
        .map(|account| account.name.clone())
        .unwrap_or_else(|| "Not signed in".into());
    let avatar_name = active_account.clone();
    let active_jobs = view.jobs.iter().filter(|job| job.active).count();
    let live_dot = || {
        text(t("●"))
            .size(9)
            .color(with_alpha(accent_c(), pulse_alpha(view.phase, 120.0)))
    };

    container(
        column![
            row![
                container(text(t("F")).size(18).color(ink_c()))
                    .center(Length::Fixed(38.0))
                    .style(logo_style(view.phase)),
                column![
                    text(t("FERRISO")).size(19).color(fg()),
                    text(t("LAUNCHER · GLASS")).size(9).color(accent_c()),
                ]
                .spacing(1),
            ]
            .spacing(10)
            .align_y(Alignment::Center),
            Space::new().height(16),
            nav,
            Space::new().height(Length::Fill),
            {
                let jobs_chip: Element<'_, Msg> = if active_jobs > 0 {
                    container(
                        row![
                            live_dot(),
                            text(format!(
                                "{active_jobs} {}",
                                t("installs running")
                            ))
                            .size(10)
                            .color(warning_c()),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    )
                    .padding([6, 8])
                    .style(glass_chip_style)
                    .into()
                } else {
                    text(t("")).size(10).into()
                };
                jobs_chip
            },
            Space::new().height(4),
            row![
                player_head(view.player_head.as_ref(), &avatar_name, 30.0),
                column![
                    text(active_account).size(12).color(fg()),
                    row![live_dot(), text(t("ready")).size(9).color(muted_c())]
                        .spacing(4)
                        .align_y(Alignment::Center),
                ]
                .spacing(2),
            ]
            .spacing(9)
            .align_y(Alignment::Center),
            text(t("Ctrl+1…6 · Ctrl+R · Ctrl+Q"))
                .size(9)
                .color(muted_c()),
            text(t("Unofficial Modrinth fork"))
                .size(8)
                .color(with_alpha(muted_c(), 190)),
        ]
        .spacing(8),
    )
    .width(218)
    .height(Length::Fill)
    .padding(18)
    .style(glass_panel_style)
    .into()
}

fn top_bar(view: &ViewModel) -> Element<'_, Msg> {
    let error_c = view.error.is_some();
    let status_color = if error_c { danger_c() } else { muted_c() };
    let mut actions = row![
        column![
            text(t("WORKSPACE")).size(9).color(accent_c()),
            text(t(view.page.label())).size(27).color(fg()),
        ]
        .spacing(2),
        Space::new().width(Length::Fill),
        container(
            row![
                text(t("●")).size(8).color(if error_c {
                    danger_c()
                } else {
                    accent_c()
                }),
                text(t(if error_c { "attention" } else { "live" }))
                    .size(10)
                    .color(status_color),
            ]
            .spacing(5)
            .align_y(Alignment::Center),
        )
        .padding([5, 9])
        .style(glass_chip_style),
        button(text(t("↻  Refresh")).size(11).color(muted_c()))
            .padding([7, 12])
            .on_press(Msg::User(Message::Refresh))
            .style(ghost_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    if view.launching.is_some() {
        actions = actions.push(
            container(text(t("Launching…")).size(10).color(ink_c()))
                .padding([5, 9])
                .style(accent_chip_style),
        );
    }

    if let Some(update) = &view.update {
        actions = actions.push(
            container(
                row![
                    text(format!("{} {}", t("Update"), update.version))
                        .size(10)
                        .color(ink_c()),
                    button(text(t("Download")).size(9).color(ink_c()))
                        .padding([3, 7])
                        .on_press(Msg::User(Message::OpenUrl(
                            update.install_url.clone(),
                        )))
                        .style(ghost_style),
                    button(text("×").size(11).color(ink_c()))
                        .padding([1, 5])
                        .on_press(Msg::User(Message::DismissUpdate))
                        .style(ghost_style),
                ]
                .spacing(4)
                .align_y(Alignment::Center),
            )
            .padding([4, 8])
            .style(accent_chip_style),
        );
    }

    container(
        column![actions, text(t(&view.status)).size(11).color(status_color),]
            .spacing(5),
    )
    .padding([15, 18])
    .style(surface_style)
    .into()
}

fn content(view: &ViewModel) -> Element<'_, Msg> {
    if !view.booted {
        return container(loading_card(&view.status))
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(14)
            .style(surface_style)
            .into();
    }

    let page = match view.page {
        Page::Home => home(view),
        Page::Browse => browse(view),
        Page::Project => project_page(view),
        Page::Library => library(view),
        Page::Instance => instance_page(view),
        Page::Account => account(view),
        Page::Skins => skins(view),
        Page::Settings => settings(view),
    };

    let mut body = column![].spacing(10).width(Length::Fill);
    if let Some(error_c) = &view.error {
        body = body.push(error_banner(error_c));
    }
    body = body.push(page);

    container(body)
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(14)
        .style(surface_style)
        .into()
}

fn loading_card(status: &str) -> Element<'_, Msg> {
    container(
        column![
            text(t("Starting Ferriso Launcher")).size(22).color(fg()),
            text(status).size(13).color(muted_c()),
        ]
        .spacing(8),
    )
    .padding(20)
    .style(card_style)
    .into()
}

fn error_banner(error: &str) -> Element<'_, Msg> {
    container(
        column![
            text(t("Something went wrong")).size(14).color(danger_c()),
            text(error).size(12).color(fg()),
        ]
        .spacing(4),
    )
    .padding(12)
    .width(Length::Fill)
    .style(danger_style)
    .into()
}

fn checklist_item<'a>(
    done: bool,
    label: &'a str,
    action: Element<'a, Msg>,
) -> Element<'a, Msg> {
    row![
        text(if done { "●" } else { "○" }).size(11).color(if done {
            accent_c()
        } else {
            muted_c()
        }),
        text(t(label))
            .size(11)
            .color(if done { muted_c() } else { fg() }),
        Space::new().width(Length::Fill),
        if done {
            Element::from(Space::new())
        } else {
            action
        },
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn home(view: &ViewModel) -> Element<'_, Msg> {
    let mut body = column![].spacing(14);

    if let Some(instance) = view.instances.first() {
        let launch: Element<'_, Msg> =
            if view.launching.as_deref() == Some(instance.id.as_str()) {
                text(t("Launching…")).size(12).color(warning_c()).into()
            } else {
                button(text(t("Launch now")).size(12).color(ink_c()))
                    .padding([9, 18])
                    .on_press(Msg::User(Message::Launch(instance.id.clone())))
                    .style(primary_style)
                    .into()
            };
        body = body.push(
            container(
                row![
                    column![
                        text(t("READY WHEN YOU ARE")).size(9).color(accent_c()),
                        text(format!(
                            "{} {}",
                            t("Jump back into"),
                            instance.name
                        ))
                        .size(29)
                        .color(fg()),
                        text(format!(
                            "{} · {} · {} {}",
                            instance.game_version,
                            instance.loader,
                            instance.playtime_minutes,
                            t("minutes played")
                        ))
                        .size(12)
                        .color(muted_c()),
                        Space::new().height(8),
                        row![
                            launch,
                            button(
                                text(t("Open library")).size(12).color(fg())
                            )
                            .padding([9, 15])
                            .on_press(Msg::User(Message::Navigate(
                                Page::Library
                            )))
                            .style(ghost_style),
                        ]
                        .spacing(8),
                    ]
                    .spacing(5)
                    .width(Length::Fill),
                    instance_avatar(instance, 104.0),
                ]
                .spacing(20)
                .align_y(Alignment::Center),
            )
            .padding(24)
            .style(hero_style(view.phase)),
        );
    } else {
        body = body.push(
            container(
                column![
                    text(t("YOUR NEXT ADVENTURE")).size(9).color(accent_c()),
                    text(t("Welcome to Ferriso Launcher")).size(31).color(fg()),
                    text(t(
                        "Create a Minecraft instance, or install a modpack from Browse."
                    ))
                    .size(14)
                    .color(muted_c()),
                    Space::new().height(10),
                    row![
                        button(text(t("New instance")).size(13).color(ink_c()))
                            .padding([9, 18])
                            .on_press(Msg::User(Message::ToggleCreate))
                            .style(primary_style),
                        button(text(t("Browse modpacks")).size(12).color(fg()))
                            .padding([9, 16])
                            .on_press(Msg::User(Message::Navigate(Page::Browse)))
                            .style(ghost_style),
                    ]
                    .spacing(8),
                ]
                .spacing(6),
            )
            .padding(24)
            .style(hero_style(view.phase)),
        );
    }

    if let Some(checklist) = &view.checklist
        && checklist.show_checklist
        && !(checklist.has_created_instance
            && checklist.has_logged_into_minecraft
            && checklist.has_logged_into_modrinth)
    {
        body = body.push(
            container(
                column![
                    text(t("GET STARTED")).size(9).color(accent_c()),
                    text(t("Finish setting up Ferriso Launcher"))
                        .size(17)
                        .color(fg()),
                    checklist_item(
                        checklist.has_created_instance,
                        "Create your first instance",
                        button(text(t("Create")).size(9).color(ink_c()))
                            .padding([4, 9])
                            .on_press(Msg::User(Message::OpenCreate))
                            .style(primary_style)
                            .into(),
                    ),
                    checklist_item(
                        checklist.has_logged_into_minecraft,
                        "Sign in with Microsoft",
                        button(text(t("Account")).size(9).color(fg()))
                            .padding([4, 9])
                            .on_press(Msg::User(Message::Navigate(
                                Page::Account
                            )))
                            .style(ghost_style)
                            .into(),
                    ),
                    checklist_item(
                        checklist.has_logged_into_modrinth,
                        "Sign in to Modrinth",
                        button(text(t("Account")).size(9).color(fg()))
                            .padding([4, 9])
                            .on_press(Msg::User(Message::Navigate(
                                Page::Account
                            )))
                            .style(ghost_style)
                            .into(),
                    ),
                ]
                .spacing(7),
            )
            .padding(14)
            .style(card_style),
        );
    }

    let active_jobs = view.jobs.iter().filter(|job| job.active).count();
    body = body.push(
        row![
            metric_card(
                "INSTANCES",
                &view.instances.len().to_string(),
                accent_c()
            ),
            metric_card(
                "ACCOUNTS",
                &view.accounts.len().to_string(),
                accent_cyan_c()
            ),
            metric_card("ACTIVE JOBS", &active_jobs.to_string(), warning_c()),
        ]
        .spacing(10),
    );

    if !view.instances.is_empty() {
        let mut recent = column![].spacing(8);
        for instance in view.instances.iter().take(4) {
            recent = recent.push(home_instance_row(instance, view));
        }
        body = body.push(
            container(
                column![
                    section_header(
                        "Continue playing",
                        "Sorted by your latest session"
                    ),
                    recent,
                ]
                .spacing(10),
            )
            .padding(16)
            .style(card_style),
        );
    }

    if view.create.open {
        body = body.push(create_card(view));
    }
    if let Some(form @ LibraryForm::Import { .. }) = &view.library_form {
        body = body.push(import_card(form));
    }
    if !view.jobs.is_empty() {
        body = body.push(jobs_card(view));
    }

    scrollable(body).height(Length::Fill).into()
}

fn home_instance_row<'a>(
    instance: &'a InstanceView,
    view: &'a ViewModel,
) -> Element<'a, Msg> {
    let launching = view.launching.as_deref() == Some(instance.id.as_str());
    let launch: Element<'a, Msg> = if launching {
        text(t("Launching…")).size(11).color(warning_c()).into()
    } else {
        button(text(t("Launch")).size(11).color(ink_c()))
            .padding([7, 14])
            .on_press(Msg::User(Message::Launch(instance.id.clone())))
            .style(primary_style)
            .into()
    };

    row![
        button(
            row![
                instance_avatar(instance, 44.0),
                column![
                    text(&instance.name).size(14).color(fg()),
                    text(format!(
                        "{} · {} · {} {}",
                        instance.game_version,
                        instance.loader,
                        instance.playtime_minutes,
                        t("min")
                    ))
                    .size(10)
                    .color(muted_c()),
                ]
                .spacing(2),
            ]
            .spacing(11)
            .align_y(Alignment::Center),
        )
        .width(Length::Fill)
        .padding([8, 10])
        .on_press(Msg::User(Message::SelectInstance(instance.id.clone())))
        .style(row_style(false)),
        launch,
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn metric_card(
    label: &'static str,
    value: &str,
    accent: Color,
) -> Element<'static, Msg> {
    container(
        column![
            text(value.to_owned()).size(26).color(fg()),
            text(t(label)).size(9).color(accent),
        ]
        .spacing(2),
    )
    .width(Length::Fill)
    .padding([12, 16])
    .style(stat_style)
    .into()
}

fn section_header(title: &str, subtitle: &str) -> Element<'static, Msg> {
    row![
        container(text(t("")).size(1).width(3.0).height(22.0))
            .style(accent_line_style),
        column![
            text(t(title).to_owned()).size(15).color(fg()),
            text(t(subtitle).to_owned()).size(9).color(muted_c()),
        ]
        .spacing(1),
    ]
    .spacing(9)
    .align_y(Alignment::Center)
    .into()
}

fn library(view: &ViewModel) -> Element<'_, Msg> {
    let header = row![
        column![
            text(t("YOUR LIBRARY")).size(9).color(accent_c()),
            text(t("Your instances")).size(25).color(fg()),
            text(format!(
                "{} {} · {}",
                view.instances.len(),
                t("instances"),
                t("shared with the Modrinth data directory")
            ))
            .size(11)
            .color(muted_c()),
        ]
        .spacing(2)
        .width(Length::Fill),
        button(text(t("Groups")).size(12).color(fg()))
            .padding([9, 14])
            .on_press(Msg::User(Message::ToggleGroups))
            .style(ghost_style),
        button(text(t("Import")).size(12).color(fg()))
            .padding([9, 14])
            .on_press(Msg::User(Message::BeginImport))
            .style(ghost_style),
        button(text(t("+  New instance")).size(12).color(ink_c()))
            .padding([9, 16])
            .on_press(Msg::User(Message::ToggleCreate))
            .style(primary_style),
    ]
    .spacing(10)
    .align_y(Alignment::Center);

    let mut body = column![header].spacing(10);
    if view.create.open {
        body = body.push(create_card(view));
    }
    if let Some(form @ LibraryForm::Import { .. }) = &view.library_form {
        body = body.push(import_card(form));
    }
    if !view.groups.is_empty() || view.groups_open {
        body = body.push(group_filters(view));
    }
    if view.groups_open {
        body = body.push(groups_card(view));
    }

    let shown: Vec<&InstanceView> = view
        .instances
        .iter()
        .filter(|instance| {
            view.selected_group.as_ref().is_none_or(|group| {
                instance.group_ids.iter().any(|id| id == group)
            })
        })
        .collect();

    if view.instances.is_empty() {
        body = body.push(
            container(
                column![
                    text(t("No instances yet")).size(22).color(fg()),
                    text(
                        "Create a vanilla or modded instance to start playing."
                    )
                    .size(12)
                    .color(muted_c()),
                ]
                .spacing(6),
            )
            .padding(24)
            .style(hero_style(view.phase)),
        );
    } else if shown.is_empty() {
        body = body.push(
            container(
                column![
                    text(t("No instances in this group")).size(18).color(fg()),
                    text(t(
                        "Assign instances to the group from their Edit card."
                    ))
                    .size(11)
                    .color(muted_c()),
                ]
                .spacing(4),
            )
            .padding(18)
            .style(card_style),
        );
    } else {
        let mut list = column![].spacing(8);
        for instance in shown {
            list = list.push(instance_row(instance, view));
        }
        body = body.push(scrollable(list).height(Length::Fill));
    }

    if !view.jobs.is_empty() {
        body = body.push(jobs_card(view));
    }

    body.height(Length::Fill).into()
}

fn group_filters(view: &ViewModel) -> Element<'_, Msg> {
    let mut filters = row![
        button(text(t("All instances")).size(10).color(
            if view.selected_group.is_none() {
                accent_c()
            } else {
                fg()
            }
        ))
        .padding([5, 9])
        .on_press(Msg::User(Message::FilterGroup(None)))
        .style(nav_style(view.selected_group.is_none(), view.phase)),
    ]
    .spacing(6);
    for group in &view.groups {
        let selected =
            view.selected_group.as_deref() == Some(group.id.as_str());
        filters = filters.push(
            button(text(&group.name).size(10).color(if selected {
                accent_c()
            } else {
                fg()
            }))
            .padding([5, 9])
            .on_press(Msg::User(Message::FilterGroup(Some(group.id.clone()))))
            .style(nav_style(selected, view.phase)),
        );
    }
    row![
        text(t("Groups")).size(10).color(muted_c()),
        filters,
        Space::new().width(Length::Fill),
    ]
    .spacing(10)
    .align_y(Alignment::Center)
    .into()
}

fn groups_card(view: &ViewModel) -> Element<'_, Msg> {
    let mut list = column![].spacing(5);
    for group in &view.groups {
        let favorites = group.id == FAVORITES_GROUP_ID;
        let mut actions = row![].spacing(5);
        if !favorites {
            actions = actions
                .push(
                    button(text(t("Rename")).size(9).color(fg()))
                        .padding([4, 8])
                        .on_press(Msg::User(Message::BeginGroupRename(
                            group.id.clone(),
                        )))
                        .style(ghost_style),
                )
                .push(
                    button(text(t("Delete")).size(9).color(danger_c()))
                        .padding([4, 8])
                        .on_press(Msg::User(Message::DeleteGroup(
                            group.id.clone(),
                        )))
                        .style(ghost_style),
                );
        }
        let count = view
            .instances
            .iter()
            .filter(|instance| {
                instance.group_ids.iter().any(|id| id == &group.id)
            })
            .count();
        list = list.push(
            container(
                row![
                    text(&group.name).size(11).color(fg()).width(Length::Fill),
                    text(format!("{count} {}", t("instances")))
                        .size(9)
                        .color(muted_c()),
                    actions,
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding([5, 7])
            .style(surface_style),
        );
    }
    if view.groups.is_empty() {
        list = list.push(text(t("No groups yet.")).size(10).color(muted_c()));
    }

    let form: Element<'_, Msg> = if let Some(form) = &view.group_form {
        group_form(form)
    } else {
        button(text(t("+ New group")).size(10).color(accent_c()))
            .padding([5, 9])
            .on_press(Msg::User(Message::BeginGroupCreate))
            .style(ghost_style)
            .into()
    };

    container(
        column![
            row![
                column![
                    text(t("Groups")).size(14).color(fg()),
                    text(t(
                        "Filter the library and assign instances from Edit."
                    ))
                    .size(9)
                    .color(muted_c()),
                ]
                .spacing(2)
                .width(Length::Fill),
                button(text(t("Close")).size(9).color(muted_c()))
                    .padding([4, 8])
                    .on_press(Msg::User(Message::ToggleGroups))
                    .style(ghost_style),
            ]
            .align_y(Alignment::Center),
            list,
            form,
        ]
        .spacing(8),
    )
    .padding(12)
    .style(card_style)
    .into()
}

fn group_form(form: &GroupForm) -> Element<'_, Msg> {
    row![
        text_input(t("Group name"), &form.name)
            .size(11)
            .padding(7)
            .on_input(|value| Msg::User(Message::GroupNameChanged(value)))
            .style(text_input_style)
            .width(Length::Fill),
        button(text(t("Save")).size(9).color(ink_c()))
            .padding([5, 10])
            .on_press(Msg::User(Message::SaveGroup))
            .style(primary_style),
        button(text(t("Cancel")).size(9).color(muted_c()))
            .padding([5, 8])
            .on_press(Msg::User(Message::CancelGroupForm))
            .style(ghost_style),
    ]
    .spacing(6)
    .align_y(Alignment::Center)
    .into()
}

fn instance_row<'a>(
    instance: &'a InstanceView,
    view: &'a ViewModel,
) -> Element<'a, Msg> {
    let selected = view.selected.as_deref() == Some(instance.id.as_str());
    let launching = view.launching.as_deref() == Some(instance.id.as_str());

    let details = column![
        text(&instance.name).size(15).color(fg()),
        text(format!(
            "{} · {} {}",
            instance.game_version,
            instance.loader,
            instance.loader_version.as_deref().unwrap_or("")
        ))
        .size(10)
        .color(muted_c()),
        text(format!(
            "{} · {} {} · {}",
            instance
                .last_played
                .as_deref()
                .map(t)
                .unwrap_or_else(|| t("Never played")),
            instance.playtime_minutes,
            t("min"),
            instance.path
        ))
        .size(9)
        .color(muted_c()),
    ]
    .spacing(3);

    let action: Element<'_, Msg> = if launching {
        text(t("Launching…")).size(11).color(warning_c()).into()
    } else {
        button(text(t("Launch")).size(11).color(ink_c()))
            .padding([7, 14])
            .on_press(Msg::User(Message::Launch(instance.id.clone())))
            .style(primary_style)
            .into()
    };

    button(
        row![
            instance_avatar(instance, 48.0),
            details.width(Length::Fill),
            action,
            text(t("›")).size(20).color(if selected {
                accent_c()
            } else {
                muted_c()
            }),
        ]
        .spacing(12)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(12)
    .on_press(Msg::User(Message::SelectInstance(instance.id.clone())))
    .style(row_style(selected))
    .into()
}

fn create_card(view: &ViewModel) -> Element<'_, Msg> {
    let mut loaders = row![].spacing(6);
    for (index, label) in LOADER_LABELS.iter().enumerate() {
        loaders = loaders.push(
            button(text(*label).size(11).color(
                if view.create.loader == index {
                    accent_c()
                } else {
                    fg()
                },
            ))
            .padding([5, 9])
            .on_press(Msg::User(Message::CreateLoaderChanged(index)))
            .style(nav_style(view.create.loader == index, view.phase)),
        );
    }

    let create_button = if view.create.busy {
        button(text(t("Creating…")).size(13).color(muted_c()))
            .padding([8, 14])
            .style(ghost_style)
    } else {
        button(text(t("Create instance")).size(13).color(ink_c()))
            .padding([8, 14])
            .on_press(Msg::User(Message::CreateInstance))
            .style(primary_style)
    };

    let mut form = column![
        row![
            text(t("New instance")).size(18).color(fg()),
            Space::new().width(Length::Fill),
            button(text(t("Close")).size(11).color(muted_c()))
                .padding([5, 9])
                .on_press(Msg::User(Message::ToggleCreate))
                .style(ghost_style),
        ]
        .align_y(Alignment::Center),
        text_input(t("Instance name"), &view.create.name)
            .padding(8)
            .style(text_input_style)
            .width(Length::Fill)
            .on_input(|value| Msg::User(Message::CreateNameChanged(value))),
        text_input(
            t("Minecraft version (e.g. 1.21.10)"),
            &view.create.game_version
        )
        .padding(8)
        .style(text_input_style)
        .width(Length::Fill)
        .on_input(|value| Msg::User(Message::CreateVersionChanged(value))),
        text(
            view.latest_release
                .as_deref()
                .map(|release| {
                    format!("{}{release}", t("Latest release: "))
                })
                .unwrap_or_else(|| t("Enter a Minecraft version").into()),
        )
        .size(10)
        .color(muted_c()),
        text(t("Loader")).size(11).color(muted_c()),
        loaders,
        row![create_button].spacing(8).align_y(Alignment::Center),
    ]
    .spacing(8);

    if let Some(error_c) = &view.create.error {
        form = form.push(text(error_c).size(11).color(danger_c()));
    }

    container(form).padding(16).style(card_style).into()
}

fn browse(view: &ViewModel) -> Element<'_, Msg> {
    let search = row![
        text_input(t("Search Modrinth…"), &view.browse.query)
            .padding(9)
            .style(text_input_style)
            .width(Length::Fill)
            .on_input(|value| Msg::User(Message::BrowseQueryChanged(value)))
            .on_submit(Msg::User(Message::BrowseSearch)),
        button(text(t("Search")).size(12).color(ink_c()))
            .padding([8, 14])
            .on_press(Msg::User(Message::BrowseSearch))
            .style(primary_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut filters = row![].spacing(6);
    for kind in ProjectKind::ALL {
        filters = filters.push(
            button(text(t(kind.label())).size(11).color(
                if view.browse.kind == kind {
                    accent_c()
                } else {
                    fg()
                },
            ))
            .padding([5, 9])
            .on_press(Msg::User(Message::BrowseKind(kind)))
            .style(nav_style(view.browse.kind == kind, view.phase)),
        );
    }

    let mut body = column![
        row![
            column![
                text(t("DISCOVER")).size(9).color(accent_c()),
                text(t("Find your next project")).size(25).color(fg()),
                text(t("Search Modrinth and install it in one click."))
                    .size(11)
                    .color(muted_c()),
            ]
            .spacing(2)
            .width(Length::Fill),
            container(
                text(format!(
                    "{} · {}",
                    t(view.browse.kind.label()),
                    view.browse.results.len()
                ))
                .size(10)
                .color(muted_c()),
            )
            .padding([5, 9])
            .style(glass_chip_style),
        ]
        .align_y(Alignment::Center),
        search,
        filters,
    ]
    .spacing(11);

    if view.browse.loading {
        body =
            body.push(text(t("Searching Modrinth…")).size(12).color(muted_c()));
    } else if let Some(error_c) = &view.browse.error {
        body = body.push(text(error_c).size(12).color(danger_c()));
    } else if view.browse.results.is_empty() {
        body =
            body.push(text(t("No projects found.")).size(12).color(muted_c()));
    } else {
        let mut results = column![].spacing(8);
        for project in &view.browse.results {
            results = results.push(project_row(project, view));
        }
        body = body.push(scrollable(results).height(Length::Fill));
    }

    body.height(Length::Fill).into()
}

fn project_row<'a>(
    project: &'a ProjectView,
    view: &'a ViewModel,
) -> Element<'a, Msg> {
    let install_label = t(if project.kind == ProjectKind::Modpack {
        "Install modpack"
    } else {
        "Add to instance"
    });
    let can_install =
        project.kind == ProjectKind::Modpack || view.selected.is_some();
    let install = if can_install {
        button(text(install_label).size(10).color(ink_c()))
            .padding([7, 12])
            .on_press(Msg::User(Message::InstallProject(project.clone())))
            .style(primary_style)
    } else {
        button(text(t("Select instance first")).size(10).color(muted_c()))
            .padding([7, 10])
            .style(ghost_style)
    };
    button(
        row![
            project_art(project, 56.0),
            column![
                row![
                    text(&project.title).size(15).color(fg()),
                    container(
                        text(t(project.kind.label())).size(8).color(accent_c())
                    )
                    .padding([2, 6])
                    .style(glass_chip_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text(&project.summary).size(10).color(muted_c()),
                text(format!(
                    "{}{} · {}{}",
                    t("by "),
                    project.author,
                    human_count(project.downloads),
                    t(" downloads")
                ))
                .size(9)
                .color(muted_c()),
            ]
            .spacing(3)
            .width(Length::Fill),
            row![
                button(text(t("Open page")).size(10).color(muted_c()))
                    .padding([7, 10])
                    .on_press(Msg::User(Message::OpenProject(
                        project.id.clone(),
                    )))
                    .style(ghost_style),
                install,
            ]
            .spacing(6),
        ]
        .spacing(13)
        .align_y(Alignment::Center),
    )
    .width(Length::Fill)
    .padding(13)
    .style(row_style(false))
    .into()
}

fn human_count(value: u64) -> String {
    if value >= 1_000_000 {
        format!("{:.1}M", value as f64 / 1_000_000.0)
    } else if value >= 1_000 {
        format!("{:.1}k", value as f64 / 1_000.0)
    } else {
        value.to_string()
    }
}

fn project_page(view: &ViewModel) -> Element<'_, Msg> {
    let Some(page) = &view.project else {
        return column![
            page_heading("PROJECT", "Project"),
            text(t("Loading project…")).size(12).color(muted_c()),
        ]
        .spacing(10)
        .into();
    };

    let back = button(text(t("← Back")).size(10).color(muted_c()))
        .padding([5, 9])
        .on_press(Msg::User(Message::Back))
        .style(ghost_style);

    if page.loading {
        return column![
            back,
            page_heading("PROJECT", "Project"),
            text(t("Loading project…")).size(12).color(muted_c()),
        ]
        .spacing(10)
        .into();
    }
    if let Some(error) = &page.error {
        return column![
            back,
            page_heading("PROJECT", "Project"),
            text(error).size(12).color(danger_c()),
        ]
        .spacing(10)
        .into();
    }
    let Some(project) = page.selected_project() else {
        return column![
            back,
            page_heading("PROJECT", "Project"),
            text(t("Project not found.")).size(12).color(muted_c()),
        ]
        .spacing(10)
        .into();
    };

    let external =
        button(text(t("View on Modrinth")).size(10).color(muted_c()))
            .padding([6, 10])
            .on_press(Msg::User(Message::OpenUrl(format!(
                "https://modrinth.com/project/{}",
                project.slug
            ))))
            .style(ghost_style);

    let header = container(
        row![
            project_art(project, 76.0),
            column![
                row![
                    text(&project.title).size(23).color(fg()),
                    container(
                        text(t(project.kind.label())).size(8).color(accent_c())
                    )
                    .padding([2, 6])
                    .style(glass_chip_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
                text(&project.summary).size(11).color(muted_c()),
                text(format!(
                    "{}{} · {}{}",
                    t("by "),
                    project.author,
                    human_count(project.downloads),
                    t(" downloads")
                ))
                .size(9)
                .color(muted_c()),
            ]
            .spacing(4)
            .width(Length::Fill),
            column![back, external].spacing(5).align_x(Alignment::End),
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .padding(16)
    .style(card_style);

    let mut gallery = row![].spacing(7);
    for item in page.gallery.iter().take(4) {
        let image = item
            .image
            .as_ref()
            .and_then(|bytes| {
                bytes_handle(bytes_key("project-gallery", bytes), bytes)
            })
            .map(|handle| {
                let art: Element<'_, Msg> = image(handle)
                    .width(180)
                    .height(100)
                    .border_radius(8)
                    .content_fit(ContentFit::Cover)
                    .into();
                art
            })
            .unwrap_or_else(|| {
                avatar(item.title.as_deref().unwrap_or("Gallery"), 60.0)
            });
        gallery = gallery.push(
            column![
                image,
                text(item.title.as_deref().unwrap_or(""))
                    .size(9)
                    .color(muted_c()),
                text(item.description.as_deref().unwrap_or(""))
                    .size(8)
                    .color(muted_c()),
            ]
            .spacing(3)
            .width(Length::Fixed(180.0)),
        );
    }

    let mut left = column![].spacing(10);
    if !page.gallery.is_empty() {
        left = left.push(gallery);
    }
    left = left.push(
        container(
            column![
                text(t("About this project")).size(15).color(fg()),
                if page.body.trim().is_empty() {
                    Element::from(
                        text(t("No description provided."))
                            .size(11)
                            .color(muted_c()),
                    )
                } else {
                    Element::from(text(&page.body).size(11).color(muted_c()))
                },
            ]
            .spacing(8),
        )
        .padding(14)
        .style(card_style),
    );

    let mut targets = row![].spacing(5);
    if project.kind != ProjectKind::Modpack {
        for instance in view.instances.iter().take(12) {
            let selected =
                page.selected_instance.as_deref() == Some(instance.id.as_str());
            targets = targets.push(
                button(text(&instance.name).size(9).color(if selected {
                    ink_c()
                } else {
                    muted_c()
                }))
                .padding([4, 8])
                .on_press(Msg::User(Message::SelectProjectInstance(
                    instance.id.clone(),
                )))
                .style(nav_style(selected, view.phase)),
            );
        }
    }

    let mut versions = column![].spacing(4);
    for (index, version) in page.versions.iter().enumerate().take(12) {
        let selected = page.selected_version == Some(index);
        versions = versions.push(
            button(
                row![
                    column![
                        text(&version.number).size(11).color(fg()),
                        text(format!(
                            "{} · {}",
                            version
                                .game_versions
                                .iter()
                                .take(3)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", "),
                            version
                                .loaders
                                .iter()
                                .take(3)
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        ))
                        .size(8)
                        .color(muted_c()),
                    ]
                    .spacing(1)
                    .width(Length::Fill),
                    text(format!("{}", version.downloads))
                        .size(8)
                        .color(muted_c()),
                    text(if selected { "●" } else { "○" })
                        .size(11)
                        .color(if selected { accent_c() } else { muted_c() }),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding(7)
            .on_press(Msg::User(Message::SelectProjectVersion(index)))
            .style(row_style(selected)),
        );
    }

    let install_label = if page.installing {
        t("Installing…")
    } else if project.kind == ProjectKind::Modpack {
        t("Install modpack")
    } else {
        t("Add to instance")
    };
    let can_install = !page.installing
        && page.selected_version.is_some()
        && (project.kind == ProjectKind::Modpack
            || page.selected_instance.is_some());
    let install_button: Element<'_, Msg> = if can_install {
        button(text(install_label).size(11).color(ink_c()))
            .padding([8, 13])
            .on_press(Msg::User(Message::InstallProjectVersion))
            .style(primary_style)
            .into()
    } else {
        button(text(install_label).size(11).color(muted_c()))
            .padding([8, 13])
            .style(ghost_style)
            .into()
    };

    let mut right = column![
        text(t("Install")).size(15).color(fg()),
        text(t("Select a version and an instance."))
            .size(9)
            .color(muted_c()),
        targets,
        versions,
        install_button,
    ]
    .spacing(8);
    if let Some(error) = &page.install_error {
        right = right.push(text(error).size(10).color(danger_c()));
    }
    if let Some(message) = &page.install_result {
        right = right.push(text(message).size(10).color(accent_c()));
    }

    let body = row![
        scrollable(left).height(Length::Fill),
        container(right)
            .padding(14)
            .width(Length::Fixed(340.0))
            .style(card_style),
    ]
    .spacing(10)
    .height(Length::Fill);

    column![header, body]
        .spacing(10)
        .height(Length::Fill)
        .into()
}

fn shared_card<'a>(
    view: &'a ViewModel,
    instance: &'a InstanceView,
) -> Element<'a, Msg> {
    let Some(shared) = &view.shared else {
        return Space::new().into();
    };
    let close = button(text(t("Close")).size(9).color(muted_c()))
        .padding([4, 8])
        .on_press(Msg::User(Message::ToggleShared))
        .style(ghost_style);

    if view.modrinth_account.is_none() {
        return container(
            column![
                row![
                    text(t("Shared instance")).size(14).color(fg()).width(Length::Fill),
                    close,
                ],
                text(t("Sign in with Modrinth from the Account page to share this instance."))
                    .size(10)
                    .color(muted_c()),
            ]
            .spacing(6),
        )
        .padding(12)
        .style(card_style)
        .into();
    }

    let link = instance.shared.as_ref();
    let mut body = column![].spacing(8);
    body = body.push(
        row![
            column![
                text(t("Shared instance")).size(14).color(fg()),
                text(
                    link.map(|link| format!(
                        "{} · {}",
                        link.id,
                        if link.role == "owner" {
                            "owner"
                        } else {
                            "member"
                        }
                    ))
                    .unwrap_or_else(|| "Not published".into())
                )
                .size(9)
                .color(muted_c()),
            ]
            .spacing(2)
            .width(Length::Fill),
            close,
        ]
        .align_y(Alignment::Center),
    );

    if let Some(error_c) = &shared.error {
        body = body.push(text(error_c).size(10).color(danger_c()));
    }
    if shared.loading {
        body = body.push(
            text(t("Loading shared instance…"))
                .size(10)
                .color(muted_c()),
        );
    }

    if link.is_none() {
        body = body.push(
            column![
                text(t("Publish this instance to invite friends. Instances stay local; only the content set is shared."))
                    .size(10)
                    .color(muted_c()),
                button(text(t("Publish shared instance")).size(11).color(ink_c()))
                    .padding([6, 12])
                    .on_press(Msg::User(Message::PublishShared))
                    .style(primary_style),
            ]
            .spacing(6),
        );
    } else {
        let Some(link) = link else {
            return Space::new().into();
        };
        let share_page = link
            .latest_version
            .map(|version| format!("{}{version}", t("Latest version ")))
            .unwrap_or_else(|| "No published version".into());
        body = body.push(
            row![
                text(share_page).size(10).color(muted_c()),
                text(format!(
                    "{}{}",
                    t("Applied "),
                    link.applied_version
                        .map(|version| version.to_string())
                        .unwrap_or_else(|| "—".into())
                ))
                .size(10)
                .color(muted_c()),
                Space::new().width(Length::Fill),
                if shared.busy {
                    Element::from(
                        text(t("Working…")).size(10).color(warning_c()),
                    )
                } else if link.role == "owner" {
                    Element::from(
                        button(text(t("Publish update")).size(9).color(fg()))
                            .padding([4, 8])
                            .on_press(Msg::User(Message::PublishShared))
                            .style(ghost_style),
                    )
                } else {
                    Element::from(Space::new())
                },
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        );

        if link.role == "owner" {
            let mut members = column![].spacing(3);
            for user in &shared.users {
                let remove: Element<'_, Msg> = if user.join_type == "owner" {
                    text(t("owner")).size(9).color(accent_c()).into()
                } else {
                    button(text(t("Remove")).size(9).color(danger_c()))
                        .padding([3, 7])
                        .on_press(Msg::User(Message::RemoveSharedUser(
                            user.id.clone(),
                        )))
                        .style(ghost_style)
                        .into()
                };
                members = members.push(
                    container(
                        row![
                            text(&user.id).size(10).color(fg()),
                            text(&user.join_type).size(9).color(muted_c()),
                            Space::new().width(Length::Fill),
                            remove,
                        ]
                        .spacing(7)
                        .align_y(Alignment::Center),
                    )
                    .padding([4, 6])
                    .style(surface_style),
                );
            }
            body = body.push(
                container(
                    column![
                        text(t("Members")).size(11).color(fg()),
                        members,
                        row![
                            text_input(
                                t("Modrinth user ID(s), comma separated"),
                                &shared.user_input
                            )
                            .size(10)
                            .padding(6)
                            .on_input(|value| Msg::User(
                                Message::SharedUserInputChanged(value)
                            ))
                            .on_submit(Msg::User(Message::InviteSharedUsers))
                            .style(text_input_style)
                            .width(Length::Fill),
                            button(text(t("Invite")).size(9).color(ink_c()))
                                .padding([5, 9])
                                .on_press(Msg::User(Message::InviteSharedUsers))
                                .style(primary_style),
                        ]
                        .spacing(6)
                        .align_y(Alignment::Center),
                    ]
                    .spacing(5),
                )
                .padding([6, 8])
                .style(surface_style),
            );

            let mut invites = column![].spacing(3);
            for invite in &shared.invites {
                invites = invites.push(
                    row![
                        text(&invite.id).size(9).color(fg()),
                        text(format!(
                            "{}/{} {} · {} {}",
                            invite.uses,
                            invite.max_uses,
                            t("uses"),
                            t("expires"),
                            invite.expiration
                        ))
                        .size(9)
                        .color(muted_c()),
                        Space::new().width(Length::Fill),
                        button(text(t("Revoke")).size(9).color(danger_c()))
                            .padding([3, 7])
                            .on_press(Msg::User(Message::RevokeSharedInvite(
                                invite.id.clone(),
                            )))
                            .style(ghost_style),
                    ]
                    .spacing(7)
                    .align_y(Alignment::Center),
                );
            }
            body =
                body.push(
                    container(
                        column![
                            text(t("Invite links")).size(11).color(fg()),
                            invites,
                            button(
                                text(t("Create invite link"))
                                    .size(9)
                                    .color(accent_c())
                            )
                            .padding([4, 8])
                            .on_press(Msg::User(Message::CreateSharedInvite))
                            .style(ghost_style),
                            if let Some(invite_id) = &shared.new_invite {
                                row![
                                text(format!("{}{invite_id}", t("Invite ID: ")))
                                    .size(9)
                                    .color(accent_c()),
                                button(
                                    text(t("Open share page"))
                                        .size(9)
                                        .color(fg())
                                )
                                .padding([3, 7])
                                .on_press(Msg::User(Message::OpenUrl(format!(
                                    "https://modrinth.com/share/{invite_id}"
                                ))))
                                .style(ghost_style),
                            ]
                                .spacing(8)
                                .align_y(Alignment::Center)
                                .into()
                            } else {
                                Element::from(Space::new())
                            },
                        ]
                        .spacing(5),
                    )
                    .padding([6, 8])
                    .style(surface_style),
                );

            body = body.push(
                row![
                    button(
                        text(t("Unpublish shared instance"))
                            .size(9)
                            .color(danger_c())
                    )
                    .padding([4, 8])
                    .on_press(Msg::User(Message::UnpublishShared))
                    .style(ghost_style),
                ]
                .spacing(6),
            );
        } else {
            body = body.push(
                column![
                    text(t("You are a member of this shared instance."))
                        .size(10)
                        .color(muted_c()),
                    button(text(t("Update from shared")).size(9).color(fg()))
                        .padding([4, 8])
                        .on_press(Msg::User(Message::UpdateShared))
                        .style(ghost_style),
                    button(
                        text(t("Leave shared instance"))
                            .size(9)
                            .color(danger_c())
                    )
                    .padding([4, 8])
                    .on_press(Msg::User(Message::UnlinkShared))
                    .style(ghost_style),
                ]
                .spacing(6),
            );
        }
    }

    container(body).padding(12).style(card_style).into()
}

fn shared_join_card(view: &ViewModel) -> Element<'_, Msg> {
    let mut body = column![
        text(t("Shared instances")).size(16).color(fg()),
        text(t("Join a friend's instance with an invite ID. You must be signed in to Modrinth."))
            .size(10)
            .color(muted_c()),
        row![
            text_input(t("Invite ID or share link"), &view.join_invite)
                .size(11)
                .padding(7)
                .on_input(|value| Msg::User(Message::JoinInviteChanged(value)))
                .on_submit(Msg::User(Message::PreviewJoinInvite))
                .style(text_input_style)
                .width(Length::Fill),
            button(text(t("Preview")).size(10).color(ink_c()))
                .padding([6, 11])
                .on_press(Msg::User(Message::PreviewJoinInvite))
                .style(primary_style),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    ]
    .spacing(7);

    if view.join_loading {
        body =
            body.push(text(t("Checking invite…")).size(10).color(warning_c()));
    }
    if let Some(preview) = &view.join_preview {
        body = body.push(
            container(
                column![
                    text(&preview.preview.name).size(13).color(fg()),
                    text(format!(
                        "{} · {} · {} {} · {} {}",
                        preview.preview.game_version,
                        preview.preview.loader.as_str(),
                        preview.preview.mod_count,
                        t("mods"),
                        preview.preview.external_file_count,
                        t("external files")
                    ))
                    .size(10)
                    .color(muted_c()),
                    text(format!(
                        "{}{}",
                        t("Shared instance "),
                        preview.shared_instance_id
                    ))
                    .size(9)
                    .color(muted_c()),
                    button(
                        text(t("Install shared instance"))
                            .size(10)
                            .color(ink_c())
                    )
                    .padding([6, 11])
                    .on_press(Msg::User(Message::InstallSharedInstance))
                    .style(primary_style),
                ]
                .spacing(5),
            )
            .padding(9)
            .style(surface_style),
        );
    }

    container(body).padding(14).style(card_style).into()
}

fn instance_page(view: &ViewModel) -> Element<'_, Msg> {
    let Some(detail) = &view.detail else {
        return text(t("Select an instance in the Library."))
            .size(13)
            .color(muted_c())
            .into();
    };
    let Some(instance) = view
        .instances
        .iter()
        .find(|instance| instance.id == detail.id)
    else {
        return text(t("This instance is no longer available."))
            .size(13)
            .color(muted_c())
            .into();
    };

    let running = view
        .processes
        .iter()
        .find(|process| process.instance_id == instance.id);
    let launch: Element<'_, Msg> = if let Some(process) = running {
        row![
            text(t("Running")).size(11).color(accent_c()),
            button(text(t("Kill")).size(11).color(danger_c()))
                .padding([6, 12])
                .on_press(Msg::User(Message::KillProcess(process.uuid.clone())))
                .style(ghost_style),
        ]
        .spacing(7)
        .align_y(Alignment::Center)
        .into()
    } else if view.launching.as_deref() == Some(instance.id.as_str()) {
        text(t("Launching…")).size(12).color(warning_c()).into()
    } else {
        button(text(t("Launch")).size(12).color(ink_c()))
            .padding([7, 14])
            .on_press(Msg::User(Message::Launch(instance.id.clone())))
            .style(primary_style)
            .into()
    };

    let header = row![
        button(text(t("← Library")).size(10).color(muted_c()))
            .padding([6, 10])
            .on_press(Msg::User(Message::Back))
            .style(ghost_style),
        instance_avatar(instance, 64.0),
        column![
            text(t("INSTANCE")).size(9).color(accent_c()),
            text(&instance.name).size(26).color(fg()),
            text(format!(
                "{} · {} · {}",
                instance.game_version, instance.loader, instance.path
            ))
            .size(10)
            .color(muted_c()),
        ]
        .spacing(2)
        .width(Length::Fill),
        button(text(t("Duplicate")).size(10).color(fg()))
            .padding([5, 9])
            .on_press(Msg::User(Message::DuplicateInstance(
                instance.id.clone()
            )))
            .style(ghost_style),
        button(text(t("Export")).size(10).color(fg()))
            .padding([5, 9])
            .on_press(Msg::User(Message::BeginExport))
            .style(ghost_style),
        button(text(t("Edit")).size(10).color(fg()))
            .padding([5, 9])
            .on_press(Msg::User(Message::BeginRename))
            .style(ghost_style),
        button(text(t("Delete")).size(10).color(danger_c()))
            .padding([5, 9])
            .on_press(Msg::User(Message::AskDelete))
            .style(ghost_style),
        button(text(t("Share")).size(10).color(fg()))
            .padding([5, 9])
            .on_press(Msg::User(Message::ToggleShared))
            .style(ghost_style),
        launch,
    ]
    .spacing(13)
    .align_y(Alignment::Center);

    let mut tabs = row![].spacing(6);
    for tab in DetailTab::ALL {
        tabs = tabs.push(
            button(text(t(tab.label())).size(11).color(if detail.tab == tab {
                accent_c()
            } else {
                fg()
            }))
            .padding([5, 10])
            .on_press(Msg::User(Message::DetailTab(tab)))
            .style(nav_style(detail.tab == tab, view.phase)),
        );
    }

    let mut body = column![
        container(header).padding(16).style(hero_style(view.phase)),
        tabs
    ]
    .spacing(12);
    if let Some(edit) = &view.edit {
        body = body.push(edit_card(edit, &view.groups));
    }
    if view.delete_confirm {
        body = body.push(delete_confirm_card(&instance.name));
    }
    if view.shared_open {
        body = body.push(shared_card(view, instance));
    }
    if let Some(form @ LibraryForm::Export { .. }) = &view.library_form {
        body = body.push(export_card(form));
    }
    if detail.loading {
        body =
            body.push(text(t("Loading instance…")).size(12).color(muted_c()));
    } else if let Some(error_c) = &detail.error {
        body = body.push(text(error_c).size(12).color(danger_c()));
    } else {
        body = body.push(match detail.tab {
            DetailTab::Overview => detail_overview(detail, instance, running),
            DetailTab::Content => detail_content(detail),
            DetailTab::Worlds => detail_worlds(detail),
            DetailTab::Logs => detail_logs(detail),
        });
    }

    body.height(Length::Fill).into()
}

fn detail_overview<'a>(
    detail: &'a DetailView,
    instance: &'a InstanceView,
    running: Option<&'a ProcessView>,
) -> Element<'a, Msg> {
    container(
        column![
            text(t("Overview")).size(16).color(fg()),
            setting_row("Instance ID", &instance.id),
            setting_row(
                "Loader version",
                instance.loader_version.as_deref().unwrap_or("—")
            ),
            setting_row(
                "Last played",
                instance.last_played.as_deref().unwrap_or("Never")
            ),
            setting_row(
                "Playtime",
                &format!("{} {}", instance.playtime_minutes, t("minutes"))
            ),
            setting_row(
                "Runtime",
                &running
                    .map(|process| format!(
                        "{} {}",
                        t("Running since"),
                        process.started_at
                    ))
                    .unwrap_or_else(|| "Stopped".into())
            ),
            setting_row(
                "Memory",
                &instance
                    .memory_mb
                    .map(|value| format!("{value} MB"))
                    .unwrap_or_else(|| "Global default".into())
            ),
            setting_row(
                "Java",
                instance.java_path.as_deref().unwrap_or("Auto-detected")
            ),
            setting_row(
                "Content",
                &format!("{} {}", detail.content.len(), t("files"))
            ),
            setting_row(
                "Worlds",
                &format!("{} {}", detail.worlds.len(), t("worlds"))
            ),
            setting_row(
                "Logs",
                &format!("{} {}", detail.logs.len(), t("log files"))
            ),
        ]
        .spacing(8),
    )
    .padding(16)
    .style(card_style)
    .into()
}

fn detail_content(detail: &DetailView) -> Element<'_, Msg> {
    let header = row![
        text(t("Content")).size(16).color(fg()),
        text(format!("{} {}", detail.content.len(), t("files")))
            .size(10)
            .color(muted_c()),
        Space::new().width(Length::Fill),
        button(text(t("Update all")).size(10).color(warning_c()))
            .padding([4, 8])
            .on_press(Msg::User(Message::UpdateAllContent))
            .style(ghost_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut body = column![header].spacing(8);
    if let Some(picker) = &detail.versions {
        body = body.push(version_picker(picker));
    }

    if detail.content.is_empty() {
        body = body.push(
            text(t("No managed content in this instance."))
                .size(12)
                .color(muted_c()),
        );
        return container(body).padding(16).style(card_style).into();
    }

    let mut list = column![].spacing(6);
    for item in &detail.content {
        list = list.push(content_row(item));
    }
    body = body.push(scrollable(list).height(Length::Fill));
    container(body)
        .padding(16)
        .height(Length::Fill)
        .style(card_style)
        .into()
}

fn version_picker(picker: &VersionPicker) -> Element<'_, Msg> {
    let mut list = column![].spacing(4);
    if picker.loading {
        list =
            list.push(text(t("Loading versions…")).size(10).color(muted_c()));
    } else if let Some(error_c) = &picker.error {
        list = list.push(text(error_c).size(10).color(danger_c()));
    } else if picker.versions.is_empty() {
        list = list
            .push(text(t("No versions available.")).size(10).color(muted_c()));
    } else {
        for version in picker.versions.iter().take(20) {
            let meta = format!(
                "{} · {}",
                if version.game_versions.is_empty() {
                    t("no game versions").to_string()
                } else {
                    version.game_versions.join(", ")
                },
                if version.loaders.is_empty() {
                    t("no loader").to_string()
                } else {
                    version.loaders.join(", ")
                }
            );
            list = list.push(
                row![
                    column![
                        text(&version.number).size(11).color(fg()),
                        text(meta).size(9).color(muted_c()),
                    ]
                    .spacing(1)
                    .width(Length::Fill),
                    button(text(t("Use")).size(10).color(accent_c()))
                        .padding([3, 7])
                        .on_press(Msg::User(Message::SwitchVersion {
                            path: picker.path.clone(),
                            version_id: version.id.clone(),
                        }))
                        .style(ghost_style),
                ]
                .spacing(6)
                .align_y(Alignment::Center),
            );
        }
    }

    container(
        column![
            row![
                text(format!("{} · {}", t("Versions"), picker.title))
                    .size(12)
                    .color(fg()),
                Space::new().width(Length::Fill),
                button(text(t("Close")).size(10).color(muted_c()))
                    .padding([3, 7])
                    .on_press(Msg::User(Message::CloseVersions))
                    .style(ghost_style),
            ],
            list,
        ]
        .spacing(6),
    )
    .padding(10)
    .style(surface_style)
    .into()
}

fn content_row(item: &ContentRow) -> Element<'_, Msg> {
    let toggle = if item.locked {
        text(t("Locked")).size(10).color(warning_c())
    } else if item.enabled {
        text(t("Enabled")).size(10).color(accent_c())
    } else {
        text(t("Disabled")).size(10).color(muted_c())
    };

    let mut actions = row![toggle].spacing(5).align_y(Alignment::Center);
    if item.locked {
        actions = actions.push(
            button(text(t("Unlock")).size(10).color(warning_c()))
                .padding([4, 8])
                .on_press(Msg::User(Message::ToggleLock {
                    path: item.file_path.clone(),
                    locked: false,
                }))
                .style(ghost_style),
        );
    } else {
        actions = actions.push(
            button(
                text(if item.enabled { "Disable" } else { "Enable" })
                    .size(10)
                    .color(fg()),
            )
            .padding([4, 8])
            .on_press(Msg::User(Message::ToggleContent {
                path: item.file_path.clone(),
                enabled: !item.enabled,
            }))
            .style(ghost_style),
        );
        actions = actions.push(
            button(text(t("Lock")).size(10).color(muted_c()))
                .padding([4, 8])
                .on_press(Msg::User(Message::ToggleLock {
                    path: item.file_path.clone(),
                    locked: true,
                }))
                .style(ghost_style),
        );
    }

    if item.has_update {
        actions = actions.push(
            button(text(t("Update")).size(10).color(warning_c()))
                .padding([4, 8])
                .on_press(Msg::User(Message::UpdateContent(
                    item.file_path.clone(),
                )))
                .style(ghost_style),
        );
    }
    if let Some(project_id) = &item.project_id {
        actions = actions.push(
            button(text(t("Versions")).size(10).color(muted_c()))
                .padding([4, 8])
                .on_press(Msg::User(Message::ShowVersions {
                    path: item.file_path.clone(),
                    project_id: project_id.clone(),
                }))
                .style(ghost_style),
        );
    }
    actions = actions.push(
        button(text(t("Remove")).size(10).color(danger_c()))
            .padding([4, 8])
            .on_press(Msg::User(Message::RemoveContent(item.file_path.clone())))
            .style(ghost_style),
    );

    container(
        row![
            column![
                text(&item.title).size(13).color(fg()),
                text(format!(
                    "{} · {}",
                    item.version.as_deref().unwrap_or(t("unknown version")),
                    item.file_name
                ))
                .size(10)
                .color(muted_c()),
            ]
            .spacing(2)
            .width(Length::Fill),
            actions,
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(10)
    .style(card_style)
    .into()
}

fn edit_card<'a>(
    edit: &'a InstanceEditForm,
    groups: &'a [InstanceGroup],
) -> Element<'a, Msg> {
    container(
        column![
            text(t("Instance settings")).size(13).color(fg()),
            row![
                text_input(t("Instance name"), &edit.name)
                    .size(12)
                    .padding(8)
                    .on_input(|value| Msg::User(Message::RenameChanged(value)))
                    .style(text_input_style)
                    .width(Length::Fill),
                button(text(t("Save")).size(10).color(ink_c()))
                    .padding([6, 12])
                    .on_press(Msg::User(Message::SaveRename))
                    .style(primary_style),
                button(text(t("Cancel")).size(10).color(muted_c()))
                    .padding([6, 10])
                    .on_press(Msg::User(Message::CancelRename))
                    .style(ghost_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            row![
                text_input(t("Memory MB (empty = inherit)"), &edit.memory)
                    .size(12)
                    .padding(8)
                    .on_input(|value| Msg::User(Message::EditMemory(value)))
                    .style(text_input_style)
                    .width(Length::FillPortion(1)),
                text_input(t("Java path (empty = auto)"), &edit.java_path)
                    .size(12)
                    .padding(8)
                    .on_input(|value| Msg::User(Message::EditJava(value)))
                    .style(text_input_style)
                    .width(Length::FillPortion(2)),
            ]
            .spacing(8),
            text_input(t("Extra JVM args (space separated)"), &edit.args)
                .size(12)
                .padding(8)
                .on_input(|value| Msg::User(Message::EditArgs(value)))
                .style(text_input_style)
                .width(Length::Fill),
            java_picker(edit),
            edit_groups(edit, groups),
            row![
                text_input(t("New icon image path (saved on Save)"), &edit.icon_path)
                    .size(12)
                    .padding(8)
                    .on_input(|value| Msg::User(Message::EditIconPath(value)))
                    .style(text_input_style)
                    .width(Length::Fill),
                button(text(t("Remove icon")).size(10).color(danger_c()))
                    .padding([6, 10])
                    .on_press(Msg::User(Message::RemoveInstanceIcon))
                    .style(ghost_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
            text(t("Overrides apply only to this instance. Launch changes take effect next run."))
                .size(9)
                .color(muted_c()),
        ]
        .spacing(8),
    )
    .padding(12)
    .style(card_style)
    .into()
}

fn edit_groups<'a>(
    edit: &'a InstanceEditForm,
    groups: &'a [InstanceGroup],
) -> Element<'a, Msg> {
    let mut chips = column![].spacing(3);
    if groups.is_empty() {
        chips = chips.push(
            text(t("No groups yet. Create one from the Library."))
                .size(9)
                .color(muted_c()),
        );
    }
    for group in groups {
        let selected = edit.group_ids.iter().any(|id| id == &group.id);
        chips = chips.push(
            button(
                row![
                    text(if selected { "✓" } else { "○" })
                        .size(10)
                        .color(if selected { accent_c() } else { muted_c() }),
                    text(&group.name).size(10).color(fg()),
                ]
                .spacing(5),
            )
            .padding([3, 7])
            .on_press(Msg::User(Message::ToggleGroupMembership(
                group.id.clone(),
            )))
            .style(nav_style(selected, 0.0)),
        );
    }
    column![
        text(t("Groups (applied on Save)"))
            .size(10)
            .color(muted_c()),
        chips,
    ]
    .spacing(4)
    .into()
}

fn java_picker(edit: &InstanceEditForm) -> Element<'_, Msg> {
    let detect_button: Element<'_, Msg> = if edit.jres_loading {
        text(t("Scanning Java…")).size(10).color(muted_c()).into()
    } else {
        button(text(t("Detect installed Java")).size(10).color(fg()))
            .padding([4, 8])
            .on_press(Msg::User(Message::DetectJava))
            .style(ghost_style)
            .into()
    };
    let required: Element<'_, Msg> = if edit.required_java_loading {
        text(t("Resolving required Java…"))
            .size(9)
            .color(muted_c())
            .into()
    } else if edit.java_installing {
        text(t("Installing Java…"))
            .size(9)
            .color(warning_c())
            .into()
    } else if let Some(version) = edit.required_java {
        row![
            text(format!("Required Java: {version}"))
                .size(9)
                .color(muted_c()),
            button(
                text(format!("Install Java {version}"))
                    .size(9)
                    .color(warning_c())
            )
            .padding([3, 7])
            .on_press(Msg::User(Message::InstallJava(version)))
            .style(ghost_style),
        ]
        .spacing(6)
        .align_y(Alignment::Center)
        .into()
    } else {
        Space::new().into()
    };
    let mut list = column![row![detect_button, required].spacing(6)].spacing(4);
    if let Some(error_c) = &edit.jres_error {
        list = list.push(text(error_c).size(9).color(danger_c()));
    }
    for jre in edit.jres.iter().take(8) {
        list = list.push(java_choice(jre));
    }
    list.into()
}

fn java_choice(jre: &JreChoice) -> Element<'_, Msg> {
    container(
        row![
            column![
                text(format!("Java {} · {}", jre.parsed_version, jre.version))
                    .size(10)
                    .color(fg()),
                text(&jre.path).size(9).color(muted_c()),
            ]
            .spacing(1)
            .width(Length::Fill),
            button(text(t("Use")).size(9).color(accent_c()))
                .padding([3, 7])
                .on_press(Msg::User(Message::UseJava(jre.path.clone())))
                .style(ghost_style),
        ]
        .spacing(6)
        .align_y(Alignment::Center),
    )
    .padding([4, 6])
    .style(surface_style)
    .into()
}

fn import_card(form: &LibraryForm) -> Element<'_, Msg> {
    let LibraryForm::Import {
        base_path,
        folder,
        launcher,
    } = form
    else {
        return Space::new().into();
    };
    container(
        column![
            row![
                text(t("Import instance")).size(13).color(fg()),
                Space::new().width(Length::Fill),
                button(text(t("Cancel")).size(10).color(muted_c()))
                    .padding([5, 9])
                    .on_press(Msg::User(Message::CancelLibraryForm))
                    .style(ghost_style),
            ],
            text_input(t("Launcher base path"), base_path)
                .size(12)
                .padding(8)
                .on_input(|value| Msg::User(Message::ImportBasePath(value)))
                .style(text_input_style)
                .width(Length::Fill),
            row![
                text_input(t("Instance folder"), folder)
                    .size(12)
                    .padding(8)
                    .on_input(|value| Msg::User(Message::ImportFolder(value)))
                    .style(text_input_style)
                    .width(Length::Fill),
                button(
                    text(format!("{}{launcher}", t("Launcher: ")))
                        .size(10)
                        .color(fg())
                )
                .padding([6, 10])
                .on_press(Msg::User(Message::CycleImportLauncher))
                .style(ghost_style),
            ]
            .spacing(8),
            row![
                Space::new().width(Length::Fill),
                button(text(t("Start import")).size(10).color(ink_c()))
                    .padding([6, 12])
                    .on_press(Msg::User(Message::StartImport))
                    .style(primary_style),
            ],
        ]
        .spacing(8),
    )
    .padding(12)
    .style(card_style)
    .into()
}

fn export_card(form: &LibraryForm) -> Element<'_, Msg> {
    let LibraryForm::Export { path } = form else {
        return Space::new().into();
    };
    container(
        column![
            text(t("Export instance")).size(13).color(fg()),
            text(t("Writes a .mrpack containing the instance content and overrides."))
                .size(9)
                .color(muted_c()),
            row![
                text_input(t("Export path"), path)
                    .size(12)
                    .padding(8)
                    .on_input(|value| Msg::User(Message::ExportPath(value)))
                    .style(text_input_style)
                    .width(Length::Fill),
                button(text(t("Export")).size(10).color(ink_c()))
                    .padding([6, 12])
                    .on_press(Msg::User(Message::StartExport))
                    .style(primary_style),
                button(text(t("Cancel")).size(10).color(muted_c()))
                    .padding([6, 10])
                    .on_press(Msg::User(Message::CancelLibraryForm))
                    .style(ghost_style),
            ]
            .spacing(8)
            .align_y(Alignment::Center),
        ]
        .spacing(8),
    )
    .padding(12)
    .style(card_style)
    .into()
}

fn delete_confirm_card(name: &str) -> Element<'_, Msg> {
    container(
        row![
            column![
                text(format!("{}{name}?", t("Delete ")))
                    .size(13)
                    .color(danger_c()),
                text(t("This removes the instance folder and its content."))
                    .size(10)
                    .color(muted_c()),
            ]
            .spacing(2)
            .width(Length::Fill),
            button(text(t("Delete permanently")).size(10).color(ink_c()))
                .padding([6, 12])
                .on_press(Msg::User(Message::DeleteInstance))
                .style(danger_button_style),
            button(text(t("Cancel")).size(10).color(muted_c()))
                .padding([6, 10])
                .on_press(Msg::User(Message::CancelDelete))
                .style(ghost_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    )
    .padding(12)
    .style(card_style)
    .into()
}

fn detail_worlds(detail: &DetailView) -> Element<'_, Msg> {
    let header = row![
        text(t("Worlds")).size(16).color(fg()),
        text(format!("{} {}", detail.worlds.len(), t("entries")))
            .size(10)
            .color(muted_c()),
        Space::new().width(Length::Fill),
        button(text(t("Add server")).size(10).color(accent_c()))
            .padding([4, 8])
            .on_press(Msg::User(Message::AddServer))
            .style(ghost_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut body = column![header].spacing(8);
    if let Some(form) = &detail.world_form {
        body = body.push(world_form_card(form));
    }
    if detail.worlds.is_empty() {
        body = body.push(
            text(t("No worlds or servers found."))
                .size(12)
                .color(muted_c()),
        );
        return container(body).padding(16).style(card_style).into();
    }

    let mut list = column![].spacing(6);
    for world in &detail.worlds {
        list = list.push(world_row(world));
    }
    body = body.push(scrollable(list).height(Length::Fill));
    container(body)
        .padding(16)
        .height(Length::Fill)
        .style(card_style)
        .into()
}

fn world_form_card(form: &WorldForm) -> Element<'_, Msg> {
    let (title, destructive) = match &form.kind {
        WorldFormKind::AddServer => (t("Add server").to_string(), false),
        WorldFormKind::EditServer { .. } => {
            (t("Edit server").to_string(), false)
        }
        WorldFormKind::RenameWorld { .. } => {
            (t("Rename world").to_string(), false)
        }
        WorldFormKind::DeleteWorld { name, .. } => {
            (format!("{}{name}?", t("Delete ")), true)
        }
        WorldFormKind::DeleteServer { name, .. } => {
            (format!("{}{name}?", t("Delete ")), true)
        }
    };

    let save_label = t(if destructive {
        "Delete permanently"
    } else {
        "Save"
    });
    let save_style = if destructive {
        danger_button_style
    } else {
        primary_style
    };

    let mut content = column![text(title).size(13).color(if destructive {
        danger_c()
    } else {
        fg()
    }),]
    .spacing(6);

    if destructive {
        let warning_c =
            if matches!(form.kind, WorldFormKind::DeleteWorld { .. }) {
                t("This removes the world data from disk.")
            } else {
                t("This removes the server from this instance.")
            };
        content = content.push(text(warning_c).size(10).color(muted_c()));
    } else {
        let rename_only =
            matches!(form.kind, WorldFormKind::RenameWorld { .. });
        content = content.push(
            text_input(t("Name"), &form.name)
                .size(12)
                .padding(8)
                .on_input(|value| Msg::User(Message::WorldFormName(value)))
                .style(text_input_style)
                .width(Length::Fill),
        );
        if !rename_only {
            content = content.push(
                row![
                    text_input(t("Server address"), &form.address)
                        .size(12)
                        .padding(8)
                        .on_input(|value| Msg::User(Message::WorldFormAddress(
                            value
                        )))
                        .style(text_input_style)
                        .width(Length::Fill),
                    button(
                        text(format!("{}{}", t("Pack: "), form.pack_status))
                            .size(10)
                            .color(fg()),
                    )
                    .padding([6, 10])
                    .on_press(Msg::User(Message::CyclePackStatus))
                    .style(ghost_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            );
        }
    }

    content = content.push(
        row![
            Space::new().width(Length::Fill),
            button(text(save_label).size(10).color(ink_c()))
                .padding([6, 12])
                .on_press(Msg::User(Message::SaveWorldForm))
                .style(save_style),
            button(text(t("Cancel")).size(10).color(muted_c()))
                .padding([6, 10])
                .on_press(Msg::User(Message::CancelWorldForm))
                .style(ghost_style),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );

    container(content).padding(10).style(surface_style).into()
}

fn world_row(world: &WorldRow) -> Element<'_, Msg> {
    let mut actions = row![].spacing(4).align_y(Alignment::Center);
    if let Some(index) = world.server_index {
        actions = actions.push(
            button(text(t("Edit")).size(10).color(fg()))
                .padding([4, 8])
                .on_press(Msg::User(Message::EditServer(index)))
                .style(ghost_style),
        );
        actions = actions.push(
            button(text(t("Delete")).size(10).color(danger_c()))
                .padding([4, 8])
                .on_press(Msg::User(Message::DeleteServer {
                    index,
                    name: world.name.clone(),
                }))
                .style(ghost_style),
        );
    } else {
        actions = actions.push(
            button(text(t("Rename")).size(10).color(fg()))
                .padding([4, 8])
                .on_press(Msg::User(Message::RenameWorld(world.detail.clone())))
                .style(ghost_style),
        );
        actions = actions.push(
            button(text(t("Backup")).size(10).color(fg()))
                .padding([4, 8])
                .on_press(Msg::User(Message::BackupWorld {
                    path: world.detail.clone(),
                    name: world.name.clone(),
                }))
                .style(ghost_style),
        );
        actions = actions.push(
            button(text(t("Delete")).size(10).color(danger_c()))
                .padding([4, 8])
                .on_press(Msg::User(Message::DeleteWorld {
                    path: world.detail.clone(),
                    name: world.name.clone(),
                }))
                .style(ghost_style),
        );
    }

    container(
        row![
            world_art(world, 40.0),
            column![
                text(&world.name).size(13).color(fg()),
                text(&world.detail).size(10).color(muted_c()),
            ]
            .spacing(2)
            .width(Length::Fill),
            text(&world.kind).size(10).color(accent_c()),
            text(world.last_played.as_deref().unwrap_or("Never played"))
                .size(10)
                .color(muted_c()),
            actions,
        ]
        .spacing(10)
        .align_y(Alignment::Center),
    )
    .padding(10)
    .style(card_style)
    .into()
}

fn world_art<'a>(world: &'a WorldRow, size: f32) -> Element<'a, Msg> {
    match &world.icon {
        Some(WorldArt::Path(path)) => {
            image_art(image::Handle::from_path(path.clone()), size)
        }
        Some(WorldArt::Bytes(bytes)) => {
            bytes_handle(format!("world:{}", world.icon_key), bytes)
                .map(|handle| image_art(handle, size))
                .unwrap_or_else(|| avatar(&world.name, size))
        }
        None => avatar(&world.name, size),
    }
}

fn image_art(handle: image::Handle, size: f32) -> Element<'static, Msg> {
    image(handle)
        .width(size)
        .height(size)
        .border_radius(size * 0.27)
        .content_fit(ContentFit::Cover)
        .into()
}

fn detail_logs(detail: &DetailView) -> Element<'_, Msg> {
    let header = row![
        text(t("Logs")).size(16).color(fg()),
        text(format!("{} {}", detail.logs.len(), t("log files")))
            .size(10)
            .color(muted_c()),
        Space::new().width(Length::Fill),
        if detail.logs.is_empty() {
            Element::from(Space::new())
        } else {
            button(text(t("Delete all")).size(10).color(danger_c()))
                .padding([4, 8])
                .on_press(Msg::User(Message::DeleteAllLogs))
                .style(ghost_style)
                .into()
        },
    ]
    .spacing(8)
    .align_y(Alignment::Center);

    let mut body = column![header].spacing(8);
    if let Some(view) = &detail.log_view {
        body = body.push(log_view_card(view));
    }
    if detail.logs.is_empty() {
        body = body.push(
            text(t("No logs or crash reports found."))
                .size(12)
                .color(muted_c()),
        );
        return container(body).padding(16).style(card_style).into();
    }

    let mut list = column![].spacing(6);
    for log in &detail.logs {
        list = list.push(
            container(
                row![
                    column![
                        text(&log.filename).size(12).color(fg()),
                        text(format!("{} {}", log.age_seconds, t("s old")))
                            .size(9)
                            .color(muted_c()),
                    ]
                    .spacing(1)
                    .width(Length::Fill),
                    text(&log.kind).size(9).color(accent_c()),
                    button(text(t("View")).size(10).color(fg()))
                        .padding([4, 8])
                        .on_press(Msg::User(Message::ShowLog {
                            filename: log.filename.clone(),
                            crash: log.crash,
                        }))
                        .style(ghost_style),
                    button(text(t("Delete")).size(10).color(danger_c()))
                        .padding([4, 8])
                        .on_press(Msg::User(Message::DeleteLog {
                            filename: log.filename.clone(),
                            crash: log.crash,
                        }))
                        .style(ghost_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            )
            .padding(10)
            .style(card_style),
        );
    }
    body = body.push(scrollable(list).height(Length::Fill));
    container(body)
        .padding(16)
        .height(Length::Fill)
        .style(card_style)
        .into()
}

fn log_view_card(view: &LogView) -> Element<'_, Msg> {
    let content: Element<'_, Msg> = if view.loading {
        text(t("Loading log…")).size(10).color(muted_c()).into()
    } else if let Some(error_c) = &view.error {
        text(error_c).size(10).color(danger_c()).into()
    } else {
        scrollable(
            text(view.content.as_deref().unwrap_or(""))
                .size(10)
                .color(muted_c()),
        )
        .height(Length::Fixed(220.0))
        .into()
    };
    container(
        column![
            row![
                text(&view.filename).size(11).color(fg()),
                Space::new().width(Length::Fill),
                button(text(t("Close")).size(10).color(muted_c()))
                    .padding([3, 7])
                    .on_press(Msg::User(Message::CloseLog))
                    .style(ghost_style),
            ]
            .align_y(Alignment::Center),
            content,
        ]
        .spacing(6),
    )
    .padding(10)
    .style(surface_style)
    .into()
}

fn account(view: &ViewModel) -> Element<'_, Msg> {
    let mut users = column![].spacing(6);
    if view.accounts.is_empty() {
        users = users.push(
            text(t("No Minecraft account signed in."))
                .size(13)
                .color(muted_c()),
        );
    } else {
        for account in &view.accounts {
            let active_control: Element<'_, Msg> = if account.active {
                text(t("Active")).size(10).color(accent_c()).into()
            } else {
                button(text(t("Use")).size(10).color(fg()))
                    .padding([4, 8])
                    .on_press(Msg::User(Message::SetAccount(
                        account.id.clone(),
                    )))
                    .style(ghost_style)
                    .into()
            };
            users = users.push(
                container(
                    row![
                        text(if account.active { "●" } else { "○" })
                            .size(14)
                            .color(if account.active {
                                accent_c()
                            } else {
                                muted_c()
                            }),
                        text(&account.name).size(14).color(fg()),
                        Space::new().width(Length::Fill),
                        active_control,
                        button(text(t("Remove")).size(10).color(danger_c()))
                            .padding([4, 8])
                            .on_press(Msg::User(Message::RemoveAccount(
                                account.id.clone()
                            )))
                            .style(ghost_style),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding(9)
                .style(card_style),
            );
        }
    }

    let mut login = column![
        button(text(t("Add Microsoft account")).size(13).color(ink_c()))
            .padding([8, 14])
            .on_press(Msg::User(Message::BeginLogin))
            .style(primary_style),
    ]
    .spacing(8);

    if let Some(url) = &view.login_url {
        login = login
            .push(text(t("Microsoft sign-in")).size(16).color(fg()))
            .push(text(url).size(10).color(muted_c()))
            .push(
                row![
                    button(text(t("Open browser")).size(12).color(muted_c()))
                        .padding([6, 10])
                        .on_press(Msg::User(Message::OpenLoginUrl))
                        .style(ghost_style),
                    button(text(t("Cancel")).size(12).color(muted_c()))
                        .padding([6, 10])
                        .on_press(Msg::User(Message::CancelLogin))
                        .style(ghost_style),
                ]
                .spacing(6),
            )
            .push(
                text_input(
                    t("Paste the final redirect URL or code"),
                    &view.login_code,
                )
                .padding(8)
                .width(Length::Fill)
                .style(text_input_style)
                .on_input(|value| Msg::User(Message::LoginCodeChanged(value))),
            )
            .push(
                button(text(t("Finish sign-in")).size(13).color(ink_c()))
                    .padding([8, 14])
                    .on_press(Msg::User(Message::FinishLogin))
                    .style(primary_style),
            );

        if !view.login_opened {
            login = login.push(
                text(t("The browser did not open automatically; open the URL manually."))
                    .size(11)
                    .color(warning_c()),
            );
        }
    }

    scrollable(
        column![
            page_heading("PROFILE", "Player profile"),
            active_player_card(view),
            modrinth_card(view),
            shared_join_card(view),
            users,
            Space::new().height(12),
            login,
        ]
        .spacing(10),
    )
    .height(Length::Fill)
    .into()
}

fn active_player_card(view: &ViewModel) -> Element<'_, Msg> {
    let Some(account) = view.accounts.iter().find(|account| account.active)
    else {
        return Space::new().height(0).into();
    };
    let head = player_head(view.player_head.as_ref(), &account.name, 52.0);
    let skin: Element<'_, Msg> = view
        .player_skin
        .as_ref()
        .and_then(|bytes| bytes_handle(bytes_key("player-skin", bytes), bytes))
        .map(|handle| {
            image(handle)
                .width(72)
                .height(144)
                .filter_method(image::FilterMethod::Nearest)
                .content_fit(ContentFit::Contain)
                .into()
        })
        .unwrap_or_else(|| avatar(&account.name, 72.0));

    container(
        row![
            head,
            column![
                text(t("CURRENT SKIN")).size(9).color(accent_c()),
                text(&account.name).size(18).color(fg()),
                text(t("This is the skin Mojang currently reports for your profile."))
                    .size(10)
                    .color(muted_c()),
            ]
            .spacing(3)
            .width(Length::Fill),
            skin,
        ]
        .spacing(14)
        .align_y(Alignment::Center),
    )
    .padding(14)
    .style(card_style)
    .into()
}

fn modrinth_card(view: &ViewModel) -> Element<'_, Msg> {
    let account: Element<'_, Msg> = match &view.modrinth_account {
        Some(account) => {
            let name = account.username.as_deref().unwrap_or(&account.user_id);
            column![
                row![
                    text(t("●")).size(14).color(accent_c()),
                    column![
                        text(name).size(14).color(fg()),
                        text(format!(
                            "{}{}",
                            t("Modrinth user "),
                            account.user_id
                        ))
                        .size(9)
                        .color(muted_c()),
                    ]
                    .spacing(1)
                    .width(Length::Fill),
                    button(text(t("Sign out")).size(10).color(danger_c()))
                        .padding([4, 8])
                        .on_press(Msg::User(Message::LogoutModrinth))
                        .style(ghost_style),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(6)
            .into()
        }
        None => {
            let button_label = if view.modrinth_login {
                "Waiting for browser…"
            } else {
                "Sign in with Modrinth"
            };
            let sign_in: Element<'_, Msg> = if view.modrinth_login {
                text(button_label).size(11).color(warning_c()).into()
            } else {
                button(text(button_label).size(12).color(ink_c()))
                    .padding([7, 12])
                    .on_press(Msg::User(Message::BeginModrinthLogin))
                    .style(primary_style)
                    .into()
            };
            column![
                text(t("Sign in to use friends and shared instances."))
                    .size(10)
                    .color(muted_c()),
                sign_in,
            ]
            .spacing(6)
            .into()
        }
    };

    let mut friends = column![].spacing(5);
    if view.modrinth_account.is_some() {
        if view.friends.is_empty() {
            friends = friends
                .push(text(t("No friends yet.")).size(10).color(muted_c()));
        }
        for friend in &view.friends {
            friends = friends.push(
                container(
                    row![
                        text(if friend.accepted { "●" } else { "○" })
                            .size(10)
                            .color(if friend.accepted {
                                accent_c()
                            } else {
                                warning_c()
                            }),
                        text(&friend.user_id).size(10).color(fg()),
                        text(if friend.accepted {
                            "accepted"
                        } else {
                            "pending"
                        })
                        .size(9)
                        .color(muted_c()),
                        Space::new().width(Length::Fill),
                        button(text(t("Remove")).size(9).color(danger_c()))
                            .padding([3, 7])
                            .on_press(Msg::User(Message::RemoveFriend(
                                friend.user_id.clone(),
                            )))
                            .style(ghost_style),
                    ]
                    .spacing(8)
                    .align_y(Alignment::Center),
                )
                .padding([5, 7])
                .style(surface_style),
            );
        }
        friends = friends.push(
            row![
                text_input(t("Friend's Modrinth user ID"), &view.friend_input)
                    .size(11)
                    .padding(7)
                    .on_input(|value| Msg::User(Message::FriendIdChanged(
                        value
                    )))
                    .on_submit(Msg::User(Message::AddFriend))
                    .style(text_input_style)
                    .width(Length::Fill),
                button(text(t("Add friend")).size(9).color(ink_c()))
                    .padding([5, 10])
                    .on_press(Msg::User(Message::AddFriend))
                    .style(primary_style),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        );
    }

    let mut body = column![
        row![
            column![
                text(t("MODRINTH")).size(9).color(accent_c()),
                text(t("Modrinth account")).size(16).color(fg()),
            ]
            .spacing(2)
            .width(Length::Fill),
        ],
        account,
    ]
    .spacing(8);
    if view.modrinth_account.is_some() {
        body = body.push(
            column![text(t("Friends")).size(12).color(fg()), friends,]
                .spacing(5),
        );
    }

    container(body).padding(14).style(card_style).into()
}

fn skin_display_name(name: &str) -> String {
    if let Some(rest) = name.strip_prefix("Saved skin ") {
        format!("{} {}", t("Saved skin"), rest)
    } else {
        name.to_string()
    }
}

fn skins(view: &ViewModel) -> Element<'_, Msg> {
    if !view.skins_loaded {
        return column![
            page_heading("WARDROBE", "Wardrobe"),
            text(t(
                "Skins and capes are applied to the active Minecraft account through Mojang."
            ))
            .size(11)
            .color(muted_c()),
            button(text(t("Load skins")).size(12).color(ink_c()))
                .padding([7, 12])
                .on_press(Msg::User(Message::LoadSkins))
                .style(primary_style),
        ]
        .spacing(11)
        .into();
    }

    let selected_skin = view
        .selected_skin
        .or_else(|| view.skins.iter().position(|skin| skin.equipped));
    let skin = selected_skin.and_then(|index| view.skins.get(index));
    let cape = view.selected_cape.and_then(|index| view.capes.get(index));
    let skin_name = skin
        .map(|skin| skin_display_name(&skin.name))
        .unwrap_or_else(|| t("No skin selected").to_string());
    let cape_name = cape.map(|cape| cape.name.as_str()).unwrap_or("No cape");
    let preview: Element<'_, Msg> = if let Some(bytes) = &view.skin_preview_rgba
    {
        rgba_handle(
            bytes_key("skin-preview-rgba", bytes),
            crate::skin3d::WIDTH,
            crate::skin3d::HEIGHT,
            bytes,
        )
        .map(|handle| {
            image(handle)
                .width(248)
                .height(306)
                .filter_method(image::FilterMethod::Nearest)
                .content_fit(ContentFit::Contain)
                .into()
        })
        .unwrap_or_else(|| avatar(&skin_name, 144.0))
    } else {
        view.skin_preview
            .as_ref()
            .and_then(|bytes| {
                bytes_handle(bytes_key("skin-preview", bytes), bytes)
            })
            .map(|handle| {
                image(handle)
                    .width(144)
                    .height(288)
                    .filter_method(image::FilterMethod::Nearest)
                    .content_fit(ContentFit::Contain)
                    .into()
            })
            .unwrap_or_else(|| avatar(&skin_name, 144.0))
    };
    let preview: Element<'_, Msg> = mouse_area(preview)
        .on_press(Msg::User(Message::SkinDragStart))
        .on_release(Msg::User(Message::SkinDragEnd))
        .on_move(|point| Msg::User(Message::SkinDragMove(point.x, point.y)))
        .interaction(mouse::Interaction::Grab)
        .into();

    let mut actions = row![].spacing(6).align_y(Alignment::Center);
    actions = actions.push(
        button(text(t("Reset view")).size(10).color(muted_c()))
            .padding([5, 9])
            .on_press(Msg::User(Message::SkinResetView))
            .style(ghost_style),
    );
    if view.skin_preview_pending {
        actions = actions
            .push(
                button(text(t("Reset")).size(11).color(fg()))
                    .padding([6, 11])
                    .on_press(Msg::User(Message::ResetSkinSelection))
                    .style(ghost_style),
            )
            .push(
                button(text(t("Apply skin and cape")).size(11).color(ink_c()))
                    .padding([6, 12])
                    .on_press(Msg::User(Message::ApplySelectedSkin))
                    .style(primary_style),
            );
    } else {
        actions = actions.push(
            container(text(t("Equipped")).size(10).color(ink_c()))
                .padding([5, 9])
                .style(accent_chip_style),
        );
    }

    let preview_panel = container(
        column![
            text(t(if view.skin_preview_pending {
                "PREVIEWING"
            } else {
                "EQUIPPED"
            }))
            .size(9)
            .color(if view.skin_preview_pending {
                warning_c()
            } else {
                accent_c()
            }),
            preview,
            text(t("Drag to rotate")).size(9).color(muted_c()),
            column![
                text(skin_name.clone()).size(14).color(fg()),
                text(format!(
                    "{} · {} {}",
                    skin.map(|skin| t(&skin.variant)).unwrap_or("—"),
                    t("cape:"),
                    t(cape_name)
                ))
                .size(9)
                .color(muted_c()),
            ]
            .spacing(2)
            .align_x(Alignment::Center),
            actions,
        ]
        .spacing(10)
        .align_x(Alignment::Center),
    )
    .padding(16)
    .width(Length::Fixed(286.0))
    .height(Length::Fill)
    .style(card_style);

    let mut list = column![].spacing(6);
    let saved_count = view
        .skins
        .iter()
        .filter(|skin| skin.source != "Default")
        .count();
    let mut tabs = row![].spacing(6);
    for (tab, label, count) in [
        (SkinTab::Saved, "Saved", saved_count),
        (
            SkinTab::Defaults,
            "Default",
            view.skins.len().saturating_sub(saved_count),
        ),
    ] {
        let selected = view.skin_tab == tab;
        tabs = tabs.push(
            button(
                text(format!("{} ({count})", t(label)))
                    .size(10)
                    .color(if selected { accent_c() } else { muted_c() }),
            )
            .padding([5, 9])
            .on_press(Msg::User(Message::SkinTab(tab)))
            .style(nav_style(selected, 0.0)),
        );
    }
    list = list.push(tabs);

    // Capes are kept next to the skin list so both sides of the preview are
    // immediately reachable instead of being buried under 60 default skins.
    list = list.push(
        row![
            text(t("Capes")).size(15).color(fg()),
            text(t("Applied together with the selected skin."))
                .size(9)
                .color(muted_c()),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    if view.capes.is_empty() {
        list = list.push(
            text(t("No capes are available for this account."))
                .size(10)
                .color(muted_c()),
        );
    } else {
        let mut grid = column![].spacing(6);
        let mut tiles: Vec<Element<'_, Msg>> = Vec::new();
        tiles.push(cape_tile(
            None,
            None,
            "No cape",
            view.selected_cape.is_none(),
            false,
        ));
        for cape in &view.capes {
            tiles.push(cape_tile(
                Some(cape.index),
                cape.preview.as_ref(),
                &cape.name,
                view.selected_cape == Some(cape.index),
                cape.equipped,
            ));
        }
        let mut row_items = row![].spacing(6);
        let mut tile_count = 0;
        for tile in tiles {
            row_items = row_items.push(tile);
            tile_count += 1;
            if tile_count % 3 == 0 {
                grid = grid.push(row_items);
                row_items = row![].spacing(6);
            }
        }
        if tile_count % 3 != 0 {
            grid = grid.push(row_items);
        }
        list = list.push(grid);
    }

    list = list.push(
        row![
            text(t("Skins")).size(15).color(fg()),
            text(format!("{} {}", t("total"), view.skins.len()))
                .size(9)
                .color(muted_c()),
        ]
        .spacing(8)
        .align_y(Alignment::Center),
    );
    let visible: Vec<&crate::app::SkinView> = view
        .skins
        .iter()
        .filter(|skin| {
            matches!(view.skin_tab, SkinTab::Saved)
                == (skin.source != "Default")
        })
        .collect();
    if visible.is_empty() {
        list = list
            .push(text(t("No skins in this tab.")).size(11).color(muted_c()));
    }
    for skin in visible {
        let selected = selected_skin == Some(skin.index);
        let preview: Element<'_, Msg> = skin
            .avatar
            .as_ref()
            .and_then(|bytes| {
                bytes_handle(bytes_key("skin-avatar", bytes), bytes)
            })
            .map(|handle| {
                image(handle)
                    .width(40)
                    .height(80)
                    .filter_method(image::FilterMethod::Nearest)
                    .content_fit(ContentFit::Contain)
                    .into()
            })
            .unwrap_or_else(|| avatar(&skin.name, 40.0));
        let status: Element<'_, Msg> = if skin.equipped {
            text(t("Equipped")).size(9).color(accent_c()).into()
        } else {
            Space::new().into()
        };
        list = list.push(
            button(
                row![
                    preview,
                    column![
                        text(skin_display_name(&skin.name))
                            .size(12)
                            .color(fg()),
                        text(format!(
                            "{} · {}{}",
                            t(&skin.variant),
                            t(&skin.source),
                            skin.cape_name
                                .as_deref()
                                .map(|cape| format!(" · {cape}"))
                                .unwrap_or_default()
                        ))
                        .size(9)
                        .color(muted_c()),
                    ]
                    .spacing(2)
                    .width(Length::Fill),
                    status,
                    text(if selected { "●" } else { "○" })
                        .size(12)
                        .color(if selected { accent_c() } else { muted_c() }),
                ]
                .spacing(10)
                .align_y(Alignment::Center),
            )
            .width(Length::Fill)
            .padding(8)
            .on_press(Msg::User(Message::SelectSkin(skin.index)))
            .style(row_style(selected)),
        );
    }

    row![
        preview_panel,
        scrollable(list).height(Length::Fill).width(Length::Fill)
    ]
    .spacing(12)
    .height(Length::Fill)
    .into()
}

fn cape_tile<'a>(
    index: Option<usize>,
    preview_bytes: Option<&Bytes>,
    name: &'a str,
    selected: bool,
    equipped: bool,
) -> Element<'a, Msg> {
    let preview: Element<'a, Msg> = preview_bytes
        .and_then(|bytes| bytes_handle(bytes_key("cape", bytes), bytes))
        .map(|handle| {
            image(handle)
                .width(20)
                .height(32)
                .filter_method(image::FilterMethod::Nearest)
                .content_fit(ContentFit::Contain)
                .into()
        })
        .unwrap_or_else(|| {
            container(text(t("×")).size(16).color(muted_c()))
                .center(Length::Fixed(20.0))
                .height(Length::Fixed(32.0))
                .into()
        });
    button(
        column![
            preview,
            text(t(name)).size(9).color(fg()),
            if equipped {
                text(t("Equipped")).size(8).color(accent_c())
            } else {
                text(t(" ")).size(8).color(muted_c())
            },
        ]
        .spacing(3)
        .align_x(Alignment::Center),
    )
    .padding([6, 8])
    .on_press(Msg::User(Message::SelectCape(index)))
    .style(row_style(selected))
    .into()
}

fn bytes_key(prefix: &str, bytes: &Bytes) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    bytes.hash(&mut hasher);
    format!("{prefix}:{:x}", hasher.finish())
}

fn theme_buttons(current: AppTheme) -> Vec<Element<'static, Msg>> {
    [
        AppTheme::Dark,
        AppTheme::Light,
        AppTheme::Oled,
        AppTheme::Retro,
        AppTheme::System,
        AppTheme::Violet,
        AppTheme::Rose,
        AppTheme::Teal,
        AppTheme::Crimson,
        AppTheme::Emerald,
    ]
    .into_iter()
    .map(|theme| {
        let selected = current.as_str() == theme.as_str();
        let preview = palette_for(theme.as_str(), false);
        let card = container(
            row![
                container(
                    column![
                        Space::new().width(7).height(7),
                        Space::new().width(7).height(7),
                        Space::new().width(7).height(7),
                    ]
                    .spacing(2),
                )
                .padding(5)
                .style(move |_| container::Style {
                    background: Some(Background::Color(preview.accent)),
                    border: Border {
                        radius: 6.0.into(),
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                column![
                    Space::new().width(52).height(6),
                    Space::new().width(34).height(5),
                ]
                .spacing(4),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .padding(8)
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(preview.bg_top)),
            border: Border {
                color: preview.border,
                width: 1.0,
                radius: 8.0.into(),
            },
            ..Default::default()
        });
        let label = if theme.as_str() == "system" {
            "Sync with system"
        } else {
            theme.as_str()
        };
        button(
            column![
                card,
                row![
                    text(if selected { "●" } else { "○" })
                        .size(9)
                        .color(if selected { accent_c() } else { muted_c() }),
                    text(t(label)).size(10).color(if selected {
                        fg()
                    } else {
                        muted_c()
                    }),
                ]
                .spacing(4),
            ]
            .spacing(5),
        )
        .width(Length::Fixed(138.0))
        .padding(5)
        .on_press(Msg::User(Message::SettingChanged(SettingChange::SetTheme(
            theme,
        ))))
        .style(nav_style(selected, 0.0))
        .into()
    })
    .collect()
}

fn theme_grid(current: AppTheme) -> Element<'static, Msg> {
    let mut grid = column![].spacing(6);
    let mut line = row![].spacing(6).align_y(Alignment::Center);
    let mut count = 0;
    for card in theme_buttons(current) {
        line = line.push(card);
        count += 1;
        if count % 5 == 0 {
            grid = grid.push(line);
            line = row![].spacing(6).align_y(Alignment::Center);
        }
    }
    if count % 5 != 0 {
        grid = grid.push(line);
    }
    grid.into()
}

fn language_row(locale: &str) -> Element<'static, Msg> {
    let locale = locale.to_ascii_lowercase();
    let english = locale.starts_with("en");
    let chinese = locale.starts_with("zh");

    row![
        text(t("Language")).size(13).color(muted_c()),
        Space::new().width(Length::Fill),
        button(text("English").size(11).color(if english {
            accent_c()
        } else {
            muted_c()
        }))
        .padding([5, 10])
        .on_press(Msg::User(Message::SettingChanged(
            SettingChange::SetLocale("en-US",)
        )))
        .style(nav_style(english, 0.0)),
        button(text("简体中文").size(11).color(if chinese {
            accent_c()
        } else {
            muted_c()
        }))
        .padding([5, 10])
        .on_press(Msg::User(Message::SettingChanged(
            SettingChange::SetLocale("zh-CN",)
        )))
        .style(nav_style(chinese, 0.0)),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn settings(view: &ViewModel) -> Element<'_, Msg> {
    let Some(settings) = &view.settings else {
        return text(t("Waiting for launcher settings…"))
            .size(13)
            .color(muted_c())
            .into();
    };

    let body = column![
        page_heading("PREFERENCES", "Preferences"),
        container(
            column![
                text(t("Downloads")).size(16).color(fg()),
                numeric_row(
                    "Concurrent downloads",
                    &settings.max_concurrent_downloads.to_string(),
                    SettingChange::Downloads(1),
                    SettingChange::Downloads(-1),
                ),
                numeric_row(
                    "Concurrent writes",
                    &settings.max_concurrent_writes.to_string(),
                    SettingChange::Writes(1),
                    SettingChange::Writes(-1),
                ),
                numeric_row(
                    "Maximum memory",
                    &format!("{} MiB", settings.memory.maximum),
                    SettingChange::Memory(512),
                    SettingChange::Memory(-512),
                ),
            ]
            .spacing(8),
        )
        .padding(16)
        .style(card_style),
        container(
            column![
                text(t("Appearance")).size(16).color(fg()),
                text(t("Theme")).size(13).color(muted_c()),
                theme_grid(settings.theme),
                language_row(&settings.locale),
                toggle_row(
                    "Telemetry",
                    settings.telemetry,
                    SettingChange::ToggleTelemetry
                ),
                toggle_row(
                    "Discord presence",
                    settings.discord_rpc,
                    SettingChange::ToggleDiscordRpc
                ),
                toggle_row(
                    "Advanced rendering",
                    settings.advanced_rendering,
                    SettingChange::ToggleAdvancedRendering
                ),
                toggle_row(
                    "Native decorations",
                    settings.native_decorations,
                    SettingChange::ToggleNativeDecorations
                ),
                toggle_row(
                    "Force fullscreen",
                    settings.force_fullscreen,
                    SettingChange::ToggleForceFullscreen
                ),
                toggle_row(
                    "Hide on game start",
                    settings.hide_on_process_start,
                    SettingChange::ToggleHideOnProcessStart
                ),
                toggle_row(
                    "Toggle sidebar",
                    settings.toggle_sidebar,
                    SettingChange::ToggleSidebar
                ),
            ]
            .spacing(8),
        )
        .padding(16)
        .style(card_style),
        container(
            column![
                text(t("Paths")).size(16).color(fg()),
                setting_row(
                    "Data directory",
                    settings
                        .custom_dir
                        .as_deref()
                        .unwrap_or("Default Modrinth location")
                ),
                setting_row(
                    "Game resolution",
                    &format!(
                        "{} × {}",
                        settings.game_resolution.0, settings.game_resolution.1
                    )
                ),
            ]
            .spacing(8),
        )
        .padding(16)
        .style(card_style),
        container(
            column![
                text(t("About Ferriso Launcher")).size(16).color(fg()),
                text(format!("{} {}", t("Version"), view.version))
                    .size(10)
                    .color(muted_c()),
                text(t(
                    "Ferriso Launcher is an unofficial fork of Modrinth App. It uses Modrinth services for authentication, project metadata, and content delivery, but it is not affiliated with or endorsed by Modrinth."
                ))
                .size(10)
                .color(muted_c()),
                row![
                    button(
                        text(t("Modrinth source")).size(10).color(accent_c())
                    )
                    .padding([5, 9])
                    .on_press(Msg::User(Message::OpenUrl(
                        "https://github.com/Modrinth/code".into(),
                    )))
                    .style(ghost_style),
                    text(t("License: GPL-3.0-only"))
                        .size(10)
                        .color(muted_c()),
                ]
                .spacing(8)
                .align_y(Alignment::Center),
            ]
            .spacing(8),
        )
        .padding(16)
        .style(card_style),
    ]
    .spacing(10);

    scrollable(body).height(Length::Fill).into()
}

fn text_input_style(
    _theme: &Theme,
    status: text_input::Status,
) -> text_input::Style {
    let focused = matches!(status, text_input::Status::Focused { .. });
    text_input::Style {
        background: Background::Color(if focused {
            palette().input_focus
        } else {
            palette().input_bg
        }),
        border: Border {
            color: if focused { accent_c() } else { border_c() },
            width: 1.0,
            radius: 11.0.into(),
        },
        icon: muted_c(),
        placeholder: muted_c(),
        value: fg(),
        selection: with_alpha(accent_c(), 90),
    }
}

fn page_heading(
    eyebrow: &'static str,
    title: &'static str,
) -> Element<'static, Msg> {
    column![
        text(t(eyebrow)).size(9).color(accent_c()),
        text(t(title)).size(27).color(fg()),
    ]
    .spacing(2)
    .into()
}

fn danger_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(fg()),
        background: Some(
            Linear::new(Radians(-1.4))
                .add_stop(0.0, palette().danger_a)
                .add_stop(1.0, palette().danger_b)
                .into(),
        ),
        border: Border {
            color: with_alpha(danger_c(), 90),
            width: 1.0,
            radius: 14.0.into(),
        },
        ..Default::default()
    }
}

fn numeric_row(
    label: &str,
    value: &str,
    up: SettingChange,
    down: SettingChange,
) -> Element<'static, Msg> {
    row![
        text(t(label).to_owned())
            .size(13)
            .color(muted_c())
            .width(Length::Fill),
        button(text(t("−")).size(12).color(fg()))
            .padding([3, 8])
            .on_press(Msg::User(Message::SettingChanged(down)))
            .style(ghost_style),
        text(t(value).to_owned()).size(12).color(fg()),
        button(text(t("+")).size(12).color(fg()))
            .padding([3, 8])
            .on_press(Msg::User(Message::SettingChanged(up)))
            .style(ghost_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn toggle_row(
    label: &str,
    value: bool,
    change: SettingChange,
) -> Element<'static, Msg> {
    row![
        text(t(label).to_owned())
            .size(13)
            .color(muted_c())
            .width(Length::Fill),
        button(
            text(t(if value { "On" } else { "Off" }))
                .size(11)
                .color(if value { accent_c() } else { muted_c() })
        )
        .padding([5, 10])
        .on_press(Msg::User(Message::SettingChanged(change)))
        .style(ghost_style),
    ]
    .spacing(8)
    .align_y(Alignment::Center)
    .into()
}

fn jobs_card(view: &ViewModel) -> Element<'_, Msg> {
    let mut list = column![].spacing(6);
    for job in view.jobs.iter().take(5) {
        list = list.push(job_row(
            job,
            view.expanded_job.as_deref() == Some(job.id.as_str()),
        ));
    }
    if view.jobs.is_empty() {
        list = list
            .push(text(t("No install jobs yet.")).size(11).color(muted_c()));
    }

    container(
        column![
            row![
                text(t("Installs")).size(16).color(fg()),
                Space::new().width(Length::Fill),
                text(format!(
                    "{} {}",
                    view.jobs.iter().filter(|job| job.active).count(),
                    t("active jobs")
                ))
                .size(10)
                .color(muted_c()),
            ],
            list,
        ]
        .spacing(8),
    )
    .padding(14)
    .style(card_style)
    .into()
}

fn job_row(job: &JobView, expanded: bool) -> Element<'_, Msg> {
    let progress = job
        .progress
        .map(|value| format!("{:.0}%", value * 100.0))
        .unwrap_or_else(|| t("working").into());
    let mut details = column![
        text(&job.kind).size(12).color(fg()),
        text(format!("{} · {}", job.phase, job.status))
            .size(10)
            .color(muted_c()),
    ]
    .spacing(2)
    .width(Length::Fill);
    if let Some(value) = job.progress {
        details = details.push(
            progress_bar(0.0..=1.0, value)
                .length(Length::Fill)
                .girth(Length::Fixed(5.0))
                .style(progress_style),
        );
    }
    if let Some(error_c) = &job.error {
        details = details.push(text(error_c).size(10).color(danger_c()));
    }
    if expanded {
        details = details.push(
            text(format!("{}{}", t("Target: "), job.target))
                .size(9)
                .color(muted_c()),
        );
        if let Some(display) = &job.display {
            details = details.push(
                text(format!("{}{display}", t("Display: ")))
                    .size(9)
                    .color(muted_c()),
            );
        }
        details = details.push(
            text(format!(
                "{}{} · {}{}",
                t("Created: "),
                job.created,
                t("Finished: "),
                job.finished.as_deref().unwrap_or("—")
            ))
            .size(9)
            .color(muted_c()),
        );
        if let Some(error_c) = &job.rollback_error {
            details = details.push(
                text(format!("{}{error_c}", t("Rollback error: ")))
                    .size(9)
                    .color(danger_c()),
            );
        }
    }
    let mut actions = row![text(progress).size(11).color(warning_c())]
        .spacing(4)
        .align_y(Alignment::Center);
    actions = actions.push(
        button(
            text(if expanded { "Hide" } else { "Details" })
                .size(9)
                .color(muted_c()),
        )
        .padding([3, 6])
        .on_press(Msg::User(Message::ToggleJobDetails(job.id.clone())))
        .style(ghost_style),
    );
    if job.can_cancel {
        actions = actions.push(
            button(text(t("Cancel")).size(9).color(danger_c()))
                .padding([3, 6])
                .on_press(Msg::User(Message::CancelJob(job.id.clone())))
                .style(ghost_style),
        );
    }
    if job.can_retry {
        actions = actions.push(
            button(text(t("Retry")).size(9).color(warning_c()))
                .padding([3, 6])
                .on_press(Msg::User(Message::RetryJob(job.id.clone())))
                .style(ghost_style),
        );
    }
    if !job.active {
        actions = actions.push(
            button(text(t("Dismiss")).size(9).color(muted_c()))
                .padding([3, 6])
                .on_press(Msg::User(Message::DismissJob(job.id.clone())))
                .style(ghost_style),
        );
    }
    container(row![details, actions].spacing(8).align_y(Alignment::Center))
        .padding(8)
        .style(card_style)
        .into()
}

fn setting_row(label: &str, value: &str) -> Element<'static, Msg> {
    row![
        text(t(label).to_owned()).size(13).color(muted_c()),
        Space::new().width(Length::Fill),
        text(t(value).to_owned()).size(13).color(fg()),
    ]
    .spacing(12)
    .align_y(Alignment::Center)
    .into()
}

fn ambient_background(phase: f32) -> Background {
    let wave = (phase * 0.5).sin();
    Linear::new(Radians(-1.15 + wave * 0.55))
        .add_stop(0.0, bg_top())
        .add_stop(0.42, bg_mid())
        .add_stop(0.78, blend(bg_mid(), accent_c(), 0.06 + wave * 0.02))
        .add_stop(1.0, bg_bottom())
        .into()
}

fn pulse_alpha(phase: f32, floor: f32) -> u8 {
    let wave = (phase * 0.9).sin() * 0.5 + 0.5;
    (floor + wave * (255.0 - floor)).clamp(0.0, 255.0) as u8
}

fn glass_panel_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(fg()),
        background: Some(
            Linear::new(Radians(-1.2))
                .add_stop(0.0, palette().panel_a)
                .add_stop(1.0, palette().panel_b)
                .into(),
        ),
        border: Border {
            color: border_c(),
            width: 1.0,
            radius: 22.0.into(),
        },
        shadow: Shadow {
            color: shadow_c(),
            offset: Vector::new(0.0, 18.0),
            blur_radius: 44.0,
        },
        ..Default::default()
    }
}

fn surface_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(fg()),
        background: Some(
            Linear::new(Radians(-1.3))
                .add_stop(0.0, palette().surface_a)
                .add_stop(1.0, palette().surface_b)
                .into(),
        ),
        border: Border {
            color: border_c(),
            width: 1.0,
            radius: 20.0.into(),
        },
        shadow: Shadow {
            color: shadow_c(),
            offset: Vector::new(0.0, 12.0),
            blur_radius: 32.0,
        },
        ..Default::default()
    }
}

fn card_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(fg()),
        background: Some(
            Linear::new(Radians(-1.4))
                .add_stop(0.0, palette().card_a)
                .add_stop(1.0, palette().card_b)
                .into(),
        ),
        border: Border {
            color: border_c(),
            width: 1.0,
            radius: 16.0.into(),
        },
        shadow: Shadow {
            color: rgba(0, 0, 0, 65),
            offset: Vector::new(0.0, 8.0),
            blur_radius: 22.0,
        },
        ..Default::default()
    }
}

fn hero_style(phase: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme| {
        let wave = (phase * 0.5).sin();
        container::Style {
            text_color: Some(fg()),
            background: Some(
                Linear::new(Radians(-1.0 + wave * 0.35))
                    .add_stop(0.0, palette().hero_a)
                    .add_stop(0.45, palette().hero_b)
                    .add_stop(1.0, palette().hero_c)
                    .into(),
            ),
            border: Border {
                color: with_alpha(accent_c(), (58.0 + wave * 24.0) as u8),
                width: 1.0,
                radius: 22.0.into(),
            },
            shadow: Shadow {
                color: rgba(0, 0, 0, 100),
                offset: Vector::new(0.0, 16.0),
                blur_radius: 38.0,
            },
            ..Default::default()
        }
    }
}

fn stat_style(_theme: &Theme) -> container::Style {
    container::Style {
        text_color: Some(fg()),
        background: Some(
            Linear::new(Radians(-1.5))
                .add_stop(0.0, palette().stat_a)
                .add_stop(1.0, palette().stat_b)
                .into(),
        ),
        border: Border {
            color: border_c(),
            width: 1.0,
            radius: 16.0.into(),
        },
        ..Default::default()
    }
}

fn logo_style(phase: f32) -> impl Fn(&Theme) -> container::Style {
    move |_theme| container::Style {
        background: Some(
            Linear::new(Radians(-0.8 + (phase * 0.4).sin() * 0.2))
                .add_stop(0.0, accent_c())
                .add_stop(1.0, accent_cyan_c())
                .into(),
        ),
        border: Border {
            radius: 12.0.into(),
            ..Default::default()
        },
        shadow: Shadow {
            color: with_alpha(accent_c(), pulse_alpha(phase, 40.0) / 2),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 22.0,
        },
        text_color: Some(ink_c()),
        ..Default::default()
    }
}

fn accent_line_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(
            Linear::new(Radians(1.57))
                .add_stop(0.0, accent_c())
                .add_stop(1.0, accent_cyan_c())
                .into(),
        ),
        border: Border {
            radius: 4.0.into(),
            ..Default::default()
        },
        ..Default::default()
    }
}

fn glass_chip_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(overlay(14))),
        border: Border {
            color: border_c(),
            width: 1.0,
            radius: 99.0.into(),
        },
        text_color: Some(muted_c()),
        ..Default::default()
    }
}

fn accent_chip_style(_theme: &Theme) -> container::Style {
    container::Style {
        background: Some(Background::Color(accent_c())),
        border: Border {
            radius: 99.0.into(),
            ..Default::default()
        },
        text_color: Some(ink_c()),
        ..Default::default()
    }
}

fn nav_style(
    selected: bool,
    phase: f32,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered =
            matches!(status, button::Status::Hovered | button::Status::Pressed);
        let glow = pulse_alpha(phase, 170.0);
        button::Style {
            background: Some(if selected {
                Linear::new(Radians(-1.4))
                    .add_stop(0.0, with_alpha(accent_c(), 58))
                    .add_stop(1.0, with_alpha(accent_c(), 14))
                    .into()
            } else if hovered {
                Background::Color(overlay(14))
            } else {
                Background::Color(Color::TRANSPARENT)
            }),
            text_color: if selected { fg() } else { muted_c() },
            border: Border {
                color: if selected {
                    with_alpha(accent_c(), glow / 4)
                } else {
                    Color::TRANSPARENT
                },
                width: 1.0,
                radius: 11.0.into(),
            },
            shadow: if selected {
                Shadow {
                    color: with_alpha(accent_c(), glow / 8),
                    offset: Vector::new(0.0, 4.0),
                    blur_radius: 16.0,
                }
            } else {
                Shadow::default()
            },
            ..Default::default()
        }
    }
}

fn row_style(
    selected: bool,
) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |_theme, status| {
        let hovered =
            matches!(status, button::Status::Hovered | button::Status::Pressed);
        button::Style {
            background: Some(if selected || hovered {
                Linear::new(Radians(-1.4))
                    .add_stop(0.0, palette().row_selected_a)
                    .add_stop(1.0, palette().row_selected_b)
                    .into()
            } else {
                Linear::new(Radians(-1.4))
                    .add_stop(0.0, palette().card_a)
                    .add_stop(1.0, palette().card_b)
                    .into()
            }),
            text_color: fg(),
            border: Border {
                color: if selected { accent_c() } else { border_c() },
                width: 1.0,
                radius: 14.0.into(),
            },
            shadow: Shadow {
                color: rgba(0, 0, 0, if hovered { 80 } else { 45 }),
                offset: Vector::new(0.0, 6.0),
                blur_radius: 18.0,
            },
            ..Default::default()
        }
    }
}

fn ghost_style(_theme: &Theme, status: button::Status) -> button::Style {
    let hovered =
        matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if hovered {
            overlay(18)
        } else {
            overlay(7)
        })),
        text_color: muted_c(),
        border: Border {
            color: if hovered {
                border_strong_c()
            } else {
                border_c()
            },
            width: 1.0,
            radius: 10.0.into(),
        },
        ..Default::default()
    }
}

fn primary_style(_theme: &Theme, status: button::Status) -> button::Style {
    let hovered =
        matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(
            Linear::new(Radians(-1.3))
                .add_stop(
                    0.0,
                    if hovered { accent_hover() } else { accent_c() },
                )
                .add_stop(1.0, accent_cyan_c())
                .into(),
        ),
        text_color: ink_c(),
        border: Border {
            color: overlay(60),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: with_alpha(accent_c(), if hovered { 90 } else { 55 }),
            offset: Vector::new(0.0, 4.0),
            blur_radius: 18.0,
        },
        ..Default::default()
    }
}

fn danger_button_style(
    _theme: &Theme,
    status: button::Status,
) -> button::Style {
    let hovered =
        matches!(status, button::Status::Hovered | button::Status::Pressed);
    button::Style {
        background: Some(Background::Color(if hovered {
            error_hover()
        } else {
            danger_c()
        })),
        text_color: ink_c(),
        border: Border {
            color: overlay(50),
            width: 1.0,
            radius: 10.0.into(),
        },
        shadow: Shadow {
            color: with_alpha(danger_c(), if hovered { 80 } else { 45 }),
            offset: Vector::new(0.0, 3.0),
            blur_radius: 14.0,
        },
        ..Default::default()
    }
}

fn progress_style(_theme: &Theme) -> progress_bar::Style {
    progress_bar::Style {
        background: Background::Color(overlay(16)),
        bar: Linear::new(Radians(1.57))
            .add_stop(0.0, accent_c())
            .add_stop(1.0, accent_cyan_c())
            .into(),
        border: Border {
            radius: 99.0.into(),
            ..Default::default()
        },
    }
}

fn instance_avatar<'a>(
    instance: &'a InstanceView,
    size: f32,
) -> Element<'a, Msg> {
    if let Some(path) = &instance.icon_path
        && path.is_file()
    {
        return image(image::Handle::from_path(path.clone()))
            .width(size)
            .height(size)
            .border_radius(size * 0.27)
            .content_fit(ContentFit::Cover)
            .into();
    }
    avatar(&instance.name, size)
}

fn project_art<'a>(project: &'a ProjectView, size: f32) -> Element<'a, Msg> {
    project
        .icon
        .as_ref()
        .and_then(|bytes| {
            bytes_handle(
                format!("project:{}:{}", project.id, bytes.len()),
                bytes,
            )
        })
        .map(|handle| image_art(handle, size))
        .unwrap_or_else(|| avatar(&project.title, size))
}

struct ImageCache {
    entries: std::collections::HashMap<String, image::Handle>,
    order: std::collections::VecDeque<String>,
    capacity: usize,
}

impl ImageCache {
    fn new(capacity: usize) -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            order: std::collections::VecDeque::new(),
            capacity,
        }
    }

    fn get(&mut self, key: &str) -> Option<image::Handle> {
        self.entries.get(key).cloned()
    }

    fn insert(&mut self, key: String, handle: image::Handle) {
        if self.entries.insert(key.clone(), handle).is_none() {
            self.order.push_back(key);
        }
        while self.order.len() > self.capacity {
            if let Some(oldest) = self.order.pop_front() {
                self.entries.remove(&oldest);
            }
        }
    }
}

fn rgba_handle(
    key: String,
    width: u32,
    height: u32,
    bytes: &Bytes,
) -> Option<image::Handle> {
    use std::sync::{Mutex, OnceLock};

    // 48 frames of 340x420 RGBA is about 28 MiB of GPU textures. Dragging
    // beyond that evicts the oldest frame instead of clearing the whole cache.
    static CACHE: OnceLock<Mutex<ImageCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(ImageCache::new(48)));
    let mut cache = cache.lock().ok()?;
    if let Some(handle) = cache.get(&key) {
        return Some(handle);
    }
    let handle = image::Handle::from_rgba(width, height, bytes.clone());
    cache.insert(key, handle.clone());
    Some(handle)
}

fn bytes_handle(key: String, bytes: &Bytes) -> Option<image::Handle> {
    use std::sync::{Mutex, OnceLock};

    static CACHE: OnceLock<Mutex<ImageCache>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(ImageCache::new(128)));
    let mut cache = cache.lock().ok()?;
    if let Some(handle) = cache.get(&key) {
        return Some(handle);
    }
    let handle = image::Handle::from_bytes(bytes.clone());
    cache.insert(key, handle.clone());
    Some(handle)
}

fn player_head(
    bytes: Option<&Bytes>,
    name: &str,
    size: f32,
) -> Element<'static, Msg> {
    let Some(bytes) = bytes else {
        return avatar(name, size);
    };
    let Some(handle) = bytes_handle(bytes_key("player-head", bytes), bytes)
    else {
        return avatar(name, size);
    };

    container(
        image(handle)
            .width(size)
            .height(size)
            .filter_method(image::FilterMethod::Nearest)
            .content_fit(ContentFit::Contain),
    )
    .width(Length::Fixed(size))
    .height(Length::Fixed(size))
    .style(move |_| container::Style {
        border: Border {
            color: overlay(45),
            width: 1.0,
            radius: (size * 0.22).into(),
        },
        ..Default::default()
    })
    .into()
}

fn avatar(name: &str, size: f32) -> Element<'static, Msg> {
    let initial = name
        .chars()
        .find(|character| !character.is_whitespace())
        .unwrap_or('?')
        .to_uppercase()
        .to_string();
    let color = color_for(name);
    container(text(initial).size(size * 0.38).color(fg()))
        .center(Length::Fixed(size))
        .style(move |_| container::Style {
            background: Some(
                Linear::new(Radians(-1.3))
                    .add_stop(0.0, color)
                    .add_stop(1.0, bg_bottom())
                    .into(),
            ),
            border: Border {
                color: overlay(35),
                width: 1.0,
                radius: (size * 0.27).into(),
            },
            shadow: Shadow {
                color: rgba(0, 0, 0, 80),
                offset: Vector::new(0.0, 4.0),
                blur_radius: 12.0,
            },
            ..Default::default()
        })
        .into()
}

fn color_for(seed: &str) -> Color {
    const PALETTE: [(u8, u8, u8); 6] = [
        (28, 180, 125),
        (58, 170, 210),
        (96, 130, 225),
        (160, 105, 220),
        (220, 125, 85),
        (190, 160, 60),
    ];
    let index = seed.bytes().fold(0usize, |hash, byte| {
        hash.wrapping_mul(31).wrapping_add(byte.into())
    }) % PALETTE.len();
    let (red, green, blue) = PALETTE[index];
    Color::from_rgb8(red, green, blue)
}
