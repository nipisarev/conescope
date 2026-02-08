use conescope_core::settings::SettingsJson;
use gpui::prelude::*;
use gpui::{Entity, MouseButton, div, px};

use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;
use crate::theme::{Theme, ThemeMode};
use crate::views::text_input::TextInput;

pub struct SettingsView {
    app_state: Entity<AppState>,
    font_family_input: Entity<TextInput>,
    editor_font_size_input: Entity<TextInput>,
    terminal_font_size_input: Entity<TextInput>,
}

impl std::fmt::Debug for SettingsView {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SettingsView").finish_non_exhaustive()
    }
}

impl SettingsView {
    #[must_use]
    pub fn new(app_state: Entity<AppState>, cx: &mut gpui::Context<Self>) -> Self {
        let settings = app_state
            .read(cx)
            .settings_store
            .read(cx)
            .settings()
            .clone();

        let font_family_input = cx.new(|cx| TextInput::new(&settings.font_family, cx));
        let editor_font_size_input =
            cx.new(|cx| TextInput::new(&settings.editor_font_size.to_string(), cx));
        let terminal_font_size_input =
            cx.new(|cx| TextInput::new(&settings.terminal_font_size.to_string(), cx));

        // Save on Enter in any input
        cx.subscribe(&font_family_input, |this: &mut Self, _, _, cx| {
            this.save_all(cx);
        })
        .detach();
        cx.subscribe(&editor_font_size_input, |this: &mut Self, _, _, cx| {
            this.save_all(cx);
        })
        .detach();
        cx.subscribe(&terminal_font_size_input, |this: &mut Self, _, _, cx| {
            this.save_all(cx);
        })
        .detach();

        // Auto-save when leaving settings mode (view mode changes away from Settings).
        cx.observe(&app_state, |this, app_state, cx| {
            let mode = app_state.read(cx).view_mode(cx);
            if mode != ViewMode::Settings {
                this.save_all(cx);
            }
        })
        .detach();

        Self {
            app_state,
            font_family_input,
            editor_font_size_input,
            terminal_font_size_input,
        }
    }

    /// Reload inputs from current settings (replaces old `reload_file`).
    pub fn reload_settings(&mut self, cx: &mut gpui::Context<Self>) {
        let settings = self
            .app_state
            .read(cx)
            .settings_store
            .read(cx)
            .settings()
            .clone();

        self.font_family_input = cx.new(|cx| TextInput::new(&settings.font_family, cx));
        self.editor_font_size_input =
            cx.new(|cx| TextInput::new(&settings.editor_font_size.to_string(), cx));
        self.terminal_font_size_input =
            cx.new(|cx| TextInput::new(&settings.terminal_font_size.to_string(), cx));

        cx.subscribe(&self.font_family_input, |this: &mut Self, _, _, cx| {
            this.save_all(cx);
        })
        .detach();
        cx.subscribe(&self.editor_font_size_input, |this: &mut Self, _, _, cx| {
            this.save_all(cx);
        })
        .detach();
        cx.subscribe(
            &self.terminal_font_size_input,
            |this: &mut Self, _, _, cx| {
                this.save_all(cx);
            },
        )
        .detach();

        cx.notify();
    }

    /// Persist all current input values to settings file and in-memory store.
    fn save_all(&mut self, cx: &mut gpui::Context<Self>) {
        let font_family = self.font_family_input.read(cx).content().to_owned();
        let editor_font_size = self
            .editor_font_size_input
            .read(cx)
            .content()
            .parse::<i64>()
            .unwrap_or(13);
        let terminal_font_size = self
            .terminal_font_size_input
            .read(cx)
            .content()
            .parse::<i64>()
            .unwrap_or(13);

        // Read current theme from settings_store (theme is applied live via toggle)
        let theme = self
            .app_state
            .read(cx)
            .settings_store
            .read(cx)
            .settings()
            .theme
            .clone();

        let new_settings = SettingsJson {
            theme,
            font_family,
            editor_font_size,
            terminal_font_size,
            ..Default::default()
        };

        let dir = SettingsJson::settings_dir();
        let _ = new_settings.save_to_file(&dir);

        let store = self.app_state.read(cx).settings_store.clone();
        store.update(cx, |s, _| s.load_settings(new_settings));
    }
}

