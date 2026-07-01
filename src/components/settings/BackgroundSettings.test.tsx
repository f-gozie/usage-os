// @vitest-environment jsdom
import { describe, it, expect, vi, beforeEach } from "vitest";
import "@testing-library/jest-dom";
import { render, screen, fireEvent } from "@testing-library/react";

// The IPC mock throws on any real invoke, so stub the tauri wrapper this component uses.
const { getLaunchAtLogin, setLaunchAtLogin } = vi.hoisted(() => ({
  getLaunchAtLogin: vi.fn(),
  setLaunchAtLogin: vi.fn().mockResolvedValue(undefined),
}));
vi.mock("@/lib/tauri", () => ({ getLaunchAtLogin, setLaunchAtLogin }));

import { BackgroundSettings } from "./BackgroundSettings";

describe("BackgroundSettings", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("reflects the system LaunchAgent state on mount", async () => {
    getLaunchAtLogin.mockResolvedValue(true);
    render(<BackgroundSettings />);
    const toggle = await screen.findByRole("switch", { name: "Start at login" });
    expect(toggle).toHaveAttribute("aria-checked", "true");
  });

  it("toggling registers / unregisters start-at-login", async () => {
    getLaunchAtLogin.mockResolvedValue(false);
    render(<BackgroundSettings />);
    const toggle = await screen.findByRole("switch", { name: "Start at login" });
    expect(toggle).toHaveAttribute("aria-checked", "false");

    fireEvent.click(toggle);
    expect(setLaunchAtLogin).toHaveBeenCalledWith(true);
    expect(toggle).toHaveAttribute("aria-checked", "true");

    fireEvent.click(toggle);
    expect(setLaunchAtLogin).toHaveBeenCalledWith(false);
    expect(toggle).toHaveAttribute("aria-checked", "false");
  });
});
