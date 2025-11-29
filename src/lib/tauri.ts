import { invoke } from '@tauri-apps/api/core';

export interface ActivityLog {
  id: number;
  process_name: string;
  window_title: string;
  start_time: number;
  end_time: number;
  is_idle: boolean;
}

/**
 * Fetch activity logs from the backend for a given time range.
 *
 * @param startTime - Unix timestamp for range start
 * @param endTime - Unix timestamp for range end
 * @returns Array of activity logs
 */
export async function getActivityStats(
  startTime: number,
  endTime: number
): Promise<ActivityLog[]> {
  return await invoke('get_activity_stats', { startTime, endTime });
}

