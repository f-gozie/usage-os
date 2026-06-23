//! Terminal front-tab cwd — the project signal for terminals (port of
//! `spikes/proc-cwd` + front-tab pid selection).
//!
//! cwd is read with `proc_pidinfo(PROC_PIDVNODEPATHINFO)` (proven non-root,
//! unsandboxed, no TCC grant — R22). The hard part the spike left open is *which*
//! pid: a terminal has one shell per tab/pane and the front tab isn't exposed by a
//! clean API. iTerm2 gives the cwd directly via AppleScript (reliable); other
//! terminals fall back to a best-effort proc heuristic. `None` → the span abstains
//! (D30), which degrades cleanly. **The fallback accuracy is on-device-verify.**

use std::ffi::c_void;
use std::process::Command;

/// Resolve the cwd of the front terminal session, or `None`.
pub fn front_cwd(bundle_id: &str, terminal_pid: i32) -> Option<String> {
    match bundle_id {
        "com.googlecode.iterm2" => iterm2_cwd().or_else(|| newest_child_cwd(terminal_pid)),
        "com.apple.Terminal" => newest_child_cwd(terminal_pid),
        _ => None, // not a known terminal
    }
}

/// iTerm2 exposes the session's path directly (most reliable when shell
/// integration is on). Requires Automation consent for iTerm2.
fn iterm2_cwd() -> Option<String> {
    let script = "tell application \"iTerm2\" to tell current window \
                  to tell current session to get variable named \"session.path\"";
    let out = Command::new("/usr/bin/osascript")
        .arg("-e")
        .arg(script)
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let s = String::from_utf8_lossy(&out.stdout).trim().to_string();
    if s.is_empty() || s == "missing value" {
        None
    } else {
        Some(s)
    }
}

/// Best-effort fallback: among the terminal's direct child processes (≈ one shell
/// per tab/pane, found via `pgrep -P`), return the highest-pid one with a readable
/// cwd — a rough proxy for the most recently opened tab. Front-tab selection has no
/// clean public API; a wrong/None result just abstains (D30). [on-device verify]
fn newest_child_cwd(terminal_pid: i32) -> Option<String> {
    let mut children = child_pids(terminal_pid);
    children.sort_unstable();
    children.into_iter().rev().find_map(read_cwd)
}

/// Direct child pids via `/usr/bin/pgrep -P <pid>` — avoids fragile proc-listing FFI.
fn child_pids(parent: i32) -> Vec<i32> {
    let out = match Command::new("/usr/bin/pgrep")
        .arg("-P")
        .arg(parent.to_string())
        .output()
    {
        Ok(o) if o.status.success() => o.stdout,
        _ => return Vec::new(),
    };
    String::from_utf8_lossy(&out)
        .lines()
        .filter_map(|l| l.trim().parse::<i32>().ok())
        .collect()
}

/// cwd of `pid` via `proc_pidinfo(PROC_PIDVNODEPATHINFO)` (ported from the spike).
fn read_cwd(pid: i32) -> Option<String> {
    // SAFETY: a C struct valid all-zero; we hand the kernel a correctly sized
    // buffer and read back the cwd path it fills in.
    let mut info: libc::proc_vnodepathinfo = unsafe { std::mem::zeroed() };
    let sz = std::mem::size_of::<libc::proc_vnodepathinfo>() as libc::c_int;
    // SAFETY: `info` is live and `sz` matches its size.
    let ret = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDVNODEPATHINFO,
            0,
            &mut info as *mut _ as *mut c_void,
            sz,
        )
    };
    if ret == sz {
        Some(vip_path_to_string(&info.pvi_cdir.vip_path))
    } else {
        None
    }
}

/// `vip_path` is `[[c_char; 32]; 32]` (a 1024-byte MAXPATHLEN buffer split to
/// satisfy old rustc); the bytes are contiguous, so read them flat to the NUL.
fn vip_path_to_string(vip: &[[libc::c_char; 32]; 32]) -> String {
    let ptr = vip.as_ptr() as *const u8;
    // SAFETY: 32*32 contiguous bytes, exactly the declared array size.
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(ptr, 32 * 32) };
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}
