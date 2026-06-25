//! Reclass the menubar glance window into a **non-activating `NSPanel`** so it floats over other
//! apps' full-screen Spaces — like the system menubar popovers (Now Playing, Wi-Fi) — and takes
//! clicks without activating UsageOS or yanking the user out of full-screen.
//!
//! Decided by a `/debate` (D56): an in-repo objc2 reclass rather than a third-party panel crate,
//! to keep the deliberately-frozen Tauri stack dependency-light and fully auditable. Public AppKit
//! only (no private selectors). `object_setClass` swaps the isa to a subclass that adds **no**
//! instance variables (layout-safe over `NSWindow`) and **preserves the window's delegate**, so
//! Tauri's `WindowEvent`s (focus/close) still fire. Verified on-device (D32/D33) — CI compiles it.

use objc2::ffi::object_setClass;
use objc2::runtime::AnyObject;
use objc2::{define_class, ClassType, MainThreadOnly};
use objc2_app_kit::{NSWindow, NSWindowCollectionBehavior, NSWindowStyleMask};
use tauri::WebviewWindow;

define_class!(
    // SAFETY: `NSPanel` adds no ivars over `NSWindow`, and this subclass adds none, so
    // `object_setClass`-ing a live Tauri `NSWindow` to it is layout-compatible. No `Drop` impl.
    #[unsafe(super(objc2_app_kit::NSPanel))]
    #[thread_kind = MainThreadOnly]
    #[name = "UsageOSGlancePanel"]
    struct GlancePanel;

    impl GlancePanel {
        // A non-activating panel must still become *key* for its own controls (the Open/Quit
        // buttons) — WITHOUT becoming main or activating the app. That pairing is what lets the
        // popover take clicks while floating over another app's full-screen Space.
        #[unsafe(method(canBecomeKeyWindow))]
        fn can_become_key_window(&self) -> bool {
            true
        }

        #[unsafe(method(canBecomeMainWindow))]
        fn can_become_main_window(&self) -> bool {
            false
        }
    }
);

/// `NSPopUpMenuWindowLevel` — above the menubar and a full-screen app's content, where system
/// popovers live. (objc2-app-kit doesn't re-export the level constants; this is the documented
/// value `CGWindowLevelForKey(kCGPopUpMenuWindowLevelKey)`.)
const NS_POPUP_MENU_WINDOW_LEVEL: isize = 101;

/// Turn the glance webview's window into the non-activating panel + configure it to float over
/// full-screen Spaces. No-op if the native handle can't be read.
pub(crate) fn configure(window: &WebviewWindow) {
    let Ok(ptr) = window.ns_window() else {
        return;
    };
    // SAFETY: ns_window() returns this window's live NSWindow and we're on the main thread (the
    // tray-click callback). The reclass adds no ivars (layout-safe) and keeps the delegate; the
    // raw-pointer deref + the AppKit setters below are the reason this block is `unsafe`.
    unsafe {
        object_setClass(ptr as *mut AnyObject, GlancePanel::class());
        let ns: &NSWindow = &*(ptr as *const NSWindow);
        ns.setStyleMask(ns.styleMask() | NSWindowStyleMask::NonactivatingPanel);
        ns.setLevel(NS_POPUP_MENU_WINDOW_LEVEL);
        ns.setCollectionBehavior(
            NSWindowCollectionBehavior::CanJoinAllSpaces
                | NSWindowCollectionBehavior::CanJoinAllApplications
                | NSWindowCollectionBehavior::FullScreenAuxiliary
                | NSWindowCollectionBehavior::Transient
                | NSWindowCollectionBehavior::IgnoresCycle,
        );
        ns.setHidesOnDeactivate(false);
    }
}
