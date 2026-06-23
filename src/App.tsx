import { useState } from "react";

import { AppShell } from "@/components/shell/AppShell";
import type { View } from "@/components/shell/TabNav";
import { formatDayParts } from "@/lib/dates";
import { ThemeProvider } from "@/providers/ThemeProvider";
import { DayView } from "@/views/DayView";
import { Placeholder } from "@/views/Placeholder";

function App() {
  const [view, setView] = useState<View>("day");
  const [date, setDate] = useState<Date>(() => new Date());

  const parts = formatDayParts(date);
  const headerDate =
    view === "day" ? (
      <>
        {parts.weekday}
        <br />
        {parts.full}
      </>
    ) : undefined;

  return (
    <ThemeProvider>
      <AppShell view={view} onViewChange={setView} headerDate={headerDate}>
        {view === "day" && <DayView date={date} onDateChange={setDate} />}
        {view === "week" && <Placeholder title="Week" />}
        {view === "timeline" && <Placeholder title="Timeline" />}
        {view === "settings" && <Placeholder title="Settings" />}
      </AppShell>
    </ThemeProvider>
  );
}

export default App;
