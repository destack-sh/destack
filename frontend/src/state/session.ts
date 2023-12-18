import BusySpinnerIcon from "@/components/basic/BusySpinnerIcon.vue";
import { formatDuration, useNow } from "@/composables/useNow";
import { graphql, useFragment } from "@/gql";
import type { SearchLogsQuery, Conditional, SearchRunsQueryVariables, Sort } from "@/gql/graphql";
import {
  RunStatus,
  type Run,
  type LogEntry,
  type WorkerSet,
  WorkerProfile,
  WorkerSetStatus,
  type CurrentRunsQueryVariables,
  type SearchLogsQueryVariables,
  type LogsChangedSubscriptionVariables,
  StartRunErrorType,
  ConditionalOp,
} from "@/gql/graphql";
import { useBenchState } from "@/state/bench";
import { combineConditionals } from "@/state/database";
import { newRunId, newSessionId } from "@/state/module";
import { useNotifications } from "@/state/notifications";
import { useSessionOps } from "@/state/operations/session";
import { getUpdatedConnectionQueryMany, type Connection, getUpdatedConnectionQuery } from "@/utils/connection";
import { getUUIDFromGlobalID, wrapValueRefs } from "@/utils/functools";
import {
  CheckCircleIcon as CheckCircleIconOutline,
  ClockIcon as ClockIconOutline,
  PauseCircleIcon as PauseCircleIconOutline,
  QuestionMarkCircleIcon as QuestionMarkCircleIconOutline,
  QueueListIcon as QueueListIconOutline,
  XCircleIcon as XCircleIconOutline,
} from "@heroicons/vue/24/outline";

import {
  CheckCircleIcon as CheckCircleIconSolid,
  ClockIcon as ClockIconSolid,
  PauseCircleIcon as PauseCircleIconSolid,
  QuestionMarkCircleIcon as QuestionMarkCircleIconSolid,
  QueueListIcon as QueueListIconSolid,
  XCircleIcon as XCircleIconSolid,
} from "@heroicons/vue/24/solid";
import { useApolloClient, useQuery, useSubscription } from "@vue/apollo-composable";
import { createSharedComposable, useDebounceFn } from "@vueuse/core";
import { DateTime } from "luxon";
import { computed, onBeforeUnmount, reactive, ref, watch, type Ref, toRef } from "vue";

