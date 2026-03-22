import { invoke } from '@tauri-apps/api/core';

export interface ActivityLog {
  id: number;
  process_name: string;
  window_title: string;
  start_time: number;
  end_time: number;
  is_idle: boolean;
  category_id?: number;
}

export interface Category {
  id: number;
  name: string;
  color: string;
}

export interface Rule {
  id: number;
  category_id: number;
  match_field: string; // "process" or "title"
  pattern: string;
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

// --- Categories ---

export async function getCategories(): Promise<Category[]> {
  return await invoke('get_categories');
}

export async function createCategory(name: string, color: string): Promise<number> {
  return await invoke('create_category', { name, color });
}

export async function deleteCategory(id: number): Promise<void> {
  return await invoke('delete_category', { id });
}

// --- Rules ---

export async function getRules(): Promise<Rule[]> {
  return await invoke('get_rules');
}

export async function createRule(
  categoryId: number,
  matchField: string,
  pattern: string
): Promise<number> {
  return await invoke('create_rule', { categoryId, matchField, pattern });
}

export async function deleteRule(id: number): Promise<void> {
  return await invoke('delete_rule', { id });
}

export async function reprocessLogs(): Promise<void> {
  return await invoke('reprocess_logs');
}

// --- Settings ---

export async function getSettings(): Promise<[string, string][]> {
  return await invoke('get_settings');
}

export async function updateSetting(key: string, value: string): Promise<void> {
  return await invoke('update_setting', { key, value });
}
