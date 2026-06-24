//! The AI seam (hard rule 5): recap narration behind a mockable trait, with the deterministic
//! template (D48) as the always-available fallback (D9 / hard rule 6).
//!
//! [`build_recap`] is the one entry point — it formats the day's [`RecapFacts`] into a prompt,
//! asks a [`Narrator`] to phrase it, and falls back to the template on ANY failure or empty
//! output (C5). The real narrator (a Foundation Models sidecar) lands in a later chunk; the
//! [`FakeNarrator`] keeps this testable with no model, so cross-platform CI stays green (C19).

use crate::rollup::{format_recap_prompt, render_template_recap, Recap, RecapFacts};

/// The real narrator: the Foundation Models Swift sidecar over stdio (chunk C). Cross-platform
/// by compile (the shell plugin is portable); on a machine without the sidecar/model it simply
/// errors → template (C5). Tests + cross-platform CI use [`FakeNarrator`] instead (C19).
pub mod sidecar;

/// `generated_by` tag set when the on-device model produced the prose (vs `"template"`).
const GENERATED_BY_MODEL: &str = "foundation-models";

/// Why a narration attempt failed. [`build_recap`] treats every variant identically — fall
/// back to the template — but keeping them distinct keeps the failure legible for logging.
#[derive(Debug, Clone)]
pub enum AiError {
    /// The model isn't usable here (Apple Intelligence off, device ineligible, not ready).
    Unavailable(String),
    /// The model ran but didn't yield usable prose (guardrail refusal, overflow, malformed).
    Model(String),
}

/// A source of recap prose. The real impl shells out to the on-device model; the fake returns
/// canned output. Generic dispatch only (no `dyn`), so the async method needs no extra crate.
#[allow(async_fn_in_trait)]
pub trait Narrator {
    /// Phrase the pre-formatted facts prompt into prose. The model is instructed not to alter
    /// numbers and constrained to prose, but [`build_recap`] still defends with the fallback.
    async fn narrate(&self, prompt: &str) -> Result<String, AiError>;
}

/// A no-model narrator for tests and CI (hard rule 5 / C19): canned prose or a forced error.
pub enum FakeNarrator {
    Prose(String),
    Fails(AiError),
}

impl Narrator for FakeNarrator {
    async fn narrate(&self, _prompt: &str) -> Result<String, AiError> {
        match self {
            FakeNarrator::Prose(text) => Ok(text.clone()),
            FakeNarrator::Fails(err) => Err(err.clone()),
        }
    }
}

/// Build the day's recap: try the narrator, fall back to the deterministic template on ANY
/// failure or empty output (C5 — the recap must always render). An empty day skips the model
/// entirely (there is nothing to narrate).
pub async fn build_recap<N: Narrator>(narrator: &N, facts: &RecapFacts) -> Recap {
    if facts.leading.is_none() {
        return render_template_recap(facts);
    }
    let prompt = format_recap_prompt(facts);
    match narrator.narrate(&prompt).await {
        Ok(text) if !text.trim().is_empty() => Recap {
            text: text.trim().to_string(),
            generated_by: GENERATED_BY_MODEL.to_string(),
        },
        _ => render_template_recap(facts),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::rollup::{CategoryFact, FocusFact};

    /// Block on a future without the tokio `macros` feature (we only enable `rt`).
    fn block_on<F: std::future::Future>(fut: F) -> F::Output {
        tokio::runtime::Builder::new_current_thread()
            .build()
            .expect("current-thread runtime")
            .block_on(fut)
    }

    fn sample_facts() -> RecapFacts {
        RecapFacts {
            active_secs: 17580, // 4h 53m
            leading: Some(CategoryFact {
                name: "Work".into(),
                secs: 11400, // 3h 10m
            }),
            second: Some(CategoryFact {
                name: "Browsing".into(),
                secs: 5400, // 1h 30m
            }),
            leading_project: Some("usageos".into()),
            longest_focus: Some(FocusFact {
                secs: 4320, // 1h 12m
                when: "in the morning",
            }),
        }
    }

    #[test]
    fn prompt_spells_units_out_and_labels_fields() {
        let p = format_recap_prompt(&sample_facts());
        assert!(p.contains("Total active time: 4 hours 53 minutes"), "{p}");
        assert!(
            p.contains("Leading category: Work, 3 hours 10 minutes"),
            "{p}"
        );
        assert!(
            p.contains("Runner-up category: Browsing, 1 hour 30 minutes"),
            "{p}"
        );
        assert!(
            p.contains("Longest unbroken stretch: 1 hour 12 minutes, in the morning"),
            "{p}"
        );
        assert!(p.contains("Main project: usageos"), "{p}");
        // The "47m → 47 million" guard the spike found: never emit the compact shorthand.
        assert!(!p.contains("53m"), "units must be spelled out: {p}");
    }

    #[test]
    fn uses_model_prose_on_success() {
        let fake = FakeNarrator::Prose("You spent the day on usageos.".into());
        let recap = block_on(build_recap(&fake, &sample_facts()));
        assert_eq!(recap.text, "You spent the day on usageos.");
        assert_eq!(recap.generated_by, "foundation-models");
    }

    #[test]
    fn falls_back_to_template_on_any_error() {
        for err in [
            AiError::Unavailable("appleIntelligenceNotEnabled".into()),
            AiError::Model("guardrailViolation".into()),
        ] {
            let fake = FakeNarrator::Fails(err);
            let recap = block_on(build_recap(&fake, &sample_facts()));
            assert_eq!(recap.generated_by, "template");
            assert!(recap.text.contains("tracked"), "{}", recap.text);
        }
    }

    #[test]
    fn falls_back_on_empty_model_output() {
        let fake = FakeNarrator::Prose("   ".into());
        let recap = block_on(build_recap(&fake, &sample_facts()));
        assert_eq!(recap.generated_by, "template");
    }

    #[test]
    fn empty_day_skips_the_model() {
        let facts = RecapFacts {
            active_secs: 0,
            leading: None,
            second: None,
            leading_project: None,
            longest_focus: None,
        };
        // Even a "successful" model is bypassed — there's nothing to narrate.
        let fake = FakeNarrator::Prose("should not be used".into());
        let recap = block_on(build_recap(&fake, &facts));
        assert_eq!(recap.generated_by, "template");
        assert_eq!(recap.text, "No activity tracked today yet.");
    }
}
