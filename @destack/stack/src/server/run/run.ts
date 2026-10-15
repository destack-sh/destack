import { SpaceAllowance, Space, space } from "@destack/account/object";
import { Caller, principal } from "@destack/access";
import * as sync from "@destack/sync";
import {
    and,
    asc,
    eq,
    lt,
    type DatabaseConnection,
    DatabaseError,
    type RowImage,
    type Select,
    Snapshot,
} from "@destack/db";
import { type Controller, Reconciliation } from "@destack/runtime/control";
import type { ObjectStore } from "@destack/object/server";
import {
    Call,
    type CallOf,
    type NextOf,
    type ObjectReconciliation,
    ObjectController,
    ObjectWatch,
    PushResult,
    type ResultOf,
    type ObjectCall,
    type ResultsOf,
} from "@destack/object";
import { aligned, Clock, Identifier, type JsonObject, present } from "@destack/schema";
import { type ExchangeAsk, TOKEN_LIFETIME_MILLISECONDS } from "@destack/runtime/authentication";
import {
    errorOf,
    failureOf,
    isCallError,
    CallError,
    TRANSIENT_STATUSES,
} from "@destack/runtime/error";
import { PackageMount } from "@destack/runtime";
import { VERSION_HEADER } from "@destack/runtime/request";
import { RunEvent, RunRequest } from "@destack/runtime/trigger";
import {
    isSpanContextValid,
    ROOT_CONTEXT,
    SpanStatusCode,
    telemetry,
    trace,
    TraceFlags,
    type Span,
} from "@destack/telemetry";
import {
    deployment,
    Installation,
    installation,
    Instance,
    instance,
    Run,
    run,
    runOutcome,
    type RunOutcome,
    trigger,
} from "../../object/index.ts";

/** The most attempts of a run: five. */
const MAX_ATTEMPTS = 5;

/** The runs attempted at once: 16 calls of 10 to 500 ms finish 30 to 1600 a second. */
const RUN_CONCURRENCY = 16;

/** The path of an object store's push, below its package's mount. */
const PUSH_PATH = "/replica/push";

/** What an installation's object store answers for one pushed mutation. */
export type PushOutcome = PushResult["outcomes"][number]["outcome"];

/** How a machine attempts runs: push each run's call to its installation's object store. */
export interface RunAttemptOptions {
    /** Push a mutation to an installation's object store at a time. */
    readonly push: (
        installation: Pick<Installation, "id" | "scope" | "packageId">,
        mutation: sync.Mutation,
        now: number,
        signal: AbortSignal,
        exchange: ExchangeAsk | null,
    ) => Promise<PushOutcome>;
    /** The machine attempting the runs of the spaces it serves. */
    readonly machine: Identifier<"machine">;
    /** The most attempts of a run including the first, five by default. */
    readonly maximumAttempts?: number;
}

/** The run attempt options with their default and the store's clock. */
type Resolved = Required<RunAttemptOptions> & { readonly clock: Clock };

/** A machine's ingress, serving a request to an installation's deployments as a caller. */
export interface Ingress {
    /** Serve a request at a path to the deployments of an installation as a caller. */
    readonly ingress: (
        installationId: Identifier<"installation">,
        path: string,
        request: Request,
        caller: Caller,
    ) => Promise<Response>;
}

/** Act for the subject of a run under its grant to the run's installation, as a verified caller. */
export type RunExchange = (
    installation: Pick<Installation, "id" | "scope" | "packageId">,
    exchange: ExchangeAsk,
) => Promise<Caller>;

