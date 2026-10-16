import { type DatabaseConnection, Lease, type Row } from "@destack/db";
import { Limits, type MethodContext, type ObjectReconciliation } from "@destack/object";
import { LEASE_MILLISECONDS } from "@destack/runtime/control";
import { CallError, isCallError } from "@destack/runtime/error";
import { RetryPolicy } from "@destack/runtime/timer";
import { Duration, schema } from "@destack/schema";
import { Failure, ObjectReference } from "@destack/sync";
import { type RunContext, type Step, StepContext, Suspension } from "../step/step.ts";
import { recorded, type Workflow } from "../workflow/workflow.ts";

/** The states of a run: due, waiting to wake, paused by its actor, or ended. */
export const RUN_STATES = [
    "pending",
    "waiting",
    "paused",
    "completed",
    "failed",
    "cancelled",
] as const;

/** The state of a run. */
export type RunState = (typeof RUN_STATES)[number];

/** The states of a live run, one per key at a time. */
export const LIVE_STATES = ["pending", "waiting", "paused"] as const satisfies readonly RunState[];

/** The states a run ends in. */
export const ENDED_STATES = [
    "completed",
    "failed",
    "cancelled",
] as const satisfies readonly RunState[];

/** A run's row as advancing it reads it. */
export const RunRecord = schema.looseObject({
    /** The run. */
    id: schema.string(),
    /** The scope the run acts in. */
    scope: schema.string(),
    /** When the run was created, which stamps the requests its steps derive. */
    createdAt: schema.number(),
    /** Whether the run is due, waits, is paused, or ended. */
    state: schema.enum(RUN_STATES),
    /** The run's input. */
    input: schema.json(),
    /** The advances that failed and were retried so far. */
    attempts: schema.number().int(),
    /** When the run is next due. */
    wakeAt: schema.number().nullable(),
    /** When the run times out. */
    deadline: schema.number().nullable(),
    /** When the run first advanced. */
    startedAt: schema.number().nullable(),
    /** The object type a waiting run waits for. */
    waitType: schema.string().nullable(),
    /** The failure the run last answered. */
    failure: Failure.nullable(),
});
/** A run's row as advancing it reads it. */
export type RunRecord = schema.Infer<typeof RunRecord>;

/** The input a run's creation takes beside its fields. */
const RunCreation = schema.looseObject({
    /** The key one live run takes at a time. */
    key: schema.string().exactOptional(),
    /** What a start of a live run's key does. */
    mode: schema.enum(["dedupe", "replace"]).exactOptional(),
    /** The run's input. */
    input: schema.json(),
    /** When the run starts. */
    wakeAt: schema.number().exactOptional(),
});

/** What one advance settled: when the run wakes, absent once ended or waiting on writes. */
type Settled = { readonly wake: number | undefined; readonly isEnded: boolean };

