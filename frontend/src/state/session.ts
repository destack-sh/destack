import { graphql, useFragment } from "@/gql";
import { RunStatus, type Run } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { toValueRef, wrapValueRefs } from "@/utils/functools";
import { useQuery, useSubscription } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, ref, type Ref } from "vue";

export const RUN_TERMINAL_STATES = [RunStatus.Aborted, RunStatus.Failed, RunStatus.Completed];

export const RunContentType = graphql(/* GraphQL */ `
  fragment RunContent on Run {
    id
    createdAt
    updatedAt
    startedAt
    terminatedAt
    duration
    cachedDuration
    cachedGeneratedAt
    status
    projectVersion {
      id
      tag
      name
    }
    session {
      id
    }
    root {
      id
    }
    parent {
      id
    }
    inputs
    outputs
    errorNice {
      kind
      type
      message
      traceback {
        line
        filename
        lineno
        name
        locals
      }
    }
    runnable {
      id
      name
    }
  }
`);

export function _useSessions(
  filter: {
    projectId: Ref<string | null>;
    projectVersionId: Ref<string | null>;
  },
  options: { live?: boolean }
) {
  // rewrap refs to prevent eager updates
  filter = wrapValueRefs(filter);

  const { loading: initialLoading, onResult: onInitialLoaded } = useQuery(
    graphql(/* GraphQL */ `
      query currentRuns($projectId: GlobalID!, $projectVersionId: GlobalID!) {
        currentRuns(projectId: $projectId, projectVersionId: $projectVersionId) {
          ...OperationInfoContent
          ... on SessionState {
            runs {
              ...RunContent
            }
          }
        }
      }
    `),
    filter as any,
    { enabled: computed(() => filter.projectId.value != null && filter.projectVersionId.value != null) } as any
  );

  // init from initial query
  const currentRuns = ref<Record<string, any>>({});
  onInitialLoaded((result) => {
    if (result?.data.currentRuns?.__typename == "SessionState" && result.data.currentRuns.runs != null) {
      for (const run of result.data.currentRuns.runs.map((r) => useFragment(RunContentType, r))) {
        currentRuns.value[run.id] = run;
      }
    }
  });

  if (options.live) {
    // then subscribe to changes as they come if live
    const {
      onResult: onSessionChange,
      start,
      stop,
    } = useSubscription(
      graphql(/* GraphQL */ `
        subscription sessionsChanged($projectId: GlobalID!, $projectVersionId: GlobalID!) {
          sessionsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {
            ... on SessionChange {
              runs {
                ...RunContent
              }
            }
          }
        }
      `),
      {
        ...filter,
      }
    );
  }

  function runsOf(statement: { id: string }) {
    return computed(() => Object.values(currentRuns.value).filter((run) => run.runnable?.id === statement.id));
  }

  return {
    loading: initialLoading,
    currentRuns,
    runsOf,
  };
}

function _useCurrentSessions() {
  const bench = useBenchState();
  const projectId = computed(() => bench.projectId);
  const projectVersionId = computed(() => bench.projectVersionId);
  return _useSessions({ projectId, projectVersionId }, { live: true });
}

export const useCurrentSessions = createSharedComposable(_useCurrentSessions);

export function useRuns(
  filter: {
    projectId: Ref<string | null>;
    projectVersionId: Ref<string | null | undefined>;
    runnableIds: Ref<string[] | null>;
    sessionId: Ref<string | null>;
    runId: Ref<string | null>;
    rootOnly: Ref<boolean>;
  },
  options: {
    live?: boolean;
    limit?: number;
    count?: boolean;
  }
) {
  filter = wrapValueRefs(filter);

  const { result: initialResult, loading: initialLoading } = useQuery(
    graphql(/* GraphQL */ `
      query runs(
        $projectId: GlobalID!
        $projectVersionId: GlobalID!
        $runnableIds: [GlobalID!]
        $sessionId: GlobalID
        $runId: GlobalID
        $rootOnly: Boolean!
      ) {
        runs(
          projectId: $projectId
          projectVersionId: $projectVersionId
          runnableIds: $runnableIds
          sessionId: $sessionId
          runId: $runId
          rootOnly: $rootOnly
        ) {
          totalCount
          pageInfo {
            hasNextPage
            hasPreviousPage
            startCursor
            endCursor
          }
          edges {
            node {
              ...RunContent
            }
            cursor
          }
        }
      }
    `),
    {
      ...filter,
      limit: options.limit,
      count: options.count,
    } as any,
    {
      enabled: computed(() => filter.projectId.value != null && filter.projectVersionId.value != null),
    }
  );

  if (options.live) {
    const sessions = useCurrentSessions();
    sessions.onRunChange((run) => {
      // nocheckin do something
    });
  }
}

export function isMostlyCached(run: { duration?: number; cachedDuration?: number }): boolean {
  return run.duration != null && run.cachedDuration != null && run.cachedDuration > run.duration * 0.8;
}

export function getCachedPercentage(run: { duration?: number | null; cachedDuration?: number | null }): number {
  return 100 - ((run.duration ?? 0) * 100) / (run.cachedDuration ?? 0);
}
