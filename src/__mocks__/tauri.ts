/**
 * Mock for @tauri-apps/api/core
 * Prevents actual IPC calls during testing.
 */
export async function invoke(_cmd: string, _args?: Record<string, unknown>): Promise<unknown> {
  throw new Error(`Tauri invoke mock: "${_cmd}" not implemented`);
}
