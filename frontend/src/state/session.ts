import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { formatDuration, useNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import { RunStatus, type Run, type LogEntry, type WorkerSet, WorkerProfile, WorkerSetStatus } from "@/gql/graphql";
import { useAuth } from "@/state/auth";
import { useBenchState } from "@/state/bench";
import { newRunId, newSessionId } from "@/state/module";
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
import { createSharedComposable } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, onBeforeUnmount, reactive, ref, watch, type Ref } from "vue";

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
    lastActiveAt
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
  const localRunsIds = reactive(new Set<string>());
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
  const onWorkerSetChangeSubscribers = ref<((workerSet: WorkerSet) => void)[]>([]);
  const onRunChangeSubscribers = ref<((run: Run) => void)[]>([]);
  if (options.live) {
    const { onResult: onSessionChange } = useSubscription(
      graphql(/* GraphQL */ `
        subscription sessionsChanged($projectId: GlobalID!, $projectVersionId: GlobalID) {
          sessionsChanged(projectId: $projectId, projectVersionId: $projectVersionId) {
            ... on SessionChange {
              runs {
                ...RunContent
              }
            }
            ... on RunsChange {
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
      { ...filter } as any,
      { enabled: computed(() => filter.projectId.value != null) as any }
    );
    onSessionChange((result) => {
      if (
        result.data?.sessionsChanged?.__typename == "SessionChange" ||
        (result.data?.sessionsChanged?.__typename == "RunsChange" && result.data.sessionsChanged.runs != null)
      ) {
        for (const run of result.data.sessionsChanged.runs.map((r) => useFragment(RunContentType, r))) {
          console.debug("run.change", run);
          currentRuns.value[run.id] = run as Run;
          for (const subscriber of onRunChangeSubscribers.value) {
            subscriber(run as Run);
          }
        }
      }
      if (result.data?.sessionsChanged?.__typename == "WorkerChange") {
        for (const ws of result.data.sessionsChanged.workerSets.map((ws) => useFragment(WorkerSetContentType, ws))) {
          console.debug("workerSet.change", ws);
          workerSets.value[ws.id] = ws as WorkerSet;
          for (const subscriber of onWorkerSetChangeSubscribers.value) {
            subscriber(ws as WorkerSet);
          }
        }
      }
    });
    // TODO @Performance @Robustness: periodically purge stale runs (that are inactive and not the latest for any runnable in the module)
  }

  function onRunChange(subscriber: (run: Run) => void): () => void {
    if (!options.live) throw new Error("onRunChange only makes sense when live");
    onRunChangeSubscribers.value.push(subscriber);
    return () => {
      const index = onRunChangeSubscribers.value.indexOf(subscriber);
      if (index >= 0) {
        onRunChangeSubscribers.value.splice(index, 1);
      }
    };
  }

  function onWorkerSetChange(subscriber: (workerSet: WorkerSet) => void): () => void {
    if (!options.live) throw new Error("onWorkerSetChange only makes sense when live");
    onWorkerSetChangeSubscribers.value.push(subscriber);
    return () => {
      const index = onWorkerSetChangeSubscribers.value.indexOf(subscriber);
      if (index >= 0) {
        onWorkerSetChangeSubscribers.value.splice(index, 1);
      }
    };
  }

  //
  // session ops
  //

  const auth = useAuth();
  const notifications = useNotifications();
  const sessionOps = useSessionOps();
  const workerSet: Ref<WorkerSet | undefined> = computed(() => Object.values(workerSets.value)[0]); // only one worker set for now
  const isWorkerSetReady = computed(() => workerSet.value?.status === WorkerSetStatus.Healthy);
  const ready = computed(() => workerSet.value?.status == WorkerSetStatus.Healthy);
  const waking = ref(false);
  const restarting = ref(false);

  // TODO @UX: auto wake worker set if user is logged in and not idle?
  // (especially when we get to proper LSP)

  // worker sets

  function wakeWorkerSet(): Promise<boolean> {
    waking.value = true;
    return sessionOps
      .wakeWorkerSet(filter.projectId.value as string)
      .then((r) => r?.data?.wakeWorkerSet?.success ?? false)
      .then((success) => {
        if (!success) {
          waking.value = false;
        }
        return success;
      })
      .catch(() => {
        waking.value = false;
      });
  }

  function restartWorkerSet(): Promise<boolean> {
    restarting.value = true;
    return sessionOps
      .restartWorkerSet(filter.projectId.value as string)
      .then((r) => r?.data?.restartWorkerSet?.success ?? false)
      .finally(() => {
        restarting.value = false;
      });
  }

  function withWorkers<T>(fn: () => Promise<T>, options?: { timeout?: number }): Promise<T> {
    if (!isWorkerSetReady.value) {
      return wakeWorkerSet().then(() => {
        // if workers are ready now, just run function
        if (isWorkerSetReady.value) {
          return fn();
        } else {
          // otherwise wait until they're ready
          return new Promise((resolve, reject) => {
            const unsub = onWorkerSetChange((ws) => {
              if (ws.status === WorkerSetStatus.Healthy) {
                unsub();
                resolve(fn());
              }
            });
            if (options?.timeout) {
              setTimeout(() => {
                unsub();
                reject(new Error("timed out waiting for workers"));
              }, options?.timeout);
            }
          });
        }
      });
    } else {
      return fn();
    }
  }

  // running

  function run(
    runnable: { id: string },
    options?: { sessionId?: string; runId?: string; arguments?: any; block?: number; keyed?: boolean }
  ): { run: Run; result: Promise<{ run: Run; logs?: LogEntry[] }> } {
    const runId = options?.runId ?? newRunId();
    const sessionId = options?.sessionId ?? newSessionId();
    const run = {
      __typename: "Run",
      id: runId,
      status: RunStatus.Queued,
      startedAt: null,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      duration: null,
      inputs: options?.arguments ?? {},
      runnable,
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
    localRunsIds.add(runId);
    console.debug("run.start", run.id, run.runnable?.name, run.runnable?.id, Object.keys(run.inputs));

    function doRunWithLogs() {
      return sessionOps
        .run(runnable.id, run.id, run.session.id, run.inputs, {
          block: options?.block,
          keyed: options?.keyed,
        })
        .then((r) => {
          if (
            r?.data?.run?.__typename == "OperationInfo" ||
            (r?.data?.run?.__typename == "RunState" && (!r?.data?.run?.success || r?.data?.run.run == null))
          ) {
            delete currentRuns.value[runId];
            notifications.show({
              kind: "error",
              type: "run.failed",
              message: "Run could not start",
              description: "Your workers are unavailable.",
            });
            throw new Error("run could not start");
          } else if (r?.data?.run.__typename == "RunState") {
            currentRuns.value[runId] = r?.data.run.run as Run;
          }
          return {
            run: currentRuns.value[runId],
            logs:
              r?.data?.run.__typename == "RunState"
                ? r?.data?.run.logs?.map((l) => useFragment(LogEntryContentType, l))
                : undefined,
          };
        })
        .catch((e) => {
          delete currentRuns.value[runId];
          if (e.message !== "run could not start") {
            // damnit, some other error
            notifications.show({
              kind: "error",
              type: "run.failed.internal",
              message: "Run crashed",
              description: "An internal error happened somewhere.",
            });
          }
          throw e;
        });
    }

    return { run, result: withWorkers(doRunWithLogs) };
  }

  function pause(run: { id: string }) {
    throw new Error("not implemented yet");
  }

  function resume(run: { id: string }) {
    throw new Error("not implemented yet");
  }

  function cancel(run: { id: string }): Promise<boolean> {
    console.debug("run.cancel", run.id);
    if (currentRuns.value[run.id] == null) {
      return Promise.resolve(false);
    }
    // TODO @Robustness @UX: cancel should not restart the entire worker
    //  Proper cancellation is annoying on the backend, we only have one concurrent run at a time right now,
    //  so while this isn't great, it works until we have proper concurrency. :RunCancellation
    return restartWorkerSet();
    // return sessionOps.cancel(run.id).then((r) => r?.data?.cancelRun?.success ?? false);
  }

  // utilities

  const currentRoots = computed(() => Object.values(currentRuns.value).filter((run) => run.parent == null));
  const activeRuns = computed(() =>
    Object.values(currentRuns.value).filter((run) => !RUN_TERMINAL_STATES.includes(run.status))
  );
  const activeRoots = computed(() => activeRuns.value.filter((run) => run.parent == null));

  function runsOf(statement: { id: string }) {
    return computed(() =>
      Object.values(currentRuns.value)
        .filter((run) => run.runnable?.id === statement.id)
        .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
    );
  }

  const now = useNow(100);
  function getDurationSeconds(run: { duration?: number; startedAt?: string; createdAt: string }): number {
    if (run.duration != null) return run.duration;
    return now.value.diff(DateTime.fromISO(run.startedAt ?? run.createdAt)).as("seconds");
  }

  function getDurationFormatted(run: { duration?: number; startedAt?: string; createdAt: string }): string {
    return formatDuration(getDurationSeconds(run) * 1000);
  }

  return {
    loading: initialLoading,
    ready,
    waking,
    restarting,
    wakeWorkerSet,
    restartWorkerSet,
    workerSet,
    currentRuns,
    currentRoots,
    localRunsIds,
    activeRuns,
    activeRoots,
    onRunChange,
    onWorkerSetChange,
    runsOf,
    run,
    pause,
    resume,
    cancel,
    getDurationSeconds,
    getDurationFormatted,
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
  options?: { live?: boolean; limit?: number; count?: boolean; skipInitialLoad?: Ref<boolean> }
) {
  /**
   * Gets all logs that match the given filter
   */
  filter = wrapValueRefs(filter);
  const client = useApolloClient();
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
  // separate useQuery to read cache since useQuery doesn't react properly if not enabled
  const { result: logs } = useQuery(LOGS_QUERY, combinedVariables as any, {
    fetchPolicy: "cache-only",
  });
  // only load if not skipping initial load
  const { loading: initialLoading } = useQuery(LOGS_QUERY, combinedVariables as any, {
    enabled: computed(() => !options?.skipInitialLoad?.value) as any,
  });

  // if not enabled, write empty result to cache
  watch(
    () => [combinedVariables.value, options?.skipInitialLoad?.value],
    () => {
      if (!options?.skipInitialLoad?.value) return;
      client.client.cache.writeQuery({
        query: LOGS_QUERY,
        variables: combinedVariables.value as any,
        data: {
          logs: {
            totalCount: 0,
            pageInfo: {
              hasNextPage: false,
              hasPreviousPage: false,
              startCursor: null,
              endCursor: null,
            },
            edges: [],
          },
        },
      });
    },
    { immediate: true, deep: true }
  );

  if (options?.live) {
    // use useSubscription because subscribeToMore doesn't work properly if the query is not enabled
    const { onResult: onLogsAdded } = useSubscription(
      graphql(/* GraphQL */ `
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
      combinedVariables as any
    );
    onLogsAdded((logs) => addLogs(logs.data?.logsChanged?.logs?.map((l) => useFragment(LogEntryContentType, l)) ?? []));
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
    logs: computed(() => logs.value?.logs.edges.map((e) => useFragment(LogEntryContentType, e.node))),
    addLogs,
  };
}

