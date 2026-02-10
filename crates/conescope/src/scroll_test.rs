//! Minimal scroll test: single `uniform_list` with stderr diagnostics.
//!
//! Prints visible range on every render to show whether scrolling works.
//! If range always starts at 0, scroll offset isn't updating.
//!
//! Run: `cargo run --bin scroll-test`

use gpui::prelude::*;
use gpui::{
    Application, UniformListScrollHandle, WindowBounds, WindowOptions, div, px, rgba, size,
    uniform_list,
};

const ITEM_COUNT: usize = 200;
const ROW_HEIGHT: f32 = 24.0;

struct MinimalScrollView {
    scroll_handle: UniformListScrollHandle,
    render_count: u32,
}

impl MinimalScrollView {
    fn new(cx: &mut gpui::Context<Self>) -> Self {
        // Auto-scroll to item 100 after 1 second to test programmatic scroll
        let handle = UniformListScrollHandle::new();
        let h = handle.clone();
        cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(std::time::Duration::from_secs(1))
                .await;
            eprintln!("[auto-scroll] scrolling to item 100 via scroll_to_item...");
            h.scroll_to_item(100, gpui::ScrollStrategy::Top);

            // Try notify via entity
            match this.update(cx, |_, cx| {
                eprintln!("[auto-scroll] notify() called successfully");
                cx.notify();
            }) {
                Ok(()) => eprintln!("[auto-scroll] update() succeeded"),
                Err(e) => eprintln!("[auto-scroll] update() FAILED: {e}"),
            }

            // Also try direct offset manipulation after 2 seconds
            cx.background_executor()
                .timer(std::time::Duration::from_secs(1))
                .await;
            eprintln!("[auto-scroll-2] directly setting base_handle offset to -500px...");
            let base = h.0.borrow().base_handle.clone();
            base.set_offset(gpui::point(px(0.), px(-500.)));
            eprintln!("[auto-scroll-2] offset after set: {:?}", base.offset());
            match this.update(cx, |_, cx| {
                eprintln!("[auto-scroll-2] notify() called");
                cx.notify();
            }) {
                Ok(()) => eprintln!("[auto-scroll-2] update() succeeded"),
                Err(e) => eprintln!("[auto-scroll-2] update() FAILED: {e}"),
            }
        })
        .detach();
        Self {
            scroll_handle: handle,
            render_count: 0,
        }
    }
}

impl Render for MinimalScrollView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        self.render_count += 1;
        let rc = self.render_count;
        let handle = self.scroll_handle.clone();
        let offset = handle.0.borrow().base_handle.offset();
        eprintln!(
            "[render #{rc}] scroll_offset = ({}, {})",
            offset.x, offset.y
        );

        uniform_list("test-list", ITEM_COUNT, move |range, _window, _cx| {
            eprintln!("[render #{rc}] visible_range = {range:?}");
            range
                .map(|ix| {
                    div()
                        .h(px(ROW_HEIGHT))
                        .px(px(8.))
                        .flex()
                        .items_center()
                        .text_size(px(14.))
                        .text_color(rgba(0xcccc_ccff))
                        .border_b_1()
                        .border_color(rgba(0x3333_33ff))
                        .child(format!("Item {ix}"))
                        .into_any_element()
                })
                .collect()
        })
        .size_full()
        .track_scroll(&self.scroll_handle)
    }
}

fn main() {
    Application::new()
        .with_assets(conescope_ui::assets::ConescopeAssets)
        .run(|cx: &mut gpui::App| {
            gpui_component::init(cx);

            cx.open_window(
                WindowOptions {
                    window_bounds: Some(WindowBounds::Windowed(gpui::Bounds {
                        origin: gpui::point(px(100.), px(100.)),
                        size: size(px(400.), px(600.)),
                    })),
                    titlebar: Some(gpui::TitlebarOptions {
                        title: Some("Minimal Scroll Test".into()),
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                |window, cx| {
                    let view = cx.new(MinimalScrollView::new);
                    cx.new(|cx| gpui_component::Root::new(view, window, cx))
                },
            )
            .expect("Failed to open window");
        });
}
