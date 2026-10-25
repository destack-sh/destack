import { AccessContext, Grant } from "@destack/access";
import {
    type DatabaseConnection,
    Lease,
    type Select,
    Snapshot,
    LEASE_MILLISECONDS,
} from "@destack/db";
import {
    Limits,
    type MethodContext,
    type MethodContextOf,
    type NextOf,
    type ObjectReconciliation,
    type ObjectType,
} from "@destack/object";
import { CallError } from "@destack/object";
import { RetryPolicy } from "@destack/environment";
import { Duration, present, schema } from "@destack/schema";
import { type Call, ObjectReference, ObjectTypeReference, Subject } from "@destack/sync";
import { Failure } from "@destack/error";
import type { ObjectStore } from "@destack/object/server";
import { type RunContext, Step, Suspension } from "../step/step.ts";
import { recorded, type RunType, type WaitOf, type Workflow } from "../workflow/workflow.ts";
import type {} from "@destack/package/import-meta";
import { WorkflowError } from "../error/index.ts";

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

/** A run's row as its workflow's run type stores it. */
export type RunRow = Select<RunType["table"]>;

/** The outcome of one advance: when the run wakes, absent once ended or waiting on writes. */
type Outcome = { readonly wake: number | undefined; readonly isEnded: boolean };

/** A run of a workflow: its state, input, attempts and wake times, advanced step by step. */
export class Run<Input = unknown, Output = unknown, Waits extends ObjectType = ObjectType> {
    /** The workflow the run belongs to. */
    readonly workflow: Workflow<Input, Output, Waits>;
    /** The run's row. */
    readonly row: RunRow;

    /** Read a run of a workflow from its row. */
    constructor(workflow: Workflow<Input, Output, Waits>, row: RunRow) {
        this.workflow = workflow;
        this.row = row;
    }

    /** The principal the run acts for. */
    get actor(): Subject {
        return ObjectReference.at("actor", this.row);
    }

    /** The run's row as the workflow's run calls target it. */
    get target() {
        return this.workflow.run.target({ id: this.row.id, scope: this.row.scope });
    }

    /** Wait until the run ends, answering it as it ended. */
    async ended(
        database: DatabaseConnection,
        signal: AbortSignal,
    ): Promise<Run<Input, Output, Waits>> {
        // read the run again on each commit until it is in an ended state
        let ended: Run<Input, Output, Waits> | undefined;
        await database.log.until(async () => {
            // read the run, answering it once ended
            const row = await Snapshot.live(database)
                .read(this.workflow.run.table)
                .findFirst({ where: { id: this.row.id } });
            const read = new Run(this.workflow, present(row, `run ${this.row.id}`));
            ended = ENDED_STATES.some((state) => state === read.row.state) ? read : undefined;

            return ended !== undefined;
        }, signal);

        // refuse a wait the signal aborted first
        signal.throwIfAborted();

        return present(ended, `the end of run ${this.row.id}`);
    }

    /** Create a run, answering its key's live run or replacing it before it began. */
    static async create<Input, Output, Waits extends ObjectType>(
        workflow: Workflow<Input, Output, Waits>,
        context: MethodContextOf<RunType, "create">,
        next: NextOf<RunType, "create">,
    ): Promise<RunRow> {
        // find the live run of the key
        const creation = context.input;
        const { key } = creation;
        const [live] =
            key === undefined || key === null
                ? []
                : await context.read(workflow.run).findMany({
                      where: {
                          scope: context.scope,
                          key,
                          state: { in: [...LIVE_STATES] },
                      },
                  });

        // replace the input and start of a live run that has not begun
        const isBegun = live !== undefined && live.startedAt !== null;
        if (live !== undefined && creation.mode === "replace" && !isBegun) {
            const [replaced] = await context.mutate(context.self, [
                workflow.run.call("replace", {
                    ...workflow.run.target(live),
                    input: creation.input,
                    wakeAt: creation.wakeAt ?? null,
                }),
            ]);

            return replaced;
        }
        // refuse replacing a live run that has begun
        else if (live !== undefined && creation.mode === "replace") {
            throw WorkflowError.of(
                "RUN_STARTED",
                `run ${live.id} of key ${creation.key} has begun`,
            );
        }
        // answer the live run of the key
        else if (live !== undefined) {
            return live;
        }

        // create the run, timing out a timeout after now
        const timeout = workflow.timeout;
        const deadline =
            timeout === undefined ? {} : { deadline: context.now + Duration.milliseconds(timeout) };

        return next(context.with({ input: { ...context.input, ...deadline } }));
    }

