import { describe, it, expect, vi, afterEach } from 'vitest';
import { formatRelativeTime } from './time';

describe('formatRelativeTime', () => {
  afterEach(() => {
    vi.useRealTimers();
  });

  it('returns "just now" for < 5 seconds', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-15T12:00:05'));
    expect(formatRelativeTime(new Date('2026-01-15T12:00:02'))).toBe('just now');
  });

  it('returns seconds for < 60 seconds', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-15T12:00:45'));
    expect(formatRelativeTime(new Date('2026-01-15T12:00:00'))).toBe('45s ago');
  });

  it('returns minutes for < 60 minutes', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-15T12:30:00'));
    expect(formatRelativeTime(new Date('2026-01-15T12:00:00'))).toBe('30m ago');
  });

  it('returns hours for < 24 hours', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-15T17:00:00'));
    expect(formatRelativeTime(new Date('2026-01-15T12:00:00'))).toBe('5h ago');
  });

  it('returns days for >= 24 hours', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-17T12:00:00'));
    expect(formatRelativeTime(new Date('2026-01-15T12:00:00'))).toBe('2d ago');
  });

  it('returns "1m ago" at exactly 60 seconds', () => {
    vi.useFakeTimers();
    vi.setSystemTime(new Date('2026-01-15T12:01:00'));
    expect(formatRelativeTime(new Date('2026-01-15T12:00:00'))).toBe('1m ago');
  });
});
