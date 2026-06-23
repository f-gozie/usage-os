import type { Meta, StoryObj } from "@storybook/react-vite";

import { DetailInspector } from "./DetailInspector";

const meta = {
  title: "UI/DetailInspector",
  component: DetailInspector,
} satisfies Meta<typeof DetailInspector>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Empty: Story = { args: { detail: null } };

export const Selected: Story = {
  args: {
    detail: {
      colorVar: "var(--c-deep)",
      title: "Deep work",
      subtitle: "usageos 1h 3m · nudge 39m · Cursor, iTerm, Claude",
      durationLabel: "1h 42m",
      rangeLabel: "09:00–10:42",
    },
  },
};
