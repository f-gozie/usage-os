//! The production [`Narrator`]: spawns the `usageos-ai` Foundation Models sidecar (the Swift
//! binary built from `sidecar/usageos-ai/`) and reads one tagged JSON line back over stdio
//! (C2/C4/C6/C7). It owns no fallback — [`super::build_recap`] renders the template on any
//! `Err` (C5); this just maps every non-`ok` outcome to a typed [`AiError`].
//!
//! Spawn is **one-shot per recap** (stateless — C2): the spike's persistence across many
//! round-trips is unproven (foundation-models.md open-Q12), and a fresh process makes a
//! crashed sidecar transparent. We write one request line, close stdin (by dropping the
//! child), and take the first response line.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use tauri::AppHandle;
use tauri_plugin_shell::process::CommandEvent;
use tauri_plugin_shell::ShellExt;

use super::{AiError, Narrator};

/// The sidecar program name passed to `shell().sidecar(...)`. It must be the **basename** Tauri
/// places next to the executable: the bundler/dev-copy strips both the `binaries/` prefix and
/// the `-$TARGET_TRIPLE` suffix, and `new_sidecar` joins the name *literally* to the exe dir
/// (it does NOT re-append the triple). So `"binaries/usageos-ai"` resolves to the non-existent
/// `<exe_dir>/binaries/usageos-ai` and fails with ENOENT — it must be just `"usageos-ai"`.
/// (The externalBin entry in tauri.conf.json stays `binaries/usageos-ai`, the *source* path.)
const SIDECAR: &str = "usageos-ai";

/// Per-recap timeout (C7). The model is ~5 s cold / ~1–2 s warm; this is generous headroom so
/// a wedged call falls back to the template instead of hanging the (already-lazy) recap.
const RECAP_TIMEOUT: Duration = Duration::from_secs(20);

/// One request line: `{"prompt": "<the Rust-formatted facts>"}`. The model only phrases it —
/// every number is already a string in the prompt (hard rule 6 / C9).
#[derive(Serialize)]
struct SidecarRequest<'a> {
    prompt: &'a str,
}

/// One response line. `status` is the only field Rust branches on (C4). `text` is present on
/// `ok`; `detail` carries the reason on `unavailable`/`error`.
#[derive(Deserialize)]
struct SidecarResponse {
    status: String,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    detail: Option<String>,
}

/// The production narrator. Holds an [`AppHandle`] (cheap to clone) because the [`Narrator`]
/// signature is `&self`-only, and reaching the shell plugin needs the handle.
pub struct SidecarNarrator {
    app: AppHandle,
}

