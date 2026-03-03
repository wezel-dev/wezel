import { useState, useEffect, useCallback, useRef } from "react";
import { useProject } from "./useProject";
import type { Overview, ScenarioSummary } from "./api";
import type { Scenario, ForagerCommit } from "./data";

const EMPTY_SCENARIOS: ScenarioSummary[] = [];
const EMPTY_COMMITS: ForagerCommit[] = [];
const EMPTY_USERS: string[] = [];
const EMPTY_OVERVIEW: Overview = {
  scenarioCount: 0,
  trackedCount: 0,
  latestCommitShortSha: null,
  latestCommitStatus: null,
};

interface AsyncState<T> {
  data: T | null;
  loading: boolean;
  error: string | null;
}

function useAsync<T>(
  fetcher: () => Promise<T>,
  deps: unknown[] = [],
): AsyncState<T> & { refetch: () => void } {
  const [state, setState] = useState<AsyncState<T>>({
    data: null,
    loading: true,
    error: null,
  });
  const fetcherRef = useRef(fetcher);
  fetcherRef.current = fetcher;

  const doFetch = useCallback((silent: boolean) => {
    if (!silent) {
      setState((s) => ({ ...s, loading: true, error: null }));
    }
    fetcherRef
      .current()
      .then((data) => setState({ data, loading: false, error: null }))
      .catch((e) =>
        silent
          ? setState((s) => ({ ...s, loading: false, error: String(e) }))
          : setState({ data: null, loading: false, error: String(e) }),
      );
  }, []);

  const refetch = useCallback(() => doFetch(false), [doFetch]);

  useEffect(() => {
    doFetch(false);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [...deps, refetch]);

  return { ...state, refetch };
}

export function useOverview() {
  const { pApi, current } = useProject();
  const { data, loading, error } = useAsync(
    () => pApi.overview(),
    [current?.id],
  );
  return { overview: data ?? EMPTY_OVERVIEW, loading, error };
}

export function useScenarios() {
  const { pApi, current } = useProject();
  const { data, loading, error, refetch } = useAsync(
    () => pApi.scenarios(),
    [current?.id],
  );

  const togglePin = useCallback(
    async (id: number) => {
      await pApi.togglePin(id);
      refetch();
    },
    [pApi, refetch],
  );

  return { scenarios: data ?? EMPTY_SCENARIOS, loading, error, togglePin };
}

export function useScenario(id: number | null) {
  const { pApi, current } = useProject();
  const { data, loading, error } = useAsync(
    () => (id != null ? pApi.scenario(id) : Promise.reject("no id")),
    [id, current?.id],
  );
  return {
    scenario: data as Scenario | null,
    loading: id != null && loading,
    error,
  };
}

export function useCommits() {
  const { pApi, current } = useProject();
  const result = useAsync(() => pApi.commits(), [current?.id]);
  return {
    commits: result.data ?? EMPTY_COMMITS,
    loading: result.loading,
    error: result.error,
  };
}

export function useCommit(sha: string | undefined) {
  const { pApi, current } = useProject();
  return useAsync(
    () => (sha ? pApi.commit(sha) : Promise.reject("no sha")),
    [sha, current?.id],
  );
}

export function useUsers() {
  const { pApi, current } = useProject();
  const result = useAsync(() => pApi.users(), [current?.id]);
  return {
    users: result.data ?? EMPTY_USERS,
    loading: result.loading,
    error: result.error,
  };
}