/** The runs of workflows: started, reset and advanced step by step. */
export const Run = {
    /** Create a run, answering its key's live run or replacing it before it began. */
    async create<Input, Output>(
        workflow: Workflow<Input, Output>,
        context: MethodContext,
        next: (context?: MethodContext) => Promise<unknown>,
    ): Promise<unknown> {
        // find the live run of the key
        const creation = RunCreation.parse(context.input);
        const [live] =
            creation.key === undefined
                ? []
                : await context.read(workflow.run).findMany({
                      where: {
                          scope: context.scope,
                          key: creation.key,
                          state: { in: [...LIVE_STATES] },
                      },
                  });

        // replace the input and start of a live run that has not begun, else answer the live run
        if (live !== undefined && creation.mode === "replace" && live["startedAt"] === null) {
            const [replaced] = await context.mutate(context.self, [
                workflow.run.call("replace", {
                    ...workflow.run.target(RunRecord.parse(live)),
                    input: creation.input,
                    wakeAt: creation.wakeAt ?? null,
                }),
            ]);

            return replaced;
        } else if (live !== undefined) {
            return live;
        }

        // create the run, timing out a timeout after now
        const timeout = workflow.timeout;
        const deadline =
            timeout === undefined ? {} : { deadline: context.now + Duration.milliseconds(timeout) };

        return next(context.with({ input: { ...context.input, ...deadline } }));
    },

    /** Run a run again from a step, forgetting that step and every later one. */
    async reset<Input, Output>(
        workflow: Workflow<Input, Output>,
        context: MethodContext,
    ): Promise<{ readonly forgotten: number }> {
        // find the step the run starts again at
        const { step: name } = schema.object({ step: schema.string() }).parse(context.input);
        const run = RunRecord.parse(context.target);
        const [from] = await context
            .read(workflow.step)
            .findMany({ where: { parentId: run.id, name } });
        if (from === undefined) {
            throw new CallError("NOT_FOUND", { message: `run ${run.id} took no step ${name}` });
        }

        // forget the step and every later one, and make the run due again from its first attempt
        const position = schema.number().parse(from["position"]);
        const later = await context
            .read(workflow.step)
            .findMany({ where: { parentId: run.id, position: { gte: position } } });
        const target = workflow.run.target(run);
        await context.mutate(context.self, [
            ...later.map((row) =>
                workflow.step.call("delete", workflow.step.target(StepTarget.parse(row))),
            ),
            workflow.run.call("restart", {
                ...target,
                attempts: 0,
                wakeAt: null,
                failure: null,
                ...(workflow.output === undefined ? {} : { output: null }),
            }),
        ]);

        return { forgotten: later.length };
    },

    /** Advance a run to its next wait, output, retry or failure, answering the wait to its wake. */
    async advance<Input, Output>(
        workflow: Workflow<Input, Output>,
        code: (step: Step, input: Input, context: RunContext) => Promise<Output>,
        row: Row,
        reconciliation: ObjectReconciliation,
    ): Promise<number | undefined> {
        // fail a run past its deadline
        const { now, database } = reconciliation;
        const run = RunRecord.parse(row);
        const target = workflow.run.target(run);
        if (run.deadline !== null && run.deadline <= now) {
            const failure = Failure.of(
                new CallError("TIMEOUT", { message: `run ${run.id} timed out` }),
            );
            await reconciliation.mutate([workflow.run.call("fail", { ...target, failure })]);
            await releaseSlot(workflow, run, database);

            return undefined;
        }

        // wait until a sleeping, retried, limited or later run is due, a run waiting for writes waking on them
        if (run.wakeAt !== null && run.wakeAt > now && run.waitType === null) {
            return waitUntil(run, run.wakeAt, now);
        }

        // take a start of the workflow's rate once per run, delaying a limited one
        if (run.startedAt === null) {
            const limited = await takeStart(workflow, run, row, database, now);
            if (limited !== undefined) {
                await reconciliation.mutate([
                    workflow.run.call("delay", {
                        ...target,
                        attempts: run.attempts,
                        wakeAt: now + limited,
                        failure: run.failure,
                    }),
                ]);

                return waitUntil(run, now + limited, now);
            }
            await reconciliation.mutate([
                workflow.run.call("begin", { ...target, startedAt: now }),
            ]);
        }

        // take a concurrency slot while advancing, delaying the run until one frees
        const slot = await takeSlot(workflow, run, row, database, now);
        if (slot !== undefined && slot.kind === "busy") {
            await reconciliation.mutate([
                workflow.run.call("delay", {
                    ...target,
                    attempts: run.attempts,
                    wakeAt: slot.lapsesAt,
                    failure: run.failure,
                }),
            ]);

            return waitUntil(run, slot.lapsesAt, now);
        }

        // advance the run, renewing its slot meanwhile and releasing it after unless the run holds it to its end
        const renewing = new AbortController();
        if (slot !== undefined) {
            void renewSlot(workflow, run, row, reconciliation, renewing.signal);
        }
        let settled: Settled;
        try {
            settled = await runSteps(workflow, code, row, run, reconciliation);
        } finally {
            renewing.abort();
        }
        if (slot !== undefined && (!workflow.isSingleton || settled.isEnded)) {
            await releaseSlot(workflow, run, database);
        }

        return settled.wake === undefined ? undefined : waitUntil(run, settled.wake, now);
    },
};

