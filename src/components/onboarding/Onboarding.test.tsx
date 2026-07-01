// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import "@testing-library/jest-dom";
import { render, screen, fireEvent } from "@testing-library/react";

// The IPC mock throws on any real invoke, so stub the tauri wrapper this component uses.
// `vi.hoisted` makes the spy exist before the hoisted `vi.mock` factory runs.
const { getPermissions, setLaunchAtLogin } = vi.hoisted(() => ({
  getPermissions: vi.fn(),
  setLaunchAtLogin: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("@/lib/tauri", () => ({
  getPermissions,
  requestAccessibility: vi.fn().mockResolvedValue(undefined),
  requestAutomation: vi.fn().mockResolvedValue("no_browser_running"),
  getSettings: vi.fn().mockResolvedValue([]),
  updateSetting: vi.fn().mockResolvedValue(undefined),
  restartApp: vi.fn().mockResolvedValue(undefined),
  getLaunchAtLogin: vi.fn().mockResolvedValue(false),
  setLaunchAtLogin,
}));

import { Onboarding } from "./Onboarding";

describe("Onboarding", () => {
  beforeEach(() => {
    getPermissions.mockResolvedValue({ accessibility: false, automation: "not_determined" });
  });

  it("walks Welcome → Privacy → grant steps (skipped) → Ready and completes", async () => {
    const onComplete = vi.fn();
    render(<Onboarding onComplete={onComplete} />);

    // Welcome
    expect(await screen.findByRole("button", { name: /Get started/i })).toBeInTheDocument();
    expect(screen.getByText(/Where did your/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Get started/i }));

    // Privacy
    expect(screen.getByText(/Nothing leaves/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Continue/i }));

    // Accessibility — not granted, so the skip affordance shows
    expect(screen.getByText(/Read what/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Maybe later/i }));

    // Automation — not granted, so Skip shows
    expect(screen.getByText(/See the sites/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Skip/i }));

    // Background — opt-in, not enabled, so the "Not now" affordance shows
    expect(screen.getByText(/the background/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Not now/i }));

    // Updates — opt-in, not enabled, so the "Not now" affordance shows
    expect(screen.getByText(/up to date/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Not now/i }));

    // Ready — degraded note because both were skipped
    expect(screen.getByText(/That’s it/i)).toBeInTheDocument();
    expect(screen.getByText(/app-level data for now/i)).toBeInTheDocument();

    fireEvent.click(screen.getByRole("button", { name: /Open my day/i }));
    expect(onComplete).toHaveBeenCalledTimes(1);
  });

  it("enables start-at-login from the Background step", async () => {
    render(<Onboarding onComplete={vi.fn()} />);
    fireEvent.click(await screen.findByRole("button", { name: /Get started/i }));
    fireEvent.click(screen.getByRole("button", { name: /Continue/i }));
    fireEvent.click(screen.getByRole("button", { name: /Maybe later/i }));
    fireEvent.click(screen.getByRole("button", { name: /Skip/i }));

    // Background — Enable registers the LaunchAgent and the footer flips to Continue
    fireEvent.click(screen.getByRole("button", { name: /Enable/i }));
    expect(setLaunchAtLogin).toHaveBeenCalledWith(true);
    expect(screen.getByText(/Enabled ✓/i)).toBeInTheDocument();
    expect(screen.getByRole("button", { name: /Continue/i })).toBeInTheDocument();
  });

  it("lets you step back to a previous step", async () => {
    render(<Onboarding onComplete={vi.fn()} />);
    fireEvent.click(await screen.findByRole("button", { name: /Get started/i }));
    expect(screen.getByText(/Nothing leaves/i)).toBeInTheDocument();
    fireEvent.click(screen.getByRole("button", { name: /Back/i }));
    expect(screen.getByText(/Where did your/i)).toBeInTheDocument();
  });
});
