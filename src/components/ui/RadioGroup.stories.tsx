import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { RadioGroup } from "./RadioGroup";

const meta = {
  title: "UI/RadioGroup",
  component: RadioGroup,
} satisfies Meta<typeof RadioGroup<string>>;

export default meta;

export const ExclusionMode: StoryObj = {
  render: () => {
    const [mode, setMode] = useState("exclude");
    return (
      <RadioGroup
        aria-label="Mode"
        value={mode}
        onChange={setMode}
        options={[
          { value: "exclude", label: "Exclude", description: "Drop the event entirely — no record." },
          { value: "private", label: "Private", description: "Count the time, store no title or URL." },
        ]}
      />
    );
  },
};
