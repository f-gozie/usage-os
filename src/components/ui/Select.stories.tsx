import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { Select } from "./Select";

const meta = {
  title: "UI/Select",
  component: Select,
} satisfies Meta<typeof Select<string>>;

export default meta;

export const Retention: StoryObj = {
  render: () => {
    const [value, setValue] = useState("365");
    return (
      <Select
        aria-label="Keep history for"
        value={value}
        onChange={setValue}
        options={[
          { value: "30", label: "30 days" },
          { value: "90", label: "90 days" },
          { value: "365", label: "1 year" },
          { value: "0", label: "Forever" },
        ]}
      />
    );
  },
};
