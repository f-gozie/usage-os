// @vitest-environment jsdom
import { useState } from "react";
import { describe, it, expect, vi } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent, act } from "@testing-library/react";

import { YearCell } from "./YearGrid";
import { YearRows, YearWall } from "./YearLayouts";
import { buildYearModel, dayLabel, dayState, YEAR_CEILING_SECS, type DayModel } from "./yearModel";
import type { YearDay } from "@/lib/tauri";

const DAY = 86_400;
const NOW = new Date(2026, 8, 20, 12, 0, 0).getTime() / 1000;
const at = (month: number, date: number) => new Date(2026, month, date).getTime() / 1000;

function day(over: Partial<YearDay> = {}): YearDay {
  return {
    day_start: NOW - DAY,
    active_secs: 0,
    categories: [],
    unwatched_secs: 0,
    coverage_known: true,
    ...over,
  };
}

const work = (secs: number) => [{ slug: "deep", name: "Work", secs, pct: 100, color: null }];

describe("dayState", () => {
  it("tells a day nobody watched from a day you simply didn't use", () => {
    expect(dayState(day({ unwatched_secs: DAY }), NOW)).toBe("unrecorded");
    expect(dayState(day({ active_secs: 0 }), NOW)).toBe("recorded");
  });

  it("refuses to blame a day that predates coverage", () => {
    expect(dayState(day({ coverage_known: false, unwatched_secs: DAY }), NOW)).toBe("unknown");
  });

  it("does not call a partly-blind day unrecorded", () => {
    expect(dayState(day({ active_secs: 7200, unwatched_secs: 6 * 3600 }), NOW)).toBe("recorded");
  });

  it("knows a day that hasn't happened", () => {
    expect(dayState(day({ day_start: NOW + DAY }), NOW)).toBe("future");
  });
});

describe("dayLabel", () => {
  it("says plainly which of the two empty days it is", () => {
    expect(dayLabel(day({ unwatched_secs: DAY }), "unrecorded", "Wed 16 September")).toBe(
      "Wed 16 September · UsageOS wasn’t running",
    );
    expect(dayLabel(day(), "recorded", "Wed 16 September")).toBe("Wed 16 September · nothing tracked");
  });

  it("names the hours on a day with activity", () => {
    expect(dayLabel(day({ active_secs: 3600 }), "recorded", "Wed 16")).toBe("Wed 16 · 1h 0m");
  });

  it("still names the hours when coverage is unknown — unknown is not 'absent'", () => {
    // Every day of history predates the coverage log. Found by looking at the real app: each one
    // claimed "before UsageOS was recording" and hid hours it had plainly recorded.
    const d = day({ active_secs: 3600, coverage_known: false });
    expect(dayLabel(d, dayState(d, NOW), "Wed 16")).toBe("Wed 16 · 1h 0m");
  });

  it("claims nothing beyond 'nothing tracked' for an empty day of unknown coverage", () => {
    const d = day({ active_secs: 0, coverage_known: false });
    expect(dayLabel(d, dayState(d, NOW), "Wed 16")).toBe("Wed 16 · nothing tracked");
  });
});

describe("buildYearModel", () => {
  it("groups days by month and keeps them in order", () => {
    const months = buildYearModel(
      [day({ day_start: at(0, 1) }), day({ day_start: at(0, 2) }), day({ day_start: at(8, 16) })],
      NOW,
    );
    expect(months).toHaveLength(12);
    expect(months[0].map((d) => d.dayStart)).toEqual([at(0, 1), at(0, 2)]);
    expect(months[8]).toHaveLength(1);
    expect(months[5]).toHaveLength(0);
  });

  it("sizes the bar against a fixed ceiling, not the busiest day", () => {
    const [jan] = buildYearModel(
      [
        day({ day_start: at(0, 1), active_secs: YEAR_CEILING_SECS / 2, categories: work(1) }),
        day({ day_start: at(0, 2), active_secs: YEAR_CEILING_SECS * 3, categories: work(1) }),
      ],
      NOW,
    );
    expect(jan[0].heightPct).toBe(50);
    expect(jan[1].heightPct).toBe(100); // a huge day clips; it never overflows its row
  });

  it("drops sub-pixel slivers but keeps every real category", () => {
    const [jan] = buildYearModel(
      [
        day({
          day_start: at(0, 1),
          active_secs: 10_000,
          categories: [
            { slug: "deep", name: "Work", secs: 9000, pct: 90, color: null },
            { slug: "comms", name: "Messaging", secs: 900, pct: 9, color: null },
            { slug: "breaks", name: "Entertainment", secs: 100, pct: 1, color: null },
          ],
        }),
      ],
      NOW,
    );
    expect(jan[0].segments.map((s) => s.key)).toEqual(["deep", "comms"]);
  });

  it("records the weekday of each day, so the wall calendar can pad the 1st", () => {
    // 1 September 2026 is a Tuesday.
    const months = buildYearModel([day({ day_start: at(8, 1) })], NOW);
    expect(months[8][0].dow).toBe(2);
  });
});

