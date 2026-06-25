// @vitest-environment jsdom
import { describe, it, expect } from "vitest";
import { act, renderHook, waitFor } from "@testing-library/react";

import { useViewData } from "./useViewData";

/** A promise plus its resolver, so a test can settle fetches out of order. */
function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

describe("useViewData", () => {
  it("ignores an earlier-but-slower response so it can't overwrite a later one", async () => {
    // Two pending fetches: the FIRST resolves AFTER the second (out of order).
    const first = deferred<string>();
    const second = deferred<string>();
    const fetches = [first.promise, second.promise];

    let key = "a";
    const { result, rerender } = renderHook(
      ({ k }: { k: string }) =>
        useViewData(() => fetches.shift()!, [k], false, "err"),
      { initialProps: { k: key } },
    );

    // Step to a new range while the first fetch is still in flight.
    key = "b";
    rerender({ k: key });

    // The LATER request resolves first → it should win.
    await act(async () => {
      second.resolve("second");
    });
    await waitFor(() => expect(result.current.data).toBe("second"));

    // The earlier (stale) request resolves last → it must be dropped.
    await act(async () => {
      first.resolve("first");
    });
    // Give any erroneous state write a chance to flush, then assert it didn't happen.
    await Promise.resolve();
    expect(result.current.data).toBe("second");
  });

  it("surfaces the error message when the latest fetch rejects", async () => {
    const { result } = renderHook(() =>
      useViewData(() => Promise.reject(new Error("boom")), [1], false, "fallback"),
    );
    await waitFor(() => expect(result.current.error).toBe("boom"));
    expect(result.current.data).toBeNull();
  });
});
