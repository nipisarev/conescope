use conescope_ui::terminal::terminal_view::{Copy, Paste, SelectAll, SendBackTab, SendTab};
use gpui::{AnyWindowHandle, AppContext, KeyBinding, WindowOptions};
use tracing::info;

use conescope_core::settings::SettingsJson;
use conescope_ui::actions::{
    CancelRename, CloseTab, CopyPath, CopyRelativePath, FileCopy, FileCut, FileDelete,
    FileDuplicate, FilePaste, FileRename, FileTrash, FocusInstance1, FocusInstance2,
    FocusInstance3, FocusInstance4, FocusInstance5, FocusInstance6, FocusInstance7, FocusInstance8,
    FocusInstance9, NewFile, NewFolder, NewInstance, OpenSettings, Quit, ReturnToOverview,
    RevealInFinder, SaveFile, ToggleEditor, ToggleGitPanel, ToggleSidebar, ToggleTerminal,
};
use conescope_ui::state::app_state::{AppState, WindowBounds as SavedWindowBounds};
use conescope_ui::state::db_worker::DbHandle;
use conescope_ui::views::app_view::AppView;
use conescope_ui::views::text_input;

/// Register a window bounds observer that saves position/size to session state.
fn register_window_bounds_save(
    app_state: &gpui::Entity<AppState>,
    window: &mut gpui::Window,
    cx: &mut gpui::App,
) -> gpui::Subscription {
    let app_state = app_state.clone();
    app_state.update(cx, |_, cx| {
        cx.observe_window_bounds(window, move |this, window, cx| {
            let size = window.viewport_size();
            let bounds = window.bounds();
            this.save_window_bounds(
                &SavedWindowBounds {
                    x: f32::from(bounds.origin.x),
                    y: f32::from(bounds.origin.y),
                    width: f32::from(size.width),
                    height: f32::from(size.height),
                },
                cx,
            );
        })
    })
}

fn db_path() -> String {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    let data_dir = format!("{home}/.conescope");
    std::fs::create_dir_all(&data_dir).ok();
    format!("{data_dir}/conescope.db")
}

fn install_panic_hook() {
    let default_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = if let Some(s) = info.payload().downcast_ref::<&str>() {
            (*s).to_owned()
        } else if let Some(s) = info.payload().downcast_ref::<String>() {
            s.clone()
        } else {
            "Unknown panic".to_owned()
        };

        let location = info.location().map_or_else(
            || "unknown".to_owned(),
            |l| format!("{}:{}:{}", l.file(), l.line(), l.column()),
        );

        let crash_msg = format!("PANIC at {location}: {message}");
        eprintln!("{crash_msg}");

        if let Ok(home) = std::env::var("HOME") {
            let path = format!("{home}/.conescope/crash.log");
            let _ = std::fs::write(&path, &crash_msg);
        }

        default_hook(info);
    }));
}

