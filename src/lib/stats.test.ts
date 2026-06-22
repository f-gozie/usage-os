import { describe, it, expect } from 'vitest';
import {
  calculateDuration,
  calculateIdleDuration,
  groupByProcess,
  groupByCategory,
  formatDuration,
  getTodayRange,
  getYesterdayRange,
  getWeekRange,
  getColorForProcess,
} from './stats';
import type { ActivityLog, Category } from './tauri';

// --- Helper factories ---

function makeLog(overrides: Partial<ActivityLog> = {}): ActivityLog {
  return {
    id: 1,
    process_name: 'firefox',
    window_title: 'GitHub',
    start_time: 1000,
    end_time: 1060,
    is_idle: false,
    category_id: null,
    ...overrides,
  };
}

// --- calculateDuration ---

describe('calculateDuration', () => {
  it('excludes idle by default', () => {
    const logs = [
      makeLog({ start_time: 0, end_time: 100, is_idle: false }),
      makeLog({ start_time: 100, end_time: 200, is_idle: true }),
      makeLog({ start_time: 200, end_time: 350, is_idle: false }),
    ];
    expect(calculateDuration(logs)).toBe(250); // 100 + 150, skip idle
  });

  it('includes idle when option set', () => {
    const logs = [
      makeLog({ start_time: 0, end_time: 100, is_idle: false }),
      makeLog({ start_time: 100, end_time: 200, is_idle: true }),
    ];
    expect(calculateDuration(logs, { includeIdle: true })).toBe(200);
  });

  it('returns 0 for empty logs', () => {
    expect(calculateDuration([])).toBe(0);
  });

  it('handles negative durations as 0', () => {
    const logs = [makeLog({ start_time: 200, end_time: 100 })];
    expect(calculateDuration(logs)).toBe(0);
  });
});

// --- calculateIdleDuration ---

describe('calculateIdleDuration', () => {
  it('sums only idle entries', () => {
    const logs = [
      makeLog({ start_time: 0, end_time: 100, is_idle: false }),
      makeLog({ start_time: 100, end_time: 250, is_idle: true }),
      makeLog({ start_time: 250, end_time: 400, is_idle: true }),
    ];
    expect(calculateIdleDuration(logs)).toBe(300); // 150 + 150
  });

  it('returns 0 when no idle entries', () => {
    const logs = [makeLog({ is_idle: false })];
    expect(calculateIdleDuration(logs)).toBe(0);
  });
});

// --- groupByProcess ---

describe('groupByProcess', () => {
  it('aggregates and sorts by duration descending', () => {
    const logs = [
      makeLog({ id: 1, process_name: 'firefox', start_time: 0, end_time: 100 }),
      makeLog({ id: 2, process_name: 'code', start_time: 100, end_time: 400 }),
      makeLog({ id: 3, process_name: 'firefox', start_time: 400, end_time: 500 }),
    ];
    const result = groupByProcess(logs);
    expect(result[0].processName).toBe('code');
    expect(result[0].totalDuration).toBe(300);
    expect(result[1].processName).toBe('firefox');
    expect(result[1].totalDuration).toBe(200);
  });

  it('excludes idle entries by default', () => {
    const logs = [
      makeLog({ process_name: 'firefox', start_time: 0, end_time: 100, is_idle: false }),
      makeLog({ process_name: 'firefox', start_time: 100, end_time: 200, is_idle: true }),
    ];
    const result = groupByProcess(logs);
    expect(result.length).toBe(1);
    expect(result[0].totalDuration).toBe(100);
  });

  it('includes idle entries when flag set', () => {
    const logs = [
      makeLog({ process_name: 'firefox', start_time: 0, end_time: 100, is_idle: false }),
      makeLog({ process_name: 'firefox', start_time: 100, end_time: 200, is_idle: true }),
    ];
    const result = groupByProcess(logs, [], true);
    expect(result.length).toBe(2); // separate entries for idle and non-idle
  });

  it('skips zero-duration entries', () => {
    const logs = [makeLog({ start_time: 100, end_time: 100 })];
    const result = groupByProcess(logs);
    expect(result.length).toBe(0);
  });

  it('attaches category info when categories provided', () => {
    const cats: Category[] = [{ id: 1, name: 'Browsers', color: '#ff0000' }];
    const logs = [makeLog({ category_id: 1 })];
    const result = groupByProcess(logs, cats);
    expect(result[0].categoryName).toBe('Browsers');
    expect(result[0].categoryColor).toBe('#ff0000');
  });
});