fn render_section_header(title: &str, theme: &Theme) -> gpui::Div {
    div()
        .w_full()
        .mt(px(24.))
        .mb(px(12.))
        .pb(px(6.))
        .border_b_1()
        .border_color(theme.border)
        .text_size(px(11.))
        .text_color(theme.text_muted)
        .child(title.to_uppercase())
}

fn render_setting_row(
    label: &str,
    description: &str,
    input: Entity<TextInput>,
    theme: &Theme,
) -> gpui::Div {
    div()
        .w_full()
        .mb(px(16.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(theme.text)
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(theme.text_muted)
                .child(description.to_owned()),
        )
        .child(
            div()
                .mt(px(4.))
                .max_w(px(240.))
                .rounded(px(4.))
                .border_1()
                .border_color(theme.border)
                .bg(theme.editor_bg)
                .px(px(8.))
                .py(px(4.))
                .child(input),
        )
}

fn render_theme_toggle(
    current_mode: ThemeMode,
    app_state: &Entity<AppState>,
    theme: &Theme,
) -> gpui::Div {
    let app_state_dark = app_state.clone();
    let app_state_light = app_state.clone();

    let dark_active = current_mode == ThemeMode::Dark;
    let light_active = current_mode == ThemeMode::Light;

    let accent = theme.accent;
    let border = theme.border;
    let surface = theme.surface;
    let text = theme.text;
    let text_muted = theme.text_muted;

    div()
        .w_full()
        .mb(px(16.))
        .flex()
        .flex_col()
        .gap(px(4.))
        .child(
            div()
                .text_size(px(13.))
                .font_weight(gpui::FontWeight::SEMIBOLD)
                .text_color(text)
                .child("Theme"),
        )
        .child(
            div()
                .text_size(px(11.))
                .text_color(text_muted)
                .child("Choose between dark and light appearance"),
        )
        .child(
            div()
                .mt(px(4.))
                .flex()
                .flex_row()
                .gap(px(0.))
                .child(
                    // Dark button
                    div()
                        .px(px(16.))
                        .py(px(5.))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .rounded_l(px(4.))
                        .border_1()
                        .border_color(if dark_active { accent } else { border })
                        .bg(if dark_active { accent } else { surface })
                        .text_color(if dark_active {
                            gpui::rgba(0xffff_ffff)
                        } else {
                            text_muted
                        })
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            app_state_dark.update(cx, |s, cx| s.set_theme(ThemeMode::Dark, cx));
                        })
                        .child("Dark"),
                )
                .child(
                    // Light button
                    div()
                        .px(px(16.))
                        .py(px(5.))
                        .text_size(px(12.))
                        .cursor_pointer()
                        .rounded_r(px(4.))
                        .border_1()
                        .border_color(if light_active { accent } else { border })
                        .bg(if light_active { accent } else { surface })
                        .text_color(if light_active {
                            gpui::rgba(0xffff_ffff)
                        } else {
                            text_muted
                        })
                        .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                            app_state_light.update(cx, |s, cx| s.set_theme(ThemeMode::Light, cx));
                        })
                        .child("Light"),
                ),
        )
}

impl Render for SettingsView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let theme = self.app_state.read(cx).theme().clone();
        let current_mode = theme.mode;

        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(theme.background)
            .child(
                div()
                    .id("settings-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(
                        div()
                            .w_full()
                            .flex()
                            .justify_center()
                            .child(
                                div()
                                    .max_w(px(600.))
                                    .w_full()
                                    .py(px(24.))
                                    .px(px(32.))
                                    // Appearance section
                                    .child(render_section_header("Appearance", &theme))
                                    .child(render_theme_toggle(
                                        current_mode,
                                        &self.app_state,
                                        &theme,
                                    ))
                                    // Editor section
                                    .child(render_section_header("Editor", &theme))
                                    .child(render_setting_row(
                                        "Font Family",
                                        "Font used in terminals and editor. Requires restart for existing terminals.",
                                        self.font_family_input.clone(),
                                        &theme,
                                    ))
                                    .child(render_setting_row(
                                        "Editor Font Size",
                                        "Font size for the code editor in pixels",
                                        self.editor_font_size_input.clone(),
                                        &theme,
                                    ))
                                    .child(render_setting_row(
                                        "Terminal Font Size",
                                        "Font size for terminal panes in pixels",
                                        self.terminal_font_size_input.clone(),
                                        &theme,
                                    )),
                            ),
                    ),
            )
    }
}
