import type { Meta, StoryObj } from "@storybook/react-vite";

import { StatTile } from "./StatTile";

const meta = {
  title: "UI/StatTile",
  component: StatTile,
} satisfies Meta<typeof StatTile>;

export default meta;

export const Row: StoryObj = {
  render: () => (
    <div className="flex max-w-[420px] border-t-[3px] border-edge">
      <StatTile value="4h 49m" label="Active" />
      <StatTile value="3h 30m" label="Deep work" colorVar="var(--c-deep)" className="border-l-2 border-edge pl-3.5" />
      <StatTile value="78%" label="Focus" colorVar="var(--c-research)" className="border-l-2 border-edge pl-3.5" />
    </div>
  ),
};
