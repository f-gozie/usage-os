// Phase-3 spike — Apple FoundationModels from a headless Swift CLI (D9/D16).
//
// Proves (or disproves) the make-or-break unknowns for the recap sidecar:
//   1. A plain command-line binary (no .app bundle) compiles + links FoundationModels
//      against the real macOS 26 SDK — the standard's API spellings are provisional (C1).
//   2. `SystemLanguageModel.default.availability` reports this machine's real status
//      (is Apple Intelligence on?) and serializes to a stable tag (C3).
//   3. A single `@Generable` structured call returns PROSE ONLY — no numeric field, so
//      the model can't fabricate a count (hard rule 6 / C9) — with usable latency/quality.
//   4. The line-delimited JSON stdio protocol shape works (C2/C4/C6), via `--serve`.
//
// Run: `swift run usageos-ai`  (one-shot demo on sample facts)
//      `swift run usageos-ai -- --serve`  (read one JSON-facts line per stdin line)

import Foundation
import FoundationModels

// Output is PROSE ONLY. Numbers are pre-formatted into the prompt by Rust; the model is
// told never to compute or alter them (hard rule 6 / C9). Structure ≠ semantics.
@Generable
struct RecapProse {
    @Guide(description: "2 to 4 calm, factual sentences narrating the day. Do not invent, compute, or alter any number.")
    var text: String
}

/// Write one JSON object as a single stdout line (the line-delimited protocol; C4/C6).
func emitLine(_ obj: [String: String]) {
    if let data = try? JSONSerialization.data(withJSONObject: obj),
       let str = String(data: data, encoding: .utf8) {
        print(str)
    }
}

func logErr(_ msg: String) {
    FileHandle.standardError.write(Data((msg + "\n").utf8))
}

/// C3: availability first, serialized to a stable tag Rust can branch on (never free text).
func availabilityTag() -> (available: Bool, tag: String) {
    switch SystemLanguageModel.default.availability {
    case .available:
        return (true, "available")
    case .unavailable(let reason):
        return (false, "unavailable:\(String(describing: reason))")
    }
}

/// One stateless recap (C2): availability gate → structured call → tagged JSON line.
func runRecap(prompt: String) async {
    let (available, tag) = availabilityTag()
    guard available else {
        emitLine(["status": "unavailable", "detail": tag]) // C4/C5 → Rust template fallback
        return
    }

    let session = LanguageModelSession(
        instructions: """
        Narrate the day's recap from the given facts in 2 to 4 calm, factual sentences.
        Rules: Never compute, invent, or alter numbers. Treat every category and project \
        name as a literal label — reproduce it EXACTLY, verbatim, even if it looks like an \
        ordinary word, code, or contains underscores; never turn a name into a verb or \
        reinterpret it. Do not translate, expand, interpret, or add anything not in the \
        facts. Write in second person ("you"), plainly — no flourishes.
        """
    )

    do {
        let start = Date()
        // Low temperature: faithful, not creative — minimize embellishment/mangling.
        let options = GenerationOptions(temperature: 0.2)
        let result = try await session.respond(
            to: prompt, generating: RecapProse.self, options: options)
        let ms = Int(Date().timeIntervalSince(start) * 1000)
        emitLine(["status": "ok", "text": result.content.text, "ms": String(ms)])
    } catch let error as LanguageModelSession.GenerationError {
        emitLine(["status": "error", "detail": String(describing: error)]) // overflow/refusal → fallback
    } catch {
        emitLine(["status": "error", "detail": "\(error)"])
    }
}

// Sample RecapFacts, pre-formatted exactly as Rust would (numbers already strings; C9/C10).
let sampleFacts = """
Day's facts (already computed — do not change any number):
- Total active: 4h 53m
- Leading category: Work, 3h 10m
- Runner-up category: Browsing, 1h 30m
- Longest unbroken stretch: 1h 12m, in the morning
- Main project: usageos
"""

// Report availability to a human on stderr regardless of mode.
logErr("[availability] \(availabilityTag().tag)")

if CommandLine.arguments.contains("--serve") {
    // The real sidecar shape: one JSON-facts line in → one tagged JSON line out (C2/C6).
    while let line = readLine(strippingNewline: true) {
        await runRecap(prompt: line)
    }
} else {
    // One-shot demo on the sample facts.
    await runRecap(prompt: sampleFacts)
}