/** Push runs' calls through a machine's ingress to the deployments serving their releases. */
export function pushThrough(dispatcher: Ingress, act: RunExchange): RunAttemptOptions["push"] {
    return async (target, mutation, now, signal, exchange) => {
        // call as the installation in its space, or for the subject who sent the call under its grant
        const subject = principal.installation.reference(target.scope, target.id);
        const caller: Caller =
            exchange === null
                ? {
                      subject,
                      subjects: [subject],
                      credential: {
                          kind: "run",
                          id: mutation.id,
                          audience: target.packageId,
                          scope: target.scope,
                          verifiedAt: now,
                          expiresAt: now + TOKEN_LIFETIME_MILLISECONDS,
                      },
                  }
                : await act(target, exchange);
        const path = `${PackageMount.path(target.packageId)}${PUSH_PATH}`;
        const request = new Request(`http://${target.id}${path}`, {
            method: "POST",
            headers: {
                "content-type": "application/json",
                [VERSION_HEADER]: aligned(mutation.calls, 0).release,
            },
            body: JSON.stringify({ scope: target.scope, mutations: [mutation] }),
            signal,
        });

        // answer the mutation's outcome
        const response = await dispatcher.ingress(target.id, path, request, caller);
        if (response.ok) {
            return aligned(PushResult.parse(await response.json()).outcomes, 0).outcome;
        }

        // throw a transient refusal for another attempt and answer a final one as the call's failure
        const failure = await failureOf(response);
        if (TRANSIENT_STATUSES.has(response.status) || response.status >= 500) {
            throw errorOf(failure);
        }

        return { error: failure };
    };
}

/** Record the runs of this machine's installations, which its run controller attempts. */
export function serveRuns() {
    return run.implement({
        methods: {
            send: (call, next) => send(call, next),
            cancel: async (call) => {
                await close(call, { kind: "skipped", skip: { reason: "cancelled" } });

                return {};
            },
            finish: async (call) => {
                await close(call, call.input.outcome);

                return {};
            },
        },
    });
}

/** Attempt the runs of a space's installations over its store. */
export function controlRuns(store: ObjectStore, attempts: RunAttemptOptions): Controller {
    const resolved: Resolved = {
        push: attempts.push,
        machine: attempts.machine,
        clock: store.clock,
        maximumAttempts: attempts.maximumAttempts ?? MAX_ATTEMPTS,
    };

    const controlled = run.implement({
        controller: {
            pending: {},
            concurrency: RUN_CONCURRENCY,
            watches: [
                ObjectWatch.of(run.table, selectQueueHead),
                ObjectWatch.of(installation.table, selectInstallationRuns),
                ObjectWatch.of(instance.table, selectInstanceRuns),
                ObjectWatch.of(space.table, (row, database) => liveRuns(row.id, database)),
                Space.watch(liveRuns),
            ],
            reconcile: (reconciliation) =>
                attempt(
                    aligned(reconciliation.rows, 0),
                    { ...reconciliation, now: resolved.clock.now() },
                    resolved,
                ),
        },
    });

    return ObjectController.control(
        store,
        controlled,
        present(controlled.controller, "the run controller"),
    );
}

/** Record a sent or fired call once per request. */
async function send(
    call: CallOf<typeof run, "send">,
    next: NextOf<typeof run, "send">,
): Promise<ResultOf<typeof run, "send">> {
    // read the fired trigger's call when the request has a trigger or else the sent call
    const { installationId, ...fields } = call.input;
    const request = parseRequest(fields);

    // resolve the fired trigger
    const fired =
        "triggerName" in request ? await firedTrigger(call, installationId, request) : undefined;
    const requestId = fired === undefined ? call.requestId : RunEvent.requestId(fired.event);
    if (requestId === undefined) {
        throw new CallError("BAD_REQUEST", { message: "a sent run needs its request" });
    }

    // answer the run its request already has
    const recorded = await recordedRun(call, installationId, fired?.id ?? null, requestId);
    if (recorded !== undefined) {
        return recorded;
    }

    // run for the subject who sent the call under its grant
    const exchange = "exchange" in request ? request.exchange : undefined;

    // record the run and queue a change trigger's runs in order
    const at = "at" in request ? request.at : undefined;
    const input = {
        requestId,
        installationId,
        call: request.call,
        ...(fired === undefined ? {} : { triggerId: fired.id, event: fired.event }),
        at: at ?? call.now,
        state: "pending" as const,
        ...(exchange === undefined ? {} : { exchange }),
        attempts: 0,
        concurrency: fired?.isChange === true ? "queue" : "allow",
    };

    // answer a request another delivery recorded meanwhile
    try {
        return await next(call.with({ input }));
    } catch (error) {
        const raced =
            error instanceof DatabaseError && error.code === "DUPLICATE"
                ? await recordedRun(call, installationId, fired?.id ?? null, requestId)
                : undefined;
        if (raced === undefined) {
            throw error;
        }

        return raced;
    }
}

