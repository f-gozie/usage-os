//! Spike ④ — read another process's cwd via `proc_pidinfo`.
//!
//! The "project" axis of UsageOS infers which repo/codebase you're in. Editors
//! give it via window title (Spike #1) and browsers via URL (Spike ③) — but
//! **terminals** are the blind spot: their title is usually `zsh` / a hostname,
//! and the real signal is the shell's **current working directory**
//! (`…/projects/usage_os` → project `usage_os`).
//!
//! To get that, the capture layer would call
//! `proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, …)` to read the cwd of the terminal's
//! shell. The feasibility audit's R22 is the kill-switch question:
//!
//!   **Can a NON-root, UNSANDBOXED process read ANOTHER process's cwd this way,
//!   or does it return `EPERM`?** Sources confirm only the negative (a *sandboxed*
//!   process is blocked); the unsandboxed-no-root case is undocumented. If it's
//!   `EPERM`, the clean terminal-cwd branch dies and we fall back to per-app
//!   routes (iTerm2's AppleScript `path`, Terminal's tty→cwd) — R24.
//!
//! This binary settles it empirically. It reads the cwd of:
//!   1. **itself** (baseline — must match `std::env::current_dir`),
//!   2. a **child it spawns** with a known cwd (correctness on a controlled pid),
//!   3. any **pids passed on argv** — real terminal shells / GUI apps it did NOT
//!      spawn (the genuine "another process" test).
//!
//! No network, no disk writes. No `unwrap()`/`expect()`/`panic!` in the logic.

#![cfg_attr(
    not(test),
    deny(clippy::unwrap_used, clippy::expect_used, clippy::panic)
)]

use std::ffi::c_void;
use std::process::Command;

fn main() {
    // SAFETY: argless C calls returning the caller's own ids.
    let (pid, euid) = unsafe { (libc::getpid(), libc::geteuid()) };

    println!(
        "proc-cwd spike ④ — read another process's cwd via proc_pidinfo(PROC_PIDVNODEPATHINFO)"
    );
    println!(
        "this process: pid={pid}  euid={euid} ({})  sandbox=NO (plain CLI, no entitlements)\n",
        if euid == 0 { "ROOT" } else { "non-root" }
    );

    // ── 1. self ───────────────────────────────────────────────────────────────
    let self_expected = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());
    print_result(
        "self",
        pid,
        &format!("(env::current_dir = {self_expected})"),
    );

    // ── 2. a spawned child with a known cwd ────────────────────────────────────
    let target_dir = "/private/tmp";
    match Command::new("/bin/sleep")
        .arg("30")
        .current_dir(target_dir)
        .spawn()
    {
        Ok(mut child) => {
            let cpid = child.id() as i32;
            print_result(
                "spawned child (/bin/sleep)",
                cpid,
                &format!("(expected cwd = {target_dir})"),
            );
            let _ = child.kill();
            let _ = child.wait();
        }
        Err(e) => println!("  (could not spawn child for the controlled test: {e})"),
    }

    // ── 3. pids passed on argv — real processes we did NOT spawn ───────────────
    let argv_pids: Vec<i32> = std::env::args()
        .skip(1)
        .filter_map(|a| a.parse::<i32>().ok())
        .collect();

    if argv_pids.is_empty() {
        println!(
            "\n(no argv pids given — pass real terminal-shell pids to test the cross-process case,\n \
             e.g.  ./proc-cwd $(pgrep -x zsh | head -4) )"
        );
    } else {
        println!("\nargv pids (processes we did NOT spawn — the real R22 test):");
        for p in argv_pids {
            print_result("argv", p, "");
        }
    }
}

/// Read `pid`'s cwd, print a classified line including the target's exe path.
fn print_result(label: &str, pid: i32, note: &str) {
    let exe = proc_path(pid);
    match read_cwd(pid) {
        CwdResult::Ok(cwd) => {
            println!("  ✅ {label:<26} pid={pid:<7} cwd={cwd:<45} [{exe}] {note}");
        }
        CwdResult::Err(errno) => {
            println!(
                "  ❌ {label:<26} pid={pid:<7} FAILED: {:<28} [{exe}] {note}",
                errno_desc(errno)
            );
        }
    }
}

/// The outcome of one cwd read.
enum CwdResult {
    Ok(String),
    /// errno captured immediately after a failed `proc_pidinfo`.
    Err(i32),
}

/// `proc_pidinfo(pid, PROC_PIDVNODEPATHINFO, …)` → the process's current dir.
fn read_cwd(pid: i32) -> CwdResult {
    // SAFETY: a C struct that is valid all-zero; we hand the kernel a correctly
    // sized buffer and read back the cwd path it fills in.
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
        CwdResult::Ok(vip_path_to_string(&info.pvi_cdir.vip_path))
    } else {
        // proc_pidinfo sets errno on failure (EPERM / ESRCH / …); a short read
        // surfaces as errno==0, which errno_desc labels distinctly.
        CwdResult::Err(std::io::Error::last_os_error().raw_os_error().unwrap_or(0))
    }
}

/// `vip_path` is declared as `[[c_char; 32]; 32]` (a 1024-byte MAXPATHLEN buffer
/// split to satisfy old rustc) — the bytes are contiguous, so read them flat to
/// the first NUL.
fn vip_path_to_string(vip: &[[libc::c_char; 32]; 32]) -> String {
    let ptr = vip.as_ptr() as *const u8;
    // SAFETY: 32*32 contiguous bytes, exactly the declared array size.
    let bytes: &[u8] = unsafe { std::slice::from_raw_parts(ptr, 32 * 32) };
    let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
    String::from_utf8_lossy(&bytes[..end]).into_owned()
}

/// Best-effort executable path of `pid`, for context in the output.
fn proc_path(pid: i32) -> String {
    let mut buf = [0u8; 4096];
    // SAFETY: buffer + length match; proc_pidpath writes up to `len` bytes.
    let ret = unsafe { libc::proc_pidpath(pid, buf.as_mut_ptr() as *mut c_void, buf.len() as u32) };
    if ret > 0 {
        let end = (ret as usize).min(buf.len());
        let slice = &buf[..buf.iter().take(end).position(|&b| b == 0).unwrap_or(end)];
        let full = String::from_utf8_lossy(slice).into_owned();
        // Show just the basename to keep the line short.
        full.rsplit('/').next().unwrap_or(&full).to_string()
    } else {
        "?".to_string()
    }
}

fn errno_desc(errno: i32) -> String {
    let name = match errno {
        libc::EPERM => "EPERM not permitted",
        libc::ESRCH => "ESRCH no such process",
        libc::EACCES => "EACCES access denied",
        libc::EINVAL => "EINVAL invalid arg",
        0 => "short read (errno 0)",
        _ => "other",
    };
    format!("{name} (errno={errno})")
}
