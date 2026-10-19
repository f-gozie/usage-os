import type { Meta, StoryObj } from "@storybook/react-vite";

import { RecapCard } from "./RecapCard";

const meta = {
  title: "UI/RecapCard",
  component: RecapCard,
} satisfies Meta<typeof RecapCard>;

export default meta;
type Story = StoryObj<typeof meta>;

export const FullDay: Story = {
  args: {
    text: "You were active from around 8 in the morning until around midnight, 12h 45m in all, counting 1h 9m just after midnight. The morning went to Messaging and Browsing, the afternoon to Work and Browsing, and the evening to Work. Work was mostly in Figma and Linear; Browsing mostly on x.com. Your longest stretch ran 1h 46m at night.",
  },
};

export const SoFar: Story = {
  args: {
    text: "You’ve been active since around 8 in the morning, 3h 16m so far. The morning went to Work and Messaging. Work was mostly in Google Docs and Notion; Messaging mostly in Messages. It came in short pieces; the longest ran 30m in the morning.",
  },
};

export const Short: Story = {
  args: { text: "8m tracked. Work led at 5m, then Messaging at 3m." },
};
