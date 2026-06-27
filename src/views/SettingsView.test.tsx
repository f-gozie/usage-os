// @vitest-environment jsdom
import { describe, it, expect, vi } from "vitest";
import "@testing-library/jest-dom";
import { render, fireEvent } from "@testing-library/react";

import { SettingsView } from "./SettingsView";

// Fixed settings payload (inside the factory to dodge hoisting).
vi.mock("@/hooks/useSettingsData", () => {
  const DATA = {
    categories: [
      { id: 1, slug: "deep", name: "Deep work", color: "#1B45BE" },
      { id: 9, slug: null, name: "Design", color: "#7A4FC2" },
    ],
    rules: [
      { id: 11, category_id: 1, match_field: "process", pattern: "Cursor", ignore_title: false },
      { id: 12, category_id: 9, match_field: "title", pattern: "*.fig", ignore_title: false },
    ],
    exclusions: [
      { id: 21, match_type: "app", pattern: "1Password", mode: "exclude", created_at: 0 },
    ],
    uncategorized: [
      { process_name: "TablePlus", total_secs: 2040, last_seen: 1000 },
    ],
    settings: { data_retention_days: "365", theme: "paper" },
  };
  return {
    useSettingsData: () => ({ data: DATA, loading: false, error: null, refresh: vi.fn() }),
  };
});

vi.mock("@/providers/ThemeProvider", () => ({
  THEMES: ["paper", "warm", "black"],
  THEME_LABELS: { paper: "Paper", warm: "Warm", black: "Black" },
  useTheme: () => ({ theme: "paper", setTheme: vi.fn() }),
}));

vi.mock("@tauri-apps/plugin-opener", () => ({ revealItemInDir: vi.fn(), openUrl: vi.fn() }));

// Action commands the view/modals import — present so interactions don't hit a real backend.
vi.mock("@/lib/tauri", () => ({
  deleteExclusion: vi.fn().mockResolvedValue(undefined),
  exportEventsCsv: vi.fn().mockResolvedValue("/tmp/usageos-export-1.csv"),
  getDatabasePath: vi.fn().mockResolvedValue("/tmp/usage.db"),
  setRetentionDays: vi.fn().mockResolvedValue(0),
  createExclusion: vi.fn(),
  createCategory: vi.fn(),
  createRule: vi.fn(),
  updateCategory: vi.fn(),
  deleteCategory: vi.fn(),
  deleteRule: vi.fn(),
  reprocessLogs: vi.fn(),
  deleteAllData: vi.fn().mockResolvedValue(undefined),
  listInstalledApps: vi.fn().mockResolvedValue([]),
  getPermissions: vi.fn().mockResolvedValue({ accessibility: true, automation: "granted" }),
  requestAccessibility: vi.fn().mockResolvedValue(undefined),
  requestAutomation: vi.fn().mockResolvedValue(undefined),
}));

describe("SettingsView", () => {
  it("renders the four buildable groups with their content", () => {
    const { getByText } = render(<SettingsView />);
    // Group headers.
    expect(getByText("Categories & rules")).toBeInTheDocument();
    expect(getByText("Privacy & exclusions")).toBeInTheDocument();
    expect(getByText("Appearance")).toBeInTheDocument();
    expect(getByText("Your data")).toBeInTheDocument();
    // A category + its rule chip.
    expect(getByText("Deep work")).toBeInTheDocument();
    expect(getByText("Cursor")).toBeInTheDocument();
    // An exclusion + the always-on incognito row.
    expect(getByText("1Password")).toBeInTheDocument();
    expect(getByText("Excluded")).toBeInTheDocument();
    expect(getByText(/Always on/)).toBeInTheDocument();
  });

  it("gates delete-all behind typing DELETE", () => {
    const { getByRole, getByLabelText } = render(<SettingsView />);
    fireEvent.click(getByRole("button", { name: "Delete" }));

    const confirm = getByRole("button", { name: "Delete everything" });
    expect(confirm).toBeDisabled();

    fireEvent.change(getByLabelText("Type DELETE to confirm"), { target: { value: "DELETE" } });
    expect(confirm).toBeEnabled();
  });
});
