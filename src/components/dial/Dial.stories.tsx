import type { Meta, StoryObj } from "@storybook/react-vite";

import type { CategoryRun } from "@/lib/tauri";
import { Dial } from "./Dial";

const meta = {
  title: "Dial/Dial",
  component: Dial,
} satisfies Meta<typeof Dial>;

export default meta;
type Story = StoryObj<typeof meta>;

// A realistic interleaved day (dayStartUnix = 0, so start/end are seconds from midnight).
const RUNS: CategoryRun[] = [
  { category_slug: "deep", category_name: "Deep work", category_color: null, start: 32400, end: 38520, secs: 6120, projects: [{ name: "usageos", secs: 4000 }, { name: "nudge", secs: 2120 }], apps: ["Cursor", "iTerm", "Claude"] },
  { category_slug: "comms", category_name: "Comms", category_color: null, start: 38520, end: 39060, secs: 540, projects: [{ name: "No project", secs: 540 }], apps: ["Slack"] },
  { category_slug: "deep", category_name: "Deep work", category_color: null, start: 39060, end: 44100, secs: 5040, projects: [{ name: "usageos", secs: 5040 }], apps: ["Cursor", "iTerm"] },
  { category_slug: "research", category_name: "Research", category_color: null, start: 45300, end: 48720, secs: 3420, projects: [{ name: "No project", secs: 3420 }], apps: ["Chrome", "Claude"] },
  { category_slug: "deep", category_name: "Deep work", category_color: null, start: 48720, end: 51300, secs: 2580, projects: [{ name: "nudge", secs: 2580 }], apps: ["Cursor", "iTerm"] },
  { category_slug: "breaks", category_name: "Breaks", category_color: null, start: 51300, end: 51780, secs: 480, projects: [{ name: "No project", secs: 480 }], apps: ["Chrome"] },
  { category_slug: "comms", category_name: "Comms", category_color: null, start: 51780, end: 52200, secs: 420, projects: [{ name: "No project", secs: 420 }], apps: ["Slack"] },
  { category_slug: "deep", category_name: "Deep work", category_color: null, start: 52200, end: 55800, secs: 3600, projects: [{ name: "usageos", secs: 3600 }], apps: ["Cursor", "Chrome", "iTerm"] },
];

export const Today: Story = {
  args: { runs: RUNS, dayStartUnix: 0, nowMinutes: 930, activeLabel: "6h 5m" },
  decorators: [(Story) => <div style={{ maxWidth: 330 }}><Story /></div>],
};

export const EmptyDay: Story = {
  args: { runs: [], dayStartUnix: 0, nowMinutes: 540, activeLabel: "0m" },
  decorators: [(Story) => <div style={{ maxWidth: 330 }}><Story /></div>],
};
