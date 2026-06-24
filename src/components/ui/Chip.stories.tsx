import type { Meta, StoryObj } from "@storybook/react-vite";

import { CANONICAL_CATEGORIES, categoryColorVar } from "@/lib/categories";
import { Chip } from "./Chip";

const meta = {
  title: "UI/Chip",
  component: Chip,
} satisfies Meta<typeof Chip>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {
  args: { label: "Deep work", colorVar: "var(--c-deep)", active: false },
};

export const Active: Story = {
  args: { label: "Comms", colorVar: "var(--c-comms)", active: true },
};

export const Legend: StoryObj = {
  render: () => (
    <div className="flex flex-wrap gap-[9px]">
      {CANONICAL_CATEGORIES.map((c) => (
        <Chip key={c.slug} label={c.name} colorVar={categoryColorVar(c.slug)} />
      ))}
    </div>
  ),
};
