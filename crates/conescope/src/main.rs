use std::time::Duration;

use gpui::{KeyBinding, WindowOptions};
use gpui_ghostty_terminal::view::{Copy, Paste, SelectAll};
use portable_pty::PtySize;

use conescope_ui::terminal::{compute_cell_metrics, spawn_terminal_pane};

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    tracing_log::LogTracer::init().ok();

    gpui::Application::new().run(|cx: &mut gpui::App| {
        cx.bind_keys([
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-v", Paste, None),
        ]);

        cx.open_window(
            WindowOptions {
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
            |window, cx| {
                let pane = spawn_terminal_pane(None, window, cx);
                let view = pane.view.clone();

                // Resize PTY on window bounds change
                let master = pane.master.clone();
                let subscription = view.update(cx, |_, cx| {
                    cx.observe_window_bounds(window, move |this, window, cx| {
                        let size = window.viewport_size();
                        let width = f32::from(size.width);
                        let height = f32::from(size.height);

                        let Some((cell_width, cell_height)) = compute_cell_metrics(window) else {
                            return;
                        };

                        #[allow(clippy::cast_sign_loss)] // max(1.0) ensures positive
                        let cols = (width / cell_width).floor().max(1.0) as u16;
                        #[allow(clippy::cast_sign_loss)]
                        let rows = (height / cell_height).floor().max(1.0) as u16;

                        let _ = master.resize(PtySize {
                            rows,
                            cols,
                            pixel_width: 0,
                            pixel_height: 0,
                        });

                        this.resize_terminal(cols, rows, cx);
                    })
                });
                subscription.detach();

                // Async task: batch PTY output every 16ms
                let stdout_rx = pane.stdout_rx;
                let view_for_task = view.clone();
                window
                    .spawn(cx, async move |cx| {
                        loop {
                            cx.background_executor()
                                .timer(Duration::from_millis(16))
                                .await;
                            let mut batch = Vec::new();
                            while let Ok(chunk) = stdout_rx.try_recv() {
                                batch.extend_from_slice(&chunk);
                            }
                            if batch.is_empty() {
                                continue;
                            }
                            cx.update(|_, cx| {
                                view_for_task.update(cx, |this, cx| {
                                    this.queue_output_bytes(&batch, cx);
                                });
                            })
                            .ok();
                        }
                    })
                    .detach();

                view
            },
        )
        .expect("Failed to open window");
    });
}