export function getRunStatusIconOutline(status: RunStatus) {
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

export function getRunStatusIconSolid(status: RunStatus) {
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

export function getRunStatusColor(status: RunStatus, options?: { gray?: string }) {
  const gray = options?.gray ?? "text-gray-700";
  if (status == RunStatus.Queued || status == RunStatus.Running || status == RunStatus.Scheduled) {
    return gray;
  } else if (status == RunStatus.Aborting || status == RunStatus.Aborted) {
    return "text-yellow-600";
  } else if (status == RunStatus.Failed) {
    return "text-red-600";
  } else if (status == RunStatus.Completed) {
    return "text-green-600";
  } else {
    return gray;
  }
}

export const WORKER_STATUS_COLOR = {
  [WorkerSetStatus.Pending]: "text-yellow-700",
  [WorkerSetStatus.Healthy]: "text-green-700",
  [WorkerSetStatus.Unavailable]: "text-red-700",
  [WorkerSetStatus.Unhealthy]: "text-yellow-700",
  [WorkerSetStatus.Updating]: "text-gray-500",
  [WorkerSetStatus.Sleeping]: "text-gray-500",
  [WorkerSetStatus.Unknown]: "text-gray-500",
};
export const WORKER_STATUS_ICON_SOLID = {
  [WorkerSetStatus.Pending]: BusySpinnerIcon,
  [WorkerSetStatus.Healthy]: CheckCircleIconSolid,
  [WorkerSetStatus.Unavailable]: XCircleIconSolid,
  [WorkerSetStatus.Unhealthy]: XCircleIconSolid,
  [WorkerSetStatus.Updating]: BusySpinnerIcon,
  [WorkerSetStatus.Sleeping]: PauseCircleIconSolid,
  [WorkerSetStatus.Unknown]: QuestionMarkCircleIconSolid,
};
export const WORKER_STATUS_ICON_OUTLINE = {
  [WorkerSetStatus.Pending]: BusySpinnerIcon,
  [WorkerSetStatus.Healthy]: CheckCircleIconOutline,
  [WorkerSetStatus.Unavailable]: XCircleIconOutline,
  [WorkerSetStatus.Unhealthy]: XCircleIconOutline,
  [WorkerSetStatus.Updating]: BusySpinnerIcon,
  [WorkerSetStatus.Sleeping]: PauseCircleIconOutline,
  [WorkerSetStatus.Unknown]: QuestionMarkCircleIconOutline,
};

export const WORKER_STATUS_TITLE = {
  [WorkerSetStatus.Pending]: "Starting",
  [WorkerSetStatus.Healthy]: "Ready",
  [WorkerSetStatus.Unavailable]: "Unavailable",
  [WorkerSetStatus.Unhealthy]: "Unhealthy",
  [WorkerSetStatus.Updating]: "Starting",
  [WorkerSetStatus.Sleeping]: "Sleeping",
  [WorkerSetStatus.Unknown]: "Unknown",
};
