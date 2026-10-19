// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent } from "@testing-library/react";

import { DateStepper } from "./DateStepper";

const props = {
  title: "Wednesday, 16 September",
  atLatest: false,
  onPrev: () => {},
  onNext: () => {},
  onRefresh: () => {},
  onLatest: () => {},
  latestLabel: "Today",
  prevLabel: "Previous day",
  nextLabel: "Next day",
};

describe("DateStepper", () => {
  it("jumps back to the latest range when the latest button is clicked", () => {
    const onLatest = vi.fn();
    const { getByRole } = render(<DateStepper {...props} onLatest={onLatest} />);

    fireEvent.click(getByRole("button", { name: "Go to today" }));
    expect(onLatest).toHaveBeenCalledTimes(1);
  });

  it("disables the latest button instead of removing it, so the row never reflows", () => {
    const { getByRole, rerender } = render(<DateStepper {...props} />);
    expect(getByRole("button", { name: "Go to today" })).toBeEnabled();

    rerender(<DateStepper {...props} atLatest />);
    expect(getByRole("button", { name: "Go to today" })).toBeDisabled();
    expect(getByRole("button", { name: "Next day" })).toBeDisabled();
  });

  it("names the latest button after the range it returns to", () => {
    const { getByRole } = render(<DateStepper {...props} latestLabel="This week" />);

    const btn = getByRole("button", { name: "Go to this week" });
    // The accessible name contains the visible label (WCAG 2.5.3).
    expect(btn).toHaveTextContent("This week");
  });
});
