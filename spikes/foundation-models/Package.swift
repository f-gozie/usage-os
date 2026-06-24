// swift-tools-version: 6.0
//
// foundation-models — Phase-3 spike (D9/D16). ISOLATED standalone Swift CLI, NOT wired
// into Tauri yet. Build/run from this dir: `swift run usageos-ai` (or `--serve` for the
// stdio loop). Subject: prove a headless Swift command-line binary can reach Apple's
// FoundationModels on this machine — availability gate, a single @Generable structured
// recap call (prose only — hard rule 6), latency/quality — before building the real
// sidecar + Rust wiring. macOS 26 + Apple Intelligence only.
import PackageDescription

let package = Package(
    name: "usageos-ai",
    platforms: [.macOS("26.0")],
    targets: [
        .executableTarget(name: "usageos-ai", path: "Sources/usageos-ai")
    ]
)
