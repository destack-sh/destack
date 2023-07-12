import { graphql, useFragment } from "@/gql";
import { RunStatus, type Run, type LogEntry } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { getUpdatedConnectionQueryMany, type Connection, getUpdatedConnectionQuery } from "@/utils/connection";
import { wrapValueRefs } from "@/utils/functools";
import { useApolloClient, useQuery, useSubscription } from "@vue/apollo-composable";
import { createSharedComposable } from "@vueuse/core";
import { computed, onBeforeUnmount, ref, type Ref } from "vue";

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

export const LogEntryContentType = graphql(/* GraphQL */ `
  fragment LogEntryContent on LogEntry {
    id
    createdAt
    projectVersionId
    sessionId
    runnableId
    runId
    stream
    level
    logger
    message
    metadata
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

  const onRunChangeSubscribers = ref<((run: Run) => void)[]>([]);
  if (options.live) {
    // then subscribe to changes as they come if live
    const { onResult: onSessionChange } = useSubscription(
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
    onSessionChange((result) => {
      if (result.data?.sessionsChanged?.__typename == "SessionChange" && result.data.sessionsChanged.runs != null) {
        for (const run of result.data.sessionsChanged.runs.map((r) => useFragment(RunContentType, r))) {
          currentRuns.value[run.id] = run;
          for (const subscriber of onRunChangeSubscribers.value) {
            subscriber(run as Run);
          }
        }
      }
    });
    // TODO @Performance @Robustness: periodically purge stale runs (that are inactive and not the latest for any runnable in the module)
  }

  function onRunChange(subscriber: (run: Run) => void): () => void {
    if (!options.live) {
      throw new Error("onRunChange only makes sense when live is true");
    }
    onRunChangeSubscribers.value.push(subscriber);
    return () => {
      const index = onRunChangeSubscribers.value.indexOf(subscriber);
      if (index >= 0) {
        onRunChangeSubscribers.value.splice(index, 1);
      }
    };
  }

  function runsOf(statement: { id: string }) {
    return computed(() => Object.values(currentRuns.value).filter((run) => run.runnable?.id === statement.id));
  }

  return {
    loading: initialLoading,
    currentRuns,
    onRunChange,
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

  const RUNS_QUERY = graphql(/* GraphQL */ `
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
  `);
  const { result: initialResult, loading: initialLoading } = useQuery(
    RUNS_QUERY,
    {
      ...filter,
      limit: options.limit,
      count: options.count,
    } as any,
    {
      enabled: computed(() => filter.projectId.value != null && filter.projectVersionId.value != null) as any,
    }
  );

  const client = useApolloClient();
  if (options.live) {
    const sessions = useCurrentSessions();
    const unsub = sessions.onRunChange((run) => {
      if (
        (filter.projectVersionId.value != null && run.projectVersion?.id === filter.projectVersionId.value) ||
        (filter.runnableIds.value != null && filter.runnableIds.value.includes(run.runnable?.id ?? "")) ||
        (filter.sessionId.value != null && run.session?.id === filter.sessionId.value)
      ) {
        return;
      }
      client.client.cache.updateQuery(
        {
          query: RUNS_QUERY,
          variables: { ...filter, limit: options.limit, count: options.count } as any,
        },
        (prev) => {
          return {
            runs: getUpdatedConnectionQuery(run, prev?.runs as Connection<Run> | undefined),
          };
        }
      );
    });
    onBeforeUnmount(unsub);
  }

  return {
    loading: initialLoading,
    runs: computed(() => initialResult.value?.runs.edges.map((e) => useFragment(RunContentType, e.node))),
  };
}

export function useLogs(
  filter: {
    projectId: Ref<string | null>;
    projectVersionId: Ref<string | null | undefined>;
    runnableIds: Ref<string[] | null>;
    sessionId: Ref<string | null>;
    runId: Ref<string | null>;
  },
  options: { live: boolean; limit: number }
) {
  const LOGS_QUERY = graphql(/* GraphQL */ `
    query logs(
      $projectId: GlobalID!
      $projectVersionId: GlobalID!
      $runnableIds: [GlobalID!]
      $sessionId: GlobalID
      $runId: GlobalID
    ) {
      logs(
        projectId: $projectId
        projectVersionId: $projectVersionId
        runnableIds: $runnableIds
        sessionId: $sessionId
        runId: $runId
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
            ...LogEntryContent
          }
          cursor
        }
      }
    }
  `);
  const {
    result: initialResult,
    loading: initialLoading,
    subscribeToMore,
  } = useQuery(
    LOGS_QUERY,
    {
      ...filter,
      limit: options.limit,
    } as any,
    {
      enabled: computed(() => filter.projectId.value != null && filter.projectVersionId.value != null) as any,
    }
  );

  const client = useApolloClient();
  if (options.live) {
    subscribeToMore({
      document: graphql(/* GraphQL */ `
        subscription logsChanged(
          $projectId: GlobalID!
          $projectVersionId: GlobalID!
          $runnableIds: [GlobalID!]
          $sessionId: GlobalID
          $runId: GlobalID
        ) {
          logsChanged(
            projectId: $projectId
            projectVersionId: $projectVersionId
            runnableIds: $runnableIds
            sessionId: $sessionId
            runId: $runId
          ) {
            logs {
              ...LogEntryContent
            }
          }
        }
      `),
      variables: {
        projectId: filter.projectId,
        projectVersionId: filter.projectVersionId,
        runnableIds: filter.runnableIds,
        sessionId: filter.sessionId,
        runId: filter.runId,
      },
      updateQuery: (prev, { subscriptionData }) => {
        if (!subscriptionData.data) return prev;
        const logs = subscriptionData.data.logsChanged.logs.map((l) => useFragment(LogEntryContentType, l));
        return {
          logs: getUpdatedConnectionQueryMany(logs, prev.logs as Connection<LogEntry>),
        };
      },
    });
  }

  function addLogs(logs: LogEntry[]) {
    client.client.cache.updateQuery(
      {
        query: LOGS_QUERY,
        variables: { ...filter, limit: options.limit },
      },
      (prev) => {
        return {
          logs: getUpdatedConnectionQueryMany(logs, prev?.logs as Connection<LogEntry> | undefined),
        };
      }
    );
  }

  return {
    logs: computed(() => initialResult.value?.logs.edges.map((e) => useFragment(LogEntryContentType, e.node))),
    addLogs,
  };
}

export function isMostlyCached(run: { duration?: number; cachedDuration?: number }): boolean {
  return run.duration != null && run.cachedDuration != null && run.cachedDuration > run.duration * 0.8;
}

export function getCachedPercentage(run: { duration?: number | null; cachedDuration?: number | null }): number {
  return 100 - ((run.duration ?? 0) * 100) / (run.cachedDuration ?? 0);
}
