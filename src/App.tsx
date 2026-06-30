import { useEffect, useState } from "react";

import { Onboarding } from "@/components/onboarding/Onboarding";
import { AppShell } from "@/components/shell/AppShell";
import type { View } from "@/components/shell/TabNav";
import { UpdateBanner } from "@/components/UpdateBanner";
import { loadIconMap } from "@/lib/appIcons";
import { formatDayParts, formatWeekRange } from "@/lib/dates";
import { getSettings, updateSetting } from "@/lib/tauri";
import { maybeAutoCheck, type Update } from "@/lib/updater";
import { ThemeProvider } from "@/providers/ThemeProvider";
import { DayView } from "@/views/DayView";
import { SettingsView } from "@/views/SettingsView";
import { TimelineView } from "@/views/TimelineView";
import { WeekView } from "@/views/WeekView";

const ONBOARDING_KEY = "onboarding_completed";

function App() {
  const [view, setView] = useState<View>("day");
  const [date, setDate] = useState<Date>(() => new Date());
  // First-run gate: show onboarding until it's been completed once. `loading` avoids a flash of
  // the empty Day view before the flag resolves; on a read error we fail open to the app.
  const [onboarding, setOnboarding] = useState<"loading" | "needed" | "done">("loading");

  // Warm the installed-app icon map once at startup so `AppIcon`s resolve without a
  // flash when the Timeline/Settings first render (offline, cached after first build).
  useEffect(() => {
    void loadIconMap();
  }, []);

  useEffect(() => {
    void getSettings()
      .then((settings) =>
        setOnboarding(
          settings.some((s) => s.key === ONBOARDING_KEY && s.value === "true") ? "done" : "needed",
        ),
      )
      .catch(() => setOnboarding("done"));
  }, []);

  const completeOnboarding = () => {
    void updateSetting(ONBOARDING_KEY, "true").catch(() => undefined);
    setOnboarding("done");
  };

  // Opt-in launch update check (D67): no-op unless the user enabled it, debounced to once/24h.
  const [update, setUpdate] = useState<Update | null>(null);
  useEffect(() => {
    if (onboarding !== "done") return;
    void maybeAutoCheck()
      .then((u) => u && setUpdate(u))
      .catch(() => undefined);
  }, [onboarding]);

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
      {onboarding === "loading" ? (
        <div className="min-h-screen bg-bg" />
      ) : onboarding === "needed" ? (
        <Onboarding onComplete={completeOnboarding} />
      ) : (
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
      )}
      {update && <UpdateBanner update={update} onDismiss={() => setUpdate(null)} />}
    </ThemeProvider>
  );
}

export default App;
