#[cfg(target_os = "macos")]
pub mod macos;

#[cfg(target_os = "macos")]
pub use macos::{add_child_window, animate_resize, remove_child_window, window_id_from_raw_handle};

pub mod sound;
