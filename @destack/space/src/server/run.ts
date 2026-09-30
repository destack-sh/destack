import { principal, sameSubject } from "@destack/access";
import {
    and,
    asc,
    eq,
    inArray,
    isNull,
    lt,
    ne,
    or,
    sql,
    type DatabaseConnection,
} from "@destack/db";
import { DatabaseError } from "@destack/db/error";
import { Condition } from "@destack/db/query";
import type { Router } from "@destack/host/router";
import { type Call, type ObjectReconciliation, PushResult } from "@destack/object";
import { SystemCall } from "@destack/object/server";
import { identifierUuid, schema, type Identifier } from "@destack/schema";
import {
    Caller,
    CALLER_LIFETIME_MILLISECONDS,
    type LendingClaim,
    type LentAuthority,
} from "@destack/service/authentication";
import { ServiceError } from "@destack/service/error";
import { VERSION_HEADER } from "@destack/service/request";
import { RunRequest, runError, type RunError } from "@destack/service/trigger";
import type * as sync from "@destack/sync";
import {
    isSpanContextValid,
    ROOT_CONTEXT,
    SpanStatusCode,
    telemetry,
    trace,
    TraceFlags,
    type Span,
} from "@destack/telemetry";
import type {} from "@destack/package/import-meta";
import {
    ACTIVE_RUN_STATES,
    deployment,
    Installation,
    installation,
    Instance,
    instance,
    type Run,
    run,
} from "../object/index.ts";

/** The most attempts of a run: five, as a Kubernetes Job's backoffLimit bounds retries. */
const MAX_ATTEMPTS = 5;

/** The runs attempted at once: 16 calls of 10 to 500 ms finish 30 to 1600 a second. */
const RUN_CONCURRENCY = 16;

/** The path of an object server's push, below its service. */
const PUSH_PATH = "/replica/push";

/** The statuses of a push worth attempting again: a timeout, a throttle, or a server failure. */
const TRANSIENT_STATUSES: ReadonlySet<number> = new Set([408, 425, 429]);

/** A refusal an object server answers, as its errors encode. */
const Refusal = schema.object({
    /** The stable failure code. */
    code: schema.string().min(1),
    /** The failure message. */
    message: schema.string(),
});

/** What an installation's object server answers for one pushed mutation. */
export type PushOutcome = PushResult["outcomes"][number]["outcome"];

/** How a cell attempts runs: pushing each run's call to its installation's object server. */
export interface RunOptions {
    /** Push a mutation to an installation's object server, on a lent authority when given. */
    push(
        installation: Installation,
        mutation: sync.Mutation,
        signal: AbortSignal,
        onBehalfOf?: LentAuthority,
    ): Promise<PushOutcome>;
    /** Read a lending of a caller's authority to an installation that this cell's host signed, refusing any other. */
    verify(token: string): Promise<LendingClaim>;
    /** The most attempts of a run, the first included, five by default. */
    readonly maximumAttempts?: number;
}

/** The run options with their defaults. */
type Resolved = Required<RunOptions> & {
    /** Read the cell's current time in UTC epoch milliseconds. */
    readonly now: () => number;
};

/** Push runs' calls through a host's ingress to the deployments serving their releases, as their installations. */
export function pushThrough(router: Pick<Router, "ingress">): RunOptions["push"] {
    return async (target, mutation, signal, onBehalfOf) => {
        // call as the installation in its space, or act for the caller whose call sent this one on its lent authority
        const installation = principal.installation.reference(target.scope, target.id);
        const acting = { subject: installation, authority: "lent" as const };
        const authority =
            onBehalfOf === undefined
                ? { subject: installation, subjects: [installation] }
                : { ...onBehalfOf, delegates: [...(onBehalfOf.delegates ?? []), acting] };
        const now = Date.now();
        const caller = new Caller({
            credential: { kind: "run", id: mutation.id },
            audience: target.packageId,
            scope: target.scope,
            ...authority,
            verifiedAt: now,
            expiresAt: now + CALLER_LIFETIME_MILLISECONDS,
        });
        const request = new Request(`http://${target.id}${PUSH_PATH}`, {
            method: "POST",
            headers: {
                "content-type": "application/json",
                [VERSION_HEADER]: mutation.calls[0]!.release,
            },
            body: JSON.stringify({ scope: target.scope, mutations: [mutation] }),
            signal,
        });

        // answer the mutation's outcome
        const response = await router.ingress(target.id, PUSH_PATH, request, caller);
        const text = await response.text();
        if (response.ok) {
            return PushResult.parse(JSON.parse(text)).outcomes[0]!.outcome;
        }

        // throw a transient refusal for another attempt, and answer a final one as the call's failure
        const refusal = Refusal.safeParse(parseJson(text));
        const code = refusal.success ? refusal.data.code : `HTTP_${response.status}`;
        const message = refusal.success ? refusal.data.message : text;
        const isTransient = TRANSIENT_STATUSES.has(response.status) || response.status >= 500;
        if (isTransient) {
            throw new ServiceError(code, { status: response.status, message });
        }

        return { error: { code, status: response.status, message } };
    };
}

