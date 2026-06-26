//! Dev CLI — seed a real on-disk WAL `usage.db` with a synthetic history for the Phase-6
//! perf/soak harness (the webview-render feel test + the later release soak). Built only with
//! `--features perf` (see `[[bin]] required-features` in Cargo.toml), so a normal
//! build/`tauri build` never sees it.
//!
//! Usage:
//!   cargo run --features perf --bin seed_db -- --months 24 --out /tmp/usage-24mo.db
//!
//! Flags: --months N (default 24) | --days N (overrides --months) | --out PATH
//!        (default ./usageos-seed.db) | --seed N | --max N (spans/day cap) | --force
//!
//! To load it in the app: quit UsageOS, then copy the produced file over the app's database
//! (Settings → Your data → "Show in Finder" reveals it, named `usage.db`), and relaunch.

use std::path::PathBuf;
use std::process::ExitCode;

use usage_os_lib::db;
use usage_os_lib::perf::{self, SeedConfig, DEFAULT_END_DAY_START};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("seed_db: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut months: i64 = 24;
    let mut days: Option<i64> = None;
    let mut out = PathBuf::from("./usageos-seed.db");
    let mut seed: u64 = 0xC0FFEE;
    let mut max_spans: usize = 2000;
    let mut force = false;

    let mut args = std::env::args().skip(1);
    while let Some(flag) = args.next() {
        let mut take = |name: &str| -> Result<String, String> {
            args.next().ok_or_else(|| format!("{name} needs a value"))
        };
        match flag.as_str() {
            "--months" => months = parse_i64(&take("--months")?)?,
            "--days" => days = Some(parse_i64(&take("--days")?)?),
            "--out" => out = PathBuf::from(take("--out")?),
            "--seed" => seed = parse_u64(&take("--seed")?)?,
            "--max" => max_spans = parse_i64(&take("--max")?)?.max(1) as usize,
            "--force" => force = true,
            "-h" | "--help" => {
                println!(
                    "seed_db --months N [--days N] [--out PATH] [--seed N] [--max N] [--force]"
                );
                return Ok(());
            }
            other => return Err(format!("unknown flag {other:?} (try --help)")),
        }
    }

    let days = days.unwrap_or(months * 30).max(1);
    if out.exists() && !force {
        return Err(format!(
            "{} already exists — pass --force to overwrite (never the live app DB!)",
            out.display()
        ));
    }
    if force && out.exists() {
        // Clear prior content (incl. WAL sidecar files) so the seed starts clean.
        let _ = std::fs::remove_file(&out);
        let _ = std::fs::remove_file(out.with_extension("db-wal"));
        let _ = std::fs::remove_file(out.with_extension("db-shm"));
    }

    let db_conn = db::init_database(&out).map_err(|e| format!("open db: {e}"))?;
    let conn = db_conn.lock().map_err(|_| "db lock poisoned".to_string())?;

    let cfg = SeedConfig {
        days,
        end_day_start: DEFAULT_END_DAY_START,
        max_spans_per_day: max_spans,
        seed,
    };
    println!(
        "seeding {days} days (~{} months) into {} …",
        days / 30,
        out.display()
    );
    let stats = perf::seed(&conn, &cfg).map_err(|e| format!("seed: {e}"))?;
    println!(
        "done — {} events across {} days, {} projects. File: {}",
        stats.events,
        stats.days,
        stats.projects,
        out.display()
    );
    println!(
        "load it: quit UsageOS, copy this over the app's usage.db (Settings → Your data → Show in Finder), relaunch."
    );
    Ok(())
}

fn parse_i64(s: &str) -> Result<i64, String> {
    s.parse::<i64>()
        .map_err(|_| format!("not an integer: {s:?}"))
}

fn parse_u64(s: &str) -> Result<u64, String> {
    // Accept 0x-prefixed hex for the seed too.
    if let Some(hex) = s.strip_prefix("0x") {
        u64::from_str_radix(hex, 16).map_err(|_| format!("not a hex u64: {s:?}"))
    } else {
        s.parse::<u64>().map_err(|_| format!("not a u64: {s:?}"))
    }
}
