import type { Meta, StoryObj } from "@storybook/react-vite";

import type { CategoryRun } from "@/lib/tauri";
import { MiniDial } from "./MiniDial";

const meta = {
  title: "Dial/MiniDial",
  component: MiniDial,
} satisfies Meta<typeof MiniDial>;

export default meta;
type Story = StoryObj<typeof meta>;

// A day's worth of runs (dayStartUnix = 0, so start/end are seconds from midnight).
const RUNS: CategoryRun[] = [
  { category_slug: "comms", category_name: "Comms", start: 30600, end: 32400, secs: 1800, projects: [{ name: "No project", secs: 1800 }], apps: ["Slack"] },
  { category_slug: "deep", category_name: "Deep work", start: 32400, end: 41400, secs: 9000, projects: [{ name: "usageos", secs: 9000 }], apps: ["Cursor", "iTerm"] },
  { category_slug: "research", category_name: "Research", start: 41400, end: 45000, secs: 3600, projects: [{ name: "No project", secs: 3600 }], apps: ["Chrome"] },
  { category_slug: "deep", category_name: "Deep work", start: 46800, end: 52200, secs: 5400, projects: [{ name: "nudge", secs: 5400 }], apps: ["Cursor"] },
  { category_slug: "breaks", category_name: "Breaks", start: 52200, end: 53400, secs: 1200, projects: [{ name: "No project", secs: 1200 }], apps: ["Spotify"] },
];

export const Today: Story = {
  args: { runs: RUNS, dayStartUnix: 0, nowMinutes: 900, label: "Sat 21" },
  decorators: [(Story) => <div style={{ maxWidth: 110 }}><Story /></div>],
};

export const PastDay: Story = {
  args: { runs: RUNS, dayStartUnix: 0, nowMinutes: null, label: "Tue 17" },
  decorators: [(Story) => <div style={{ maxWidth: 110 }}><Story /></div>],
};

export const EmptyDay: Story = {
  args: { runs: [], dayStartUnix: 0, nowMinutes: null, label: "Sun 15" },
  decorators: [(Story) => <div style={{ maxWidth: 110 }}><Story /></div>],
};
