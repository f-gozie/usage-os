// Day-shape recap facts — the board's working model of what Rust will compute.
// Shared by design/recap.html (live, tunable) and the on-device eval (prompts are printed from
// here, so the model is judged on exactly the facts the board shows). Synthetic days only.
(function (root) {
  "use strict";

  const CATS = ["Work", "Browsing", "Messaging", "Entertainment", "Personal"];

  const DEFAULTS = {
    partMinActive: 45, // minutes of activity before a part of the day gets described
    partMostly: 55, // % one category needs to be "mostly" that part
    partPair: 65, // % the top two need together to be "mostly X and Y"
    whereMin: 20, // minutes an app/site needs inside its category to be named
    whereShare: 20, // % of its category an app/site needs to be named
    longStretch: 30, // minutes for a run to count as a "long stretch"
    textureLong: 60, // % of the day in long stretches → "mostly long stretches"
    textureShort: 25, // % of the day in long stretches at or below which → "short pieces"
    breakMin: 75, // minutes of no activity before a break is mentioned
    tailMin: 15, // minutes after midnight (before the night's gap) worth mentioning
  };

  // ── synthetic day generator (deterministic) ──────────────────────────────
  function rng(seed) {
    let a = seed >>> 0;
    return () => {
      a = (a + 0x6d2b79f5) >>> 0;
      let t = a;
      t = Math.imul(t ^ (t >>> 15), t | 1);
      t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
      return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
    };
  }
  const hm = (s) => {
    const [h, m] = s.split(":").map(Number);
    return h * 60 + m;
  };

  /** A phase: [from, to, {Cat: [weight, [labels…]]}, [minPiece, maxPiece], gapChance]. */
  function generate(seed, phases) {
    const r = rng(seed);
    const segs = [];
    for (const [from, to, mix, piece, gapChance] of phases) {
      let t = hm(from);
      const end = hm(to);
      const entries = Object.entries(mix);
      const total = entries.reduce((s, [, [w]]) => s + w, 0);
      while (t < end) {
        let pick = r() * total;
        let cat = entries[0][0];
        let labels = entries[0][1][1];
        for (const [c, [w, l]] of entries) {
          if ((pick -= w) <= 0) {
            cat = c;
            labels = l;
            break;
          }
        }
        const len = Math.round(piece[0] + r() * (piece[1] - piece[0]));
        const e = Math.min(end, t + len);
        const label = labels[Math.floor(r() * labels.length) % labels.length];
        segs.push([t, e, cat, label]);
        t = e;
        if (r() < (gapChance ?? 0.15)) t += 3 + Math.round(r() * 9);
      }
    }
    return segs;
  }

  const L = {
    work: ["Cursor", "Figma", "Linear", "Terminal"],
    workDocs: ["Google Docs", "Notion"],
    browse: ["github.com", "news.ycombinator.com", "x.com", "en.wikipedia.org"],
    msg: ["Slack", "Mail", "Messages"],
    ent: ["youtube.com", "Spotify"],
    pers: ["WhatsApp", "Photos", "Calendar"],
  };

  const DAYS = [
    {
      id: "split",
      name: "A long day that changed halfway",
      note:
        "Like the day in your screenshot: a late night carried over, a morning of Browsing and Messaging, then Work from late afternoon into the night. The current recap says none of that.",
      segs: generate(14, [
        ["00:00", "01:20", { Work: [3, L.work], Entertainment: [1, L.ent] }, [8, 25], 0.2],
        ["08:10", "12:40", { Browsing: [4, L.browse], Messaging: [4, L.msg], Work: [1, L.work], Personal: [0.6, L.pers] }, [4, 14], 0.45],
        ["13:30", "16:00", { Browsing: [3, L.browse], Messaging: [2, L.msg], Work: [2, L.work] }, [5, 16], 0.45],
        ["16:00", "19:10", { Work: [6, L.work], Browsing: [2, L.browse] }, [8, 35], 0.3],
        ["20:00", "23:50", { Work: [6, L.work], Browsing: [2, L.browse] }, [8, 40], 0.3],
      ]),
      unwatched: [],
    },
    {
      id: "focused",
      name: "A focused morning on one project",
      note: "Long blocks of Work on one project, a long midday break, then Messaging in the evening.",
      project: "usageos",
      segs: generate(7, [
        ["08:40", "13:00", { Work: [9, ["Cursor", "Cursor", "Terminal"]], Browsing: [1, ["github.com"]] }, [25, 70], 0.3],
        ["15:20", "17:00", { Work: [3, ["Cursor", "Linear"]], Messaging: [2, L.msg] }, [10, 30], 0.2],
        ["19:00", "21:10", { Messaging: [5, ["Messages", "Mail"]], Personal: [2, L.pers], Entertainment: [1, L.ent] }, [8, 25], 0.2],
      ]),
      unwatched: [],
    },
    {
      id: "weekend",
      name: "A light weekend afternoon",
      note: "Under three hours, mostly Entertainment and Personal. Fewer facts clear the thresholds, so the recap stays short.",
      segs: generate(21, [
        ["13:10", "15:40", { Entertainment: [5, ["youtube.com", "Spotify"]], Personal: [3, ["Photos", "WhatsApp"]], Browsing: [1, L.browse] }, [10, 35], 0.25],
        ["21:00", "21:40", { Personal: [3, ["WhatsApp"]], Messaging: [1, ["Messages"]] }, [8, 15], 0.1],
      ]),
      unwatched: [],
    },
    {
      id: "today",
      name: "Today, at 11:20 in the morning",
      note: "A day still in progress: it hasn’t ended, so the recap says “so far”, never “until”.",
      today: hm("11:20"),
      segs: generate(5, [
        ["07:50", "11:15", { Messaging: [3, L.msg], Work: [4, L.workDocs.concat(["Figma"])], Browsing: [1, L.browse] }, [8, 30], 0.2],
      ]),
      unwatched: [],
    },
    {
      id: "gap",
      name: "A day UsageOS wasn’t running for part of",
      note:
        "Quit from 13:00 to 16:30. That stretch is not a break — nobody knows what happened — so it’s never called one, and the recap says the totals are partial.",
      segs: generate(33, [
        ["09:00", "13:00", { Work: [5, L.work], Messaging: [2, L.msg], Browsing: [2, L.browse] }, [10, 40], 0.15],
        ["16:30", "19:30", { Work: [3, L.work], Browsing: [3, L.browse], Messaging: [1, L.msg] }, [8, 30], 0.2],
      ]),
      unwatched: [[hm("13:00"), hm("16:30")]],
    },
    {
      id: "tiny",
      name: "Eight minutes",
      note: "Too little for any shape. Every new fact is held back by its threshold, so this recap is unchanged.",
      segs: [
        [hm("18:02"), hm("18:07"), "Work", "Linear"],
        [hm("18:07"), hm("18:10"), "Messaging", "Slack"],
      ],
      unwatched: [],
    },
  ];

  // ── facts ─────────────────────────────────────────────────────────────────
  function runsOf(segs) {
    // Consecutive same-category pieces join (gaps ≤ 2 min bridged); a blip under 4 minutes is
    // absorbed into the run around it, roughly what the Rust segmenter does for the dial.
    const runs = [];
    for (const [s, e, cat] of segs) {
      const last = runs[runs.length - 1];
      if (last && s - last.end <= 2 && (last.cat === cat || e - s < 4)) {
        last.end = e;
        last.mins += e - s;
      } else {
        runs.push({ start: s, end: e, cat, mins: e - s });
      }
    }
    return runs;
  }

  function partOf(min) {
    if (min >= 300 && min < 720) return "morning";
    if (min >= 720 && min < 1020) return "afternoon";
    if (min >= 1020) return "evening";
    return "night";
  }

  function computeFacts(day, t, families) {
    const on = (f) => !families || families[f] !== false;
    const segs = day.segs;
    const active = segs.reduce((s, [a, b]) => s + (b - a), 0);
    const byCat = {};
    for (const [a, b, c] of segs) byCat[c] = (byCat[c] || 0) + (b - a);
    const ranked = Object.entries(byCat).sort((x, y) => y[1] - x[1]);
    const facts = {
      active,
      leading: ranked[0] ? { name: ranked[0][0], mins: ranked[0][1] } : null,
      second: ranked[1] ? { name: ranked[1][0], mins: ranked[1][1] } : null,
      project: day.project || null,
      unwatched: day.unwatched.reduce((s, [a, b]) => s + (b - a), 0),
      today: day.today != null,
    };
    if (!active) return facts;

    // The night's tail: activity before 5am that a gap of 3h+ separates from the day proper.
    let mainStart = 0;
    for (let i = 0; i < segs.length - 1; i++) {
      if (segs[i][1] <= 300 && segs[i + 1][0] - segs[i][1] >= 180) mainStart = i + 1;
    }
    const tail = segs.slice(0, mainStart);
    const main = segs.slice(mainStart);
    const tailMins = tail.reduce((s, [a, b]) => s + (b - a), 0);
    if (on("span") && tailMins >= t.tailMin) facts.tail = { mins: tailMins, until: tail[tail.length - 1][1] };

    const runs = runsOf(main);
    const longest = runsOf(segs).reduce((m, r) => (r.mins > (m?.mins ?? 0) ? r : m), null);
    if (longest && longest.mins >= 15) facts.longest = { mins: longest.mins, at: longest.start };

    // Span: from the first to the last run of five minutes or more.
    const real = runs.filter((r) => r.mins >= 5);
    if (on("span") && active >= 60 && real.length) {
      const end = real[real.length - 1].end;
      facts.span = {
        from: real[0].start,
        to: end,
        ongoing: day.today != null,
      };
    }

    // Parts of the day.
    if (on("parts")) {
      const parts = { morning: {}, afternoon: {}, evening: {} };
      for (const [a, b, c] of main) {
        for (let m = a; m < b; m++) {
          const p = partOf(m);
          if (parts[p]) parts[p][c] = (parts[p][c] || 0) + 1;
        }
      }
      const described = [];
      for (const [name, cats] of Object.entries(parts)) {
        const tot = Object.values(cats).reduce((s, v) => s + v, 0);
        if (tot < t.partMinActive) continue;
        const top = Object.entries(cats).sort((x, y) => y[1] - x[1]);
        let what = null;
        if ((top[0][1] / tot) * 100 >= t.partMostly) what = top[0][0];
        else if (top[1] && ((top[0][1] + top[1][1]) / tot) * 100 >= t.partPair)
          what = `${top[0][0]} and ${top[1][0]}`;
        if (what) described.push({ part: name, what });
      }
      // Neighbouring parts that say the same thing merge: "afternoon and evening".
      const merged = [];
      for (const d of described) {
        const prev = merged[merged.length - 1];
        if (prev && prev.what === d.what) prev.part += ` and ${d.part}`;
        else merged.push({ ...d });
      }
      if (merged.length) facts.parts = merged;
    }

    // Where the top two categories happened.
    if (on("where")) {
      facts.where = [];
      for (const c of [facts.leading, facts.second].filter(Boolean)) {
        const labels = {};
        for (const [a, b, cat, label] of segs) if (cat === c.name) labels[label] = (labels[label] || 0) + (b - a);
        const named = Object.entries(labels)
          .filter(([, m]) => m >= t.whereMin && (m / c.mins) * 100 >= t.whereShare)
          .sort((x, y) => y[1] - x[1])
          .slice(0, 2)
          .map(([l]) => l);
        if (named.length) facts.where.push({ cat: c.name, labels: named });
      }
      if (!facts.where.length) delete facts.where;
    }

    // How the time came.
    if (on("texture") && active >= 90) {
      const inLong = runs.filter((r) => r.mins >= t.longStretch).reduce((s, r) => s + r.mins, 0);
      const share = (inLong / active) * 100;
      if (share >= t.textureLong) facts.texture = "mostly in long stretches";
      else if (share <= t.textureShort && runs.length >= 8) facts.texture = "in short pieces, switching often";
    }

    // The longest break inside the span, measured over every bit of activity (a few scattered
    // minutes mean it wasn't a break) — never across a stretch UsageOS wasn't running.
    if (on("break") && real.length > 1) {
      const from = real[0].start;
      const to = real[real.length - 1].end;
      let away = from;
      let best = null;
      for (const [a, b] of main) {
        if (b <= from || a >= to) continue;
        const blind = day.unwatched.some(([u0, u1]) => u0 < a && u1 > away);
        if (!blind && a - away >= t.breakMin && (!best || a - away > best.mins)) best = { mins: a - away, from: away };
        away = Math.max(away, b);
      }
      if (best) facts.break = best;
    }
    return facts;
  }

  // ── phrasing helpers (Rust will own these) ──────────────────────────────────
  function long(mins) {
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    const p = (n, u) => `${n} ${u}${n === 1 ? "" : "s"}`;
    if (!h) return p(m, "minute");
    if (!m) return p(h, "hour");
    return `${p(h, "hour")} ${p(m, "minute")}`;
  }
  function short(mins) {
    const h = Math.floor(mins / 60);
    const m = mins % 60;
    return h ? (m ? `${h}h ${m}m` : `${h}h`) : `${m}m`;
  }
  /** Rounded to the half hour, with a coarse part of day: "around 8:30 in the morning". */
  function clock(min) {
    const r = Math.round(min / 30) * 30;
    const h = Math.floor(r / 60) % 24;
    const m = r % 60;
    if (h === 0 && m === 0) return "around midnight";
    if (h === 12 && m === 0) return "around noon";
    const h12 = h % 12 === 0 ? 12 : h % 12;
    const t = m ? `${h12}:30` : `${h12}`;
    const part = h < 5 ? "at night" : h < 12 ? "in the morning" : h < 17 ? "in the afternoon" : h < 21 ? "in the evening" : "at night";
    return `around ${t} ${part}`;
  }
  /** The same coarse phrase the shipped recap already uses for the longest stretch. */
  function when(min) {
    const h = Math.floor(min / 60);
    if (h >= 5 && h <= 7) return "early in the morning";
    if (h >= 8 && h <= 11) return "in the morning";
    if (h === 12) return "around midday";
    if (h >= 13 && h <= 16) return "in the afternoon";
    if (h >= 17 && h <= 20) return "in the evening";
    return "at night";
  }
  const cap = (s) => s[0].toUpperCase() + s.slice(1);
  const isSite = (label) => label.includes(".");
  /** "in Cursor and Linear", "on x.com", "in Slack and on github.com" — apps are in, sites are on. */
  function where(labels) {
    const parts = labels.map((l, i) => {
      const prep = isSite(l) ? "on" : "in";
      return i && isSite(labels[i - 1]) === isSite(l) ? l : `${prep} ${l}`;
    });
    return andList(parts);
  }
  const andList = (xs) =>
    xs.length < 3 ? xs.join(" and ") : `${xs.slice(0, -1).join(", ")}, and ${xs[xs.length - 1]}`;

  // ── the prompt the model narrates ───────────────────────────────────────────
  function formatPrompt(f) {
    const lines = ["The day's facts, already computed — narrate only these, change nothing:"];
    lines.push(`- Total active time: ${long(f.active)}`);
    if (f.leading) lines.push(`- Leading category: ${f.leading.name}, ${long(f.leading.mins)}`);
    if (f.second) lines.push(`- Runner-up category: ${f.second.name}, ${long(f.second.mins)}`);
    if (f.span)
      lines.push(
        f.span.ongoing
          ? `- Active since ${clock(f.span.from)}, and still going`
          : `- Active from ${clock(f.span.from)} until ${clock(f.span.to)}`,
      );
    if (f.tail) lines.push(`- Also ${long(f.tail.mins)} just after midnight, carried over from the night before`);
    for (const p of f.parts || []) lines.push(`- ${cap(p.part)}: mostly ${p.what}`);
    for (const w of f.where || []) lines.push(`- ${w.cat} was mostly ${where(w.labels)}`);
    if (f.texture) lines.push(`- The time came ${f.texture}`);
    if (f.longest) lines.push(`- Longest unbroken stretch: ${long(f.longest.mins)}, ${when(f.longest.at)}`);
    if (f.break) lines.push(`- Longest break: ${long(f.break.mins)}, from ${clock(f.break.from)}`);
    if (f.project) lines.push(`- Main project: ${f.project}`);
    if (f.unwatched >= 30)
      lines.push(
        `- UsageOS was not running for ${long(f.unwatched)} of this day, so these totals cover only part of it. Say so plainly at the end. Do not guess what happened during that time.`,
      );
    return lines.join("\n");
  }

  // ── the recap the app writes (deterministic — no model) ──────────────────────
  function template(f) {
    if (!f.leading) return "No activity tracked today yet.";
    const out = [];
    const tail = f.tail ? `, counting ${short(f.tail.mins)} just after midnight` : "";
    if (f.span) {
      out.push(
        f.span.ongoing
          ? `You’ve been active since ${clock(f.span.from)}, ${short(f.active)} so far.`
          : `You were active from ${clock(f.span.from)} until ${clock(f.span.to)}, ${short(f.active)} in all${tail}.`,
      );
    } else {
      out.push(`${short(f.active)} tracked.`);
    }
    if (f.parts) {
      // "The morning went to Messaging and Browsing, the afternoon to Work, and the evening to Work."
      const ps = f.parts.map((p, i) => (i ? `the ${p.part} to ${p.what}` : `The ${p.part} went to ${p.what}`));
      out.push(`${ps.length === 2 ? ps.join(", and ") : andList(ps)}.`);
    } else {
      out.push(
        f.second
          ? `${f.leading.name} led at ${short(f.leading.mins)}, then ${f.second.name} at ${short(f.second.mins)}.`
          : `All of it was ${f.leading.name}.`,
      );
    }
    if (f.where) {
      const ws = f.where.map((w, i) => (i ? `${w.cat} mostly ${where(w.labels)}` : `${w.cat} was mostly ${where(w.labels)}`));
      out.push(`${ws.join("; ")}.`);
    }
    if (f.longest) {
      const s = short(f.longest.mins);
      const at = when(f.longest.at);
      out.push(
        f.texture === "in short pieces, switching often"
          ? `It came in short pieces; the longest ran ${s} ${at}.`
          : f.texture === "mostly in long stretches"
            ? `Most of it came in long stretches, the longest ${s} ${at}.`
            : `Your longest stretch ran ${s} ${at}.`,
      );
    }
    if (f.break) out.push(`The longest break was ${short(f.break.mins)}, from ${clock(f.break.from)}.`);
    if (f.project) out.push(`Mostly on ${f.project}.`);
    if (f.unwatched >= 30) out.push(`UsageOS wasn’t running for ${short(f.unwatched)} of it, so this is only part of the day.`);
    return out.join(" ");
  }

  /** What ships today (v3): the same prompt minus every day-shape line. */
  function currentPrompt(f) {
    return formatPrompt({ ...f, span: null, tail: null, parts: null, where: null, texture: null, break: null });
  }
  function currentTemplate(f) {
    if (!f.leading) return "No activity tracked today yet.";
    let t = f.second
      ? `${short(f.active)} tracked. ${f.leading.name} led at ${short(f.leading.mins)}, then ${f.second.name} at ${short(f.second.mins)}.`
      : `${short(f.active)} tracked, all ${f.leading.name.toLowerCase()}.`;
    if (f.longest) t += ` Your longest stretch ran ${short(f.longest.mins)} ${when(f.longest.at)}.`;
    if (f.project) t += ` Mostly on ${f.project}.`;
    if (f.unwatched >= 30) t += ` UsageOS wasn’t running for ${short(f.unwatched)} of it, so this is only part of the day.`;
    return t;
  }

  const api = { CATS, DEFAULTS, DAYS, computeFacts, formatPrompt, template, currentPrompt, currentTemplate, short };
  if (typeof module !== "undefined") module.exports = api;
  else root.RecapShape = api;
})(typeof window !== "undefined" ? window : globalThis);
