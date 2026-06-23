import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { SegmentedControl } from "./SegmentedControl";

const meta = {
  title: "UI/SegmentedControl",
  component: SegmentedControl,
} satisfies Meta<typeof SegmentedControl<string>>;

export default meta;

export const Themes: StoryObj = {
  render: () => {
    const [value, setValue] = useState("paper");
    return (
      <SegmentedControl
        aria-label="Theme"
        value={value}
        onChange={setValue}
        options={[
          { value: "paper", label: "Paper" },
          { value: "warm", label: "Warm" },
          { value: "black", label: "Black" },
        ]}
      />
    );
  },
};