export const TERMINAL_RUN_STATUSES = [RunStatus.Cancelled, RunStatus.Aborted, RunStatus.Failed, RunStatus.Completed];
export const ACTIVE_RUN_STATUSES = [RunStatus.Queued, RunStatus.Running, RunStatus.Aborting]; // scheduled doesn't count as active

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
      ... on Run {
        id
      }
      ... on Session {
        id
      }
    }
    statementCk
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
      ... on Run {
        id
      }
      ... on Session {
        id
      }
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
    value
    statementCk
    # trigger
    triggerType
    trigger {
      id
      type
    }
    triggerUser {
      id
      username
      name
    }
    triggerAccessToken {
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
    statementId
    statementCk
    runId
    stream
    level
    logger
    message
    value
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

  const { client } = useApolloClient();
  const bench = useBenchState();

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
    filter as CurrentRunsQueryVariables,
    { enabled: computed(() => filter.projectId.value != null && filter.projectVersionId.value != null) } as any
  );

  // init from initial query
  const workerSets = ref<Record<string, WorkerSet>>({});
  const currentRuns = ref<Record<string, Run>>({});
  const localRunsIds = reactive(new Set<string>());
  const killingRunsIds = reactive(new Set<string>());
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
      { ...filter } as CurrentRunsQueryVariables,
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
    // TODO @Performance @Robustness: periodically purge stale runs (that are inactive and not the latest for any statement in the module)
  }

  function onRunChange(subscriber: (run: Run) => void): () => void {
    if (!options.live) throw new Error("onRunChange only works when live");
    onRunChangeSubscribers.value.push(subscriber);
    return () => {
      const index = onRunChangeSubscribers.value.indexOf(subscriber);
      if (index >= 0) {
        onRunChangeSubscribers.value.splice(index, 1);
      }
    };
  }

  function subscribeToRun(run: { id: string }, subscriber: (run: Run) => void): void {
    // register on run change subscriber until the given run has terminated
    const unsub = onRunChange((r) => {
      if (r.id == run.id) {
        subscriber(r);
        if (TERMINAL_RUN_STATUSES.includes(r.status)) {
          unsub();
        }
      }
    });
  }

  function onWorkerSetChange(subscriber: (workerSet: WorkerSet) => void): () => void {
    if (!options.live) throw new Error("onWorkerSetChange only works when live");
    onWorkerSetChangeSubscribers.value.push(subscriber);
    return () => {
      const index = onWorkerSetChangeSubscribers.value.indexOf(subscriber);
      if (index >= 0) {
        onWorkerSetChangeSubscribers.value.splice(index, 1);
      }
    };
  }

  // automatically refetch worker sets if they're not ready and the last update is >5s ago
  // (this doesn't actually _do_ anything, it's just to ensure the UI remains fresh)
  // TODO @Cleanup @Architecture: manual worker set syncing should not be needed?
  watch(
    () => Object.values(workerSets.value).map((w) => w.updatedAt),
    async () => {
      if (workerSetReady.value) return;
      const lastUpdate = Math.max(...Object.values(workerSets.value).map((w) => w.updatedAt));
      if (lastUpdate > Date.now() - 5000) return;
      refetchWorkerSetsDebounced();
    }
  );

  async function _refetchWorkerSets() {
    console.debug("workerSet.refetch");
    const { data } = await client.query({
      query: graphql(/* GraphQL */ `
        query refetchProjectWorkerSets($projectId: GlobalID!) {
          project(id: $projectId) {
            id
            workerSets {
              ...WorkerSetContent
            }
          }
        }
      `),
      variables: { projectId: filter.projectId.value },
    });
    if (data?.project?.workerSets != null) {
      for (const ws of data.project.workerSets.map((ws) => useFragment(WorkerSetContentType, ws))) {
        workerSets.value[ws.id] = ws as WorkerSet;
        for (const subscriber of onWorkerSetChangeSubscribers.value) {
          subscriber(ws as WorkerSet);
        }
      }
    }
  }

  const refetchWorkerSetsDebounced = useDebounceFn(_refetchWorkerSets, 5000, { maxWait: 30000 });

  //
  // session ops
  //

  const notifications = useNotifications();
  const sessionOps = useSessionOps();
  const workerSet: Ref<WorkerSet | undefined> = computed(() => Object.values(workerSets.value)[0]); // only one worker set for now
  const workerSetReady = computed(() => workerSet.value?.status === WorkerSetStatus.Healthy);
  const ready = computed(() => workerSet.value?.status == WorkerSetStatus.Healthy);
  const waking = ref(false);
  const restarting = ref(false);

  // worker sets

  function wakeWorkerSet(): Promise<boolean> {
    if (workerSetReady.value) return Promise.resolve(true);
    waking.value = true;
    const unsub = onWorkerSetChange((ws) => {
      if (ws.status === WorkerSetStatus.Healthy) {
        unsub();
        waking.value = false;
      }
    });
    return sessionOps
      .wakeWorkerSet(filter.projectId.value as string)
      .then((r) => r?.data?.wakeWorkerSet.__typename == "WakeWorkerSetPayload" && r?.data?.wakeWorkerSet?.success)
      .then((success) => {
        if (!success) {
          waking.value = false;
        }
        return success;
      })
      .catch(() => {
        waking.value = false;
        return false;
      });
  }

  function restartWorkerSet(): Promise<boolean> {
    restarting.value = true;
    return sessionOps
      .restartWorkerSet(filter.projectId.value as string)
      .then(
        (r) => r?.data?.restartWorkerSet.__typename != "RestartWorkerSetPayload" || !r?.data?.restartWorkerSet?.success
      )
      .finally(() => {
        restarting.value = false;
      });
  }

  function withWorkers<T>(fn: () => Promise<T>, options?: { timeout?: number }): Promise<T> {
    if (!workerSetReady.value) {
      return wakeWorkerSet().then((success) => {
        if (!success) {
          return Promise.reject(new Error("wake workers failed"));
        }
        // if workers are ready now, just run function
        if (workerSetReady.value) {
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
    statementOrCode: { id: string; ck: string } | string,
    options?: {
      statementId?: string;
      scope?: string;
      sessionId?: string;
      runId?: string;
      inputs?: any;
      block?: number;
      keyed?: boolean;
      rootValue?: any;
      globalValue?: any;
      accessLevel?: number;
      timeoutSeconds?: number;
      tags?: string[];
    }
  ): { run: Run; firstResult: Promise<{ run: Run; logs?: LogEntry[] }>; finalResult: Promise<{ run: Run }> } {
    const runId = options?.runId ?? newRunId();
    const sessionId = options?.sessionId ?? newSessionId();
    const code = typeof statementOrCode === "string" ? statementOrCode : undefined;
    const statement = typeof statementOrCode === "string" ? undefined : statementOrCode;
    if (!bench.canUse) {
      throw new Error("cannot run bench");
    }

    const run = {
      __typename: "Run",
      id: runId,
      status: RunStatus.Queued,
      startedAt: null,
      createdAt: new Date().toISOString(),
      updatedAt: new Date().toISOString(),
      duration: null,
      statementCk: statement?.ck,
      // should these be keyed?
      inputs: options?.inputs ?? {},
      outputs: null,
      value: options?.rootValue ?? null,
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
    console.debug("run.start", run.id, run.statementCk, Object.keys(run.inputs));

    function _discardRun() {
      delete currentRuns.value[runId];
      localRunsIds.delete(runId);
    }

    function doRunWithLogs() {
      return sessionOps
        .run(statement?.id ?? options?.statementId, options?.scope, code, run.id, run.session?.id, run.inputs, {
          block: options?.block,
          keyed: options?.keyed,
          globalValue: options?.globalValue,
          rootValue: options?.rootValue,
          accessLevel: options?.accessLevel ?? SessionAccessLevel.Full,
          timeoutSeconds: options?.timeoutSeconds,
          tags: options?.tags,
        })
        .then((r) => {
          if (
            r?.data?.run?.__typename == "OperationInfo" ||
            (r?.data?.run?.__typename == "RunState" && (!r?.data?.run?.success || r?.data?.run.run == null))
          ) {
            delete currentRuns.value[runId];
            let message;
            if (r?.data?.run?.__typename == "OperationInfo") {
              message = "Could not connect.";
            } else if ((r?.data?.run as Run)?.error == StartRunErrorType.Unavailable) {
              message = "Workers are unavailable.";
            } else {
              message = "Invalid request.";
            }
            notifications.show({
              kind: "error",
              type: "run.failed",
              message: "Run could not start",
              description: message,
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
        });
    }

    const firstResult = withWorkers(doRunWithLogs).catch((e) => {
      _discardRun();
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
    const finalResult = new Promise<{ run: Run }>((resolve, reject) => {
      // if first result is terminal, return that, otherwise subscribe until termination
      firstResult.then(({ run, logs }) => {
        if (TERMINAL_RUN_STATUSES.includes(run.status)) {
          resolve({ run });
        } else {
          const unsub = onRunChange((r) => {
            if (r.id === run.id && TERMINAL_RUN_STATUSES.includes(r.status)) {
              unsub();
              resolve({ run: r });
            }
          });
        }
      });
    });

    return {
      run,
      firstResult,
      finalResult,
    };
  }

  function pause(run: { id: string }) {
    throw new Error("not implemented yet");
  }

  function resume(run: { id: string }) {
    throw new Error("not implemented yet");
  }

  function kill(run: { id: string }): Promise<boolean> {
    console.debug("run.kill", run.id);
    if (currentRuns.value[run.id] == null) {
      return Promise.resolve(false);
    }
    killingRunsIds.add(run.id);
    return sessionOps.kill(run.id, true).then((r) => {
      killingRunsIds.delete(run.id);
      if (r?.data?.killRun.__typename != "KillRunPayload") {
        return false;
      } else {
        const killed = r?.data?.killRun?.run as Run | null;
        if (killed == null) {
          delete currentRuns.value[run.id]; // why would this happen?
        } else {
          currentRuns.value[run.id] = killed;
        }
        return true;
      }
    });
  }

  // utilities

  const currentRoots = computed(() => Object.values(currentRuns.value).filter((run) => run.parent == null));
  const activeRuns = computed(() =>
    Object.values(currentRuns.value).filter((run) => ACTIVE_RUN_STATUSES.includes(run.status))
  );
  const activeRoots = computed(() => activeRuns.value.filter((run) => run.parent == null));

  function runsOf(statement: { ck: string }) {
    return computed(() =>
      Object.values(currentRuns.value)
        .filter((run) => run.statementCk === statement.ck)
        .sort((a, b) => b.createdAt.localeCompare(a.createdAt))
    );
  }

  function currentRunOf(statement: { ck: string }) {
    const runs = runsOf(statement);
    const currentRun = computed(() => runs.value.filter((r) => r.status != RunStatus.Scheduled)[0]);
    const currentRunActive = computed(() => ACTIVE_RUN_STATUSES.includes(currentRun.value?.status ?? ""));
    return { runs, currentRun, currentRunActive };
  }

  const now = useNow(100);
  function getDurationSeconds(run: Pick<Run, "createdAt" | "startedAt" | "terminatedAt" | "duration">): number {
    if (run.duration != null) return run.duration;
    const end = run.terminatedAt != null ? DateTime.fromISO(run.terminatedAt) : now.value;
    const duration = end.diff(DateTime.fromISO(run.startedAt ?? run.createdAt)).as("seconds");
    return Math.max(0, duration);
  }

  function getDurationFormatted(
    run: Pick<Run, "createdAt" | "startedAt" | "terminatedAt" | "duration">,
    options?: { hideMillis?: boolean }
  ): string {
    return formatDuration(getDurationSeconds(run) * 1000, options);
  }

  function isActive(run: { id: string }) {
    const currentRun = currentRuns.value[run.id];
    return (currentRun != null && ACTIVE_RUN_STATUSES.includes(currentRun.status)) || killingRunsIds.has(run.id);
  }

  function isKilling(run: { id: string }) {
    return killingRunsIds.has(run.id);
  }

  return {
    loading: initialLoading,
    ready,
    waking,
    restarting,
    wakeWorkerSet,
    restartWorkerSet,
    workerSet,
    workerSetReady,
    currentRuns,
    currentRoots,
    localRunsIds,
    activeRuns,
    activeRoots,
    onRunChange,
    subscribeToRun,
    onWorkerSetChange,
    runsOf,
    currentRunOf,
    isActive,
    isKilling,
    run,
    pause,
    resume,
    kill,
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

export type RunsQuery = {
  statementIds?: string[];
  sessionId?: string;
  runId?: string;
  rootOnly?: boolean;
  query?: Conditional;
};

export function useRuns(
  filter: {
    projectId: Ref<string | null>;
    projectVersionId: Ref<string | null | undefined>;
    statementCks: Ref<string[] | null | undefined>;
    sessionId: Ref<string | null>;
    runId: Ref<string | null>;
    rootOnly: Ref<boolean>;
    query: Ref<Conditional | null | undefined>;
    sort: Ref<Sort[] | null | undefined>;
    after: Ref<string | null | undefined>;
  },
  options?: {
    live?: boolean;
    limit?: number;
    count?: boolean;
    insertAt?: "start" | "end";
    // we don't parse & apply the query locally, therefore filters must be checked manually
    queryAsFilter?: (run: Run) => boolean;
    neverUnsubscribe?: boolean;
  }
) {
  /**
   * Gets all runs that match the given filter (without descendants)
   */
  filter = wrapValueRefs(filter);

  const combinedVariables: Ref<SearchRunsQueryVariables> = computed(() => {
    // remap run filters into query (probably should remove them outright and only keep query?)
    const clauses: Conditional[] = [];
    if (filter.query.value != null) {
      clauses.push(filter.query.value);
    }
    if (filter.projectId.value != null) {
      clauses.push({
        op: ConditionalOp.Equals,
        field: "project_id",
        value: getUUIDFromGlobalID(filter.projectId.value),
      });
    }
    if (filter.statementCks.value?.length) {
      clauses.push({ op: ConditionalOp.In, field: "statement_ck", value: filter.statementCks.value });
    }
    if (filter.sessionId.value != null) {
      clauses.push({ op: ConditionalOp.Equals, field: "session_id", value: filter.sessionId.value });
    }
    if (filter.runId.value != null) {
      clauses.push({ op: ConditionalOp.Equals, field: "id", value: filter.runId.value });
    }
    if (filter.rootOnly.value) {
      clauses.push({ op: ConditionalOp.NotExists, field: "root_id" });
    }
    const query = combineConditionals(ConditionalOp.And, clauses);

    return {
      projectId: filter.projectId.value,
      query: query,
      sort: filter.sort.value,
      after: filter.after.value,
      limit: options?.limit,
      count: options?.count,
    };
  });
  const RUNS_QUERY = graphql(/* GraphQL */ `
    query searchRuns(
      $projectId: GlobalID!
      $query: Conditional
      $sort: [Sort!]
      $after: String
      $limit: Int
      $count: Boolean
    ) {
      searchRuns(projectId: $projectId, query: $query, sort: $sort, after: $after, limit: $limit, count: $count) {
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
        ((filter.statementCks.value?.length ?? 0) && !filter.statementCks.value?.includes(run.statementCk ?? "")) ||
        (filter.sessionId.value != null && run.session?.id !== filter.sessionId.value) ||
        (options?.queryAsFilter != null && !options.queryAsFilter(run))
      ) {
        return;
      }
      client.client.cache.updateQuery(
        {
          query: RUNS_QUERY,
          variables: combinedVariables.value,
        },
        (prev) => {
          const updated = getUpdatedConnectionQuery(
            run,
            prev?.searchRuns as Connection<Run> | undefined,
            options?.limit,
            options?.insertAt ?? "start",
            "RunEdge"
          ) as any;
          return { searchRuns: updated };
        }
      );
    });
    if (!options.neverUnsubscribe) {
      onBeforeUnmount(unsub);
    }
  }

  return {
    loading: initialLoading,
    totalCount: computed(() => result.value?.searchRuns.totalCount),
    pageInfo: computed(() => result.value?.searchRuns.pageInfo),
    runs: computed(() => result.value?.searchRuns.edges.map((e) => useFragment(RunContentType, e.node))),
  };
}

export function useRun(rootId: Ref<string | null>, options?: { live?: boolean }) {
  /**
   * Gets the entire trace of a single session/run
   */
  // rootId = toValueRef(rootId);
  const RUN_QUERY = graphql(/* GraphQL */ `
    query runById($id: GlobalID!) {
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
    enabled: computed(() => rootId.value != null) as any,
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
    return descendants.value ?? [run.value];
  });
  const childrenByParentId = computed(() => {
    const nodesByParent: Record<string, Run[]> = {};
    for (const node of nodes.value ?? []) {
      if (node.parent?.__typename == "Run") {
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

export type LogsQuery = {
  statementIds?: string[] | null | undefined;
  sessionId?: string | null | undefined;
  runId?: string | null | undefined;
  query?: Conditional | null | undefined;
};

export function useLogs(
  filter: {
    projectId: Ref<string>;
    projectVersionId: Ref<string | null | undefined>;
    statementIds: Ref<string[] | null | undefined>;
    statementCks: Ref<string[] | null | undefined>;
    sessionId: Ref<string | null | undefined>;
    runId: Ref<string | null | undefined>;
    query: Ref<Conditional | null | undefined>;
    sort: Ref<Sort[] | null | undefined>;
    after: Ref<string | null | undefined>;
  },
  options?: { live?: boolean; limit?: number; count?: boolean }
) {
  /**
   * Gets all logs that match the given filter
   */
  filter = wrapValueRefs(filter);
  const client = useApolloClient();
  const combinedVariables: Ref<SearchLogsQueryVariables> = computed(() => ({
    projectId: filter.projectId.value,
    projectVersionId: filter.projectVersionId.value,
    statementIds: filter.statementIds.value,
    statementCks: filter.statementCks.value,
    sessionId: filter.sessionId.value,
    runId: filter.runId.value,
    query: filter.query.value,
    sort: filter.sort.value,
    after: filter.after.value,
    limit: options?.limit,
    count: options?.count,
  }));
  const LOGS_QUERY = graphql(/* GraphQL */ `
    query searchLogs(
      $projectId: GlobalID!
      $projectVersionId: GlobalID
      $statementIds: [GlobalID!]
      $statementCks: [UUID!]
      $sessionId: GlobalID
      $runId: GlobalID
      $query: Conditional
      $sort: [Sort!]
      $after: String
      $limit: Int
      $count: Boolean
    ) {
      searchLogs(
        projectId: $projectId
        projectVersionId: $projectVersionId
        statementIds: $statementIds
        statementCks: $statementCks
        sessionId: $sessionId
        runId: $runId
        query: $query
        sort: $sort
        after: $after
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
  const { loading: initialLoading, result: logs, onResult } = useQuery(LOGS_QUERY, combinedVariables, {});

  // we temporarily remember logs that were added live/manually to restore them in case the loading took longer
  // the whole log syncing situation is a bit messy because of sync issues like this...
  const preAddedLogs: LogEntry[] = [];
  onResult((logs) => {
    if (preAddedLogs.length > 0) {
      addLogs(preAddedLogs);
      preAddedLogs.length = 0;
    }
  });

  if (options?.live) {
    // use useSubscription because subscribeToMore doesn't work properly if the query is not enabled
    const { onResult: onLogsAdded } = useSubscription(
      graphql(/* GraphQL */ `
        subscription logsChanged(
          $projectId: GlobalID!
          $projectVersionId: GlobalID!
          $statementIds: [GlobalID!]
          $statementCks: [UUID!]
          $sessionId: GlobalID
          $runId: GlobalID
        ) {
          logsChanged(
            projectId: $projectId
            projectVersionId: $projectVersionId
            statementIds: $statementIds
            statementCks: $statementCks
            sessionId: $sessionId
            runId: $runId
          ) {
            logs {
              ...LogEntryContent
            }
          }
        }
      `),
      combinedVariables as Ref<LogsChangedSubscriptionVariables>
    );
    onLogsAdded((logs) => addLogs(logs.data?.logsChanged?.logs?.map((l) => useFragment(LogEntryContentType, l)) ?? []));
  }

  function addLogs(newLogs: LogEntry[]) {
    if (logs.value == null) {
      preAddedLogs.push(...newLogs);
    }
    client.client.cache.updateQuery(
      {
        query: LOGS_QUERY,
        variables: combinedVariables.value,
      },
      (prev) => {
        return {
          searchLogs: getUpdatedConnectionQueryMany(newLogs, prev?.searchLogs as Connection<LogEntry> | undefined),
        } as SearchLogsQuery;
      }
    );
  }

  return {
    loading: initialLoading,
    logs: computed(() => logs.value?.searchLogs.edges.map((e) => useFragment(LogEntryContentType, e.node))),
    addLogs,
  };
}

export enum SessionAccessLevel { // :SessionAccessLevel
  None = 0,
  Read = 1,
  Create = 2,
  Update = 3,
  Delete = 4,
  Full = 4,
}

export const SESSION_ACCESS_LEVELS = [
  SessionAccessLevel.Read,
  SessionAccessLevel.Create,
  SessionAccessLevel.Update,
  SessionAccessLevel.Delete,
];

export const SESSION_ACCESS_LEVEL_NAME: Record<SessionAccessLevel, string> = {
  [SessionAccessLevel.None]: "None",
  [SessionAccessLevel.Read]: "Read",
  [SessionAccessLevel.Create]: "Create",
  [SessionAccessLevel.Update]: "Update",
  [SessionAccessLevel.Delete]: "Delete",
};

export function getRunStatusIconOutline(status: RunStatus) {
  return RUN_STATUS_ICON_OUTLINE[status];
}

export const RUN_STATUS_ICON_OUTLINE: Record<RunStatus, any> = {
  [RunStatus.Scheduled]: ClockIconOutline,
  [RunStatus.Queued]: QueueListIconOutline,
  [RunStatus.Running]: BusySpinnerIcon,
  [RunStatus.Suspended]: PauseCircleIconOutline,
  [RunStatus.Aborting]: XCircleIconOutline,
  [RunStatus.Aborted]: XCircleIconOutline,
  [RunStatus.Cancelled]: XCircleIconOutline,
  [RunStatus.Failed]: XCircleIconOutline,
  [RunStatus.Completed]: CheckCircleIconOutline,
};

export function getRunStatusIconSolid(status: RunStatus) {
  return RUN_STATUS_ICON_SOLID[status];
}

export const RUN_STATUS_ICON_SOLID: Record<RunStatus, any> = {
  [RunStatus.Scheduled]: ClockIconSolid,
  [RunStatus.Queued]: QueueListIconSolid,
  [RunStatus.Running]: BusySpinnerIcon,
  [RunStatus.Suspended]: PauseCircleIconSolid,
  [RunStatus.Aborting]: XCircleIconSolid,
  [RunStatus.Aborted]: XCircleIconSolid,
  [RunStatus.Cancelled]: XCircleIconSolid,
  [RunStatus.Failed]: XCircleIconSolid,
  [RunStatus.Completed]: CheckCircleIconSolid,
};

export const RUN_STATUS_NAME: Record<RunStatus, string> = {
  [RunStatus.Aborted]: "Aborted",
  [RunStatus.Aborting]: "Aborting",
  [RunStatus.Cancelled]: "Cancelled",
  [RunStatus.Completed]: "Completed",
  [RunStatus.Failed]: "Failed",
  [RunStatus.Queued]: "Queued",
  [RunStatus.Running]: "Running",
  [RunStatus.Scheduled]: "Scheduled",
  [RunStatus.Suspended]: "Suspended",
};

export function getRunStatusColor(status: RunStatus, options?: { gray?: string }) {
  if (status == RunStatus.Aborting || status == RunStatus.Aborted || status == RunStatus.Cancelled) {
    return "text-yellow-600";
  } else if (status == RunStatus.Failed) {
    return "text-red-600";
  } else if (status == RunStatus.Completed) {
    return "text-green-700";
  } else {
    return options?.gray ?? "text-gray-700";
  }
}

export const WORKER_STATUS_COLOR = {
  [WorkerSetStatus.Pending]: "text-yellow-600",
  [WorkerSetStatus.Healthy]: "text-green-700",
  [WorkerSetStatus.Unavailable]: "text-red-600",
  [WorkerSetStatus.Unhealthy]: "text-yellow-600",
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
