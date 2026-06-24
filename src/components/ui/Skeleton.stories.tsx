import type { Meta, StoryObj } from "@storybook/react-vite";

import { Skeleton } from "./Skeleton";

const meta = {
  title: "UI/Skeleton",
  component: Skeleton,
} satisfies Meta<typeof Skeleton>;

export default meta;

export const Loading: StoryObj = {
  render: () => (
    <div className="flex max-w-[460px] items-center gap-6">
      <Skeleton circle className="h-[140px] w-[140px]" />
      <div className="flex-1 space-y-2.5">
        <Skeleton className="h-[60px] w-full" />
        <Skeleton className="h-3.5 w-3/4" />
        <Skeleton className="h-3.5 w-1/2" />
      </div>
    </div>
  ),
};