/** Serve runs on the cell's clock: record the calls installations send once per cause, and attempt every run on this cell. */
export function serveRuns(options: RunOptions, now: () => number) {
    const resolved: Resolved = {
        push: options.push,
        verify: options.verify,
        maximumAttempts: options.maximumAttempts ?? MAX_ATTEMPTS,
        now,
    };

    return {
        run: run.handle({ send: (call, next) => send(call, next, resolved), cancel }).control({
            pending: Condition.oneOf("state", [...ACTIVE_RUN_STATES]),
            concurrency: RUN_CONCURRENCY,
            watches: [
                { table: run.table, keys: changedRun },
                { table: installation.table, keys: waiting },
                { table: instance.table, keys: served },
            ],
            reconcile: (reconciliation) =>
                attempt(
                    reconciliation.rows[0]!,
                    { ...reconciliation, now: resolved.now() },
                    resolved,
                ),
        }),
    };
}

/** Record a call an installation sends once per cause, and to run now unless it gives a time. */
async function send(
    call: Call<typeof run.table>,
    next: (call?: Call<typeof run.table>) => Promise<unknown>,
    options: Resolved,
): Promise<unknown> {
    // refuse the schedule cause, reserved to the cell
    const {
        spaceId: _spaceId,
        installation: _installation,
        requestId: _requestId,
        ...fields
    } = call.input;
    if (fields.cause === "schedule") {
        throw new ServiceError("BAD_REQUEST", {
            message: "a schedule's runs are the cell's to record",
        });
    }

    // read the cause's own fields, listing the fields that do not fit it
    const parsed = RunRequest.safeParse(
        Object.fromEntries(
            Object.entries(fields).filter(([, value]) => value !== undefined && value !== null),
        ),
    );
    if (!parsed.success) {
        const misfits = parsed.error.issues.flatMap((issue) =>
            issue.code === "unrecognized_keys" ? issue.keys : [issue.path.join(".")],
        );
        throw new ServiceError("BAD_REQUEST", {
            message: `a ${String(fields.cause)} run does not take these fields: ${misfits.join(", ")}`,
        });
    }
    const request = parsed.data;
    const installationId = call.input.installation as Identifier<"installation">;

    // answer the run a webhook's or a watch's cause already has
    const recorded = await recordedCause(call.database, installationId, request);
    if (recorded !== undefined) {
        return recorded;
    }

    // run on behalf of the caller lending its authority, or record the run failed when the lending is invalid
    const delegation = request.cause === "send" ? request.delegation : undefined;
    const lending =
        delegation === undefined
            ? undefined
            : await lent(delegation, installationId, call.scope, options);
    const state =
        lending === undefined || "authority" in lending
            ? {
                  state: "pending",
                  ...(lending === undefined ? {} : { onBehalfOf: lending.authority }),
              }
            : { state: "failed", finishedAt: call.now, error: lending.error };

    // record the cause's own columns, now unless the request gives a time, watches one at a time in order
    const columns =
        request.cause === "send" ? { cause: request.cause, call: request.call } : request;
    const at = request.cause === "send" ? request.at : undefined;
    const input = {
        spaceId: call.input.spaceId,
        requestId: call.input.requestId,
        installation: installationId,
        ...columns,
        at: at ?? call.now,
        ...state,
        attempts: 0,
        concurrency: request.cause === "watch" ? "queue" : "allow",
    };

    // answer a cause another request recorded meanwhile
    try {
        return await next(call.with({ input }));
    } catch (error) {
        const raced =
            error instanceof DatabaseError && error.code === "DUPLICATE"
                ? await recordedCause(call.database, installationId, request)
                : undefined;
        if (raced === undefined) {
            throw error;
        }

        return raced;
    }
}

