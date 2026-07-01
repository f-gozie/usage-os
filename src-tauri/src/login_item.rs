//! Start-at-login via SMAppService (macOS 13+). The agent plist ships inside the app bundle
//! (Contents/Library/LaunchAgents), so Login Items and the background-items notification
//! attribute the item to UsageOS itself rather than to the signing certificate (D69).

#[cfg(target_os = "macos")]
mod imp {
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

    pub fn is_enabled() -> bool {
        // SAFETY: reading the registration status has no preconditions.
        let status = unsafe { service().status() };
        // RequiresApproval (switched off in System Settings) reads as disabled.
        status == SMAppServiceStatus::Enabled
    }

    pub fn set_enabled(on: bool) -> Result<(), String> {
        let service = service();
        // SAFETY: no preconditions; failure (unbundled dev binary, denied in System
        // Settings) is reported via NSError, not UB.
        let result = unsafe {
            if on {
                service.registerAndReturnError()
            } else {
                service.unregisterAndReturnError()
            }
        };
        result.map_err(|e| e.localizedDescription().to_string())
    }

    /// One-time cleanup for pre-0.1.2 installs: the old autostart plugin wrote a bare plist
    /// into ~/Library/LaunchAgents, which macOS attributes to the certificate, not the app
    /// (D69). The file's presence means the toggle was flipped on at some point (System
    /// Settings can disable the item without deleting the file — migration re-registers
    /// those; accepted, the new item lands as approval-pending rather than silently on).
    pub fn migrate_legacy() {
        let Some(home) = std::env::var_os("HOME") else {
            return;
        };
        let legacy = std::path::Path::new(&home).join("Library/LaunchAgents/UsageOS.plist");
        if !legacy.exists() {
            return;
        }
        // Register the bundled agent BEFORE removing the old plist — a failed registration
        // must not cost the user their working login item. ("Already registered" counts as
        // success: the agent is in place, the file can go.)
        if let Err(e) = set_enabled(true) {
            if !is_enabled() {
                eprintln!("[LoginItem] migration register failed (legacy plist kept): {e}");
                return;
            }
        }
        if let Err(e) = std::fs::remove_file(&legacy) {
            eprintln!("[LoginItem] failed to remove legacy plist: {e}");
        }
    }

    /// Try to take the app-wide instance lock — an flock held for the process's lifetime and
    /// released by the OS on exit, so it can't race the way a process-list snapshot can.
    /// Open/create failures count as "acquired": never block a launch over a lock hiccup.
    pub fn try_acquire_instance_lock() -> bool {
        use std::os::fd::IntoRawFd;
        let Some(home) = std::env::var_os("HOME") else {
            return true;
        };
        // Lives next to the app's data — the dir name must match tauri.conf.json `identifier`.
        let dir = std::path::Path::new(&home).join("Library/Application Support/com.usageos.app");
        if std::fs::create_dir_all(&dir).is_err() {
            return true;
        }
        let Ok(file) = std::fs::File::create(dir.join("instance.lock")) else {
            return true;
        };
        let fd = file.into_raw_fd();
        // SAFETY: `fd` is a valid descriptor we own; LOCK_NB makes the call non-blocking.
        // The fd is deliberately leaked so the lock lives exactly as long as the process.
        unsafe { libc::flock(fd, libc::LOCK_EX | libc::LOCK_NB) == 0 }
    }

    /// The agent launch is a trampoline: launchd owns whatever process it spawns for the
    /// job, so unregistering the agent (the Settings toggle) would kill that process — the
    /// long-running app must not be launchd's child. Respawn detached, then the caller exits.
    pub fn respawn_detached() {
        use std::os::unix::process::CommandExt;
        let exe = match std::env::current_exe() {
            Ok(exe) => exe,
            Err(e) => {
                eprintln!("[LoginItem] current_exe failed, agent launch dropped: {e}");
                return;
            }
        };
        let mut cmd = std::process::Command::new(exe);
        cmd.arg("--hidden")
            .env_remove("USAGEOS_AGENT_LAUNCH")
            .env("USAGEOS_DETACHED", "1");
        // SAFETY: setsid is async-signal-safe (no allocation, no locks) and moves the child
        // into its own session, out of reach of launchd's job teardown.
        unsafe {
            cmd.pre_exec(|| {
                libc::setsid();
                Ok(())
            });
        }
        if let Err(e) = cmd.spawn() {
            eprintln!("[LoginItem] detached respawn failed: {e}");
        }
    }
}

#[cfg(target_os = "macos")]
pub use imp::{
    is_enabled, migrate_legacy, respawn_detached, set_enabled, try_acquire_instance_lock,
};

#[cfg(not(target_os = "macos"))]
mod stub {
    pub fn is_enabled() -> bool {
        false
    }
    pub fn set_enabled(_on: bool) -> Result<(), String> {
        Err("start at login is only supported on macOS".into())
    }
    pub fn migrate_legacy() {}
    pub fn try_acquire_instance_lock() -> bool {
        true
    }
    pub fn respawn_detached() {}
}

#[cfg(not(target_os = "macos"))]
pub use stub::{
    is_enabled, migrate_legacy, respawn_detached, set_enabled, try_acquire_instance_lock,
};
