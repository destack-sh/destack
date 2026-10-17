import { AccessContext, Grant } from "@destack/access";
import { type DatabaseConnection, Lease, type Row, Snapshot } from "@destack/db";
import { Limits, type MethodContext, type ObjectReconciliation } from "@destack/object";
import { LEASE_MILLISECONDS } from "@destack/runtime/control";
import { CallError } from "@destack/runtime/error";
import { RetryPolicy } from "@destack/runtime/timer";
import { Duration, type JsonValue, present, schema } from "@destack/schema";
import { Failure, ObjectReference, Subject } from "@destack/sync";
import type { ObjectStore } from "@destack/object/server";
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

/** The schema of a run as advancing it reads it, whatever workflow's table stores it. */
const runSchema = schema.looseObject({
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

/** A run of a workflow: its state, input, attempts and wake times, advanced step by step. */
export class Run<Input = unknown, Output = unknown> {
    /** The workflow the run belongs to. */
    readonly workflow: Workflow<Input, Output>;
    /** The run's row, whose actor and input its limits read. */
    readonly row: Row;
    /** The run. */
    readonly id: string;
    /** The scope the run acts in. */
    readonly scope: string;
    /** When the run was created, which stamps the requests its steps derive. */
    readonly createdAt: number;
    /** Whether the run is due, waits, is paused, or ended. */
    readonly state: RunState;
    /** The run's input. */
    readonly input: JsonValue;
    /** The advances that failed and were retried so far. */
    readonly attempts: number;
    /** When the run is next due. */
    readonly wakeAt: number | null;
    /** When the run times out. */
    readonly deadline: number | null;
    /** When the run first advanced. */
    readonly startedAt: number | null;
    /** The object type a waiting run waits for. */
    readonly waitType: string | null;
    /** The failure the run last answered. */
    readonly failure: Failure | null;
    /** The principal the run acts for. */
    readonly actor: Subject;

    /** Read a run of a workflow from its row. */
    constructor(workflow: Workflow<Input, Output>, row: Row) {
        // parse the row's fields
        const fields = runSchema.parse(row);
        this.workflow = workflow;
        this.row = row;
        this.id = fields.id;
        this.scope = fields.scope;
        this.createdAt = fields.createdAt;
        this.state = fields.state;
        this.input = fields.input;
        this.attempts = fields.attempts;
        this.wakeAt = fields.wakeAt;
        this.deadline = fields.deadline;
        this.startedAt = fields.startedAt;
        this.waitType = fields.waitType;
        this.failure = fields.failure;
        this.actor = ObjectReference.at("actor", row);
    }

    /** The run's row as the workflow's run calls target it. */
    get target() {
        return this.workflow.run.target({ id: this.id, scope: this.scope });
    }

    /** Wait until the run ends, answering it as it ended. */
    async ended(database: DatabaseConnection, signal: AbortSignal): Promise<Run<Input, Output>> {
        // read the run again on each commit until it is in an ended state
        let ended: Run<Input, Output> | undefined;
        await database.log.until(async () => {
            // read the run, answering it once ended
            const row = await Snapshot.live(database)
                .read(this.workflow.run.table)
                .findFirst({ where: { id: this.id } });
            const read = new Run(this.workflow, present(row, `run ${this.id}`));
            ended = ENDED_STATES.some((state) => state === read.state) ? read : undefined;

            return ended !== undefined;
        }, signal);

        // refuse a wait the signal aborted first
        signal.throwIfAborted();

        return present(ended, `the end of run ${this.id}`);
    }

    /** Create a run, answering its key's live run or replacing it before it began. */
    static async create<Input, Output>(
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
                    ...workflow.run.target(new Run(workflow, live)),
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
    }

    /** Run the run again from a step, forgetting that step and every later one. */
    async reset(context: MethodContext): Promise<{ readonly forgotten: number }> {
        // find the step the run starts again at
        const { step: name } = schema.object({ step: schema.string() }).parse(context.input);
        const [from] = await context
            .read(this.workflow.step)
            .findMany({ where: { parentId: this.id, name } });
        if (from === undefined) {
            throw new CallError("NOT_FOUND", { message: `run ${this.id} took no step ${name}` });
        }

        // forget the step and every later one, and make the run due again from its first attempt
        const position = schema.number().parse(from["position"]);
        const later = await context
            .read(this.workflow.step)
            .findMany({ where: { parentId: this.id, position: { gte: position } } });
        const target = this.target;
        await context.mutate(context.self, [
            ...later.map((row) =>
                this.workflow.step.call("delete", this.workflow.step.target(StepTarget.parse(row))),
            ),
            this.workflow.run.call("restart", {
                ...target,
                attempts: 0,
                wakeAt: null,
                failure: null,
                ...(this.workflow.output === undefined ? {} : { output: null }),
            }),
        ]);

        return { forgotten: later.length };
    }

    /** Read the caller the run calls as: its serving code, or for others within their grant. */
    async caller(store: ObjectStore, now: number): Promise<AccessContext> {
        // call as the code serving the run when it is the actor
        const self = store.serving(this.workflow.run, now);
        if (Subject.same(self.subject, this.actor)) {
            return self;
        }

        // call for its space or a user as the code serving the run, within their live grant to it
        const parties = { delegator: this.actor, delegate: self.subject };
        const caller = await Grant.act(Snapshot.live(store.database), parties, now);

        return AccessContext.of(caller, now);
    }

    /** Advance the run to its next wait, output, retry or failure, answering when it wakes. */
    async advance(
        code: (step: Step, input: Input, context: RunContext) => Promise<Output>,
        reconciliation: ObjectReconciliation,
    ): Promise<number | undefined> {
        // fail a run past its deadline
        const { now, database } = reconciliation;
        const { target } = this;
        if (this.deadline !== null && this.deadline <= now) {
            const failure = Failure.of(
                new CallError("TIMEOUT", { message: `run ${this.id} timed out` }),
            );
            await reconciliation.mutate([this.workflow.run.call("fail", { ...target, failure })]);
            await this.#releaseSlot(database);

            return undefined;
        }

        // wait until a sleeping, retried, limited or later run is due, a run waiting for writes waking on them
        if (this.wakeAt !== null && this.wakeAt > now && this.waitType === null) {
            return this.#waitUntil(this.wakeAt, now);
        }

        // take a start of the workflow's rate once per run, delaying a limited one
        if (this.startedAt === null) {
            const limited = await this.#takeStart(database, now);
            if (limited !== undefined) {
                await reconciliation.mutate([
                    this.workflow.run.call("delay", {
                        ...target,
                        attempts: this.attempts,
                        wakeAt: now + limited,
                        failure: this.failure,
                    }),
                ]);

                return this.#waitUntil(now + limited, now);
            }
            await reconciliation.mutate([
                this.workflow.run.call("begin", { ...target, startedAt: now }),
            ]);
        }

        // take a concurrency slot while advancing, delaying the run until one frees
        const slot = await this.#takeSlot(database, now);
        if (slot !== undefined && slot.kind === "busy") {
            await reconciliation.mutate([
                this.workflow.run.call("delay", {
                    ...target,
                    attempts: this.attempts,
                    wakeAt: slot.lapsesAt,
                    failure: this.failure,
                }),
            ]);

            return this.#waitUntil(slot.lapsesAt, now);
        }

        // advance the run, renewing its slot meanwhile and releasing it after unless the run holds it to its end
        const renewing = new AbortController();
        if (slot !== undefined) {
            void this.#renewSlot(reconciliation, renewing.signal);
        }
        let settled: Settled;
        try {
            settled = await this.#runSteps(code, reconciliation);
        } finally {
            renewing.abort();
        }
        if (slot !== undefined && (!this.workflow.isSingleton || settled.isEnded)) {
            await this.#releaseSlot(database);
        }

        return settled.wake === undefined ? undefined : this.#waitUntil(settled.wake, now);
    }

    /** Run the workflow's function once over the run's recorded steps, settling what it did. */
    async #runSteps(
        code: (step: Step, input: Input, context: RunContext) => Promise<Output>,
        reconciliation: ObjectReconciliation,
    ): Promise<Settled> {
        // replay the run's recorded steps and take the rest, acting as its actor while its grant lasts
        const { store, now } = reconciliation;
        const { target } = this;
        try {
            const steps = new StepContext(
                this.workflow,
                this,
                this.actor,
                await this.caller(store, now),
                store,
                await StepContext.load(store, this.workflow, this),
            );
            const input = this.workflow.input.parse(this.input);
            const output = await code(steps, input, { id: this.id, scope: this.scope });

            // complete the run, with its output where the workflow declares one
            await reconciliation.mutate([
                this.workflow.run.call(
                    "complete",
                    this.workflow.output === undefined
                        ? target
                        : { ...target, output: recorded(this.workflow.output.parse(output)) },
                ),
            ]);

            return { wake: undefined, isEnded: true };
        } catch (error) {
            // read a refusal, a domain one too
            const refusal = CallError.of(error);

            // wait at a waiting step until its time, or for its type's writes, unless the run waits so already
            if (error instanceof Suspension) {
                const wakeAt = error.at ?? null;
                const waitType = error.waitType ?? null;
                if (
                    this.state === "waiting" &&
                    this.wakeAt === wakeAt &&
                    this.waitType === waitType
                ) {
                    return { wake: error.at, isEnded: false };
                }
                await reconciliation.mutate([
                    this.workflow.run.call("wait", { ...target, waitType, wakeAt }),
                ]);

                return { wake: error.at, isEnded: false };
            }
            // fail on a final refusal, or an input or output its schema refuses
            else if (
                (refusal !== undefined && CallError.isRefusal(refusal.status)) ||
                error instanceof schema.Error
            ) {
                await reconciliation.mutate([
                    this.workflow.run.call("fail", {
                        ...target,
                        failure: Failure.of(refusal ?? error),
                    }),
                ]);

                return { wake: undefined, isEnded: true };
            }
            // retry another failure after its backoff while attempts remain, else fail with it
            else {
                const attempts = this.attempts + 1;
                const failure = Failure.of(error);
                if (RetryPolicy.isRetried(this.workflow.retries, attempts)) {
                    const wakeAt = now + RetryPolicy.interval(this.workflow.retries, attempts);
                    await reconciliation.mutate([
                        this.workflow.run.call("delay", { ...target, attempts, wakeAt, failure }),
                    ]);

                    return { wake: wakeAt, isEnded: false };
                }
                await reconciliation.mutate([
                    this.workflow.run.call("fail", { ...target, failure }),
                ]);

                return { wake: undefined, isEnded: true };
            }
        }
    }

    /** Take a start token of the workflow's rate, answering the wait of a limited start. */
    async #takeStart(database: DatabaseConnection, now: number): Promise<number | undefined> {
        // take a token when the workflow limits its starts
        const { rate } = this.workflow.limits;

        return rate === undefined
            ? undefined
            : await Limits.take(database, this.#limitName, this.#limitKey, rate, now);
    }

    /** Take or renew a concurrency slot for the run, absent when the workflow limits none. */
    async #takeSlot(database: DatabaseConnection, now: number) {
        // take a slot when the workflow limits how many runs advance at once
        const { concurrency } = this.workflow.limits;

        return concurrency === undefined
            ? undefined
            : await Lease.acquire(database, {
                  name: this.#limitName,
                  key: this.#limitKey,
                  holder: this.id,
                  duration: LEASE_MILLISECONDS,
                  now,
                  slots: concurrency.slots,
              });
    }

    /** Renew the run's slot a third into its lease until the advance ends. */
    async #renewSlot(reconciliation: ObjectReconciliation, signal: AbortSignal): Promise<void> {
        const clock = reconciliation.store.clock;
        while (
            await clock.sleep(LEASE_MILLISECONDS / 3, signal).then(
                () => true,
                () => false,
            )
        ) {
            await this.#takeSlot(reconciliation.database, clock.now());
        }
    }

    /** Release the run's concurrency slot. */
    async #releaseSlot(database: DatabaseConnection): Promise<void> {
        if (this.workflow.limits.concurrency !== undefined) {
            await Lease.release(database, { holder: this.id, name: this.#limitName });
        }
    }

    /** Answer the wait until a time, no later than the run's deadline. */
    #waitUntil(at: number, now: number): number {
        const due = this.deadline === null ? at : Math.min(at, this.deadline);

        return Math.max(0, due - now);
    }

    /** The name of the limits of the workflow's runs. */
    get #limitName(): string {
        return `workflow:${this.workflow.package.name}/${this.workflow.name}`;
    }

    /** Whose limit the run counts against: its scope's, its actor's or an input value's. */
    get #limitKey(): string {
        return Limits.key(this.workflow.limits.key, {
            scope: this.scope,
            caller: this.actor,
            input: this.input,
        });
    }
}

/** A step's row as its target reads it. */
const StepTarget = schema.looseObject({
    /** The step. */
    id: schema.string(),
    /** The scope it lives in. */
    scope: schema.string(),
});
