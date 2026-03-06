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

/// Defers `setFrame:display:animate:` to the next main-queue iteration via
/// `dispatch_async_f`. This is required because the native animation pumps a
/// nested run loop, which re-enters GPUI's `display_layer` callback. If called
/// from inside any `App::update` / `AsyncApp::update` context the `AppCell`
/// `RefCell` is already borrowed → panic.
#[allow(unsafe_code)]
pub fn animate_resize(handle: RawWindowHandle, width: f64, height: f64) {
    use std::ffi::c_void;

    type DispatchQueue = *const c_void;
    type DispatchFunction = extern "C" fn(*mut c_void);

    unsafe extern "C" {
        // `dispatch_get_main_queue()` is a C macro; the real symbol is `_dispatch_main_q`.
        #[link_name = "_dispatch_main_q"]
        static DISPATCH_MAIN_Q: c_void;
        fn dispatch_async_f(queue: DispatchQueue, context: *mut c_void, work: DispatchFunction);
    }

    struct Params {
        handle: RawWindowHandle,
        width: f64,
        height: f64,
    }

    extern "C" fn do_animate(ctx: *mut c_void) {
        let params = unsafe { Box::from_raw(ctx.cast::<Params>()) };
        let Some(window) = ns_window_from_raw(params.handle) else {
            return;
        };
        let mut frame = window.frame();
        frame.size.width = params.width;
        frame.size.height = params.height;
        window.setFrame_display_animate(frame, true, true);
    }

    let params = Box::new(Params {
        handle,
        width,
        height,
    });
    unsafe {
        dispatch_async_f(
            &raw const DISPATCH_MAIN_Q,
            Box::into_raw(params).cast::<c_void>(),
            do_animate,
        );
    }
}
