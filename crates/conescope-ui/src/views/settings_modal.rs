use gpui::prelude::*;
use gpui::{Entity, MouseButton, SharedString, div, px, rgba};

use crate::state::app_state::AppState;
use crate::views::text_input::{TextInput, TextInputEvent};

const MIN_FONT_SIZE: i64 = 8;
const MAX_FONT_SIZE: i64 = 32;

#[derive(Debug)]
pub struct SettingsModal {
    app_state: Entity<AppState>,
    /// Cached sorted list of system font names (populated on first render).
    font_names: Vec<String>,
    /// Whether the font family dropdown is open.
    dropdown_open: bool,
    /// Filter text input for the font dropdown.
    filter_input: Option<Entity<TextInput>>,
    /// Current filter string (kept in sync with `filter_input`).
    filter_text: String,
    /// Font family at modal creation time (to detect changes).
    initial_font_family: String,
}

impl SettingsModal {
    #[must_use]
    pub fn new(app_state: Entity<AppState>) -> Self {
        Self {
            app_state,
            font_names: Vec::new(),
            dropdown_open: false,
            filter_input: None,
            filter_text: String::new(),
            initial_font_family: String::new(),
        }
    }

    /// Populate `self.font_names` once from the text system (sorted alphabetically).
    fn ensure_font_names(&mut self, window: &gpui::Window) {
        if !self.font_names.is_empty() {
            return;
        }
        let mut all = window.text_system().all_font_names();
        all.sort_by_key(|a| a.to_lowercase());
        all.dedup();
        self.font_names = all;
    }

    fn toggle_dropdown(&mut self, window: &mut gpui::Window, cx: &mut gpui::Context<Self>) {
        self.dropdown_open = !self.dropdown_open;
        if self.dropdown_open {
            self.filter_text.clear();
            // Create a fresh filter input each time
            let input = cx.new(|cx| TextInput::new("", cx));
            cx.subscribe(&input, |this, _input, event, _cx| match event {
                TextInputEvent::Cancel => {
                    this.dropdown_open = false;
                    this.filter_text.clear();
                }
                TextInputEvent::Submit(_) => {}
            })
            .detach();
            cx.observe(&input, |this, input, cx| {
                input.read(cx).content().clone_into(&mut this.filter_text);
            })
            .detach();
            input.read(cx).focus_handle.clone().focus(window, cx);
            self.filter_input = Some(input);
        }
        cx.notify();
    }
}

fn read_font_size(key: &str, app_state: &Entity<AppState>, cx: &gpui::App) -> i64 {
    let state = app_state.read(cx);
    let store = state.settings_store.read(cx);
    store.settings().get_i64(key).unwrap_or(13)
}

fn set_font_size(key: &str, value: i64, app_state: &Entity<AppState>, cx: &mut gpui::App) {
    let clamped = value.clamp(MIN_FONT_SIZE, MAX_FONT_SIZE);
    let store = app_state.read(cx).settings_store.clone();
    store.update(cx, |s, _| s.set(key, &clamped.to_string()));
}

fn font_size_row(
    label: &str,
    setting_key: &'static str,
    app_state: &Entity<AppState>,
    cx: &gpui::App,
) -> gpui::Div {
    let current = read_font_size(setting_key, app_state, cx);

    let app_dec = app_state.clone();
    let app_inc = app_state.clone();

    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .py(px(8.))
        .child(
            div()
                .text_color(rgba(0xcccc_ccff))
                .text_size(px(13.))
                .child(label.to_owned()),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(8.))
                // Decrement button
                .child(stepper_button(
                    "\u{2212}",
                    setting_key,
                    move |_, _, cx| {
                        set_font_size(setting_key, current - 1, &app_dec, cx);
                    },
                ))
                // Current value
                .child(
                    div()
                        .w(px(32.))
                        .text_color(rgba(0xdddd_ddff))
                        .text_size(px(13.))
                        .flex()
                        .justify_center()
                        .child(current.to_string()),
                )
                // Increment button
                .child(stepper_button("+", setting_key, move |_, _, cx| {
                    set_font_size(setting_key, current + 1, &app_inc, cx);
                })),
        )
}

