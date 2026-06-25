// swift-tools-version: 6.0
//
// usageos-ai — the recap-narration sidecar (D9/D16, Phase 3 step 2). The ONLY Swift in
// the project (CLAUDE.md): a headless command-line binary that reaches Apple's
// FoundationModels and phrases the day's pre-computed facts into calm prose. It never
// computes a number (hard rule 6) — Rust formats every figure into the prompt.
//
// Build for the Tauri sidecar slot with `../build.sh` (emits
// `src-tauri/binaries/usageos-ai-$TARGET_TRIPLE`). Local one-shot check:
//   echo '{"prompt":"..."}' | swift run usageos-ai
// macOS 26 + Apple Intelligence only; everything degrades to the Rust template otherwise.
import PackageDescription

let package = Package(
    name: "usageos-ai",
    platforms: [.macOS("26.0")],
    targets: [
        .executableTarget(name: "usageos-ai", path: "Sources/usageos-ai")
    ]
)
