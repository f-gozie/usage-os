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

const deep = { category_slug: "deep", category_name: "Deep work" };

// A Deep run that absorbed a brief Comms detour (D34a) — the Slack segment carries its own
// category, so the expand marks it; the run's secs/projects stay Deep-only.
const MULTI: TimelineRun = {
  category_slug: "deep",
  category_name: "Deep work",
  start: 1_000_000_000,
  end: 1_000_006_120,
  secs: 5580, // host (Deep) only — the 540s Comms detour is excluded
  projects: [
    { name: "usageos", secs: 3400 },
    { name: "nudge", secs: 1200 },
    { name: "No project", secs: 980 },
  ],
  apps: ["Cursor", "iTerm", "Claude"],
  segments: [
    { start: 1_000_000_000, end: 1_000_001_800, app: "Cursor", ...deep, project: "usageos", secs: 1800 },
    { start: 1_000_001_800, end: 1_000_003_400, app: "iTerm", ...deep, project: "usageos", secs: 1600 },
    { start: 1_000_003_400, end: 1_000_003_940, app: "Slack", category_slug: "comms", category_name: "Comms", project: null, secs: 540 },
    { start: 1_000_003_940, end: 1_000_005_140, app: "Cursor", ...deep, project: "nudge", secs: 1200 },
    { start: 1_000_005_140, end: 1_000_006_120, app: "Claude", ...deep, project: null, secs: 980 },
  ],
};

const SINGLE: TimelineRun = {
  category_slug: "comms",
  category_name: "Comms",
  start: 1_000_010_000,
  end: 1_000_010_540,
  secs: 540,
  projects: [{ name: "No project", secs: 540 }],
  apps: ["Slack"],
  segments: [
    {
      start: 1_000_010_000,
      end: 1_000_010_540,
      app: "Slack",
      category_slug: "comms",
      category_name: "Comms",
      project: null,
      secs: 540,
    },
  ],
};

export const Collapsed: Story = { args: { run: MULTI } };
export const Expanded: Story = { args: { run: MULTI, defaultOpen: true } };
export const NoProject: Story = { args: { run: SINGLE } };
