// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent } from "@testing-library/react";

import { DayView } from "./DayView";

vi.mock("@/hooks/useDayData", () => ({
  useDayData: () => ({
    data: {
      active_secs: 3 * 3600,
      idle_secs: 0,
      categories: [
        { slug: "deep", name: "Work", secs: 2 * 3600, pct: 67, color: null },
        { slug: "comms", name: "Messaging", secs: 3600, pct: 33, color: null },
      ],
      runs: [],
      gaps: [],
      recap: { text: "A recap.", generated_by: "template" },
    },
    loading: false,
    error: null,
    refresh: () => {},
  }),
}));
vi.mock("@/hooks/useCaptureHealth", () => ({
  useCaptureHealth: () => ({ healthy: true, problem: null, refetch: () => {} }),
}));
vi.mock("@/lib/appIcons", () => ({ loadIconMap: () => Promise.resolve(), iconFor: () => null }));

/** Ledger rows are the only `div` buttons on the page (chips and steppers are real buttons). */
function ledgerRows(getAllByRole: (role: string) => HTMLElement[]) {
  return getAllByRole("button").filter((el) => el.tagName === "DIV");
}

describe("DayView", () => {
  it("never filters on hover: a passing cursor leaves the ledger as it was", () => {
    const { getAllByRole } = render(
      <DayView date={new Date(2026, 8, 16)} onDateChange={() => {}} />,
    );
    const [work, messaging] = ledgerRows(getAllByRole);

    fireEvent.mouseEnter(work);
    fireEvent.mouseLeave(work);
    expect(messaging).not.toHaveClass("opacity-30");

    fireEvent.click(work);
    expect(messaging).toHaveClass("opacity-30");
    fireEvent.click(work);
    expect(messaging).not.toHaveClass("opacity-30");
  });

  it("offers the way back only when it was opened from somewhere", () => {
    const onBack = vi.fn();
    const { getByRole, queryByRole, rerender } = render(
      <DayView date={new Date(2026, 8, 16)} onDateChange={() => {}} />,
    );
    expect(queryByRole("button", { name: /September 2026/ })).not.toBeInTheDocument();

    rerender(
      <DayView
        date={new Date(2026, 8, 16)}
        onDateChange={() => {}}
        back={{ label: "September 2026", onBack }}
      />,
    );
    fireEvent.click(getByRole("button", { name: /September 2026/ }));
    expect(onBack).toHaveBeenCalledTimes(1);
  });
});
