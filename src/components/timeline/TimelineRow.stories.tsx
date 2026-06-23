import type { Meta, StoryObj } from "@storybook/react-vite";

import type { TimelineRun } from "@/lib/tauri";
import { TimelineRow } from "./TimelineRow";

const meta = {
  title: "Timeline/TimelineRow",
  component: TimelineRow,
  decorators: [(Story) => <div style={{ maxWidth: 760 }}><Story /></div>],
} satisfies Meta<typeof TimelineRow>;

export default meta;
type Story = StoryObj<typeof meta>;

const MULTI: TimelineRun = {
  context_slug: "deep",
  context_name: "Deep work",
  start: 1_000_000_000,
  end: 1_000_006_120,
  secs: 6120,
  projects: [
    { name: "usageos", secs: 4000 },
    { name: "nudge", secs: 2120 },
  ],
  apps: ["Cursor", "iTerm", "Claude"],
  segments: [
    { start: 1_000_000_000, end: 1_000_001_800, app: "Cursor", project: "usageos", secs: 1800 },
    { start: 1_000_001_800, end: 1_000_003_400, app: "iTerm", project: "usageos", secs: 1600 },
    { start: 1_000_003_400, end: 1_000_004_600, app: "Cursor", project: "nudge", secs: 1200 },
    { start: 1_000_004_600, end: 1_000_006_120, app: "Claude", project: null, secs: 1520 },
  ],
};

const SINGLE: TimelineRun = {
  context_slug: "comms",
  context_name: "Comms",
  start: 1_000_010_000,
  end: 1_000_010_540,
  secs: 540,
  projects: [{ name: "No project", secs: 540 }],
  apps: ["Slack"],
  segments: [{ start: 1_000_010_000, end: 1_000_010_540, app: "Slack", project: null, secs: 540 }],
};

export const Collapsed: Story = { args: { run: MULTI } };
export const Expanded: Story = { args: { run: MULTI, defaultOpen: true } };
export const NoProject: Story = { args: { run: SINGLE } };
