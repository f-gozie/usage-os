import type { Meta, StoryObj } from "@storybook/react-vite";

import { RecapCard } from "./RecapCard";

const meta = {
  title: "UI/RecapCard",
  component: RecapCard,
} satisfies Meta<typeof RecapCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Template: Story = {
  args: {
    generatedBy: "template",
    text: "4h 49m tracked. Deep work led at 3h 30m. Most of it on usageos.",
  },
};

export const OnDevice: Story = {
  args: {
    generatedBy: "fm",
    text: "A focused day. You gave usageos a clean 90-minute deep-work block before 9:30, then split the afternoon with nudge.",
  },
};
