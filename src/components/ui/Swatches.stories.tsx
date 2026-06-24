import type { Meta, StoryObj } from "@storybook/react-vite";
import { useState } from "react";

import { Swatches } from "./Swatches";

const meta = {
  title: "UI/Swatches",
  component: Swatches,
} satisfies Meta<typeof Swatches>;

export default meta;

const COLORS = ["#1B45BE", "#E0241B", "#EAB308", "#7A4FC2", "#1D9E75", "#161616"];

export const Picker: StoryObj = {
  render: () => {
    const [color, setColor] = useState(COLORS[3]);
    return <Swatches colors={COLORS} value={color} onChange={setColor} aria-label="Colour" />;
  },
};
