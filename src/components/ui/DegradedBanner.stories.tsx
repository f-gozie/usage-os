import type { Meta, StoryObj } from "@storybook/react-vite";

import { DegradedBanner } from "./DegradedBanner";

const meta = {
  title: "UI/DegradedBanner",
  component: DegradedBanner,
} satisfies Meta<typeof DegradedBanner>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: {
    title: "Running with app-level data",
    description: "Grant Accessibility to see window titles and pages, not just app names.",
    actionLabel: "Open settings",
    onAction: () => {},
  },
};
