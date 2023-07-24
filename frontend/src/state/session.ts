import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { graphql, useFragment } from "@/gql";
import { RunStatus, type Run, type LogEntry, type WorkerSet, WorkerProfile, WorkerSetStatus } from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { newRunId } from "@/state/module";
import { useNotifications } from "@/state/notifications";
import { useSessionOps } from "@/state/operations/session";
import { getUpdatedConnectionQueryMany, type Connection, getUpdatedConnectionQuery } from "@/utils/connection";
import { toValueRef, wrapValueRefs } from "@/utils/functools";
import {
  CheckCircleIcon as CheckCircleIconOutline,
  PauseCircleIcon as PauseCircleIconOutline,
  QuestionMarkCircleIcon as QuestionMarkCircleIconOutline,
  XCircleIcon as XCircleIconOutline,
} from "@heroicons/vue/24/outline";

import {
  CheckCircleIcon as CheckCircleIconSolid,
  PauseCircleIcon as PauseCircleIconSolid,
  QuestionMarkCircleIcon as QuestionMarkCircleIconSolid,
  XCircleIcon as XCircleIconSolid,
} from "@heroicons/vue/24/solid";
import { useApolloClient, useQuery, useSubscription } from "@vue/apollo-composable";
import { createSharedComposable, whenever } from "@vueuse/core";
import { computed, onBeforeUnmount, ref, type Ref } from "vue";

export const RUN_TERMINAL_STATES = [RunStatus.Aborted, RunStatus.Failed, RunStatus.Completed];

export const WorkerSetContentType = graphql(/* GraphQL */ `
  fragment WorkerSetContent on WorkerSet {
    id
    project {
      id
    }
    region
    profile
    sleeping
    status
    desiredReplicas
    targetReplicas
    availableReplicas
    readyReplicas
  }
`);

export const RunHeaderType = graphql(/* GraphQL */ `
  fragment RunHeader on Run {
    id
    createdAt
    updatedAt
    startedAt
    terminatedAt
    duration
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
    runnable {
      id
      name
    }
  }
`);

