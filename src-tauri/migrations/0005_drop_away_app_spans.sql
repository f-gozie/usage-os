-- D41: remove already-recorded lock/away spans. `loginwindow` and `ScreenSaverEngine` are the
-- macOS lock-screen and screensaver surfaces — time on them is "away," not active. Before D41
-- the idle gate (blind while the screen is locked — input-idle reads ~0) let them accrue; one
-- overnight lock logged a single 46-minute span. Capture now drops these going forward; this
-- clears the historical phantom so the totals and the dial read true.
DELETE FROM activity_logs
WHERE process_name IN ('loginwindow', 'ScreenSaverEngine');
