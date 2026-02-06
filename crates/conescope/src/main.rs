use gpui::{AppContext, KeyBinding, WindowOptions};
use gpui_ghostty_terminal::view::{Copy, Paste, SelectAll};
use tracing::info;

use conescope_ui::state::app_state::AppState;
use conescope_ui::state::db_worker::DbHandle;
use conescope_ui::views::app_view::AppView;

fn db_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let data_dir = format!("{home}/.conescope");
    std::fs::create_dir_all(&data_dir).ok();
    format!("{data_dir}/conescope.db")
}

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

        // Open main window with AppView
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
                let view = cx.new(|cx| AppView::new(app_state.clone(), cx));

                // Register PTY resize observer for Focus mode
                let resize_sub =
                    conescope_ui::views::focus_view::register_focus_resize(&app_state, window, cx);
                resize_sub.detach();

                view
            },
        )
        .expect("Failed to open window");
    });
}
