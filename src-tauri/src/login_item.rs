//! Start-at-login via SMAppService (macOS 13+). The agent plist ships inside the app bundle
//! (Contents/Library/LaunchAgents), so Login Items and the background-items notification
//! attribute the item to UsageOS itself rather than to the signing certificate (D69).

#[cfg(target_os = "macos")]
mod imp {
    use objc2_app_kit::NSRunningApplication;
    use objc2_foundation::NSString;
    use objc2_service_management::{SMAppService, SMAppServiceStatus};

    /// The bundled agent plist's filename; its `Label` is the filename minus `.plist`.
    const PLIST_NAME: &str = "com.usageos.app.agent.plist";

    fn service() -> objc2::rc::Retained<SMAppService> {
        let name = NSString::from_str(PLIST_NAME);
        // SAFETY: `name` is a valid NSString. A missing plist (e.g. the unbundled dev binary)
        // surfaces later as a register error, not a crash.
        unsafe { SMAppService::agentServiceWithPlistName(&name) }
    }

    pub fn is_enabled() -> Result<bool, String> {
        // SAFETY: reading the registration status has no preconditions.
        let status = unsafe { service().status() };
        // RequiresApproval (user switched it off in System Settings) reads as disabled.
        Ok(status == SMAppServiceStatus::Enabled)
    }

    pub fn set_enabled(on: bool) -> Result<(), String> {
        let service = service();
        // SAFETY: plain calls; failures (unbundled dev binary, denied in System Settings)
        // come back as NSError, mapped to a message for the UI (whose toggle reverts).
        let result = unsafe {
            if on {
                service.registerAndReturnError()
            } else {
                service.unregisterAndReturnError()
            }
        };
        result.map_err(|e| e.localizedDescription().to_string())
    }

    /// True when another UsageOS process is already running. launchd starts the agent the
    /// moment it's registered (and again at login even if macOS's window restore relaunched
    /// the app), so a `--hidden` launch must bow out instead of running a duplicate.
    pub fn another_instance_running() -> bool {
        let bundle_id = NSString::from_str("com.usageos.app");
        let apps = NSRunningApplication::runningApplicationsWithBundleIdentifier(&bundle_id);
        let me = std::process::id() as i32;
        apps.iter().any(|app| app.processIdentifier() != me)
    }

    /// One-time cleanup for pre-0.1.2 installs: the old autostart plugin wrote a bare plist
    /// into ~/Library/LaunchAgents, which macOS attributes to the certificate, not the app
    /// (D69). Its presence means the toggle was on — remove it and register the bundled agent.
    pub fn migrate_legacy() {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let legacy = std::path::Path::new(&home).join("Library/LaunchAgents/UsageOS.plist");
        if !legacy.exists() {
            return;
        }
        match std::fs::remove_file(&legacy) {
            Ok(()) => {
                if let Err(e) = set_enabled(true) {
                    eprintln!("[LoginItem] legacy plist removed but re-register failed: {e}");
                }
            }
            Err(e) => eprintln!("[LoginItem] failed to remove legacy plist: {e}"),
        }
    }
}

#[cfg(target_os = "macos")]
pub use imp::{another_instance_running, is_enabled, migrate_legacy, set_enabled};

#[cfg(not(target_os = "macos"))]
pub fn is_enabled() -> Result<bool, String> {
    Ok(false)
}

#[cfg(not(target_os = "macos"))]
pub fn set_enabled(_on: bool) -> Result<(), String> {
    Err("start at login is only supported on macOS".into())
}

#[cfg(not(target_os = "macos"))]
pub fn migrate_legacy() {}

#[cfg(not(target_os = "macos"))]
pub fn another_instance_running() -> bool {
    false
}