fn bind_keys(cx: &mut gpui::App) {
    cx.bind_keys([KeyBinding::new("cmd-q", Quit, None)]);
    cx.on_action(|_: &Quit, cx| cx.quit());

    cx.bind_keys([
        KeyBinding::new("cmd-a", SelectAll, None),
        KeyBinding::new("cmd-c", Copy, None),
        KeyBinding::new("cmd-v", Paste, None),
    ]);

    // Terminal-scoped: override gpui_component Root's tab focus cycling
    cx.bind_keys([
        KeyBinding::new("tab", SendTab, Some("Terminal")),
        KeyBinding::new("shift-tab", SendBackTab, Some("Terminal")),
    ]);

    text_input::register_key_bindings(cx);

    cx.bind_keys([
        KeyBinding::new("cmd-n", NewInstance, Some("Overview")),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-0", ReturnToOverview, None),
        KeyBinding::new("cmd-1", FocusInstance1, None),
        KeyBinding::new("cmd-2", FocusInstance2, None),
        KeyBinding::new("cmd-3", FocusInstance3, None),
        KeyBinding::new("cmd-4", FocusInstance4, None),
        KeyBinding::new("cmd-5", FocusInstance5, None),
        KeyBinding::new("cmd-6", FocusInstance6, None),
        KeyBinding::new("cmd-7", FocusInstance7, None),
        KeyBinding::new("cmd-8", FocusInstance8, None),
        KeyBinding::new("cmd-9", FocusInstance9, None),
        KeyBinding::new("cmd-s", SaveFile, None),
        KeyBinding::new("cmd-b", ToggleSidebar, None),
        KeyBinding::new("cmd-shift-g", ToggleGitPanel, None),
        KeyBinding::new("cmd-e", ToggleEditor, None),
        KeyBinding::new("cmd-t", ToggleTerminal, None),
        KeyBinding::new("cmd-,", OpenSettings, None),
        // Escape for CloseSettings handled via on_key_down in SettingsView
        // to avoid capturing escape from Terminal key_context.
    ]);

    // FileTree-scoped keybindings (active only when file tree is focused)
    cx.bind_keys([
        KeyBinding::new("escape", CancelRename, Some("FileTree")),
        KeyBinding::new("cmd-n", NewFile, Some("FileTree")),
        KeyBinding::new("alt-cmd-n", NewFolder, Some("FileTree")),
        KeyBinding::new("alt-cmd-r", RevealInFinder, Some("FileTree")),
        KeyBinding::new("cmd-x", FileCut, Some("FileTree")),
        KeyBinding::new("cmd-c", FileCopy, Some("FileTree")),
        KeyBinding::new("cmd-d", FileDuplicate, Some("FileTree")),
        KeyBinding::new("cmd-v", FilePaste, Some("FileTree")),
        KeyBinding::new("alt-cmd-c", CopyPath, Some("FileTree")),
        KeyBinding::new("alt-cmd-shift-c", CopyRelativePath, Some("FileTree")),
        KeyBinding::new("f2", FileRename, Some("FileTree")),
        KeyBinding::new("backspace", FileTrash, Some("FileTree")),
        KeyBinding::new("alt-cmd-backspace", FileDelete, Some("FileTree")),
    ]);
}

fn load_data_async(
    db: DbHandle,
    app_state: &gpui::Entity<AppState>,
    window_handle: AnyWindowHandle,
    cx: &mut gpui::App,
) {
    let app_state = app_state.clone();
    let project_store = app_state.read(cx).project_store.clone();
    let instance_list = app_state.read(cx).instance_list.clone();
    let settings_store = app_state.read(cx).settings_store.clone();

    cx.spawn(async move |cx| {
        if let Ok(Ok(projects)) = db.get_all_projects().recv() {
            cx.update(|cx| {
                project_store.update(cx, |store, _| store.load(projects));
            });
            info!("Projects loaded");
        }

        if let Ok(Ok(instances)) = db.get_all_instances().recv() {
            let instances: Vec<_> = instances
                .into_iter()
                .filter(|i| i.ended_at.is_none())
                .collect();

            cx.update(|cx| {
                instance_list.update(cx, |list, cx| list.load_from_db(instances, cx));
            });
            info!("Instances loaded");

            let project_store_for_restore = project_store.clone();
            let settings_for_restore = settings_store.clone();
            let app_state_for_restore = app_state.clone();
            let _ = cx.update_window(window_handle, |_view, window, cx| {
                let settings = settings_for_restore.read(cx).settings().clone();
                let font_family = Some(settings.font_family.clone());
                #[allow(clippy::cast_precision_loss)]
                let tfs = settings.terminal_font_size as f32;
                let lhr = settings.terminal_line_height as f32;
                let colors = app_state_for_restore.read(cx).theme().terminal_colors();
                instance_list.update(cx, |list, cx| {
                    list.restore_terminals(
                        &project_store_for_restore,
                        font_family.as_deref(),
                        tfs,
                        lhr,
                        &colors,
                        window,
                        cx,
                    );
                });

                // Initialize git store for the focused instance's project
                let focused_project_path = app_state_for_restore
                    .read(cx)
                    .focused_instance_id(cx)
                    .and_then(|fid| {
                        instance_list
                            .read(cx)
                            .find_by_id(fid, cx)
                            .and_then(|e| e.read(cx).instance.project_id.clone())
                    })
                    .and_then(|pid| {
                        project_store_for_restore
                            .read(cx)
                            .get(&pid)
                            .map(|p| p.path.clone())
                    });
                if let Some(path) = focused_project_path {
                    let git_store = app_state_for_restore.read(cx).git_store.clone();
                    git_store.update(cx, |store, cx| store.set_project(Some(&path), cx));
                }
            });
            info!("Terminals restored");
        }
    })
    .detach();
}

