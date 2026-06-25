-- Delete historical lock-screen / screensaver spans (`loginwindow`, `ScreenSaverEngine`) — time
-- away, not active. The idle gate is blind while locked, so these accrued before capture started
-- dropping them; this clears the historical phantom. See D41.
DELETE FROM activity_logs
WHERE process_name IN ('loginwindow', 'ScreenSaverEngine');