/** Parse a run request as a fired trigger's call or a sent call. */
function parseRequest(fields: Readonly<Record<string, unknown>>): RunRequest {
    // read the fired trigger's call when the request has a trigger or else the sent call
    const variant = "triggerName" in fields ? RunRequest.fired : RunRequest.sent;
    const parsed = variant.safeParse(fields);
    if (parsed.success) {
        return parsed.data;
    }

    // refuse the fields the request takes no part of and the missing or invalid ones
    const issues = parsed.error.issues;
    const extra = issues.flatMap((issue) => (issue.code === "unrecognized_keys" ? issue.keys : []));
    const invalid = issues
        .filter((issue) => issue.code !== "unrecognized_keys")
        .map((issue) => issue.path.join("."));
    throw new CallError("BAD_REQUEST", {
        message:
            extra.length > 0
                ? `a run does not take these fields: ${extra.join(", ")}`
                : `a run needs valid fields: ${invalid.join(", ")}`,
    });
}

/** Resolve the declared trigger a workload's request fired. */
async function firedTrigger(
    call: Call<typeof run.table>,
    installationId: Identifier<"installation">,
    request: Extract<RunRequest, { readonly triggerName: string }>,
) {
    // find the trigger the installation's package declares under the name
    const [found] = await call.read(trigger).findMany({
        where: {
            installationId,
            origin: "declared",
            name: request.triggerName,
        },
    });
    // refuse an unknown trigger
    if (found === undefined) {
        throw new CallError("NOT_FOUND", { message: `no trigger ${request.triggerName}` });
    }
    // refuse a schedule's trigger the machine records runs for
    else if ("schedule" in found.on) {
        throw new CallError("BAD_REQUEST", {
            message: `trigger ${request.triggerName} fires on a schedule, whose runs are the machine's to record`,
        });
    }
    // refuse a paused trigger
    else if (found.isPaused) {
        throw new CallError("CONFLICT", { message: `trigger ${request.triggerName} is paused` });
    }

    // refuse an event of another kind than the trigger fires on
    const isChange = "change" in found.on;
    if (!((isChange ? "change" : "delivery") in request.event)) {
        throw new CallError("BAD_REQUEST", {
            message: `trigger ${request.triggerName} fires on ${isChange ? "changes" : "webhook deliveries"}`,
        });
    }

    return { id: found.id, event: request.event, isChange };
}

/** Close a live run: record its run event under its trigger's name and remove its row. */
async function close(
    call: Call<typeof run.table, JsonObject, Select<typeof run.table>>,
    outcome: RunOutcome,
) {
    // read the name of the trigger that fired the run
    const current = call.target;
    const fired =
        current.triggerId === null
            ? undefined
            : await call.read(trigger).findFirst({ where: { id: current.triggerId } });

    // record the event and remove the row
    const event = Run.event(current, outcome, fired?.name ?? null, call.now);
    await call.mutate(call.self, [
        runOutcome.call("record", { ...runOutcome.in(current.scope), items: [event] }),
    ]);
    await call.remove();
}

/** Find the run a request already recorded for an installation's trigger or its sent calls. */
async function recordedRun(
    call: Call,
    installationId: Identifier<"installation">,
    triggerId: Identifier<"trigger"> | null,
    requestId: string,
): Promise<Run | undefined> {
    const [found] = await call.read(run).findMany({
        where: {
            installationId,
            triggerId: triggerId === null ? { isNull: true } : triggerId,
            requestId,
        },
    });

    return found;
}

/** Select the live runs of a space. */
async function liveRuns(spaceId: Identifier<"space">, database: DatabaseConnection) {
    return await database
        .select({ id: run.table.id })
        .from(run.table)
        .where(eq(run.table.scope, spaceId));
}

/** Select the head of the queue a changed or finished queued run belongs to. */
async function selectQueueHead(changed: RowImage<typeof run.table>, database: DatabaseConnection) {
    // look at nothing else for a run outside a queue
    if (changed.concurrency !== "queue" || changed.triggerId === null) {
        return [];
    }

    // select the head of the trigger's queue: its live run of the earliest request
    const [head] = await database
        .select({ id: run.table.id })
        .from(run.table)
        .where(eq(run.table.triggerId, changed.triggerId))
        .orderBy(asc(run.table.requestId))
        .limit(1);

    return head === undefined ? [] : [head];
}