/** Verify the token a sent call carries, lending its sender's authority to the sending installation in the call's space. */
async function lent(
    token: string,
    installationId: Identifier<"installation">,
    scope: string,
    options: Resolved,
): Promise<{ readonly authority: LentAuthority } | { readonly error: RunError }> {
    // refuse a lending this cell's host did not sign or that lapsed
    const installation = principal.installation.reference(scope, installationId);
    let claim: LendingClaim;
    try {
        claim = await options.verify(token);
    } catch (error) {
        return { error: { code: "DELEGATION_REFUSED", message: runError(error).message } };
    }

    // require the lending to the sending installation in its space
    if (claim.scope !== scope || !sameSubject(claim.installation, installation)) {
        return {
            error: {
                code: "DELEGATION_REFUSED",
                message: `the authority is not lent to ${installation.id}`,
            },
        };
    }

    // keep the lent authority without the lending's own fields
    const {
        installation: _installation,
        scope: _scope,
        expiresAt: _expiresAt,
        ...authority
    } = claim;

    return { authority };
}

/** Cancel a waiting or attempting run, aborting its attempt. */
async function cancel(
    call: Call<typeof run.table>,
    next: (call?: Call<typeof run.table>) => Promise<unknown>,
): Promise<unknown> {
    // refuse a finished run
    const target = call.target!;
    if (!isActive(target)) {
        throw new ServiceError("CONFLICT", { message: `run ${target.id} is ${target.state}` });
    }

    return next(
        call.with({
            input: {
                ...call.input,
                state: "skipped",
                finishedAt: call.now,
                error: { code: "CANCELLED", message: "the run was cancelled" },
            },
        }),
    );
}

/** Find the run a webhook's delivery or a watch's change already has. */
async function recordedCause(
    database: DatabaseConnection,
    installationId: Identifier<"installation">,
    request: RunRequest,
): Promise<Run | undefined> {
    // match the cause's own columns, a sent call being recorded once by its request instead
    if (request.cause === "send") {
        return undefined;
    }
    const cause =
        request.cause === "webhook"
            ? eq(run.table.deliveryId, request.deliveryId)
            : and(
                  eq(run.table.epoch, request.epoch),
                  eq(run.table.sequence, request.sequence),
                  request.key === undefined
                      ? isNull(run.table.key)
                      : eq(run.table.key, request.key),
              );
    const [found] = await database
        .select()
        .from(run.table)
        .where(
            and(
                eq(run.table.installation, installationId),
                eq(run.table.cause, request.cause),
                eq(run.table.packageId, request.packageId),
                eq(run.table.trigger, request.trigger),
                cause,
            ),
        );

    return found;
}

/** Select the runs a changed run concerns: itself while active, and the head of its queue once it finished. */
async function changedRun(
    row: Readonly<Record<string, unknown>>,
    database: DatabaseConnection,
): Promise<readonly Readonly<Record<string, unknown>>[]> {
    // attempt an active run, and look at nothing else for a finished run outside a queue
    const changed = row as unknown as Run;
    if (isActive(changed)) {
        return [{ id: changed.id }];
    } else if (changed.concurrency !== "queue") {
        return [];
    }

    // select the head of the queue: the earliest change of the finished run's epoch, or the earliest epoch
    const of = and(
        eq(run.table.installation, changed.installation),
        eq(run.table.cause, "watch"),
        eq(run.table.packageId, changed.packageId!),
        eq(run.table.trigger, changed.trigger!),
        inArray(run.table.state, [...ACTIVE_RUN_STATES]),
    );
    const [inEpoch] = await database
        .select({ id: run.table.id })
        .from(run.table)
        .where(and(of, eq(run.table.epoch, changed.epoch!)))
        .orderBy(asc(run.table.sequence), asc(sql`coalesce(${run.table.key}, '')`))
        .limit(1);
    const [earliest] =
        inEpoch !== undefined
            ? [inEpoch]
            : await database
                  .select({ id: run.table.id })
                  .from(run.table)
                  .where(of)
                  .orderBy(asc(run.table.createdAt), asc(run.table.id))
                  .limit(1);

    return earliest === undefined ? [] : [earliest];
}

