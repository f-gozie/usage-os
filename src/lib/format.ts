/**
 * Display formatting for the new UI. Durations read like the dial ("4h 15m" / "45m" /
 * "30s") — minute granularity, matching the Rust `human_secs` used by the recap.
 */
export function formatDuration(secs: number): string {
  const total = Math.max(0, Math.floor(secs));
  const hours = Math.floor(total / 3600);
  const minutes = Math.floor((total % 3600) / 60);
  if (hours > 0) return `${hours}h ${minutes}m`;
  if (minutes > 0) return `${minutes}m`;
  return `${total}s`;
}

/** Local wall-clock "HH:MM" for a Unix-second timestamp. */
export function formatClock(unixSec: number): string {
  const d = new Date(unixSec * 1000);
  return `${String(d.getHours()).padStart(2, "0")}:${String(d.getMinutes()).padStart(2, "0")}`;
}