/** Select the waiting runs of an installation that serves again. */
async function selectInstallationRuns(
    row: RowImage<typeof installation.table>,
    database: DatabaseConnection,
) {
    return database
        .select({ id: run.table.id })
        .from(run.table)
        .where(and(eq(run.table.installationId, row.id), eq(run.table.state, "pending")));
}

/** Select the waiting runs of an installation whose instance changed, such as one now running. */
async function selectInstanceRuns(
    row: RowImage<typeof instance.table>,
    database: DatabaseConnection,
) {
    return database
        .select({ id: run.table.id })
        .from(run.table)
        .innerJoin(deployment.table, eq(deployment.table.installationId, run.table.installationId))
        .where(and(eq(deployment.table.id, row.deploymentId), eq(run.table.state, "pending")));
}

/** Attempt a due run once in the trace it records, returning the wait until it is due. */
async function attempt(
    current: Run,
    reconciliation: ObjectReconciliation,
    options: Resolved,
): Promise<number | undefined> {
    // wait for a run not due
    if (current.at > reconciliation.now) {
        return current.at - reconciliation.now;
    }

    // leave a run whose installation is not served here or runs no instance
    const target = await Installation.findServed(reconciliation.database, current.installationId);
    const endpoints =
        target === undefined ? [] : await Instance.endpoints(reconciliation.database, target.id);
    if (target === undefined || endpoints.length === 0) {
        return undefined;
    }

    // leave a run of a space whose compute allowance is exceeded
    const owner = await SpaceAllowance.read(Snapshot.live(reconciliation.database), current.scope);
    if (SpaceAllowance.isExceeded(owner, "compute")) {
        return undefined;
    }

    // leave a queued run behind an earlier one of its trigger
    const queue = current.concurrency === "queue" ? current.triggerId : null;
    if (queue !== null && (await isBehind(reconciliation.database, queue, current))) {
        return undefined;
    }

    // fail a run whose attempts ran out even when they never returned
    if (current.attempts >= options.maximumAttempts) {
        const failure = current.failure ?? {
            code: "ATTEMPTS_EXHAUSTED",
            status: 500,
            message: `${current.attempts} attempts started without finishing`,
        };
        await mutateIfCurrent(reconciliation, finished(current, { kind: "failed", failure }));

        return undefined;
    }

    // push the run's call within its trace
    return pushInTrace(current, target, reconciliation, options);
}

/** Push a run's call within its trace: the one it continues, or its first attempt's. */
function pushInTrace(
    current: Run,
    target: Installation,
    reconciliation: ObjectReconciliation,
    options: Resolved,
): Promise<undefined> {
    // continue the run's trace or start the one its first attempt records
    const tracer = telemetry.scope(import.meta.destack.package).tracer;
    const parent =
        current.traceId === null
            ? ROOT_CONTEXT
            : trace.setSpanContext(ROOT_CONTEXT, {
                  traceId: current.traceId,
                  spanId: crypto.getRandomValues(new Uint8Array(8)).toHex(),
                  traceFlags: TraceFlags.SAMPLED,
                  isRemote: true,
              });
    const attributes = {
        ...(current.triggerId === null ? {} : { "destack.run.trigger": current.triggerId }),
        "destack.run.method": sync.Call.label(current.call),
    };

    return tracer.startActiveSpan(
        `run ${sync.Call.label(current.call)}`,
        { attributes },
        parent,
        async (span): Promise<undefined> => {
            try {
                await push(current, target, span, reconciliation, options);

                return undefined;
            } finally {
                span.end();
            }
        },
    );
}

