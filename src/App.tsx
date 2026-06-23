import { useState } from "react";

import { AppShell } from "@/components/shell/AppShell";
import type { View } from "@/components/shell/TabNav";
import { formatDayParts, formatWeekRange } from "@/lib/dates";
import { ThemeProvider } from "@/providers/ThemeProvider";
import { DayView } from "@/views/DayView";
import { Placeholder } from "@/views/Placeholder";
import { TimelineView } from "@/views/TimelineView";
import { WeekView } from "@/views/WeekView";

function App() {
  const [view, setView] = useState<View>("day");
  const [date, setDate] = useState<Date>(() => new Date());

  const parts = formatDayParts(date);
  const headerDate =
    view === "day" || view === "timeline" ? (
      <>
        {parts.weekday}
        <br />
        {parts.full}
      </>
    ) : view === "week" ? (
      <>
        Week of
        <br />
        {formatWeekRange(date)}
      </>
    ) : undefined;

  return (
    <ThemeProvider>
      <AppShell view={view} onViewChange={setView} headerDate={headerDate}>
        {view === "day" && <DayView date={date} onDateChange={setDate} />}
        {view === "week" && (
          <WeekView
            date={date}
            onDateChange={setDate}
            onOpenDay={(d) => {
              setDate(d);
              setView("day");
            }}
          />
        )}
        {view === "timeline" && <TimelineView date={date} onDateChange={setDate} />}
        {view === "settings" && <Placeholder title="Settings" />}
      </AppShell>
    </ThemeProvider>
  );
}

export default App;
