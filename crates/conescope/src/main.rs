use gpui::prelude::*;

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    tracing_log::LogTracer::init().ok();

    gpui::Application::new().run(|cx: &mut gpui::App| {
        cx.open_window(
            gpui::WindowOptions {
                window_bounds: Some(gpui::WindowBounds::Windowed(gpui::Bounds {
                    origin: gpui::point(gpui::px(100.), gpui::px(100.)),
                    size: gpui::size(gpui::px(1400.), gpui::px(900.)),
                })),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Conescope".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(gpui::point(gpui::px(12.), gpui::px(12.))),
                }),
                ..Default::default()
            },
            |_window, cx| cx.new(|_| HelloView),
        )
        .expect("Failed to open window");
    });
}

struct HelloView;

impl Render for HelloView {
    fn render(
        &mut self,
        _window: &mut gpui::Window,
        _cx: &mut gpui::Context<Self>,
    ) -> impl IntoElement {
        gpui::div()
            .size_full()
            .bg(gpui::rgb(0x001e_1e1e))
            .flex()
            .items_center()
            .justify_center()
            .child(
                gpui::div()
                    .text_color(gpui::rgb(0x00ff_ffff))
                    .text_size(gpui::px(24.))
                    .child("Conescope — Rust/GPUI"),
            )
    }
}
