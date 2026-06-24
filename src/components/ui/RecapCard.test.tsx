// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import "@testing-library/jest-dom";
import { render } from "@testing-library/react";

import { RecapCard } from "./RecapCard";

describe("RecapCard", () => {
  it("shows the on-device badge for the Foundation Models value", () => {
    // Guards the badge against the Rust `generated_by` contract: the value is
    // "foundation-models", not "fm" — a mismatch silently downgrades every recap to "Template".
    const { getByText } = render(
      <RecapCard text="You spent the day on usage_os." generatedBy="foundation-models" />,
    );
    expect(getByText("⌁ Summarized on-device")).toBeInTheDocument();
  });

  it("shows the template badge for the deterministic fallback", () => {
    const { getByText } = render(<RecapCard text="4h 53m tracked." generatedBy="template" />);
    expect(getByText("≡ Template")).toBeInTheDocument();
  });

  it("renders the recap prose verbatim", () => {
    const { getByText } = render(
      <RecapCard text="Mostly on usage_os." generatedBy="foundation-models" />,
    );
    expect(getByText("Mostly on usage_os.")).toBeInTheDocument();
  });
});