fn main() {
    install_panic_hook();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();
    tracing_log::LogTracer::init().ok();

    let db_path = db_path();
    info!("Database path: {db_path}");
    let db = DbHandle::spawn(&db_path).expect("Failed to open database");

    gpui::Application::new()
        .with_assets(conescope_ui::assets::ConescopeAssets)
        .run(move |cx: &mut gpui::App| {
            gpui_component::init(cx);
            bind_keys(cx);

            let app_state = AppState::new(db.clone(), cx);

            // Load user settings from JSON file (with DB migration on first run)
            let settings_dir = SettingsJson::settings_dir();
            let settings_path = SettingsJson::file_path(&settings_dir);
            let user_settings = if settings_path.exists() {
                SettingsJson::load_from_file(&settings_dir)
            } else {
                // First run: migrate from DB
                let migrated = if let Ok(Ok(db_settings)) = db.get_all_settings().recv() {
                    SettingsJson::migrate_from_db(&db_settings)
                } else {
                    SettingsJson::default()
                };
                let _ = migrated.save_to_file(&settings_dir);
                info!("Migrated settings to {}", settings_path.display());
                migrated
            };

            // Load session state from DB
            if let Ok(Ok(db_settings)) = db.get_all_settings().recv() {
                let settings_store = app_state.read(cx).settings_store.clone();
                settings_store.update(cx, |store, _| store.load_session(db_settings));
                info!("Session state loaded");
            }

            // Load user settings into store
            {
                let settings_store = app_state.read(cx).settings_store.clone();
                settings_store.update(cx, |store, _| store.load_settings(user_settings.clone()));
            }

            // Apply saved theme
            {
                let mode =
                    conescope_ui::theme::ThemeMode::from_str_or_default(&user_settings.theme);
                if mode != app_state.read(cx).theme().mode {
                    app_state.update(cx, |s, cx| s.set_theme(mode, cx));
                }
                // Sync our theme colors into gpui-component's global theme
                // (set_theme does this internally, but we need it for the default case too)
                let theme = app_state.read(cx).theme().clone();
                conescope_ui::state::app_state::sync_gpui_component_theme(&theme, cx);
            }

            // Read saved window bounds
            let session = app_state.read(cx).settings_store.read(cx).session().clone();
            let window_bounds = gpui::WindowBounds::Windowed(gpui::Bounds {
                origin: gpui::point(
                    gpui::px(session.window_x.unwrap_or(100.0)),
                    gpui::px(session.window_y.unwrap_or(100.0)),
                ),
                size: gpui::size(
                    gpui::px(session.window_width.unwrap_or(1400.0)),
                    gpui::px(session.window_height.unwrap_or(900.0)),
                ),
            });

            let window_handle: AnyWindowHandle = cx
                .open_window(
                    WindowOptions {
                        window_bounds: Some(window_bounds),
                        titlebar: Some(gpui::TitlebarOptions {
                            title: Some("Conescope".into()),
                            appears_transparent: true,
                            traffic_light_position: Some(gpui::point(gpui::px(12.), gpui::px(12.))),
                        }),
                        ..Default::default()
                    },
                    |window, cx| {
                        let view = cx.new(|cx| AppView::new(app_state.clone(), cx));
                        conescope_ui::views::focus_view::register_focus_resize(
                            &app_state, window, cx,
                        )
                        .detach();
                        register_window_bounds_save(&app_state, window, cx).detach();
                        cx.new(|cx| gpui_component::Root::new(view, window, cx))
                    },
                )
                .expect("Failed to open window")
                .into();

            load_data_async(db, &app_state, window_handle, cx);
            cx.activate(true);
        });
}
