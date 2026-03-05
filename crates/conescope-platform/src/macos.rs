use objc2_app_kit::{NSView, NSWindowOrderingMode};
use raw_window_handle::RawWindowHandle;

#[allow(unsafe_code)]
fn ns_window_from_raw(
    handle: RawWindowHandle,
) -> Option<objc2::rc::Retained<objc2_app_kit::NSWindow>> {
    match handle {
        RawWindowHandle::AppKit(appkit_handle) => {
            let ns_view_ptr = appkit_handle.ns_view.as_ptr().cast::<NSView>();
            let ns_view: &NSView = unsafe { &*ns_view_ptr };
            ns_view.window()
        }
        _ => None,
    }
}

#[must_use]
pub fn window_id_from_raw_handle(handle: RawWindowHandle) -> Option<u32> {
    let ns_window = ns_window_from_raw(handle)?;
    let window_number = ns_window.windowNumber();
    if window_number > 0 {
        #[allow(clippy::cast_sign_loss)]
        Some(window_number as u32)
    } else {
        None
    }
}

#[allow(unsafe_code)]
pub fn add_child_window(parent: RawWindowHandle, child: RawWindowHandle) {
    let Some(parent_win) = ns_window_from_raw(parent) else {
        return;
    };
    let Some(child_win) = ns_window_from_raw(child) else {
        return;
    };
    unsafe { parent_win.addChildWindow_ordered(&child_win, NSWindowOrderingMode::Above) };
}

pub fn remove_child_window(parent: RawWindowHandle, child: RawWindowHandle) {
    let Some(parent_win) = ns_window_from_raw(parent) else {
        return;
    };
    let Some(child_win) = ns_window_from_raw(child) else {
        return;
    };
    parent_win.removeChildWindow(&child_win);
}

pub fn configure_overlay_panel(handle: RawWindowHandle) {
    use objc2_app_kit::NSWindowCollectionBehavior;

    let Some(window) = ns_window_from_raw(handle) else {
        return;
    };

    window.setCollectionBehavior(NSWindowCollectionBehavior::FullScreenAuxiliary);
    window.setHasShadow(true);
}

pub fn position_overlay_at_parent(parent: RawWindowHandle, child: RawWindowHandle) {
    let Some(parent_win) = ns_window_from_raw(parent) else {
        return;
    };
    let Some(child_win) = ns_window_from_raw(child) else {
        return;
    };
    let parent_frame = parent_win.frame();
    child_win.setFrameOrigin(parent_frame.origin);
}
