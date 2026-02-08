use std::time::Duration;

use conescope_ui::terminal::terminal_view::{Copy, Paste, SelectAll};
use gpui::{AppContext, KeyBinding, WindowOptions};
use portable_pty::PtySize;

use conescope_ui::multi_terminal::MultiTerminalGrid;
use conescope_ui::terminal::{compute_cell_metrics, spawn_terminal_pane};

#[allow(clippy::too_many_lines)] // gate test binary, not production code
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
                    origin: gpui::point(gpui::px(50.), gpui::px(50.)),
                    size: gpui::size(gpui::px(1600.), gpui::px(1000.)),
                })),
                titlebar: Some(gpui::TitlebarOptions {
                    title: Some("Conescope — Multi-Terminal Gate Test".into()),
                    appears_transparent: true,
                    traffic_light_position: Some(gpui::point(gpui::px(12.), gpui::px(12.))),
                }),
                ..Default::default()
            },
            |window, cx| {
                let tc = conescope_ui::terminal::TerminalColors::default();
                let panes: Vec<_> = (0..4)
                    .map(|_| spawn_terminal_pane(None, None, &tc, window, cx))
                    .collect();

                let views = [
                    panes[0].view.clone(),
                    panes[1].view.clone(),
                    panes[2].view.clone(),
                    panes[3].view.clone(),
                ];

                // Collect masters and receivers for resize + output
                let masters: Vec<_> = panes.iter().map(|p| p.master.clone()).collect();
                let receivers: Vec<_> = panes.into_iter().map(|p| p.stdout_rx).collect();

                let views_for_resize = views.to_vec();

                let grid = cx.new(|_| MultiTerminalGrid { views });

                // Resize all PTYs on window bounds change
                let subscription = grid.update(cx, |_, cx| {
                    cx.observe_window_bounds(window, move |_, window, cx| {
                        let size = window.viewport_size();
                        let width = f32::from(size.width);
                        let height = f32::from(size.height);

                        let Some((cell_width, cell_height)) =
                            compute_cell_metrics(window, None, None)
                        else {
                            return;
                        };

                        // Each pane gets half the width/height minus divider
                        let divider = 1.0_f32;
                        let pane_width = ((width - divider) / 2.0).max(1.0);
                        let pane_height = ((height - divider) / 2.0).max(1.0);

                        #[allow(clippy::cast_sign_loss)]
                        let cols = (pane_width / cell_width).floor().max(1.0) as u16;
                        #[allow(clippy::cast_sign_loss)]
                        let rows = (pane_height / cell_height).floor().max(1.0) as u16;

                        for master in &masters {
                            let _ = master.resize(PtySize {
                                rows,
                                cols,
                                pixel_width: 0,
                                pixel_height: 0,
                            });
                        }

                        for v in &views_for_resize {
                            v.update(cx, |this, cx| this.resize_terminal(cols, rows, cx));
                        }
                    })
                });
                subscription.detach();

                // Async task: batch output for all 4 terminals
                let [r0, r1, r2, r3] = {
                    let mut iter = receivers.into_iter();
                    [
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                        iter.next().unwrap(),
                    ]
                };

                let v0 = grid.read(cx).views[0].clone();
                let v1 = grid.read(cx).views[1].clone();
                let v2 = grid.read(cx).views[2].clone();
                let v3 = grid.read(cx).views[3].clone();

                window
                    .spawn(cx, async move |cx| {
                        let pairs = [(r0, v0), (r1, v1), (r2, v2), (r3, v3)];
                        loop {
                            cx.background_executor()
                                .timer(Duration::from_millis(16))
                                .await;

                            for (rx, view) in &pairs {
                                let mut batch = Vec::new();
                                while let Ok(chunk) = rx.try_recv() {
                                    batch.extend_from_slice(&chunk);
                                }
                                if !batch.is_empty() {
                                    let view = view.clone();
                                    cx.update(|_, cx| {
                                        view.update(cx, |this, cx| {
                                            this.queue_output_bytes(&batch, cx);
                                        });
                                    })
                                    .ok();
                                }
                            }
                        }
                    })
                    .detach();

                grid
            },
        )
        .expect("Failed to open window");
    });
}