    /** Run the run again from a step, forgetting that step and every later one. */
    async reset(
        context: MethodContextOf<RunType, "reset">,
    ): Promise<{ readonly forgotten: number }> {
        // find the step the run starts again at
        const { step: name } = context.input;
        const [from] = await context
            .read(this.workflow.step)
            .findMany({ where: { parentId: this.row.id, name } });
        if (from === undefined) {
            throw WorkflowError.of("STEP_NOT_FOUND", `run ${this.row.id} has no step ${name}`);
        }

        // forget the step and every later one, and make the run due again from its first attempt
        const later = await context
            .read(this.workflow.step)
            .findMany({ where: { parentId: this.row.id, position: { gte: from.position } } });
        const target = this.target;
        await context.mutate(context.self, [
            ...later.map((row) =>
                this.workflow.step.call("delete", this.workflow.step.target(row)),
            ),
            this.workflow.run.call("restart", {
                ...target,
                attempts: 0,
                wakeAt: null,
                failure: null,
                output: null,
            }),
        ]);

        return { forgotten: later.length };
    }

    /** End a run through a transition, cancelling in the same call the children ending with it. */
    static async end<Input, Output, Waits extends ObjectType, Ended>(
        workflow: Workflow<Input, Output, Waits>,
        context: Pick<MethodContext, "read" | "objects" | "mutate" | "self">,
        target: RunRow,
        next: () => Promise<Ended>,
    ): Promise<Ended> {
        // end the run, then cancel its live children that end with it
        const ended = await next();
        const closing = await new Run(workflow, target).closeChildren(context);
        if (closing.length > 0) {
            await context.mutate(context.self, closing);
        }

        return ended;
    }

    /** Build the calls cancelling the live child runs the run's invoke steps started. */
    async closeChildren(context: Pick<MethodContext, "read" | "objects">): Promise<Call[]> {
        // read the children of the run's invoke steps that end with it
        const steps = await context.read(this.workflow.step.table).findMany({
            where: { parentId: this.row.id },
        });
        const children = steps.flatMap((step) =>
            step.child?.onParentClose === "cancel" ? [step.child] : [],
        );

        // cancel each child while it is live
        const calls: Call[] = [];
        for (const child of children) {
            const key = ObjectTypeReference.key(child.type);
            const type = present(
                context.objects.find((each) => ObjectTypeReference.key(each.typeReference) === key),
                `the child run type ${key}`,
            );
            const live = await context.read(type.table).findMany({
                where: { id: child.id, state: { in: [...LIVE_STATES] } },
            });
            const target = type.target({ scope: this.row.scope, id: child.id });
            calls.push(...live.map(() => type.call("cancel", target)));
        }

        return calls;
    }

