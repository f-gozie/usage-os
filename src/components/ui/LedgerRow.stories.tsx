import type { Meta, StoryObj } from "@storybook/react-vite";

import { LedgerRow } from "./LedgerRow";

const meta = {
  title: "UI/LedgerRow",
  component: LedgerRow,
} satisfies Meta<typeof LedgerRow>;

export default meta;

export const Ledger: StoryObj = {
  render: () => (
    <div className="max-w-[460px]">
      <LedgerRow name="Deep work" colorVar="var(--c-deep)" durationLabel="3h 30m" pct={72} />
      <LedgerRow name="Research" colorVar="var(--c-research)" durationLabel="45m" pct={15} />
      <LedgerRow name="Comms" colorVar="var(--c-comms)" durationLabel="30m" pct={10} dimmed />
      <LedgerRow name="Breaks" colorVar="var(--c-breaks)" durationLabel="12m" pct={3} />
    </div>
  ),
};
