import type { Meta, StoryObj } from "@storybook/react-vite";

import { GrantedPill } from "./GrantedPill";

const meta = {
  title: "UI/GrantedPill",
  component: GrantedPill,
} satisfies Meta<typeof GrantedPill>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = {};