    /** Read the caller the run calls as: its serving code, or for others within their grant. */
    async caller(store: ObjectStore, now: number): Promise<AccessContext> {
        // call as the code serving the run when it is the actor
        const self = store.as(this.workflow.run, now);
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
        code: (step: Step<WaitOf<Waits>>, input: Input, context: RunContext) => Promise<Output>,
        reconciliation: ObjectReconciliation,
    ): Promise<number | undefined> {
        // fail a run past its deadline
        const { now, database } = reconciliation;
        const { target } = this;
        if (this.row.deadline !== null && this.row.deadline <= now) {
            const failure = Failure.of(
                WorkflowError.of("RUN_TIMED_OUT", `run ${this.row.id} timed out`),
            );
            await reconciliation.mutate([this.workflow.run.call("fail", { ...target, failure })]);
            await this.#releaseSlot(database);

            return undefined;
        }

        // wait until a sleeping, retried, limited or later run is due, a run waiting for writes waking on them
        if (this.row.wakeAt !== null && this.row.wakeAt > now && this.row.waitType === null) {
            return this.#waitUntil(this.row.wakeAt, now);
        }

        // take a start of the workflow's rate once per run, delaying a limited one
        if (this.row.startedAt === null) {
            const limited = await this.#takeStart(database, now);
            if (limited !== undefined) {
                await reconciliation.mutate([
                    this.workflow.run.call("delay", {
                        ...target,
                        attempts: this.row.attempts,
                        wakeAt: now + limited,
                        failure: this.row.failure,
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
                    attempts: this.row.attempts,
                    wakeAt: slot.lapsesAt,
                    failure: this.row.failure,
                }),
            ]);

            return this.#waitUntil(slot.lapsesAt, now);
        }

        // advance the run while renewing its slot
        const renewing = new AbortController();
        const renewal =
            slot === undefined
                ? Promise.resolve(undefined)
                : this.#renewSlot(reconciliation, renewing.signal);
        let settled: Outcome;
        try {
            settled = await this.#runSteps(code, reconciliation);
        } finally {
            renewing.abort();
        }

        // release the slot after the advance, except a singleton's until its end
        const renewalFailure = await renewal;
        if (slot !== undefined && (!this.workflow.isSingleton || settled.isEnded)) {
            await this.#releaseSlot(database);
        }

        // fail with a renewal's failure while the run goes on
        if (renewalFailure !== undefined && !settled.isEnded) {
            throw renewalFailure;
        }

        return settled.wake === undefined ? undefined : this.#waitUntil(settled.wake, now);
    }

    /** Run the workflow's function once over the run's recorded steps, settling what it did. */
    async #runSteps(
        code: (step: Step<WaitOf<Waits>>, input: Input, context: RunContext) => Promise<Output>,
        reconciliation: ObjectReconciliation,
    ): Promise<Outcome> {
        // replay the run's recorded steps and take the rest, acting as its actor while its grant lasts
        const { store, now } = reconciliation;
        const { target } = this;
        try {
            const steps = new Step<WaitOf<Waits>>(
                this,
                await this.caller(store, now),
                store,
                await Step.load(store, this),
            );
            const input = this.workflow.input.parse(this.row.input);
            const output = await code(steps, input, { id: this.row.id, scope: this.row.scope });

            // complete the run with its output
            await reconciliation.mutate([
                this.workflow.run.call("complete", {
                    ...target,
                    output: recorded(this.workflow.output.parse(output)),
                }),
            ]);

            return { wake: undefined, isEnded: true };
        } catch (error) {
            // wait at a waiting step until its time, or for its type's writes, unless the run waits so already
            if (error instanceof Suspension) {
                const wakeAt = error.at ?? null;
                const waitType = error.waitType ?? null;
                if (
                    this.row.state === "waiting" &&
                    this.row.wakeAt === wakeAt &&
                    this.row.waitType === waitType
                ) {
                    return { wake: error.at, isEnded: false };
                }
                await reconciliation.mutate([
                    this.workflow.run.call("wait", { ...target, waitType, wakeAt }),
                ]);

                return { wake: error.at, isEnded: false };
            }
            // fail on a final refusal, or an input or output its schema refuses
            else if (CallError.refusal(error) !== undefined || error instanceof schema.Error) {
                await reconciliation.mutate([
                    this.workflow.run.call("fail", { ...target, failure: Failure.of(error) }),
                ]);

                return { wake: undefined, isEnded: true };
            }
            // retry another failure after its backoff while attempts remain, else fail with it
            else {
                const attempts = this.row.attempts + 1;
                const failure = Failure.of(error);
                if (RetryPolicy.isRetried(this.workflow.retries, attempts)) {
                    const drawn = RetryPolicy.draw(`${this.row.id} ${attempts}`);
                    const wakeAt =
                        now + RetryPolicy.interval(this.workflow.retries, attempts, () => drawn);
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
                  holder: this.row.id,
                  duration: LEASE_MILLISECONDS,
                  now,
                  slots: concurrency.slots,
              });
    }

    /** Renew the run's slot a third into its lease until the advance ends, answering a failure. */
    async #renewSlot(
        reconciliation: ObjectReconciliation,
        signal: AbortSignal,
    ): Promise<Error | undefined> {
        // renew until the advance ends, stopping at the first failure
        const clock = reconciliation.store.clock;
        try {
            while (
                await clock.sleep(LEASE_MILLISECONDS / 3, signal).then(
                    () => true,
                    () => false,
                )
            ) {
                await this.#takeSlot(reconciliation.database, clock.now());
            }
        } catch (error) {
            return Failure.error(Failure.of(error));
        }

        return undefined;
    }

    /** Release the run's concurrency slot. */
    async #releaseSlot(database: DatabaseConnection): Promise<void> {
        if (this.workflow.limits.concurrency !== undefined) {
            await Lease.release(database, { holder: this.row.id, name: this.#limitName });
        }
    }

    /** Answer the wait until a time, no later than the run's deadline. */
    #waitUntil(at: number, now: number): number {
        const due = this.row.deadline === null ? at : Math.min(at, this.row.deadline);

        return Math.max(0, due - now);
    }

    /** The name of the limits of the workflow's runs. */
    get #limitName(): string {
        return `workflow:${this.workflow.package.name}/${this.workflow.name}`;
    }

    /** Whose limit the run counts against: its scope's, its actor's or an input value's. */
    get #limitKey(): string {
        return Limits.key(this.workflow.limits.key, {
            scope: this.row.scope,
            caller: this.actor,
            input: this.row.input,
        });
    }
}
