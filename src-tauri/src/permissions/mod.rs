//! Capture's two macOS permissions — **Accessibility** (window titles; capture degrades to
//! app-only without it) and **Automation** (browser URLs; falls back to the window-title site
//! without it) — surfaced to the UI for onboarding + Settings.
//!
//! The native queries live in [`macos`] (cfg-gated, verified on-device per D32/D33); a non-macOS
//! stub keeps the commands callable in CI. Hard rule 5: this is the only place that reads the
//! grants, behind a small typed seam.

use serde::{Deserialize, Serialize};

#[cfg(target_os = "macos")]
mod macos;

/// One permission's state. Accessibility is a plain granted/not (`AXIsProcessTrusted`) so it
/// rides as a bool on [`Permissions`]; Automation is per-target and genuinely tri-state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum PermissionState {
    Granted,
    Denied,
    NotDetermined,
    /// Not a macOS build (Linux CI / dev) — the grant doesn't apply.
    NotApplicable,
}

/// Snapshot of both capture permissions for the UI.
#[derive(Debug, Clone, Serialize, Deserialize, specta::Type)]
pub struct Permissions {
    pub accessibility: bool,
    pub automation: PermissionState,
}

/// Which System Settings → Privacy pane a request should open.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, specta::Type)]
#[serde(rename_all = "snake_case")]
pub enum SettingsPane {
    Accessibility,
    Automation,
}

// OSStatus codes from `AEDeterminePermissionToAutomateTarget` that we classify (the rest —
// -1744 undetermined, -600 not-running — are inconclusive). Kept here so the reduction below is
// unit-tested without the AppleEvents FFI.
#[cfg(any(target_os = "macos", test))]
pub(crate) const AE_AUTHORIZED: i32 = 0;
#[cfg(any(target_os = "macos", test))]
pub(crate) const AE_NOT_PERMITTED: i32 = -1743;

/// Reduce the per-browser Automation OSStatus codes to one state: any authorized browser ⇒
/// `Granted`; else any explicitly denied ⇒ `Denied`; else (undetermined / not-running / unknown)
/// ⇒ `NotDetermined`. Pure + platform-agnostic so the policy is tested without the FFI.
#[cfg(any(target_os = "macos", test))]
pub(crate) fn aggregate_automation(statuses: impl IntoIterator<Item = i32>) -> PermissionState {
    let mut granted = false;
    let mut denied = false;
    for status in statuses {
        match status {
            AE_AUTHORIZED => granted = true,
            AE_NOT_PERMITTED => denied = true,
            _ => {}
        }
    }
    if granted {
        PermissionState::Granted
    } else if denied {
        PermissionState::Denied
    } else {
        PermissionState::NotDetermined
    }
}

/// Current state of both permissions.
pub(crate) fn status() -> Permissions {
    #[cfg(target_os = "macos")]
    {
        Permissions {
            accessibility: macos::accessibility_trusted(),
            automation: macos::automation_state(),
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        Permissions {
            accessibility: true,
            automation: PermissionState::NotApplicable,
        }
    }
}

/// Prompt for Accessibility and open its System Settings → Privacy pane.
pub(crate) fn request_accessibility() {
    #[cfg(target_os = "macos")]
    macos::request_accessibility();
}

/// Trigger the Automation consent prompt for running browsers and open its Privacy pane.
pub(crate) fn request_automation() {
    #[cfg(target_os = "macos")]
    macos::request_automation();
}

/// Open a System Settings → Privacy pane directly.
pub(crate) fn open_settings(pane: SettingsPane) {
    #[cfg(target_os = "macos")]
    macos::open_settings(pane);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = pane;
    }
}

/// Whether Accessibility is granted (macOS only) — capture uses this for its degraded-mode log.
#[cfg(target_os = "macos")]
pub(crate) fn accessibility_trusted() -> bool {
    macos::accessibility_trusted()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn permission_state_serializes_snake_case() {
        assert_eq!(
            serde_json::to_string(&PermissionState::Granted).unwrap(),
            "\"granted\""
        );
        assert_eq!(
            serde_json::to_string(&PermissionState::NotDetermined).unwrap(),
            "\"not_determined\""
        );
        assert_eq!(
            serde_json::to_string(&PermissionState::NotApplicable).unwrap(),
            "\"not_applicable\""
        );
    }

    #[test]
    fn aggregate_prefers_granted_then_denied_then_undetermined() {
        assert_eq!(aggregate_automation([]), PermissionState::NotDetermined);
        assert_eq!(
            aggregate_automation([AE_AUTHORIZED]),
            PermissionState::Granted
        );
        assert_eq!(
            aggregate_automation([AE_NOT_PERMITTED]),
            PermissionState::Denied
        );
        // An authorized browser wins over a denied one (the URL feature works for at least one).
        assert_eq!(
            aggregate_automation([AE_NOT_PERMITTED, AE_AUTHORIZED]),
            PermissionState::Granted
        );
        // Undetermined (-1744) / not-running (-600) are inconclusive, not a denial.
        assert_eq!(
            aggregate_automation([-1744, -600]),
            PermissionState::NotDetermined
        );
    }

    #[cfg(not(target_os = "macos"))]
    #[test]
    fn non_macos_stub_reports_not_applicable() {
        let permissions = status();
        assert!(permissions.accessibility);
        assert_eq!(permissions.automation, PermissionState::NotApplicable);
    }
}
