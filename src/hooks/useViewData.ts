import { useCallback, useEffect, useRef, useState } from "react";

export interface ViewDataState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
  refresh: () => void;
}

/**
 * Fetches a view's computed data and guards against out-of-order responses: a request-id
 * latch drops any resolution that isn't the latest, so fast day/week stepping can't render
 * an earlier range's data. Shows a loading state only on `deps` change (not on background
 * refresh) and, when `live`, polls every 30s.
 *
 * @param fetchFn  resolves the view payload (closes over the range)
 * @param deps     identity of the current range — a change refetches
 * @param live     poll every 30s while true (viewing today / this week)
 * @param errorMessage  shown when the fetch rejects
 */
export function useViewData<T>(
  fetchFn: () => Promise<T>,
  deps: React.DependencyList,
  live: boolean,
  errorMessage: string,
): ViewDataState<T> {
  const [data, setData] = useState<T | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);

  // Monotonic request id. Only the newest request may write state, so a slow earlier
  // response resolving after a faster later one is ignored. Survives re-renders.
  const latest = useRef(0);

  // eslint-disable-next-line react-hooks/exhaustive-deps
  const fetch = useCallback(fetchFn, deps);

  const runFetch = useCallback(async () => {
    const token = ++latest.current;
    try {
      const next = await fetch();
      if (token !== latest.current) return; // superseded by a newer request
      setError(null);
      setData(next);
    } catch (e) {
      if (token !== latest.current) return;
      setError(e instanceof Error ? e.message : errorMessage);
    } finally {
      if (token === latest.current) setLoading(false);
    }
  }, [fetch, errorMessage]);

  useEffect(() => {
    setLoading(true);
    void runFetch();
    if (!live) return;
    const id = setInterval(() => void runFetch(), 30_000);
    return () => clearInterval(id);
  }, [runFetch, live]);

  return { data, loading, error, refresh: runFetch };
}
