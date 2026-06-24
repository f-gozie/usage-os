import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { Toggle } from "./Toggle";

const meta = {
  title: "UI/Toggle",
  component: Toggle,
} satisfies Meta<typeof Toggle>;

export default meta;

export const States: StoryObj = {
  render: () => {
    const [on, setOn] = useState(true);
    return (
      <div className="flex items-center gap-6">
        <Toggle checked={on} onChange={setOn} aria-label="Demo" />
        <Toggle checked={false} onChange={() => {}} aria-label="Off" />
        <Toggle checked disabled onChange={() => {}} aria-label="Disabled" />
      </div>
    );
  },
};
