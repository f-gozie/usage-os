# Feasibility spikes

Before committing to the redesign's risky native bets, each one was settled with a small,
throwaway crate that proved (or disproved) the thing in isolation — so the real codebase was
built on confirmed ground, not hope. Each folder has its own README with the method and result.
This is the risk-burndown behind [`context/feasibility/`](../context/feasibility/) and the
decisions in [`context/decisions.md`](../context/decisions.md).

| Spike | The question / risk | Outcome | Fed into |
|---|---|---|---|
| [`ax-titles`](ax-titles/) | Can we read focused-window titles with **Accessibility alone**, no Screen Recording? (R4) | ✅ Yes — real titles from Chromium/Electron apps + editors, Screen Recording off | capture via AX only (D29) |
| [`ax-observer`](ax-observer/) | Does an **event-driven** `NSWorkspace` + per-PID `AXObserver` model work end-to-end on the main run loop? (R6, R8–R13) | ✅ Yes — activation + title-change events marshal to the async side cleanly | the event-driven capture core (D29) |
| [`browser-url`](browser-url/) | Can we read the active browser **URL**, and reliably **never** record incognito/private windows? (R18) | ✅ Yes via Apple Events — and private windows are detected first, so URL+title are dropped | URL capture + the privacy invariant (D8) |
| [`proc-cwd`](proc-cwd/) | Can we read another process's working directory (a terminal's cwd) via `proc_pidinfo`? | ✅ Yes — the cwd → git remote is the strongest project signal | project inference (D30/D31) |
| [`project-inference`](project-inference/) | How accurate is project inference, and where should it **abstain** rather than guess wrong? | ✅ A git-remote-first model with an abstain threshold | project inference + abstain (D30/D31) |
| [`foundation-models`](foundation-models/) | Can a thin Swift sidecar produce a recap with Apple's on-device **Foundation Models**? | ✅ Yes — with a deterministic template fallback when unavailable | the recap sidecar (D9/D16) |
| [`embeddings`](embeddings/) | Do on-device **embeddings** categorize better than deterministic rules? (D26, R43) | ❌ **Shelved** — measured below the rules baseline | dropped embeddings; rules engine (D47) |

The `embeddings` spike is kept deliberately: a feature measured and *declined* is as much a part
of the record as the ones that shipped.