// --- groupByCategory ---

describe('groupByCategory', () => {
  it('groups by category with uncategorized bucket', () => {
    const cats: Category[] = [
      { id: 1, name: 'Dev', color: '#00ff00' },
    ];
    const logs = [
      makeLog({ id: 1, category_id: 1, start_time: 0, end_time: 100 }),
      makeLog({ id: 2, category_id: undefined, start_time: 100, end_time: 250 }),
    ];
    const result = groupByCategory(logs, cats);
    expect(result.length).toBe(2);
    // Sorted by duration desc, so uncategorized (150) > dev (100)
    expect(result[0].categoryName).toBe('Uncategorized');
    expect(result[0].totalDuration).toBe(150);
    expect(result[1].categoryName).toBe('Dev');
    expect(result[1].totalDuration).toBe(100);
  });

  it('returns empty for empty logs', () => {
    expect(groupByCategory([], []).length).toBe(0);
  });
});

// --- formatDuration ---

describe('formatDuration', () => {
  it('formats 0 seconds', () => {
    expect(formatDuration(0)).toBe('0s');
  });

  it('formats seconds only (< 60)', () => {
    expect(formatDuration(45)).toBe('45s');
    expect(formatDuration(59)).toBe('59s');
  });

  it('formats minutes + seconds', () => {
    expect(formatDuration(90)).toBe('1m 30s');
    expect(formatDuration(3599)).toBe('59m 59s');
  });

  it('formats hours + minutes (drops seconds)', () => {
    expect(formatDuration(3600)).toBe('1h 0m');
    expect(formatDuration(5400)).toBe('1h 30m');
    expect(formatDuration(7200)).toBe('2h 0m');
  });
});

// --- Time range functions ---

describe('getTodayRange', () => {
  it('returns valid Unix timestamp pair', () => {
    const [start, end] = getTodayRange();
    expect(start).toBeLessThan(end);
    expect(end - start).toBeLessThanOrEqual(86399); // max 1 day
    // Both should be positive
    expect(start).toBeGreaterThan(0);
  });
});

describe('getYesterdayRange', () => {
  it('returns range before today', () => {
    const [yStart, yEnd] = getYesterdayRange();
    const [tStart] = getTodayRange();
    expect(yEnd).toBeLessThan(tStart);
    expect(yEnd - yStart).toBeLessThanOrEqual(86399);
  });
});

describe('getWeekRange', () => {
  it('returns range spanning ~7 days', () => {
    const [start, end] = getWeekRange();
    const span = end - start;
    // Should be at least 6 days and at most 8 days (due to edge cases)
    expect(span).toBeGreaterThanOrEqual(6 * 86400);
    expect(span).toBeLessThanOrEqual(8 * 86400);
  });
});

// --- getColorForProcess ---

describe('getColorForProcess', () => {
  it('returns valid HSL strings', () => {
    for (let i = 0; i < 10; i++) {
      const color = getColorForProcess('test', i);
      expect(color).toMatch(/^hsl\(\d+(\.\d+)?, \d+%, \d+%\)$/);
    }
  });

  it('generates distinct colors for different indices', () => {
    const colors = Array.from({ length: 5 }, (_, i) => getColorForProcess('test', i));
    const unique = new Set(colors);
    expect(unique.size).toBe(5);
  });
});