/** Run the workflow's function once over the run's recorded steps, settling what it did. */
async function runSteps<Input, Output>(
    workflow: Workflow<Input, Output>,
    code: (step: Step, input: Input, context: RunContext) => Promise<Output>,
    row: Row,
    run: RunRecord,
    reconciliation: ObjectReconciliation,
): Promise<Settled> {
    // replay the run's recorded steps and take the rest
    const { store, now } = reconciliation;
    const target = workflow.run.target(run);
    const steps = new StepContext(
        workflow,
        run,
        ObjectReference.at("actor", row),
        store,
        await StepContext.load(store, workflow, run),
    );
    try {
        const input = workflow.input.parse(run.input);
        const output = await code(steps, input, { id: run.id, scope: run.scope });

        // complete the run, with its output where the workflow declares one
        await reconciliation.mutate([
            workflow.run.call(
                "complete",
                workflow.output === undefined
                    ? target
                    : { ...target, output: recorded(workflow.output.parse(output)) },
            ),
        ]);

        return { wake: undefined, isEnded: true };
    } catch (error) {
        // wait at a waiting step until its time, or for its type's writes, unless the run waits so already
        if (error instanceof Suspension) {
            const wakeAt = error.at ?? null;
            const waitType = error.waitType ?? null;
            if (run.state === "waiting" && run.wakeAt === wakeAt && run.waitType === waitType) {
                return { wake: error.at, isEnded: false };
            }
            await reconciliation.mutate([
                workflow.run.call("wait", { ...target, waitType, wakeAt }),
            ]);

            return { wake: error.at, isEnded: false };
        }
        // fail on a final refusal or an input or output its schema refuses
        else if (
            (isCallError(error) && CallError.isRefusal(error.status)) ||
            error instanceof schema.Error
        ) {
            await reconciliation.mutate([
                workflow.run.call("fail", { ...target, failure: Failure.of(error) }),
            ]);

            return { wake: undefined, isEnded: true };
        }
        // retry another failure after its backoff while attempts remain, else fail with it
        else {
            const attempts = run.attempts + 1;
            const failure = Failure.of(error);
            if (RetryPolicy.isRetried(workflow.retries, attempts)) {
                const wakeAt = now + RetryPolicy.interval(workflow.retries, attempts);
                await reconciliation.mutate([
                    workflow.run.call("delay", { ...target, attempts, wakeAt, failure }),
                ]);

                return { wake: wakeAt, isEnded: false };
            }
            await reconciliation.mutate([workflow.run.call("fail", { ...target, failure })]);

            return { wake: undefined, isEnded: true };
        }
    }
}

/** Take a start token of the workflow's rate, answering the wait of a limited start. */
async function takeStart<Input, Output>(
    workflow: Workflow<Input, Output>,
    run: RunRecord,
    row: Row,
    database: DatabaseConnection,
    now: number,
): Promise<number | undefined> {
    // take a token when the workflow limits its starts
    const { rate } = workflow.limits;

    return rate === undefined
        ? undefined
        : await Limits.take(database, limitName(workflow), limitKey(workflow, run, row), rate, now);
}

/** Take or renew a concurrency slot for the run, absent when the workflow limits none. */
async function takeSlot<Input, Output>(
    workflow: Workflow<Input, Output>,
    run: RunRecord,
    row: Row,
    database: DatabaseConnection,
    now: number,
) {
    // take a slot when the workflow limits how many runs advance at once
    const { concurrency } = workflow.limits;

    return concurrency === undefined
        ? undefined
        : await Lease.acquire(database, {
              name: limitName(workflow),
              key: limitKey(workflow, run, row),
              holder: run.id,
              duration: LEASE_MILLISECONDS,
              now,
              slots: concurrency.slots,
          });
}

/** Renew the run's slot a third into its lease until the advance ends. */
async function renewSlot<Input, Output>(
    workflow: Workflow<Input, Output>,
    run: RunRecord,
    row: Row,
    reconciliation: ObjectReconciliation,
    signal: AbortSignal,
): Promise<void> {
    const clock = reconciliation.store.clock;
    while (
        await clock.sleep(LEASE_MILLISECONDS / 3, signal).then(
            () => true,
            () => false,
        )
    ) {
        await takeSlot(workflow, run, row, reconciliation.database, clock.now());
    }
}

/** Release the run's concurrency slot. */
async function releaseSlot<Input, Output>(
    workflow: Workflow<Input, Output>,
    run: RunRecord,
    database: DatabaseConnection,
): Promise<void> {
    if (workflow.limits.concurrency !== undefined) {
        await Lease.release(database, { holder: run.id, name: limitName(workflow) });
    }
}

/** Answer the wait until a time, no later than the run's deadline. */
function waitUntil(run: RunRecord, at: number, now: number): number {
    const due = run.deadline === null ? at : Math.min(at, run.deadline);

    return Math.max(0, due - now);
}

/** Name the limits of a workflow's runs. */
function limitName(workflow: Pick<Workflow, "package" | "name">): string {
    return `workflow:${workflow.package.name}/${workflow.name}`;
}

/** Read whose limit a run counts against: its scope's, its actor's or an input value's. */
function limitKey(workflow: Pick<Workflow, "limits">, run: RunRecord, row: Row): string {
    return Limits.key(workflow.limits.key, {
        scope: run.scope,
        caller: ObjectReference.at("actor", row),
        input: run.input,
    });
}

/** A step's row as its target reads it. */
const StepTarget = schema.looseObject({
    /** The step. */
    id: schema.string(),
    /** The scope it lives in. */
    scope: schema.string(),
});
