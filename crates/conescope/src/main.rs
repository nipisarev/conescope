use std::time::Duration;

use gpui::{KeyBinding, WindowOptions};
use gpui_ghostty_terminal::view::{Copy, Paste, SelectAll};
use portable_pty::PtySize;
use tracing::info;

use conescope_ui::state::app_state::AppState;
use conescope_ui::state::db_worker::DbHandle;
use conescope_ui::terminal::{compute_cell_metrics, spawn_terminal_pane};

fn db_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let data_dir = format!("{home}/.conescope");
    std::fs::create_dir_all(&data_dir).ok();
    format!("{data_dir}/conescope.db")
}

#[allow(clippy::too_many_lines)] // bootstrap logic, will be split in Phase 5
fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    tracing_log::LogTracer::init().ok();

    let db_path = db_path();
    info!("Database path: {db_path}");
    let db = DbHandle::spawn(&db_path).expect("Failed to open database");

    gpui::Application::new().run(move |cx: &mut gpui::App| {
        cx.bind_keys([
            KeyBinding::new("cmd-a", SelectAll, None),
            KeyBinding::new("cmd-c", Copy, None),
            KeyBinding::new("cmd-v", Paste, None),
        ]);

        // Create root state entity
        let app_state = AppState::new(db.clone(), cx);

        // Async: load settings, projects, instances from DB
        let settings_store = app_state.read(cx).settings_store.clone();
        let project_store = app_state.read(cx).project_store.clone();
        let instance_list = app_state.read(cx).instance_list.clone();
        let db_for_load = db.clone();

        cx.spawn(async move |cx| {
            if let Ok(Ok(settings)) = db_for_load.get_all_settings().recv() {
                cx.update(|cx| {
                    settings_store.update(cx, |store, _| store.load(settings));
                });
                info!("Settings loaded");
            }

            if let Ok(Ok(projects)) = db_for_load.get_all_projects().recv() {
                cx.update(|cx| {
                    project_store.update(cx, |store, _| store.load(projects));
                });
                info!("Projects loaded");
            }

            if let Ok(Ok(instances)) = db_for_load.get_all_instances().recv() {
                cx.update(|cx| {
                    instance_list.update(cx, |list, cx| list.load_from_db(instances, cx));
                });
                info!("Instances loaded");
            }
        })
        .detach();

        // Keep existing terminal rendering (Phase 5 replaces with real UI)
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
