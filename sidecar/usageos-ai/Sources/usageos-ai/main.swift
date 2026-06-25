// usageos-ai — the recap-narration sidecar (D9/D16). Spawned headlessly by Tauri; talks
// line-delimited JSON over stdio (C2/C4/C6). One request → one response, stateless per
// recap (C2): a crashed sidecar restarts transparently. Numbers are pre-formatted by Rust
// and reproduced verbatim — the model only phrases (hard rule 6 / C9).
//
// Protocol (one JSON object per line, both ways):
//   in   {"prompt": "<the Rust-formatted facts, newlines escaped>"}
//   out  {"status":"ok","text":"<prose>","ms":"1234"}
//        {"status":"unavailable","detail":"appleIntelligenceNotEnabled"}   (C3/C5 → template)
//        {"status":"error","detail":"<GenerationError>"}                   (refusal/overflow → template)
//
// Modes:
//   (default / --serve)  read stdin lines until EOF, one response per line. Rust uses this
//                        one-shot: it writes a single line, then closes stdin so we exit.
//   --prewarm            availability gate + `prewarm()` to warm the shared on-device model
//                        (the load is system-level), emit one status line, exit. Best-effort.
//
// stdout MUST stay unbuffered: as a Tauri child our stdout is a PIPE, not a TTY, so Swift's
// `print` would FULLY buffer and the caller's per-line read would hang. We write straight to
// the file descriptor via `FileHandle`, which is unbuffered (the spike ran in a TTY and got
// away with `print`; the real sidecar cannot).

import Foundation
import FoundationModels

// MARK: - Output (PROSE ONLY)

// No numeric field — Guided Generation guarantees structure, not semantics, so a number
// field is a footgun that lets the model fabricate a count (hard rule 6 / C9). Numbers ride
// in the prompt as strings; the model returns only narration.
@Generable
struct RecapProse {
    @Guide(description: "2 to 4 calm, factual sentences narrating the day. Do not invent, compute, or alter any number.")
    var text: String
}

// Short and fixed (C11) — every instruction token competes with the prose for the shared
// context budget. The verbatim-names + second-person rules are the spike's proven fix for
// proper-noun mangling ("nudge"→"nudged", "usage_os"→"the operating system"). No personal
// or project names are baked in here (OSS hygiene) — names arrive only as runtime facts.
let recapInstructions = """
Narrate the day's recap from the given facts in 2 to 4 calm, factual sentences. \
Rules: Never compute, invent, or alter numbers. Treat every category and project \
name as a literal label — reproduce it EXACTLY, verbatim, even if it looks like an \
ordinary word, code, or contains underscores; never turn a name into a verb or \
reinterpret it. Do not translate, expand, interpret, or add anything not in the \
facts. Do not mention or open with the day itself — no "today", "yesterday", weekday, \
or date (the day is shown separately); start directly with the activity and narrate in \
the past tense (e.g. "You spent…"). A time-of-day phrase already in the facts (like \
"in the morning") is fine to keep. Write in second person ("you"), plainly — no flourishes.
"""

/// Write one JSON object as a single unbuffered stdout line (the line-delimited protocol;
/// C4/C6). Unbuffered because our stdout is a pipe — see the file header.
func emitLine(_ obj: [String: String]) {
    guard let data = try? JSONSerialization.data(withJSONObject: obj),
          var str = String(data: data, encoding: .utf8)
    else {
        // Last-ditch: a hand-rolled error line so Rust still gets a parseable status.
        FileHandle.standardOutput.write(Data(#"{"status":"error","detail":"encode-failed"}"#.utf8))
        FileHandle.standardOutput.write(Data("\n".utf8))
        return
    }
    str += "\n"
    FileHandle.standardOutput.write(Data(str.utf8))
}

/// Human-readable diagnostics go to stderr (unbuffered), never stdout — stdout is the protocol.
func logErr(_ msg: String) {
    FileHandle.standardError.write(Data((msg + "\n").utf8))
}

// MARK: - Availability (C3)

/// Availability first, every run, serialized to a stable reason Rust can branch on (never
/// free text). `reason` is empty when available.
func availability() -> (available: Bool, reason: String) {
    switch SystemLanguageModel.default.availability {
    case .available:
        return (true, "")
    case .unavailable(let reason):
        return (false, String(describing: reason))
    }
}

// MARK: - One recap (C2: fresh session per request, stateless)

func runRecap(prompt: String) async {
    let (available, reason) = availability()
    guard available else {
        emitLine(["status": "unavailable", "detail": reason]) // C4/C5 → Rust template fallback
        return
    }

    let session = LanguageModelSession(instructions: recapInstructions)
    do {
        let start = Date()
        // Low temperature: faithful, not creative — minimizes embellishment/mangling.
        let options = GenerationOptions(temperature: 0.2)
        let result = try await session.respond(
            to: prompt, generating: RecapProse.self, options: options)
        let ms = Int(Date().timeIntervalSince(start) * 1000)
        emitLine(["status": "ok", "text": result.content.text, "ms": String(ms)])
    } catch let error as LanguageModelSession.GenerationError {
        // Guardrail refusal / context overflow / etc. → tagged error → Rust template fallback (C5).
        emitLine(["status": "error", "detail": String(describing: error)])
    } catch {
        emitLine(["status": "error", "detail": "\(error)"])
    }
}

/// Pull the prompt out of one `{"prompt": "..."}` request line. Returns nil (caller emits a
/// tagged error) on anything malformed — the protocol is strict so Rust always gets a status.
func decodePrompt(_ line: String) -> String? {
    guard let data = line.data(using: .utf8),
          let obj = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
          let prompt = obj["prompt"] as? String
    else {
        return nil
    }
    return prompt
}

// MARK: - Prewarm (best-effort)

/// Warm the shared on-device model at app launch so the first real recap is ~warm, not the
/// ~5s cold load (the model load is system-level, so a separate prewarm process still helps).
func runPrewarm() {
    let (available, reason) = availability()
    guard available else {
        emitLine(["status": "unavailable", "detail": reason])
        return
    }
    let session = LanguageModelSession(instructions: recapInstructions)
    session.prewarm()
    emitLine(["status": "ok", "detail": "prewarmed"])
}

// MARK: - Entry

let args = CommandLine.arguments
logErr("[usageos-ai] availability=\(availability().available ? "available" : availability().reason)")

if args.contains("--prewarm") {
    runPrewarm()
} else {
    // Default + `--serve`: one JSON line in → one tagged JSON line out, until stdin EOF.
    // Rust's one-shot call writes a single line then closes stdin, so we exit after one.
    while let line = readLine(strippingNewline: true) {
        if line.isEmpty { continue }
        if let prompt = decodePrompt(line) {
            await runRecap(prompt: prompt)
        } else {
            emitLine(["status": "error", "detail": "malformed-request"]) // C4
        }
    }
}
