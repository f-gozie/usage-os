import { ActivityLog, Category } from './tauri';

export interface ProcessStats {
  processName: string;
  totalDuration: number;
  percentage: number;
  isIdle?: boolean;
  displayName: string;
  categoryName?: string;
  categoryColor?: string;
}

export interface CategoryStats {
  categoryId: number;
  categoryName: string;
  categoryColor: string;
  totalDuration: number;
  percentage: number;
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
 * @param categories - Array of available categories for lookup
 * @param includeIdle - Whether to include idle entries in results
 * @returns Array of process stats sorted by duration (descending)
 */
export function groupByProcess(
  logs: ActivityLog[],
  categories: Category[] = [],
  includeIdle = false,
): ProcessStats[] {
  const grouped = new Map<
    string,
    { processName: string; isIdle: boolean; duration: number; categoryId?: number | null }
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
        categoryId: log.category_id,
      });
    } else {
        // Update category if the newer log has one (or just keep existing)
        // Usually all logs for same process might have same category if rule based on process
        // But if rule based on title, different logs for same process might have different categories.
        // This makes grouping by process tricky for category assignment.
        // "Dominant category" logic? Or just "Last"?
        // For simplicity, let's use the first encountered category ID or the current one if not set.
        const entry = grouped.get(key)!;
        if (!entry.categoryId && log.category_id) {
            entry.categoryId = log.category_id;
        }
    }
    grouped.get(key)!.duration += duration;
  });

  const total = Array.from(grouped.values()).reduce(
    (sum, entry) => sum + entry.duration,
    0,
  );

  return Array.from(grouped.values())
    .map((entry) => {
      const category = categories.find(c => c.id === entry.categoryId);
      return {
        processName: entry.processName,
        totalDuration: entry.duration,
        percentage: total > 0 ? (entry.duration / total) * 100 : 0,
        isIdle: entry.isIdle,
        displayName: entry.isIdle
            ? `${entry.processName || 'Unknown'} (Idle)`
            : entry.processName || 'Unknown',
        categoryName: category?.name,
        categoryColor: category?.color,
      };
    })
    .sort((a, b) => b.totalDuration - a.totalDuration);
}

/**
 * Group activity logs by category and aggregate durations.
 */
export function groupByCategory(
    logs: ActivityLog[],
    categories: Category[],
    includeIdle = false
): CategoryStats[] {
    const grouped = new Map<number, number>(); // categoryId -> duration
    let uncategorizedDuration = 0;

    logs.forEach(log => {
        if (!includeIdle && log.is_idle) return;
        const duration = Math.max(log.end_time - log.start_time, 0);
        if (duration === 0) return;

        if (log.category_id) {
            grouped.set(log.category_id, (grouped.get(log.category_id) || 0) + duration);
        } else {
            uncategorizedDuration += duration;
        }
    });

    const totalDuration = calculateDuration(logs, { includeIdle });
    
    const stats: CategoryStats[] = [];

    // Add categorized
    grouped.forEach((duration, categoryId) => {
        const category = categories.find(c => c.id === categoryId);
        if (category) {
            stats.push({
                categoryId,
                categoryName: category.name,
                categoryColor: category.color,
                totalDuration: duration,
                percentage: totalDuration > 0 ? (duration / totalDuration) * 100 : 0
            });
        }
    });

    // Add uncategorized if > 0
    if (uncategorizedDuration > 0) {
        stats.push({
            categoryId: -1,
            categoryName: 'Uncategorized',
            categoryColor: '#666666', // Grey
            totalDuration: uncategorizedDuration,
            percentage: totalDuration > 0 ? (uncategorizedDuration / totalDuration) * 100 : 0
        });
    }

    return stats.sort((a, b) => b.totalDuration - a.totalDuration);
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
 * Get color for process using Golden Angle approximation to ensure distinctness.
 * This generates maximal contrast between adjacent indices in the sorted list.
 */
export function getColorForProcess(_processName: string, index: number): string {
  // Golden angle approximation (137.508 degrees) to maximize hue separation
  const goldenAngle = 137.508;
  const hue = (index * goldenAngle) % 360;
  
  // Use high saturation and medium lightness for neon/cyberpunk vibe
  const saturation = 80 + (index % 3) * 5; // Slight variation 80-90%
  const lightness = 60 + (index % 2) * 10; // Slight variation 60-70%

  return `hsl(${hue}, ${saturation}%, ${lightness}%)`;
}