impl SidecarNarrator {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl Narrator for SidecarNarrator {
    async fn narrate(&self, prompt: &str) -> Result<String, AiError> {
        let request = serde_json::to_string(&SidecarRequest { prompt })
            .map_err(|e| AiError::Model(format!("encode: {e}")))?;
        let line = format!("{request}\n");

        let read = async {
            let (mut rx, mut child) = self
                .app
                .shell()
                .sidecar(SIDECAR)
                .map_err(|e| AiError::Unavailable(format!("sidecar: {e}")))?
                .spawn()
                .map_err(|e| AiError::Unavailable(format!("spawn: {e}")))?;

            child
                .write(line.as_bytes())
                .map_err(|e| AiError::Model(format!("write: {e}")))?;

            // C6: stdout arrives as arbitrary byte chunks — reassemble into lines and take the
            // first complete one (the sidecar emits exactly one response line per request).
            let mut buf = String::new();
            while let Some(event) = rx.recv().await {
                match event {
                    CommandEvent::Stdout(bytes) => {
                        buf.push_str(&String::from_utf8_lossy(&bytes));
                        if let Some(nl) = buf.find('\n') {
                            let json: String = buf.drain(..=nl).collect();
                            return parse_response(json.trim());
                        }
                    }
                    // If the child dies without a response, return now instead of waiting out
                    // the whole timeout.
                    CommandEvent::Terminated(t) => {
                        return Err(AiError::Model(format!(
                            "sidecar exited early (code {:?})",
                            t.code
                        )));
                    }
                    _ => {}
                }
            }
            Err(AiError::Model("no response".into()))
        };

        // C7: a wedged model call must not hang the on-open render — time out, then fall back.
        match tokio::time::timeout(RECAP_TIMEOUT, read).await {
            Ok(result) => result,
            Err(_) => Err(AiError::Model("timeout".into())),
        }
    }
}

/// Branch on the explicit status tag, never free text (C4). Anything but `ok` (or `ok`
/// without usable prose) returns a typed error so [`super::build_recap`] renders the
/// template (C5).
fn parse_response(line: &str) -> Result<String, AiError> {
    let resp: SidecarResponse =
        serde_json::from_str(line).map_err(|e| AiError::Model(format!("decode: {e}")))?;
    match resp.status.as_str() {
        "ok" => resp
            .text
            .filter(|t| !t.trim().is_empty())
            .ok_or_else(|| AiError::Model("ok without text".into())),
        "unavailable" => Err(AiError::Unavailable(resp.detail.unwrap_or_default())),
        _ => Err(AiError::Model(
            resp.detail.unwrap_or_else(|| resp.status.clone()),
        )),
    }
}

/// Best-effort: warm the shared on-device model at launch via `--prewarm`, off the UI path,
/// so the first real recap is closer to warm latency. Errors are swallowed — prewarm is an
/// optimization, never a correctness requirement (the template always covers a cold/missing
/// model). Spawn the future with [`tauri::async_runtime::spawn`]; don't block startup.
pub async fn prewarm(app: &AppHandle) {
    if let Ok((mut rx, _child)) = app
        .shell()
        .sidecar(SIDECAR)
        .and_then(|cmd| cmd.args(["--prewarm"]).spawn())
    {
        // Drain briefly so the child runs to completion rather than being torn down early;
        // we don't care about the contents.
        let _ =
            tokio::time::timeout(RECAP_TIMEOUT, async { while rx.recv().await.is_some() {} }).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ok_into_prose() {
        let out = parse_response(
            r#"{"status":"ok","text":"You spent the day on usage_os.","ms":"1200"}"#,
        );
        assert_eq!(out.unwrap(), "You spent the day on usage_os.");
    }

    #[test]
    fn ok_without_text_is_an_error() {
        // `ok` but no/blank prose must not surface a broken recap — it falls back (C5).
        assert!(matches!(
            parse_response(r#"{"status":"ok","text":"   "}"#),
            Err(AiError::Model(_))
        ));
        assert!(matches!(
            parse_response(r#"{"status":"ok"}"#),
            Err(AiError::Model(_))
        ));
    }

    #[test]
    fn unavailable_maps_to_unavailable_with_reason() {
        match parse_response(r#"{"status":"unavailable","detail":"appleIntelligenceNotEnabled"}"#) {
            Err(AiError::Unavailable(reason)) => assert_eq!(reason, "appleIntelligenceNotEnabled"),
            other => panic!("expected Unavailable, got {other:?}"),
        }
    }

    #[test]
    fn error_and_malformed_map_to_model_error() {
        assert!(matches!(
            parse_response(r#"{"status":"error","detail":"guardrailViolation"}"#),
            Err(AiError::Model(_))
        ));
        assert!(matches!(
            parse_response("not json at all"),
            Err(AiError::Model(_))
        ));
    }

    #[test]
    fn request_serializes_with_escaped_newlines() {
        // The multi-line facts prompt must ride as ONE JSON line (escaped \n) — the protocol
        // is line-delimited (C6); a raw newline would split one request into two.
        let json = serde_json::to_string(&SidecarRequest {
            prompt: "line one\nline two",
        })
        .unwrap();
        assert_eq!(json, r#"{"prompt":"line one\nline two"}"#);
        assert!(!json.contains('\n'));
    }
}
