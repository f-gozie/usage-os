import type { Meta, StoryObj } from "@storybook/react-vite";

import { AppIcon } from "./AppIcon";

const meta = {
  title: "UI/AppIcon",
  component: AppIcon,
  args: { name: "Cursor" },
  decorators: [
    (Story) => (
      <div style={{ display: "flex", alignItems: "center", gap: 12 }}>
        <Story />
      </div>
    ),
  ],
} satisfies Meta<typeof AppIcon>;

export default meta;
type Story = StoryObj<typeof meta>;

export const Default: Story = { args: { name: "Cursor", size: 24 } };

export const Sizes: Story = {
  render: () => (
    <>
      {[14, 16, 20, 28].map((s) => (
        <AppIcon key={s} name="Slack" size={s} />
      ))}
    </>
  ),
};

// Storybook has no Tauri backend, so every name resolves to its monogram fallback —
// which is exactly what an unresolved name (the app's own dev binary, an obscure tool)
// looks like in the real app: first letter on the ink colour.
export const MonogramFallback: Story = {
  render: () => (
    <>
      {["usage-os", "Nudge", "Some Unknown Tool", "Zed"].map((n) => (
        <span key={n} style={{ display: "inline-flex", alignItems: "center", gap: 6 }}>
          <AppIcon name={n} size={20} />
          <span style={{ fontSize: 12 }}>{n}</span>
        </span>
      ))}
    </>
  ),
};