/** Select the waiting runs of an installation that serves again. */
async function waiting(
    row: Readonly<Record<string, unknown>>,
    database: DatabaseConnection,
): Promise<readonly Readonly<Record<string, unknown>>[]> {
    return database
        .select({ id: run.table.id })
        .from(run.table)
        .where(
            and(
                eq(run.table.installation, row.id as Identifier<"installation">),
                inArray(run.table.state, [...ACTIVE_RUN_STATES]),
            ),
        );
}

/** Select the waiting runs of an installation whose instance changed, such as one that started running. */
async function served(
    row: Readonly<Record<string, unknown>>,
    database: DatabaseConnection,
): Promise<readonly Readonly<Record<string, unknown>>[]> {
    return database
        .select({ id: run.table.id })
        .from(run.table)
        .innerJoin(deployment.table, eq(deployment.table.installationId, run.table.installation))
        .where(
            and(
                eq(deployment.table.id, row.deploymentId as Identifier<"deployment">),
                inArray(run.table.state, [...ACTIVE_RUN_STATES]),
            ),
        );
}

/** Attempt a due run once, in the trace it records, returning the wait until it is due. */
async function attempt(
    current: Run,
    reconciliation: ObjectReconciliation,
    options: Resolved,
): Promise<number | undefined> {
    // leave a run whose installation this cell does not serve now or that runs no instance, and wait for one not due
    const target = await Installation.serving(reconciliation.database, current.installation);
    const endpoints =
        target === undefined ? [] : await Instance.endpoints(reconciliation.database, target.id);
    if (target === undefined || endpoints.length === 0) {
        return undefined;
    } else if (current.at > reconciliation.now) {
        return current.at - reconciliation.now;
    }

    // leave a queued run behind an earlier one of its watch
    if (current.concurrency === "queue" && (await isBehind(reconciliation.database, current))) {
        return undefined;
    }

    // fail a run whose attempts ran out, even ones that never returned
    if (current.attempts >= options.maximumAttempts) {
        await update(reconciliation, current, {
            state: "failed",
            finishedAt: reconciliation.now,
            error: current.error ?? {
                code: "ATTEMPTS_EXHAUSTED",
                message: `${current.attempts} attempts started without finishing`,
            },
        });

        return undefined;
    }

    // continue the run's trace, or start the one its first attempt records
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
        "destack.run.cause": current.cause,
        "destack.run.method": current.call.method,
    };

    return tracer.startActiveSpan(
        `run ${current.call.method}`,
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

/** Push a run's call as its installation, and record the outcome. */
async function push(
    current: Run,
    target: Installation,
    span: Span,
    reconciliation: ObjectReconciliation,
    options: Resolved,
): Promise<void> {
    // start the attempt, fenced by the revision read: a host that took the run over since wins
    const started = await update(reconciliation, current, {
        state: "running",
        attempts: current.attempts + 1,
        startedAt: current.startedAt ?? reconciliation.now,
        traceId: current.traceId ?? traceIdOf(span),
    });
    if (started === undefined) {
        return;
    }

    // push the call once under the run's own request, aborted once the run is cancelled or replaced
    const ended = new AbortController();
    const settled = new AbortController();
    const ending = endedElsewhere(reconciliation, started, ended, settled.signal);
    const signal = AbortSignal.any([ended.signal, reconciliation.signal]);
    const mutation = { id: identifierUuid(started.id), calls: [started.call] };
    let outcome: PushOutcome | undefined;
    let thrown: unknown;
    try {
        outcome = await options.push(target, mutation, signal, started.onBehalfOf ?? undefined);
    } catch (error) {
        thrown = error;
    } finally {
        settled.abort();
        await ending;
    }

    // record the call's result, or its final failure
    const now = options.now();
    if (outcome !== undefined && "value" in outcome) {
        await update(reconciliation, started, { state: "succeeded", finishedAt: now, error: null });

        return;
    } else if (outcome !== undefined) {
        const error = { code: outcome.error.code, message: outcome.error.message };
        span.setStatus({ code: SpanStatusCode.ERROR, message: error.message });
        await update(reconciliation, started, { state: "failed", error, finishedAt: now });

        return;
    }

    // leave a run cancelled, replaced or stopped, and fail one out of attempts
    const error = runError(thrown);
    span.setStatus({ code: SpanStatusCode.ERROR, message: error.message });
    if (signal.aborted) {
        return;
    } else if (started.attempts >= options.maximumAttempts) {
        await update(reconciliation, started, { state: "failed", error, finishedAt: now });

        return;
    }

    // wait for the next attempt through the control loop's backoff
    await update(reconciliation, started, { state: "pending", error });
    throw thrown;
}

/** Report whether an earlier change of a queued run's watch still waits or runs. */
async function isBehind(database: DatabaseConnection, current: Run): Promise<boolean> {
    // find an active change of the same watch and key recorded earlier
    const key = sql`coalesce(${run.table.key}, '')`;
    const currentKey = current.key ?? "";
    const [earlier] = await database
        .select({ id: run.table.id })
        .from(run.table)
        .where(
            and(
                eq(run.table.installation, current.installation),
                eq(run.table.cause, "watch"),
                eq(run.table.packageId, current.packageId!),
                eq(run.table.trigger, current.trigger!),
                inArray(run.table.state, [...ACTIVE_RUN_STATES]),
                or(
                    // an earlier change of the same log
                    and(
                        eq(run.table.epoch, current.epoch!),
                        or(
                            lt(run.table.sequence, current.sequence!),
                            and(eq(run.table.sequence, current.sequence!), lt(key, currentKey)),
                        ),
                    ),
                    // a change of an earlier log
                    and(
                        ne(run.table.epoch, current.epoch!),
                        lt(run.table.createdAt, current.createdAt),
                    ),
                ),
            ),
        )
        .limit(1);

    return earlier !== undefined;
}

/** Abort an attempt once its run is no longer active. */
async function endedElsewhere(
    reconciliation: ObjectReconciliation,
    started: Run,
    ended: AbortController,
    settled: AbortSignal,
): Promise<void> {
    // look again at each change of the run until the attempt settles
    const settling = new Promise<void>((resolve) =>
        settled.addEventListener("abort", () => resolve(), { once: true }),
    );
    while (!settled.aborted) {
        await Promise.race([reconciliation.changed(), settling]);
        if (settled.aborted) {
            return;
        }

        // abort once another host changed or removed the run
        const [current] = await reconciliation.database
            .select({ revision: run.table.revision })
            .from(run.table)
            .where(eq(run.table.id, started.id));
        if (current === undefined || current.revision !== started.revision) {
            ended.abort(new ServiceError("CONFLICT", { message: "the run ended elsewhere" }));

            return;
        }
    }
}

/** Change a run still at the revision read, returning it unless another host changed or removed it since. */
async function update(
    reconciliation: ObjectReconciliation,
    current: Run,
    values: Partial<Run>,
): Promise<Run | undefined> {
    try {
        const [updated] = await reconciliation.server.executeAsSystem(
            run,
            "update",
            [{ ...SystemCall.of(current), input: values }],
            reconciliation.now,
        );

        return updated as Run;
    } catch (error) {
        // leave a run another host changed or removed since
        const isGone =
            error instanceof ServiceError &&
            (error.code === "CONFLICT" || error.code === "NOT_FOUND");
        if (isGone) {
            return undefined;
        }
        throw error;
    }
}

/** Report whether a run claims its cause, waiting for or making an attempt. */
function isActive(entry: Pick<Run, "state">): boolean {
    return ACTIVE_RUN_STATES.includes(entry.state as never);
}

/** Parse a body as JSON, reading a body of another kind as undefined. */
function parseJson(text: string): unknown {
    try {
        return JSON.parse(text);
    } catch {
        return undefined;
    }
}

/** Read the trace a span records, absent when no tracer records one. */
function traceIdOf(span: Span): string | null {
    const context = span.spanContext();

    return isSpanContextValid(context) ? context.traceId : null;
}
