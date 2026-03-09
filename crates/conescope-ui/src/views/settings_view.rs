use conescope_core::settings::SettingsJson;
use gpui::prelude::*;
use gpui::{Entity, MouseButton, deferred, div, px};

use crate::state::app_state::AppState;
use crate::state::settings_store::ViewMode;
use crate::theme::{Theme, ThemeMode};
use crate::views::text_input::TextInput;

pub struct SettingsView {
    app_state: Entity<AppState>,
    font_family_input: Entity<TextInput>,
    editor_font_size_input: Entity<TextInput>,
    terminal_font_size_input: Entity<TextInput>,
    terminal_line_height_input: Entity<TextInput>,
    theme_dropdown_open: bool,
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
        let terminal_line_height_input =
            cx.new(|cx| TextInput::new(&settings.terminal_line_height.to_string(), cx));

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
        cx.subscribe(&terminal_line_height_input, |this: &mut Self, _, _, cx| {
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
            terminal_line_height_input,
            theme_dropdown_open: false,
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
        self.terminal_line_height_input =
            cx.new(|cx| TextInput::new(&settings.terminal_line_height.to_string(), cx));

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
        cx.subscribe(
            &self.terminal_line_height_input,
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
        let terminal_line_height = self
            .terminal_line_height_input
            .read(cx)
            .content()
            .parse::<f64>()
            .unwrap_or(1.2);

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
            terminal_line_height,
            ..Default::default()
        };

        let dir = SettingsJson::settings_dir();
        let _ = new_settings.save_to_file(&dir);

        let store = self.app_state.read(cx).settings_store.clone();
        store.update(cx, |s, cx| {
            s.load_settings(new_settings);
            cx.notify();
        });
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

struct ThemeDropdownCtx {
    text: gpui::Rgba,
    text_muted: gpui::Rgba,
    accent: gpui::Rgba,
    element_hover: gpui::Rgba,
    active_name: String,
    app_state: Entity<AppState>,
}

fn render_theme_item(
    name: &str,
    dctx: &ThemeDropdownCtx,
    cx: &gpui::Context<SettingsView>,
) -> gpui::Stateful<gpui::Div> {
    let is_active = name == dctx.active_name;
    let app_click = dctx.app_state.clone();
    let app_hover = dctx.app_state.clone();
    let name_click = name.to_owned();
    let name_hover = name.to_owned();
    let element_hover = dctx.element_hover;

    div()
        .id(gpui::SharedString::from(format!("theme-item-{name}")))
        .px(px(10.))
        .py(px(4.))
        .text_size(px(12.))
        .cursor_pointer()
        .text_color(if is_active { dctx.accent } else { dctx.text })
        .font_weight(if is_active {
            gpui::FontWeight::SEMIBOLD
        } else {
            gpui::FontWeight::NORMAL
        })
        .hover(move |s| s.bg(element_hover))
        .on_mouse_move(cx.listener(move |_this, _, _, cx| {
            app_hover.update(cx, |s, cx| s.preview_theme(Some(&name_hover), cx));
        }))
        .on_click(cx.listener(move |this, _, _, cx| {
            app_click.update(cx, |s, cx| s.set_theme_by_name(&name_click, cx));
            this.theme_dropdown_open = false;
        }))
        .child(name.to_owned())
}

fn render_theme_section(
    label: &str,
    entries: &[crate::theme::ThemeEntry],
    dctx: &ThemeDropdownCtx,
    cx: &gpui::Context<SettingsView>,
) -> gpui::Div {
    let mut section = div().flex().flex_col().child(
        div()
            .px(px(10.))
            .py(px(4.))
            .text_size(px(10.))
            .text_color(dctx.text_muted)
            .child(label.to_owned()),
    );
    for entry in entries {
        section = section.child(render_theme_item(&entry.name, dctx, cx));
    }
    section
}

fn render_theme_overlay(
    dctx: &ThemeDropdownCtx,
    theme: &Theme,
    cx: &gpui::Context<SettingsView>,
) -> (gpui::Stateful<gpui::Div>, gpui::Stateful<gpui::Div>) {
    let entries = dctx.app_state.read(cx).theme_registry().list();
    let dark: Vec<_> = entries
        .iter()
        .filter(|e| e.mode == ThemeMode::Dark)
        .cloned()
        .collect();
    let light: Vec<_> = entries
        .iter()
        .filter(|e| e.mode == ThemeMode::Light)
        .cloned()
        .collect();

    let mut list = div().flex().flex_col();
    if !dark.is_empty() {
        list = list.child(render_theme_section("DARK", &dark, dctx, cx));
    }
    if !light.is_empty() {
        list = list.child(render_theme_section("LIGHT", &light, dctx, cx).mt(px(4.)));
    }

    let app_revert = dctx.app_state.clone();
    let backdrop = div()
        .id("theme-dropdown-backdrop")
        .absolute()
        .top(px(-1000.))
        .left(px(-1000.))
        .w(px(5000.))
        .h(px(5000.))
        .on_mouse_down(
            MouseButton::Left,
            cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                app_revert.update(cx, |s, cx| s.preview_theme(None, cx));
                this.theme_dropdown_open = false;
            }),
        );

    let popup = div()
        .id("theme-dropdown-popup")
        .absolute()
        .left_0()
        .top(px(34.))
        .w(px(240.))
        .max_h(px(300.))
        .overflow_y_scroll()
        .rounded(px(6.))
        .border_1()
        .border_color(theme.border)
        .bg(theme.surface)
        .shadow_lg()
        .py(px(4.))
        .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
        .on_mouse_move(|_, _, cx| cx.stop_propagation())
        .child(list);

    (backdrop, popup)
}

impl SettingsView {
    fn render_theme_selector(&self, theme: &Theme, cx: &gpui::Context<Self>) -> gpui::Div {
        let current_name = theme.name.clone();
        let dropdown_open = self.theme_dropdown_open;

        let mut trigger_wrapper = div().mt(px(4.)).relative().child(
            div()
                .id("theme-dropdown-trigger")
                .max_w(px(240.))
                .px(px(10.))
                .py(px(5.))
                .text_size(px(12.))
                .cursor_pointer()
                .rounded(px(4.))
                .border_1()
                .border_color(if dropdown_open {
                    theme.accent
                } else {
                    theme.border
                })
                .bg(theme.surface)
                .text_color(theme.text)
                .flex()
                .flex_row()
                .items_center()
                .justify_between()
                .child(current_name)
                .child(div().text_size(px(10.)).text_color(theme.text_muted).child(
                    if dropdown_open {
                        "\u{25B2}"
                    } else {
                        "\u{25BC}"
                    },
                ))
                .on_click(cx.listener(|this, _, _, _cx| {
                    this.theme_dropdown_open = !this.theme_dropdown_open;
                })),
        );

        if dropdown_open {
            let dctx = ThemeDropdownCtx {
                text: theme.text,
                text_muted: theme.text_muted,
                accent: theme.accent,
                element_hover: theme.element_hover,
                active_name: self.app_state.read(cx).theme().name.clone(),
                app_state: self.app_state.clone(),
            };
            let (backdrop, popup) = render_theme_overlay(&dctx, theme, cx);
            // deferred() delays painting until after all ancestors, so these
            // render on top of sibling settings rows below the trigger.
            trigger_wrapper = trigger_wrapper
                .child(deferred(backdrop).with_priority(1))
                .child(deferred(popup).with_priority(2));
        }

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
                    .child("Theme"),
            )
            .child(
                div()
                    .text_size(px(11.))
                    .text_color(theme.text_muted)
                    .child("Select a color theme for the application"),
            )
            .child(trigger_wrapper)
    }
}

impl Render for SettingsView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        let theme = self.app_state.read(cx).theme().clone();

        let theme_selector = self.render_theme_selector(&theme, cx);

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .bg(theme.background)
            .on_key_down(cx.listener(|this, event: &gpui::KeyDownEvent, _window, cx| {
                if event.keystroke.key == "escape" {
                    if this.theme_dropdown_open {
                        this.app_state
                            .update(cx, |s, cx| s.preview_theme(None, cx));
                        this.theme_dropdown_open = false;
                    } else {
                        this.app_state.update(cx, AppState::close_settings);
                    }
                }
            }))
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
                                    .child(theme_selector)
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
                                    ))
                                    .child(render_setting_row(
                                        "Terminal Line Height",
                                        "Line height multiplier for terminals (e.g. 1.0, 1.2, 1.5)",
                                        self.terminal_line_height_input.clone(),
                                        &theme,
                                    )),
                            ),
                    ),
            )
    }
}
