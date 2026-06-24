import type { Meta, StoryObj } from "@storybook/react-vite";

import { TextInput } from "./TextInput";

const meta = {
  title: "UI/TextInput",
  component: TextInput,
} satisfies Meta<typeof TextInput>;

export default meta;

export const States: StoryObj = {
  render: () => (
    <div className="flex max-w-[320px] flex-col gap-4">
      <TextInput label="Category name" placeholder="e.g. Design" />
      <TextInput label="Category name · filled" defaultValue="Design" />
      <TextInput label="Pattern · error" error="Pattern can't be empty." defaultValue="" />
      <TextInput label="Path · disabled" defaultValue="~/Library/…/usage.db" disabled />
    </div>
  ),
};