/** Push a run's call as its installation and record the outcome. */
async function push(
    current: Run,
    target: Installation,
    span: Span,
    reconciliation: ObjectReconciliation,
    options: Resolved,
): Promise<void> {
    // start the attempt fenced by the revision read
    const [started] = await mutateIfCurrent(
        reconciliation,
        run.call("update", {
            ...run.target(current),
            state: "running",
            attempts: current.attempts + 1,
            startedAt: current.startedAt ?? reconciliation.now,
            traceId: current.traceId ?? readTrace(span),
        }),
    );
    if (started === undefined) {
        return;
    }

    // push the call once under the run's own request until the run is cancelled or replaced
    const pushed = await pushOnce(started, target, reconciliation, options);

    // finish an answered call: succeeded with its value, failed with its refusal
    if (pushed.kind === "answered") {
        const { outcome } = pushed;
        // finish a call answered with its value
        if ("value" in outcome) {
            await mutateIfCurrent(reconciliation, finished(started, { kind: "succeeded" }));
        }
        // fail a call answered with its refusal
        else {
            span.setStatus({ code: SpanStatusCode.ERROR, message: outcome.error.message });
            const failed = { kind: "failed", failure: outcome.error } as const;
            await mutateIfCurrent(reconciliation, finished(started, failed));
        }

        return;
    }

    // mark the span failed with the thrown failure
    const failure = sync.Failure.of(pushed.error);
    span.setStatus({ code: SpanStatusCode.ERROR, message: failure.message });

    // leave a run cancelled, replaced or stopped
    if (pushed.kind === "aborted") {
        return;
    }
    // fail a run out of attempts
    else if (started.attempts >= options.maximumAttempts) {
        await mutateIfCurrent(reconciliation, finished(started, { kind: "failed", failure }));

        return;
    }

    // wait for the next attempt through the control loop's backoff
    const waiting = { ...run.target(started), state: "pending", failure } as const;
    await mutateIfCurrent(reconciliation, run.call("update", waiting));
    throw pushed.error;
}

/** How one push of a run's call ended. */
type Push =
    | {
          /** The installation answered the call. */
          readonly kind: "answered";
          /** The call's outcome. */
          readonly outcome: PushOutcome;
      }
    | {
          /** The push threw, or threw once aborted. */
          readonly kind: "thrown" | "aborted";
          /** The thrown failure. */
          readonly error: unknown;
      };

/** Push a started run's call once under its own request. */
async function pushOnce(
    started: Run,
    target: Installation,
    reconciliation: ObjectReconciliation,
    options: Resolved,
): Promise<Push> {
    // push the call, answering its outcome, or the failure it threw and whether it was aborted
    const mutation = { id: Identifier.uuid(started.id), calls: [started.call] };

    return Reconciliation.runUntil(
        reconciliation,
        () => endedElsewhere(reconciliation, started),
        async (signal): Promise<Push> => {
            try {
                const outcome = await options.push(
                    target,
                    mutation,
                    options.clock.now(),
                    signal,
                    started.exchange,
                );

                return { kind: "answered", outcome };
            } catch (error) {
                return { kind: signal.aborted ? "aborted" : "thrown", error };
            }
        },
    );
}

/** Report whether a queued run's trigger has a live run of an earlier request. */
async function isBehind(
    database: DatabaseConnection,
    triggerId: NonNullable<Run["triggerId"]>,
    current: Pick<Run, "requestId">,
): Promise<boolean> {
    const [earlier] = await database
        .select({ id: run.table.id })
        .from(run.table)
        .where(and(eq(run.table.triggerId, triggerId), lt(run.table.requestId, current.requestId)))
        .limit(1);

    return earlier !== undefined;
}

/** Read why a run ended elsewhere. */
async function endedElsewhere(
    reconciliation: ObjectReconciliation,
    started: Run,
): Promise<Error | undefined> {
    const [current] = await reconciliation.database
        .select({ revision: run.table.revision })
        .from(run.table)
        .where(eq(run.table.id, started.id));

    return current === undefined || current.revision !== started.revision
        ? new CallError("CONFLICT", { message: "the run ended elsewhere" })
        : undefined;
}

/** Finish a run with an outcome through its method. */
function finished(current: Run, outcome: RunOutcome) {
    return run.call("finish", { ...run.target(current), outcome });
}

/** Run one call on a run as the store's principal. */
async function mutateIfCurrent<const Changed extends ObjectCall>(
    reconciliation: ObjectReconciliation,
    changed: Changed,
): Promise<ResultsOf<readonly [Changed]> | []> {
    try {
        return await reconciliation.mutate([changed] as const);
    } catch (error) {
        // leave a run another machine changed or removed since
        if (isGone(error)) {
            return [];
        }
        throw error;
    }
}

/** Report whether a change failed because another machine changed or removed the run. */
function isGone(error: unknown): boolean {
    return isCallError(error) && (error.code === "CONFLICT" || error.code === "NOT_FOUND");
}

/** Read the trace a span records, absent when no tracer records one. */
function readTrace(span: Span): string | null {
    const context = span.spanContext();

    return isSpanContextValid(context) ? context.traceId : null;
}
