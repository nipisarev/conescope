#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::{
    add_child_window, animate_resize, configure_overlay_panel, position_overlay_at_parent,
    remove_child_window, window_id_from_raw_handle,
};

pub mod sound;
