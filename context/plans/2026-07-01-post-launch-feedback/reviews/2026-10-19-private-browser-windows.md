# Review — private browser windows (`fix/private-browser-titles`, D76)

**Date:** 2026-10-19 · **Scope:** branch (`main...HEAD`) · **Files:** 13
**Plan:** [plan.md](../plan.md) ("Private browser windows that can't be checked") · **Impl-plan:** none (scoped from the #44/#45 debate)
**Codex:** ran three rounds on the diff

## Merge gates

| Gate | Result |
|---|---|
| cargo fmt --check | ✅ |
| cargo clippy --all-targets --all-features -D warnings | ✅ |
| cargo test | ✅ 188 passed |
| tsc --noEmit | ✅ |
| vitest | ✅ 97 passed |
| bindings fresh | ✅ |

## Findings, all verified against the code and fixed

1. **Critical: unlisted browsers kept private titles.** Comet (`ai.perplexity.comet`) and Dia (`company.thebrowser.dia`) are on this Mac and were on neither list. A fixed list can't keep up, so a browser is now any app macOS lists as opening both `https` links and `public.html` (via `NSWorkspace`, cached per app), with terminals excepted and the list kept as a backstop.
   - Checked on this Mac: https-only would have swept in ChatGPT, cmux and iTerm. Adding "opens HTML" removes ChatGPT and cmux, and the terminal list removes iTerm.
2. **Critical: "no window" kept the earlier title.** `NOWIN` now fails closed.
3. **Critical, predates this branch: the Accessibility title and the AppleScript privacy check were separate reads.** A window switch in between could let a private title through, and the script resolved `front window` twice. The script now binds the window once and returns its tab URL and title together, and the app records that title.
4. **Warning: onboarding implied page detail in every browser.** It now names the supported browsers and says Safari and Firefox are time only.
5. **Warning: trimming dropped the separator before an empty tab title**, losing its URL. Only the line ending is trimmed now.

No hard-rule findings: no network, no new command, no prod `unwrap`, no SQL change. The one new dependency is the optional `objc2-uniform-type-identifiers` feature of `objc2-app-kit`, which only runs on macOS.

## Not verified
- On device in the signed build:
  - Safari, Firefox, Comet and Dia recording no title;
  - Chrome, Brave and Arc titles arriving from the script, which only works if each browser's AppleScript dictionary exposes `title of active tab` (Chromium's does; Arc's should).
- Titles recorded before 0.1.6 are untouched (D76). Whether to clear past Safari/Firefox titles is open.

## Definition of Done
- [x] plan.md
- [x] decisions.md: D76
- [ ] impl-plan: none
- [x] docs and copy move with code (README, Onboarding, Settings, CHANGELOG)