fn stepper_button(
    label: &str,
    key: &str,
    on_click: impl Fn(&gpui::ClickEvent, &mut gpui::Window, &mut gpui::App) + 'static,
) -> gpui::Stateful<gpui::Div> {
    div()
        .id(gpui::SharedString::from(format!("stepper-{key}-{label}")))
        .w(px(28.))
        .h(px(28.))
        .rounded(px(4.))
        .cursor_pointer()
        .bg(rgba(0x3c3c_3cff))
        .hover(|s| s.bg(rgba(0x5555_55ff)))
        .active(|s| s.bg(rgba(0x2a2a_2aff)))
        .flex()
        .items_center()
        .justify_center()
        .text_color(rgba(0xdddd_ddff))
        .text_size(px(14.))
        .child(label.to_owned())
        .on_click(on_click)
}

fn setting_row_label(label: &str, value: &str) -> gpui::Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .py(px(8.))
        .child(
            div()
                .text_color(rgba(0xcccc_ccff))
                .text_size(px(13.))
                .child(label.to_owned()),
        )
        .child(
            div()
                .text_color(rgba(0x8888_88ff))
                .text_size(px(13.))
                .child(value.to_owned()),
        )
}

/// Render the font family dropdown trigger button.
fn font_family_trigger(current: &str) -> gpui::Div {
    div()
        .flex()
        .flex_row()
        .items_center()
        .justify_between()
        .py(px(8.))
        .cursor_pointer()
        .child(
            div()
                .text_color(rgba(0xcccc_ccff))
                .text_size(px(13.))
                .child("Font family"),
        )
        .child(
            div()
                .flex()
                .flex_row()
                .items_center()
                .gap(px(6.))
                .px(px(8.))
                .py(px(4.))
                .rounded(px(4.))
                .bg(rgba(0x3c3c_3cff))
                .hover(|s| s.bg(rgba(0x4c4c_4cff)))
                .child(
                    div()
                        .text_color(rgba(0xdddd_ddff))
                        .text_size(px(12.))
                        .max_w(px(160.))
                        .overflow_x_hidden()
                        .child(current.to_owned()),
                )
                .child(
                    div()
                        .text_color(rgba(0x8888_88ff))
                        .text_size(px(10.))
                        .child("\u{25bc}"), // ▼
                ),
        )
}

/// Render a single font option row in the dropdown list.
fn font_option_row(
    name: &str,
    is_selected: bool,
    app_state: Entity<AppState>,
) -> gpui::Stateful<gpui::Div> {
    let font_name = name.to_owned();
    let font_name_click = font_name.clone();

    let bg = if is_selected {
        rgba(0x0947_71ff)
    } else {
        rgba(0x0000_0000)
    };

    div()
        .id(SharedString::from(format!("font-opt-{font_name}")))
        .px(px(8.))
        .py(px(4.))
        .cursor_pointer()
        .bg(bg)
        .hover(|s| s.bg(rgba(0x3c3c_3cff)))
        .text_color(rgba(0xdddd_ddff))
        .text_size(px(12.))
        .font_family(SharedString::from(font_name.clone()))
        .child(font_name)
        .on_click(move |_, _, cx| {
            let store = app_state.read(cx).settings_store.clone();
            store.update(cx, |s, _| s.set("font_family", &font_name_click));
        })
}

/// Build the dropdown panel with filter input and scrollable font list.
fn font_dropdown_panel(
    font_names: &[String],
    filter_text: &str,
    current_font: &str,
    filter_input: Option<&Entity<TextInput>>,
    app_state: &Entity<AppState>,
) -> gpui::Div {
    let filter_lower = filter_text.to_lowercase();
    let filtered: Vec<&String> = if filter_text.is_empty() {
        font_names.iter().collect()
    } else {
        font_names
            .iter()
            .filter(|n| n.to_lowercase().contains(&filter_lower))
            .collect()
    };

    let mut list = div().flex().flex_col();
    for name in filtered {
        list = list.child(font_option_row(
            name,
            name == current_font,
            app_state.clone(),
        ));
    }

    let mut panel = div()
        .mt(px(4.))
        .rounded(px(4.))
        .border_1()
        .border_color(rgba(0x4c4c_4cff))
        .bg(rgba(0x2525_26ff))
        .flex()
        .flex_col()
        .overflow_hidden();

    // Filter input
    if let Some(input) = filter_input {
        panel = panel.child(
            div()
                .p(px(6.))
                .border_b_1()
                .border_color(rgba(0x3c3c_3cff))
                .child(input.clone()),
        );
    }

    // Scrollable list
    panel = panel.child(
        div()
            .id("font-dropdown-list")
            .max_h(px(200.))
            .overflow_scroll()
            .child(list),
    );

    panel
}

