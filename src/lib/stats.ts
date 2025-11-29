import { ActivityLog } from './tauri';

export interface ProcessStats {
  processName: string;
  totalDuration: number;
  percentage: number;
  isIdle?: boolean;
  displayName: string;
}

/**
 * Get Unix timestamp range for today (start of day to end of day).
 */
export function getTodayRange(): [number, number] {
  const now = new Date();
  const startOfDay = new Date(now.getFullYear(), now.getMonth(), now.getDate());
  const endOfDay = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 23, 59, 59);
  
  return [
    Math.floor(startOfDay.getTime() / 1000),
    Math.floor(endOfDay.getTime() / 1000)
  ];
}

/**
 * Get Unix timestamp range for yesterday.
 */
export function getYesterdayRange(): [number, number] {
  const now = new Date();
  const yesterday = new Date(now.getFullYear(), now.getMonth(), now.getDate() - 1);
  const startOfYesterday = new Date(yesterday.getFullYear(), yesterday.getMonth(), yesterday.getDate());
  const endOfYesterday = new Date(yesterday.getFullYear(), yesterday.getMonth(), yesterday.getDate(), 23, 59, 59);
  
  return [
    Math.floor(startOfYesterday.getTime() / 1000),
    Math.floor(endOfYesterday.getTime() / 1000)
  ];
}

/**
 * Get Unix timestamp range for the past 7 days.
 */
export function getWeekRange(): [number, number] {
  const now = new Date();
  const weekAgo = new Date(now.getTime() - 7 * 24 * 60 * 60 * 1000);
  const startOfWeek = new Date(weekAgo.getFullYear(), weekAgo.getMonth(), weekAgo.getDate());
  const endOfToday = new Date(now.getFullYear(), now.getMonth(), now.getDate(), 23, 59, 59);
  
  return [
    Math.floor(startOfWeek.getTime() / 1000),
    Math.floor(endOfToday.getTime() / 1000)
  ];
}

/**
 * Calculate total active duration from activity logs.
 *
 * @param logs - Array of activity logs
 * @param options - Configuration options
 * @param options.includeIdle - Whether to include idle time in calculation
 * @returns Total duration in seconds
 */
export function calculateDuration(
  logs: ActivityLog[],
  options: { includeIdle?: boolean } = {},
): number {
  const { includeIdle = false } = options;
  return logs.reduce((total, log) => {
    const duration = Math.max(log.end_time - log.start_time, 0);
    if (!includeIdle && log.is_idle) {
      return total;
    }
    return total + duration;
  }, 0);
}

/**
 * Calculate total idle duration from activity logs.
 */
export function calculateIdleDuration(logs: ActivityLog[]): number {
  return logs.reduce((total, log) => {
    if (log.is_idle) {
      return total + Math.max(log.end_time - log.start_time, 0);
    }
    return total;
  }, 0);
}

/**
 * Group activity logs by process name and aggregate durations.
 *
 * @param logs - Array of activity logs
 * @param includeIdle - Whether to include idle entries in results
 * @returns Array of process stats sorted by duration (descending)
 */
export function groupByProcess(
  logs: ActivityLog[],
  includeIdle = false,
): ProcessStats[] {
  const grouped = new Map<
    string,
    { processName: string; isIdle: boolean; duration: number }
  >();

  logs.forEach((log) => {
    if (!includeIdle && log.is_idle) return;
    const duration = Math.max(log.end_time - log.start_time, 0);
    if (duration === 0) return;

    const key = log.is_idle ? `idle-${log.process_name}` : log.process_name;
    if (!grouped.has(key)) {
      grouped.set(key, {
        processName: log.process_name || 'Unknown',
        isIdle: log.is_idle,
        duration: 0,
      });
    }
    grouped.get(key)!.duration += duration;
  });

  const total = Array.from(grouped.values()).reduce(
    (sum, entry) => sum + entry.duration,
    0,
  );

  return Array.from(grouped.values())
    .map((entry) => ({
      processName: entry.processName,
      totalDuration: entry.duration,
      percentage: total > 0 ? (entry.duration / total) * 100 : 0,
      isIdle: entry.isIdle,
      displayName: entry.isIdle
        ? `${entry.processName || 'Unknown'} (Idle)`
        : entry.processName || 'Unknown',
    }))
    .sort((a, b) => b.totalDuration - a.totalDuration);
}

/**
 * Format duration in seconds to human-readable string.
 *
 * @param seconds - Duration in seconds
 * @returns Formatted string (e.g., "5h 23m", "45m 12s", "30s")
 */
export function formatDuration(seconds: number): string {
  const hours = Math.floor(seconds / 3600);
  const minutes = Math.floor((seconds % 3600) / 60);
  const secs = seconds % 60;

  if (hours > 0) {
    return `${hours}h ${minutes}m`;
  } else if (minutes > 0) {
    return `${minutes}m ${secs}s`;
  } else {
    return `${secs}s`;
  }
}

/**
 * Get color for process based on index.
 * Uses deterministic color rotation from predefined palette.
 */
export function getColorForProcess(_processName: string, index: number): string {
  const colors = [
    'hsl(189, 100%, 62%)',
    'hsl(280, 100%, 70%)',
    'hsl(142, 76%, 56%)',
    'hsl(45, 100%, 60%)',
    'hsl(0, 100%, 67%)',
  ];
  
  return colors[index % colors.length];
}