export const RunContentType = graphql(/* GraphQL */ `
  fragment RunContent on Run {
    id
    createdAt
    updatedAt
    startedAt
    terminatedAt
    duration
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
    metadata
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

// :WorkerProfiles
export const WORKER_RESOURCES_BY_PROFILE = {
  [WorkerProfile.Tiny]: { name: "Tiny", cpu: 0.5, mem: 0.5 },
  [WorkerProfile.Small]: { name: "Small", cpu: 1, mem: 2 },
  [WorkerProfile.Medium]: { name: "Medium", cpu: 2, mem: 4 },
  [WorkerProfile.Large]: { name: "Large", cpu: 4, mem: 8 },
  [WorkerProfile.XlargeCpu]: { name: "XLarge CPU", cpu: 8, mem: 16 },
  [WorkerProfile.XlargeMem]: { name: "XLarge RAM", cpu: 4, mem: 64 },
};

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
            workerSet {
              ...WorkerSetContent
            }
          }
        }
      }
    `),
    filter as any,
    { enabled: computed(() => filter.projectId.value != null && filter.projectVersionId.value != null) } as any
  );

  // init from initial query
  const workerSets = ref<Record<string, WorkerSet>>({});
  const currentRuns = ref<Record<string, Run>>({});
  onInitialLoaded((result) => {
    if (result?.data?.currentRuns?.__typename == "SessionState") {
      if (result.data.currentRuns.runs != null) {
        for (const run of result.data.currentRuns.runs.map((r) => useFragment(RunContentType, r))) {
          currentRuns.value[run.id] = run as Run;
        }
      }
      if (result.data.currentRuns.workerSet != null) {
        const workerSet = useFragment(WorkerSetContentType, result.data.currentRuns.workerSet);
        workerSets.value[workerSet.id] = workerSet as WorkerSet;
      }
    }
  });

  // then subscribe to changes as they come if live
  const onRunChangeSubscribers = ref<((run: Run) => void)[]>([]);
  if (options.live) {
    const { onResult: onSessionChange } = useSubscription(
      graphql(/* GraphQL */ `
        subscription sessionsChanged($projectId: GlobalID!, $projectVersionId: GlobalID!) {
          sessionsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {
            ... on SessionChange {
              runs {
                ...RunContent
              }
            }
            ... on WorkerChange {
              workerSets {
                ...WorkerSetContent
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
          currentRuns.value[run.id] = run as Run;
          for (const subscriber of onRunChangeSubscribers.value) {
            subscriber(run as Run);
          }
        }
      }
      if (result.data?.sessionsChanged?.__typename == "WorkerChange") {
        for (const ws of result.data.sessionsChanged.workerSets.map((ws) => useFragment(WorkerSetContentType, ws))) {
          workerSets.value[ws.id] = ws as WorkerSet;
        }
      }
    });
    // TODO @Performance @Robustness: periodically purge stale runs (that are inactive and not the latest for any runnable in the module)
  }

  function onRunChange(subscriber: (run: Run) => void): () => void {
    if (!options.live) {
      throw new Error("onRunChange only makes sense when live");
    }
    onRunChangeSubscribers.value.push(subscriber);
    return () => {
      const index = onRunChangeSubscribers.value.indexOf(subscriber);
      if (index >= 0) {
        onRunChangeSubscribers.value.splice(index, 1);
      }
    };
  }

  // session ops

  const notifications = useNotifications();
  const sessionOps = useSessionOps();
  const workerSet: Ref<WorkerSet | undefined> = computed(() => Object.values(workerSets.value)[0]); // only one worker set for now
  const isWorkerSetReady = computed(() => workerSet.value?.status === WorkerSetStatus.Healthy);
  const wakingPromise = ref<Promise<boolean> | undefined>(undefined); // if currently waking the worker set

  function wakeWorkerSet(): Promise<boolean> {
    // only wake if not already waking
    if (wakingPromise.value != null) {
      return wakingPromise.value;
    } else {
      wakingPromise.value = sessionOps
        .wakeWorkerSet(filter.projectId.value as string)
        .then((r) => r?.data?.wakeWorkerSet?.__typename == "WakeWorkerSetPayload" ?? false);
      return wakingPromise.value;
    }
  }

  // nocheckin: need some better mechanism for the wake/sleep cycle
  const ready = computed(() => workerSet.value?.status == WorkerSetStatus.Healthy);
  whenever(ready, () => (wakingPromise.value = undefined));

  function withWorkers<T>(fn: () => Promise<T>) {
    if (!isWorkerSetReady.value) {
      return wakeWorkerSet().then(fn);
    } else {
      return fn();
    }
  }

  function run(
    runnable: { id: string },
    options?: { sessionId?: string; runId?: string; arguments?: any; block?: boolean; keyed?: boolean }
  ): { run: Run; promise: Promise<Run> } {
    const runId = options?.runId ?? newRunId();
    const sessionId = options?.sessionId ?? newRunId();
    const run = {
      __typename: "Run",
      id: runId,
      status: RunStatus.Queued,
      startedAt: new Date().toISOString(),
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      duration: null,
      inputs: options?.arguments ?? {},
      outputs: null,
      metadata: null,
      error: null,
      session: {
        __typename: "Session",
        id: sessionId,
      },
      root: null,
      parent: null,
    } as Run;

    currentRuns.value[runId] = run;

    const promise = withWorkers(() =>
      sessionOps.run(runnable.id, run.id, run.session.id, run.inputs, {
        block: options?.block,
        keyed: options?.keyed,
      })
    );
    return { run, promise: promise as Promise<Run> };
  }

  function pause(run: { id: string }) {
    throw new Error("not implemented yet");
  }

  function resume(run: { id: string }) {
    throw new Error("not implemented yet");
  }

  function cancel(run: { id: string }): Promise<boolean> {
    return withWorkers(() => sessionOps.cancel(run.id).then((r) => r?.data?.cancelRun?.success ?? false));
  }

  // utilities

  const currentRoots = computed(() => Object.values(currentRuns.value).filter((run) => run.parent == null));
  const activeRuns = computed(() =>
    Object.values(currentRuns.value).filter((run) => !RUN_TERMINAL_STATES.includes(run.status))
  );
  const activeRoots = computed(() => activeRuns.value.filter((run) => run.parent == null));

  function runsOf(statement: { id: string }) {
    return computed(() => Object.values(currentRuns.value).filter((run) => run.runnable?.id === statement.id));
  }

  return {
    loading: initialLoading,
    ready,
    waking: computed(() => wakingPromise.value != null),
    workerSet,
    currentRuns,
    currentRoots,
    activeRuns,
    activeRoots,
    onRunChange,
    runsOf,
    run,
    pause,
    resume,
    cancel,
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
  options?: {
    live?: boolean;
    limit?: number;
    count?: boolean;
  }
) {
  /**
   * Gets all runs that match the given filter (without descendants)
   */
  filter = wrapValueRefs(filter);

  const combinedVariables = computed(() => ({
    projectId: filter.projectId.value,
    projectVersionId: filter.projectVersionId.value,
    runnableIds: filter.runnableIds.value,
    sessionId: filter.sessionId.value,
    runId: filter.runId.value,
    rootOnly: filter.rootOnly.value,
    limit: options?.limit,
    count: options?.count,
  }));
  const RUNS_QUERY = graphql(/* GraphQL */ `
    query runs(
      $projectId: GlobalID!
      $projectVersionId: GlobalID!
      $runnableIds: [GlobalID!]
      $sessionId: GlobalID
      $runId: GlobalID
      $rootOnly: Boolean!
      $limit: Int
      $count: Boolean
    ) {
      runs(
        projectId: $projectId
        projectVersionId: $projectVersionId
        runnableIds: $runnableIds
        sessionId: $sessionId
        runId: $runId
        rootOnly: $rootOnly
        limit: $limit
        count: $count
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
  const { result: result, loading: initialLoading } = useQuery(RUNS_QUERY, combinedVariables as any, {
    enabled: computed(() => filter.projectId.value != null && filter.projectVersionId.value != null) as any,
    fetchPolicy: "network-only",
  });

  const client = useApolloClient();
  if (options?.live) {
    const sessions = useCurrentSessions();
    const unsub = sessions.onRunChange((run) => {
      if (
        (filter.projectVersionId.value != null && run.projectVersion?.id !== filter.projectVersionId.value) ||
        (filter.runnableIds.value != null && !filter.runnableIds.value.includes(run.runnable?.id ?? "")) ||
        (filter.sessionId.value != null && run.session?.id !== filter.sessionId.value)
      ) {
        return;
      }
      client.client.cache.updateQuery(
        {
          query: RUNS_QUERY,
          variables: combinedVariables.value as any,
        },
        (prev) => {
          return {
            runs: getUpdatedConnectionQuery(run, prev?.runs as Connection<Run> | undefined, options?.limit),
          };
        }
      );
    });
    onBeforeUnmount(unsub);
  }

  return {
    loading: initialLoading,
    totalCount: computed(() => result.value?.runs.totalCount),
    runs: computed(() => result.value?.runs.edges.map((e) => useFragment(RunContentType, e.node))),
  };
}

export function useRun(rootId: Ref<string>, options?: { live?: boolean }) {
  /**
   * Gets the entire trace of a single session/run
   */
  rootId = toValueRef(rootId);
  const RUN_QUERY = graphql(/* GraphQL */ `
    # getRun as not to conflict with run from runtime
    query getRun($id: GlobalID!) {
      run(id: $id) {
        ...RunContent
        descendants {
          ...RunContent
        }
      }
    }
  `);

  const { result: initialResult, loading: initialLoading } = useQuery(RUN_QUERY, { id: rootId } as any, {
    fetchPolicy: "network-only",
  });

  const client = useApolloClient();
  if (options?.live) {
    const sessions = useCurrentSessions();
    const unsub = sessions.onRunChange((run) => {
      if (run.id === rootId.value) {
        // run was just created
        client.client.cache.updateQuery(
          {
            query: RUN_QUERY,
            variables: { id: rootId.value },
          },
          (prev) => {
            return {
              run: {
                ...run,
                descendants: prev?.run?.descendants ?? run?.descendants ?? [],
              },
            };
          }
        );
      } else if (run.root?.id !== rootId.value) {
        return; // ignore from other run
      } else {
        // add descendant if it's not already there
        client.client.cache.updateQuery(
          {
            query: RUN_QUERY,
            variables: { id: rootId.value },
          },
          (prev) => {
            const descendants = prev?.run?.descendants ?? [];
            const index = descendants.findIndex((r) => (r as Run).id === run.id);
            if (index != -1) {
              return prev;
            }
            return {
              run: {
                ...prev?.run,
                descendants: [...descendants, run],
              },
            };
          }
        );
      }
    });
    onBeforeUnmount(unsub);
  }

  const run = computed(() => useFragment(RunContentType, initialResult.value?.run));
  const descendants = computed(() => initialResult.value?.run?.descendants.map((r) => useFragment(RunContentType, r)));
  const nodes = computed(() => {
    if (run.value == null) return null;
    return [run.value, ...(descendants.value ?? [])];
  });
  const childrenByParentId = computed(() => {
    const nodesByParent: Record<string, Run[]> = {};
    for (const node of nodes.value ?? []) {
      if (node.parent != null) {
        if (nodesByParent[node.parent.id] == null) {
          nodesByParent[node.parent.id] = [];
        }
        nodesByParent[node.parent.id].push(node as Run);
      }
    }
    return nodesByParent;
  });

  return {
    loading: initialLoading,
    run,
    descendants,
    childrenByParentId,
    nodes,
  };
}

export function useLogs(
  filter: {
    projectId: Ref<string>;
    projectVersionId: Ref<string | null | undefined>;
    runnableIds: Ref<string[] | null | undefined>;
    sessionId: Ref<string | null | undefined>;
    runId: Ref<string | null | undefined>;
  },
  options?: { live?: boolean; limit?: number; count?: boolean }
) {
  /**
   * Gets all logs that match the given filter
   */
  filter = wrapValueRefs(filter);
  const combinedVariables = computed(() => ({
    projectId: filter.projectId.value,
    projectVersionId: filter.projectVersionId.value,
    runnableIds: filter.runnableIds.value,
    sessionId: filter.sessionId.value,
    runId: filter.runId.value,
    limit: options?.limit,
    count: options?.count,
  }));
  const LOGS_QUERY = graphql(/* GraphQL */ `
    query logs(
      $projectId: GlobalID!
      $projectVersionId: GlobalID
      $runnableIds: [GlobalID!]
      $sessionId: GlobalID
      $runId: GlobalID
      $limit: Int
      $count: Boolean
    ) {
      logs(
        projectId: $projectId
        projectVersionId: $projectVersionId
        runnableIds: $runnableIds
        sessionId: $sessionId
        runId: $runId
        limit: $limit
        count: $count
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
  } = useQuery(LOGS_QUERY, combinedVariables as any, {
    fetchPolicy: "network-only",
  });

  const client = useApolloClient();
  if (options?.live) {
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
      variables: combinedVariables as any,
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
        variables: combinedVariables.value,
      },
      (prev) => {
        return {
          logs: getUpdatedConnectionQueryMany(logs, prev?.logs as Connection<LogEntry> | undefined),
        };
      }
    );
  }

  return {
    loading: initialLoading,
    logs: computed(() => initialResult.value?.logs.edges.map((e) => useFragment(LogEntryContentType, e.node))),
    addLogs,
  };
}

export function getStatusIconOutline(status: RunStatus) {
  if (status == RunStatus.Queued || status == RunStatus.Running) {
    return BusySpinnerIcon;
  } else if (status == RunStatus.Aborting || status == RunStatus.Aborted) {
    return XCircleIconOutline;
  } else if (status == RunStatus.Suspended) {
    return PauseCircleIconOutline;
  } else if (status == RunStatus.Failed) {
    return XCircleIconOutline;
  } else if (status == RunStatus.Completed) {
    return CheckCircleIconOutline;
  } else {
    return QuestionMarkCircleIconOutline;
  }
}

export function getStatusIconSolid(status: RunStatus) {
  if (status == RunStatus.Queued || status == RunStatus.Running) {
    return BusySpinnerIcon;
  } else if (status == RunStatus.Aborting || status == RunStatus.Aborted) {
    return XCircleIconSolid;
  } else if (status == RunStatus.Suspended) {
    return PauseCircleIconSolid;
  } else if (status == RunStatus.Failed) {
    return XCircleIconSolid;
  } else if (status == RunStatus.Completed) {
    return CheckCircleIconSolid;
  } else {
    return QuestionMarkCircleIconSolid;
  }
}

export function getStatusColor(status: RunStatus) {
  if (status == RunStatus.Queued || status == RunStatus.Running || status == RunStatus.Scheduled) {
    return "text-gray-700";
  } else if (status == RunStatus.Aborting || status == RunStatus.Aborted) {
    return "text-gray-700";
  } else if (status == RunStatus.Failed) {
    return "text-red-600";
  } else if (status == RunStatus.Completed) {
    return "text-green-700";
  } else {
    return "text-gray-700";
  }
}