impl Render for SettingsModal {
    #[allow(clippy::too_many_lines)]
    fn render(
        &mut self,
        window: &mut gpui::Window,
        cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        self.ensure_font_names(window);

        let current_font = {
            let state = self.app_state.read(cx);
            state
                .settings_store
                .read(cx)
                .settings()
                .get("font_family")
                .unwrap_or("Menlo")
                .to_owned()
        };

        // Capture initial font on first render so we can detect changes.
        if self.initial_font_family.is_empty() {
            self.initial_font_family.clone_from(&current_font);
        }
        let font_changed = self.initial_font_family != current_font;

        let app_state_backdrop = self.app_state.clone();
        let app_state_close = self.app_state.clone();

        let trigger = font_family_trigger(&current_font);
        let dropdown_open = self.dropdown_open;

        let mut font_section = div().flex().flex_col();
        font_section = font_section.child(
            div()
                .on_mouse_down(
                    MouseButton::Left,
                    cx.listener(|this, _, window, cx| {
                        this.toggle_dropdown(window, cx);
                    }),
                )
                .child(trigger),
        );

        if dropdown_open {
            font_section = font_section.child(font_dropdown_panel(
                &self.font_names,
                &self.filter_text,
                &current_font,
                self.filter_input.as_ref(),
                &self.app_state,
            ));
        }

        // Backdrop
        div()
            .id("settings-backdrop")
            .absolute()
            .size_full()
            .top_0()
            .left_0()
            .bg(rgba(0x0000_0080))
            .flex()
            .items_center()
            .justify_center()
            .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                app_state_backdrop.update(cx, |s, cx| {
                    s.settings_modal_open = false;
                    cx.notify();
                });
            })
            // Modal box
            .child(
                div()
                    .w(px(360.))
                    .max_h(px(500.))
                    .bg(rgba(0x2d2d_2dff))
                    .rounded(px(8.))
                    .border_1()
                    .border_color(rgba(0x4c4c_4cff))
                    .flex()
                    .flex_col()
                    .overflow_hidden()
                    // Swallow clicks inside modal so they don't close the backdrop
                    .on_mouse_down(MouseButton::Left, |_, _, cx| {
                        cx.stop_propagation();
                    })
                    // Header
                    .child(
                        div()
                            .px(px(16.))
                            .py(px(12.))
                            .flex()
                            .flex_row()
                            .items_center()
                            .border_b_1()
                            .border_color(rgba(0x3c3c_3cff))
                            .child(
                                div()
                                    .flex_1()
                                    .text_color(rgba(0xdddd_ddff))
                                    .child("Settings"),
                            )
                            .child(
                                div()
                                    .cursor_pointer()
                                    .text_color(rgba(0x8888_88ff))
                                    .hover(|s| s.text_color(rgba(0xcccc_ccff)))
                                    .child("\u{00d7}")
                                    .on_mouse_down(MouseButton::Left, move |_, _, cx| {
                                        app_state_close.update(cx, |s, cx| {
                                            s.settings_modal_open = false;
                                            cx.notify();
                                        });
                                    }),
                            ),
                    )
                    // Content
                    .child(
                        div()
                            .id("settings-content")
                            .flex_1()
                            .min_h_0()
                            .p(px(16.))
                            .overflow_y_scroll()
                            .child(setting_row_label("Theme", "Dark"))
                            .child(font_section)
                            .when(font_changed, |el| {
                                el.child(
                                    div()
                                        .py(px(4.))
                                        .text_size(px(11.))
                                        .text_color(rgba(0xccaa_33ff))
                                        .child("Restart app to apply font change to existing terminals"),
                                )
                            })
                            .child(font_size_row(
                                "Editor font size",
                                "editor_font_size",
                                &self.app_state,
                                cx,
                            ))
                            .child(font_size_row(
                                "Terminal font size",
                                "terminal_font_size",
                                &self.app_state,
                                cx,
                            )),
                    ),
            )
    }
}
