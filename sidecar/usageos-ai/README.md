# usageos-ai — recap narration sidecar

The **only Swift in UsageOS** (CLAUDE.md / D16). A headless command-line binary that reaches
Apple's **FoundationModels** to phrase the day's recap. Rust computes every number (hard
rule 6 / C9); this binary only narrates them, and the Rust template (D48) is the
always-available fallback when the model is unavailable, refuses, or mangles output (C5).

Productionized from `spikes/foundation-models/` (verdict: viable — see that README for the
latency/quality findings). macOS 26 + Apple Intelligence only.

## Protocol (line-delimited JSON over stdio, C2/C4/C6)

One JSON object per line, both directions. Stateless per request — a fresh
`LanguageModelSession` each call (C2), so a crash restarts transparently.

```
in   {"prompt": "<the Rust-formatted facts; newlines escaped>"}
out  {"status":"ok","text":"<prose>","ms":"1234"}
     {"status":"unavailable","detail":"appleIntelligenceNotEnabled"}   → Rust template
     {"status":"error","detail":"<GenerationError>"}                   → Rust template
     {"status":"error","detail":"malformed-request"}                   → Rust template
```

Rust branches on `status`, never on free text (C4). Anything but `ok` (incl. spawn failure,
timeout, non-zero exit) routes to the template (C5).

**stdout is unbuffered on purpose.** As a Tauri sidecar child our stdout is a pipe, not a
TTY, so Swift's `print` would fully buffer and the caller's per-line read would hang. We
write straight to the file descriptor via `FileHandle`.

## Modes

| invocation | behavior |
|---|---|
| `usageos-ai` / `usageos-ai --serve` | read stdin lines until EOF, one response per line. Rust's one-shot call writes a single line then closes stdin, so the process exits after one recap. |
| `usageos-ai --prewarm` | availability gate + `prewarm()` to warm the shared on-device model at app launch, emit one status line, exit. Best-effort. |

## Build

```sh
../build.sh          # swift build -c release → src-tauri/binaries/usageos-ai-$TARGET_TRIPLE
```

Local sanity check (needs Apple Intelligence on):

```sh
echo '{"prompt":"The day'\''s facts:\n- Total active time: 4 hours 53 minutes\n- Leading category: Work, 3 hours 10 minutes"}' \
  | swift run usageos-ai
```

## Security (C8 / hard rule 1)

`entitlements.plist` is deliberately empty — no `com.apple.security.network.client`, so the
"nothing leaves the machine" guarantee is enforced and auditable, not merely observed. The
Tauri capability scopes spawn to exactly this one named sidecar (`sidecar: true`), never a
general `shell:allow-execute`.
