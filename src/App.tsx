import { useEffect, useState } from "react";

import { AppShell } from "@/components/shell/AppShell";
import type { View } from "@/components/shell/TabNav";
import { loadIconMap } from "@/lib/appIcons";
import { formatDayParts, formatWeekRange } from "@/lib/dates";
import { ThemeProvider } from "@/providers/ThemeProvider";
import { DayView } from "@/views/DayView";
import { SettingsView } from "@/views/SettingsView";
import { TimelineView } from "@/views/TimelineView";
import { WeekView } from "@/views/WeekView";

function App() {
  const [view, setView] = useState<View>("day");
  const [date, setDate] = useState<Date>(() => new Date());

  // Warm the installed-app icon map once at startup so `AppIcon`s resolve without a
  // flash when the Timeline/Settings first render (offline, cached after first build).
  useEffect(() => {
    void loadIconMap();
  }, []);

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
        {view === "settings" && <SettingsView />}
      </AppShell>
    </ThemeProvider>
  );
}

export default App;
