// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import "@testing-library/jest-dom";
import { render } from "@testing-library/react";

import { RecapCard } from "./RecapCard";

describe("RecapCard", () => {
  it("renders the recap prose verbatim", () => {
    const { getByText } = render(<RecapCard text="Mostly on usage_os." />);
    expect(getByText("Mostly on usage_os.")).toBeInTheDocument();
  });

  it("carries no badge: the recap is the app’s own, not a placeholder", () => {
    const { queryByText } = render(<RecapCard text="12m tracked. All of it was Work." />);
    expect(queryByText(/Template|on-device/)).not.toBeInTheDocument();
  });
});
