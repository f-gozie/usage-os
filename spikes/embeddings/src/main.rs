//! Embeddings spike — NLEmbedding via `objc2-natural-language` (D26).
//!
//! Proves (or disproves) the open questions that gate "embeddings in Rust":
//!   Q15 — `objc2-natural-language` 0.3.2 exposes a *callable* `NLEmbedding`, and
//!         it works **off the main thread** (the real enrichment pass is a worker
//!         thread — no AX-style main-thread/run-loop requirement allowed).
//!   Q16 — `sentenceEmbedding(for: .english)` returns a non-nil **512-d** vector
//!         with **no asset download**, sub-millisecond.
//!   R43 — accuracy: does cosine-KNN over a handful of exemplars per category
//!         actually sort realistic app/window titles into the right category,
//!         and **abstain** when the best match is weak?
//!
//! This needs **no Apple Intelligence** — NaturalLanguage ships in-OS. Build/run
//! from this dir (own `target/`, no `tauri dev` collision):
//!   cargo run --release

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("embeddings spike is macOS-only (Apple NaturalLanguage).");
}

#[cfg(target_os = "macos")]
fn main() {
    macos::run();
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2::rc::Retained;
    use objc2_foundation::{NSArray, NSNumber, NSString};
    use objc2_natural_language::NLEmbedding;
    use std::time::Instant;

    /// `NLLanguage` is an `NSString`-backed typed enum whose raw value is a BCP-47
    /// code; "en" == `NLLanguageEnglish`. Passing the raw string avoids depending
    /// on the constant's binding shape.
    fn load_model() -> Option<Retained<NLEmbedding>> {
        let lang = NSString::from_str("en");
        unsafe { NLEmbedding::sentenceEmbeddingForLanguage(&lang) }
    }

    /// Embed one string → `Vec<f32>` (None if the model returns nil for it).
    fn embed(model: &NLEmbedding, text: &str) -> Option<Vec<f32>> {
        let s = NSString::from_str(text);
        let arr: Retained<NSArray<NSNumber>> = unsafe { model.vectorForString(&s) }?;
        let n = arr.count();
        let mut v = Vec::with_capacity(n);
        for i in 0..n {
            v.push(arr.objectAtIndex(i).doubleValue() as f32);
        }
        Some(v)
    }

    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        let mut dot = 0.0f32;
        let mut na = 0.0f32;
        let mut nb = 0.0f32;
        for (x, y) in a.iter().zip(b.iter()) {
            dot += x * y;
            na += x * x;
            nb += y * y;
        }
        if na == 0.0 || nb == 0.0 {
            return 0.0;
        }
        dot / (na.sqrt() * nb.sqrt())
    }

    pub fn run() {
        println!("=== UsageOS embeddings spike — NLEmbedding via objc2-natural-language ===\n");

        // --- Q15/Q16: model loads, non-nil, dimension, latency, no download ---
        let t = Instant::now();
        let Some(model) = load_model() else {
            println!("FAIL Q16: sentenceEmbeddingForLanguage(en) returned nil — no English sentence model on this OS.");
            return;
        };
        let load_ms = t.elapsed();
        let dim = unsafe { model.dimension() };
        println!("PASS Q15/Q16: model loaded in {load_ms:?}; dimension = {dim} (expect 512)");

        let t = Instant::now();
        let Some(v) = embed(&model, "the quick brown fox jumps over the lazy dog") else {
            println!("FAIL: vectorForString returned nil for a plain English sentence.");
            return;
        };
        println!(
            "  first embed: {:?}, len = {}, sample = {:?}",
            t.elapsed(),
            v.len(),
            &v[..4.min(v.len())]
        );
        // Warm-call latency (what the enrichment pass actually pays per title).
        let t = Instant::now();
        let _ = embed(&model, "Cursor — db.rs — usage_os");
        println!("  warm embed latency = {:?}", t.elapsed());

        // --- Q15 (Send): create + use the model ENTIRELY on a worker thread ---
        let handle = std::thread::spawn(|| {
            let m = load_model()?;
            embed(&m, "embedded off the main thread").map(|v| v.len())
        });
        match handle.join() {
            Ok(Some(n)) => println!("PASS Q15(off-main-thread): worker-thread embed ok, len = {n}"),
            Ok(None) => println!("WARN Q15(off-main-thread): worker embed returned nil"),
            Err(_) => println!("FAIL Q15(off-main-thread): worker thread panicked"),
        }

        // --- R43: accuracy probe on the user's REAL five categories ---
        title_probe(&model);
        app_name_loo_probe(&model);
        app_name_centroid_loo(&model);
    }

    /// The user's real app→category map (read from their live DB 2026-06-24).
    /// Shared by both app-name probes so they test the same ground truth.
    fn real_app_map() -> Vec<(&'static str, &'static [&'static str])> {
        vec![
            (
                "Work",
                &[
                    "ChatGPT", "Claude", "Code", "Codex", "Cursor", "DataGrip", "Figma", "Finder",
                    "Ghostty", "Neovim", "Notion", "TablePlus", "Terminal", "Vim", "Warp", "Xcode",
                    "Zed", "iTerm", "usage-os",
                ][..],
            ),
            ("Browsing", &["Arc", "Brave", "Chrome", "Firefox", "Preview", "Safari"][..]),
            (
                "Messaging",
                &["Discord", "Mail", "Messages", "Slack", "Teams", "Telegram", "Zoom"][..],
            ),
            ("Entertainment", &["Netflix", "Reddit", "Steam", "YouTube"][..]),
            (
                "Personal",
                &[
                    "FaceTime", "Music", "Nudge", "Prime Video", "QuickTime Player", "Spotify",
                    "VLC", "WhatsApp",
                ][..],
            ),
        ]
    }

    /// Airtight the negative: instead of k=1 nearest neighbour, match each
    /// held-out app to the nearest CATEGORY CENTROID (mean of that category's
    /// other app vectors). If even centroids can't separate the categories, the
    /// problem is the signal (brand-name embeddings), not the matching scheme.
    fn app_name_centroid_loo(model: &NLEmbedding) {
        let truth = real_app_map();
        // (app, cat_index, embedding)
        let mut items: Vec<(&str, usize, Vec<f32>)> = Vec::new();
        for (ci, (_, apps)) in truth.iter().enumerate() {
            for &app in apps.iter() {
                if let Some(v) = embed(model, app) {
                    items.push((app, ci, v));
                }
            }
        }
        let dim = items.first().map(|(_, _, v)| v.len()).unwrap_or(0);

        let mut hits = 0u32;
        for (i, (_, actual_ci, vi)) in items.iter().enumerate() {
            // Build each category's centroid from every app EXCEPT i (true LOO).
            let mut best = (usize::MAX, f32::MIN);
            for (ci, _) in truth.iter().enumerate() {
                let mut sum = vec![0.0f32; dim];
                let mut count = 0u32;
                for (j, (_, cj, vj)) in items.iter().enumerate() {
                    if j == i || *cj != ci {
                        continue;
                    }
                    for (k, x) in vj.iter().enumerate() {
                        sum[k] += x;
                    }
                    count += 1;
                }
                if count == 0 {
                    continue;
                }
                for x in &mut sum {
                    *x /= count as f32;
                }
                let s = cosine(vi, &sum);
                if s > best.1 {
                    best = (ci, s);
                }
            }
            if best.0 == *actual_ci {
                hits += 1;
            }
        }
        let majority = truth
            .iter()
            .map(|(_, a)| a.len())
            .max()
            .unwrap_or(0) as f32
            / items.len() as f32;
        println!(
            "\ncentroid LOO accuracy: {hits}/{} = {:.0}%  (majority-class baseline = {:.0}%)",
            items.len(),
            100.0 * hits as f32 / items.len() as f32,
            100.0 * majority
        );
    }

    /// The decisive test for the *actual* feature: the user categorizes by APP,
    /// and the embedding fallback's job is "this app is uncategorized — which of
    /// my categories does it belong to?" So: take the user's REAL app→category
    /// assignments (read from their live DB 2026-06-24) as ground truth, and do
    /// leave-one-out — hold out each app, embed its name, find the nearest other
    /// app by cosine, predict THAT app's category, compare to the held-out app's
    /// real category. This measures whether app-name embeddings cluster the way
    /// the user actually thinks, with no hand-authored exemplars to flatter it.
    fn app_name_loo_probe(model: &NLEmbedding) {
        let truth = real_app_map();

        // Flatten to (app, category, embedding).
        let mut items: Vec<(&str, &str, Vec<f32>)> = Vec::new();
        for (cat, apps) in truth {
            for &app in apps.iter() {
                if let Some(v) = embed(model, app) {
                    items.push((app, cat, v));
                }
            }
        }

        println!("\n=== R43 leave-one-out on your REAL app→category map (ground-truthed) ===");
        println!("{:<20} {:<14} {:<14} {:<16} nn-cos", "app", "predicted", "actual", "nearest");
        println!("{}", "-".repeat(78));

        let mut hits = 0u32;
        for (i, (app, actual, vi)) in items.iter().enumerate() {
            // nearest OTHER app
            let mut best = ("", "unclassified", f32::MIN);
            for (j, (napp, ncat, vj)) in items.iter().enumerate() {
                if i == j {
                    continue;
                }
                let s = cosine(vi, vj);
                if s > best.2 {
                    best = (napp, ncat, s);
                }
            }
            let ok = best.1 == *actual;
            if ok {
                hits += 1;
            }
            println!(
                "{app:<20} {:<14} {actual:<14} {:<16} {:.3} {}",
                best.1,
                best.0,
                best.2,
                if ok { "✓" } else { "✗" }
            );
        }
        println!("{}", "-".repeat(78));
        println!(
            "leave-one-out accuracy on your real apps: {hits}/{} = {:.0}%",
            items.len(),
            100.0 * hits as f32 / items.len() as f32
        );
    }

    /// The user's actual categories (read from their live DB 2026-06-24), with a
    /// few representative window-title exemplars each. The matcher embeds each
    /// exemplar, then scores a test title by its MAX cosine to any exemplar of a
    /// category (KNN, k=1 per category) and assigns the argmax — or `unclassified`
    /// if the best score is below ABSTAIN.
    fn title_probe(model: &NLEmbedding) {
        const ABSTAIN: f32 = 0.30; // tunable; sentence-embedding cosines run moderate

        let categories: &[(&str, &[&str])] = &[
            (
                "Work",
                &[
                    "Cursor — db.rs — usage_os",
                    "Visual Studio Code — main.rs",
                    "Xcode — build settings",
                    "Figma — UsageOS dial design",
                    "Notion — product roadmap",
                    "Terminal — cargo test",
                ],
            ),
            (
                "Browsing",
                &[
                    "Google Chrome — Rust async traits - Stack Overflow",
                    "Safari — MDN Web Docs",
                    "Arc — GitHub pull request",
                    "Brave — search results",
                ],
            ),
            (
                "Messaging",
                &[
                    "Slack — #engineering",
                    "Mail — Inbox",
                    "Messages — conversation",
                    "Zoom — team standup meeting",
                    "Discord — general channel",
                ],
            ),
            (
                "Entertainment",
                &[
                    "YouTube — music video",
                    "Netflix — watching a series",
                    "Reddit — r/programming",
                    "Steam — playing a game",
                ],
            ),
            (
                "Personal",
                &[
                    "Spotify — Discover Weekly playlist",
                    "WhatsApp — family group chat",
                    "FaceTime — call with a friend",
                    "VLC — movie",
                ],
            ),
        ];

        // Pre-embed exemplars.
        let mut cat_vecs: Vec<(&str, Vec<Vec<f32>>)> = Vec::new();
        for (name, exemplars) in categories {
            let mut vecs = Vec::new();
            for ex in *exemplars {
                if let Some(v) = embed(model, ex) {
                    vecs.push(v);
                }
            }
            cat_vecs.push((name, vecs));
        }

        // Realistic test titles with the category I'd expect by hand. Includes
        // short, code-heavy, ambiguous, and a non-English case (R43 stressors).
        let tests: &[(&str, &str)] = &[
            ("Cursor — lib.rs — usage_os", "Work"),
            ("Terminal — git push origin main", "Work"),
            ("Figma — Bauhaus components", "Work"),
            ("TablePlus — usage.db", "Work"),
            ("Safari — Apple Developer Documentation", "Browsing"),
            ("Google Chrome — booking flights to Lisbon", "Browsing"),
            ("Slack — #random", "Messaging"),
            ("Zoom — 1:1 with manager", "Messaging"),
            ("Mail", "Messaging"),
            ("YouTube — lofi beats to study to", "Entertainment"),
            ("Reddit — r/macapps", "Entertainment"),
            ("Steam — Football Manager", "Entertainment"),
            ("Spotify", "Personal"),
            ("WhatsApp — mamá", "Personal"), // non-English stressor
            ("FaceTime", "Personal"),
            ("Preview — invoice.pdf", "?"), // genuinely ambiguous — no hard expectation
        ];

        println!("\n=== R43 accuracy probe (ABSTAIN < {ABSTAIN}) ===");
        println!("{:<42} {:<14} {:<14} score", "title", "predicted", "expected");
        println!("{}", "-".repeat(86));

        let mut scored = 0u32;
        let mut hits = 0u32;
        for (title, expected) in tests {
            let Some(tv) = embed(model, title) else {
                println!("{title:<42} <nil embedding>");
                continue;
            };
            let mut best = ("unclassified", f32::MIN);
            for (name, vecs) in &cat_vecs {
                let s = vecs.iter().map(|e| cosine(&tv, e)).fold(f32::MIN, f32::max);
                if s > best.1 {
                    best = (name, s);
                }
            }
            let predicted = if best.1 < ABSTAIN { "unclassified" } else { best.0 };
            let mark = if *expected == "?" {
                "·"
            } else {
                scored += 1;
                if predicted == *expected {
                    hits += 1;
                    "✓"
                } else {
                    "✗"
                }
            };
            println!(
                "{title:<42} {predicted:<14} {expected:<14} {:.3} {mark}",
                best.1
            );
        }
        println!("{}", "-".repeat(86));
        if scored > 0 {
            println!(
                "accuracy on titles with a hard expectation: {hits}/{scored} = {:.0}%",
                100.0 * hits as f32 / scored as f32
            );
        }
        println!("\n(Eyeball the scores: tight clusters = sentence embeddings discriminate; \
                  everything ~equal = they don't, and rules must stay primary.)");
    }
}