describe("YearCell", () => {
  const model = (over: Partial<YearDay> = {}) => buildYearModel([day(over)], NOW).flat()[0];

  it("paints the precomputed height and one segment per category", () => {
    const m = model({ active_secs: YEAR_CEILING_SECS / 2, categories: work(1) });
    const { container } = render(<YearCell model={m} index={0} barWidthClass="w-[13px]" />);
    expect((container.querySelector(".cal-bar") as HTMLElement).style.height).toBe("50%");
    expect(container.querySelectorAll(".cal-bar > span")).toHaveLength(1);
  });

  it("carries its day for delegation, and a future day carries none", () => {
    const past = render(<YearCell model={model({ active_secs: 60 })} index={0} barWidthClass="w-[13px]" />);
    expect(past.getByRole("button")).toHaveAttribute("data-day", String(NOW - DAY));
    past.unmount();

    const future = render(
      <YearCell model={model({ day_start: NOW + DAY })} index={0} barWidthClass="w-[13px]" />,
    );
    expect(future.getByRole("button")).toBeDisabled();
    expect(future.getByRole("button")).not.toHaveAttribute("data-day");
  });

  it("gives every cell a name a screen reader can read", () => {
    const { getByRole } = render(
      <YearCell model={model({ unwatched_secs: DAY })} index={0} barWidthClass="w-[13px]" />,
    );
    expect(getByRole("button").getAttribute("aria-label")).toMatch(/UsageOS wasn’t running$/);
  });
});

describe.each([
  ["YearRows", YearRows],
  ["YearWall", YearWall],
])("%s", (_name, Layout) => {
  const months = buildYearModel(
    [
      day({ day_start: at(8, 15), active_secs: 3600, categories: work(3600) }),
      day({ day_start: at(8, 16), unwatched_secs: DAY }),
    ],
    NOW,
  );

  it("opens a day from one delegated listener, not a handler per cell", () => {
    const onOpenDay = vi.fn();
    const { getAllByRole } = render(
      <Layout months={months} currentMonth={8} onOpenMonth={() => {}} onOpenDay={onOpenDay} />,
    );
    const bar = getAllByRole("button").find((b) => b.getAttribute("data-day") === String(at(8, 15)));
    fireEvent.click(bar as HTMLElement);
    expect(onOpenDay).toHaveBeenCalledWith(at(8, 15));
  });

  it("opens a month from its name", () => {
    const onOpenMonth = vi.fn();
    const { getByRole } = render(
      <Layout months={months} currentMonth={8} onOpenMonth={onOpenMonth} onOpenDay={() => {}} />,
    );
    fireEvent.click(getByRole("button", { name: "Sep" }));
    expect(onOpenMonth).toHaveBeenCalledWith(at(8, 15));
  });
});

describe("render cost", () => {
  // How renders are counted: `YearCell` reads `model.label` during render (for its aria-label).
  // Each model here exposes `label` as an accessor that bumps a counter, so the counter rises
  // if and only if a cell's render function actually ran. No Profiler guesswork, no production
  // hooks — and, unlike a DOM-mutation count, it distinguishes a memo bail-out from a re-render
  // that happened to produce identical output.
  function countingYear() {
    const reads = { count: 0 };
    const months = buildYearModel(
      Array.from({ length: 365 }, (_, i) =>
        day({ day_start: at(0, 1) + i * DAY, active_secs: 3600, categories: work(3600) }),
      ),
      NOW,
    ).map((month) =>
      month.map((m) => {
        const label = m.label;
        return Object.defineProperty({ ...m }, "label", {
          enumerable: true,
          get() {
            reads.count += 1;
            return label;
          },
        });
      }),
    );
    return { months, reads };
  }

  it("re-renders no cells when the parent re-renders with the same model and callbacks", () => {
    // The regression this pins: `Date.now()` in render plus a closure per cell meant ANY parent
    // state change re-ran all 365 cells.
    const { months, reads } = countingYear();
    const onOpenDay = () => {};
    const onOpenMonth = () => {};

    let bump = () => {};
    function Parent() {
      const [, setTick] = useState(0);
      bump = () => setTick((t) => t + 1);
      return <YearRows months={months} currentMonth={8} onOpenMonth={onOpenMonth} onOpenDay={onOpenDay} />;
    }

    render(<Parent />);
    const afterMount = reads.count;
    expect(afterMount).toBeGreaterThanOrEqual(365); // sanity: mounting really did render every cell

    act(() => bump());
    act(() => bump());
    expect(reads.count - afterMount).toBe(0);
  });

  it("CONTROL: the counter does rise when memoization is defeated", () => {
    // Without this, the test above could pass simply because the instrument is blind. Handing
    // the layout a fresh model array each render defeats `memo`, and the counter must show it.
    const { months, reads } = countingYear();

    let bump = () => {};
    function Parent() {
      const [, setTick] = useState(0);
      bump = () => setTick((t) => t + 1);
      // A new outer + inner array identity every render, and fresh cell objects: memo cannot hold.
      const fresh = months.map((month) =>
        month.map((m) => Object.defineProperties({}, Object.getOwnPropertyDescriptors(m)) as DayModel),
      );
      return <YearRows months={fresh} currentMonth={8} onOpenMonth={() => {}} onOpenDay={() => {}} />;
    }

    render(<Parent />);
    const afterMount = reads.count;
    act(() => bump());
    expect(reads.count - afterMount).toBeGreaterThanOrEqual(365);
  });
});
