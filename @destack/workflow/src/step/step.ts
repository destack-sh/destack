import { AccessContext, Caller } from "@destack/access";
import { and, eq, TABLE, Transaction } from "@destack/db";
import { CallSite, MethodContext, ObjectType } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { CallError, errorOf, isCallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { Duration, type JsonValue, schema } from "@destack/schema";
import { Failure, ObjectReference, ObjectTypeReference, Subject } from "@destack/sync";
import type * as sync from "@destack/sync";
import { Approval, approval, type ApprovalState } from "../approval/approval.ts";
import type { RunRecord } from "../run/run.ts";
import { recorded, type Workflow } from "../workflow/workflow.ts";

/** A step's row as replaying it reads it. */
export const StepRecord = schema.looseObject({
    /** The step. */
    id: schema.string(),
    /** The step's name. */
    name: schema.string(),
    /** The step's place in its run, in the order the run first took it. */
    position: schema.number().int(),
    /** The step's output once it completed. */
    output: schema.json().nullable(),
    /** The failure a call step answered. */
    failure: Failure.nullable(),
    /** When a sleeping or waiting step wakes. */
    wakeAt: schema.number().nullable(),
    /** When the step completed, null while it waits. */
    completedAt: schema.number().nullable(),
    /** When the run first took the step, which stamps the request a prepared call derives. */
    createdAt: schema.number(),
});
/** A step's row as replaying it reads it. */
export type StepRecord = schema.Infer<typeof StepRecord>;

/** What steps read of their workflow: its name, the types it waits for, its run and step types. */
type WorkflowObjects = Pick<Workflow, "name" | "waits" | "run" | "step">;

/** A workflow another run invokes: its name, input, runs, how they start, and their output. */
export type InvokedWorkflow<Input, Output> = Pick<
    Workflow<Input, Output>,
    "name" | "input" | "output" | "run" | "start"
>;

/** Who decides on an object, how many approvals approve it, and how long the run waits. */
export interface ApprovalRequest {
    /** The principals asked to decide. */
    readonly approvers: readonly Subject[];
    /** The approvals that approve the object, one by default. */
    readonly count?: number;
    /** How long the run waits for the decision, absent to wait until it comes. */
    readonly timeout?: Duration;
}

/** The steps a workflow's function takes, each memoised by its name across its run's advances. */
export interface Step {
    /** Make an object method call once as the run's actor. */
    readonly call: <Result>(name: string, call: sync.Call<Result>) => Promise<Result>;
    /** Read an object query method once as the run's actor, answering the same on replay. */
    readonly query: <Result>(name: string, call: sync.Call<Result>) => Promise<Result>;
    /** Run local code once, recording its output, which later advances read by its schema. */
    readonly run: <Output>(
        name: string,
        output: schema.Schema<Output>,
        work: () => Promise<Output>,
    ) => Promise<Output>;
    /** Wait for a duration from the step's first advance. */
    readonly sleep: (name: string, duration: Duration) => Promise<void>;
    /** Wait until a time. */
    readonly sleepUntil: (name: string, at: number) => Promise<void>;
    /** Wait for a listing's first object as its type's rows change, none past a timeout. */
    readonly waitFor: <Item>(
        name: string,
        listing: sync.Call<{ readonly items: readonly Item[] }>,
        options?: { readonly timeout?: Duration },
    ) => Promise<Item | undefined>;
    /** Ask approvers to decide on an object, answering the decision, requested past a timeout. */
    readonly approve: (
        name: string,
        subject: ObjectReference,
        request: ApprovalRequest,
    ) => Promise<ApprovalState>;
    /** Start a run of a workflow the store serves once, answering its output once it ends. */
    readonly invoke: <Input, Output>(
        name: string,
        workflow: InvokedWorkflow<Input, Output>,
        input: NoInfer<Input>,
    ) => Promise<Output>;
}

/** An invoked run's row as its invoking step reads it. */
const InvokedRun = schema.looseObject({
    /** The run. */
    id: schema.string(),
    /** Whether the run is live or ended. */
    state: schema.string(),
    /** The run's output once completed, absent for a workflow returning nothing. */
    output: schema.json().nullable().exactOptional(),
    /** The failure the run ended with. */
    failure: Failure.nullable(),
});

/** What a run's function knows of its run. */
export interface RunContext {
    /** The run. */
    readonly id: string;
    /** The scope the run acts in. */
    readonly scope: string;
}

/** The interruption of a run's function at a step that waits, with when it wakes. */
export class Suspension extends Error {
    /** When the run wakes, absent while it waits for an object alone. */
    readonly at: number | undefined;
    /** The object type the run waits for, by its reference key, absent otherwise. */
    readonly waitType: string | undefined;

    /** Interrupt at a waiting step. */
    constructor(at: number | undefined, waitType: string | undefined) {
        super("the run waits");
        this.at = at;
        this.waitType = waitType;
    }
}

/** A run's steps as one advance of its function takes them, recorded as the run's step objects. */
export class StepContext implements Step {
    /** The workflow. */
    readonly #workflow: WorkflowObjects;
    /** The run. */
    readonly #run: RunRecord;
    /** The principal the run acts as. */
    readonly #actor: Subject;
    /** The store the steps act through. */
    readonly #store: ObjectStore;
    /** The caller the run's calls run as. */
    readonly #caller: AccessContext;
    /** The steps recorded before this advance, by name. */
    readonly #recorded: ReadonlyMap<string, StepRecord>;
    /** The names taken in this advance in order, which no two steps share. */
    readonly #taken: string[] = [];
    /** The recorded steps' names, by their place in the run. */
    readonly #positions: ReadonlyMap<number, string>;

    /** Take a run's steps over the step objects recorded so far. */
    constructor(
        workflow: WorkflowObjects,
        run: RunRecord,
        actor: Subject,
        store: ObjectStore,
        recordedSteps: readonly StepRecord[],
    ) {
        // hold the run, its actor and its steps by name
        this.#workflow = workflow;
        this.#run = run;
        this.#actor = actor;
        this.#store = store;
        this.#caller = AccessContext.of(Caller.of(actor), store.clock.now());
        this.#recorded = new Map(recordedSteps.map((step) => [step.name, step]));
        this.#positions = new Map(recordedSteps.map((step) => [step.position, step.name]));
    }

    /** Read the steps a run recorded. */
    static async load(
        store: ObjectStore,
        workflow: WorkflowObjects,
        run: RunRecord,
    ): Promise<StepRecord[]> {
        const table = workflow.step.table;
        const rows = await store.database
            .select()
            .from(table)
            .where(eq(table[TABLE].column("parentId"), run.id));

        return rows.map((row) => StepRecord.parse(row));
    }

    /** Make an object method call once as the run's actor. */
    async call<Result>(name: string, call: sync.Call<Result>): Promise<Result> {
        // answer a recorded call again by its method's output schema, its failure thrown again
        const recordedStep = this.#take(name);
        const completed = StepContext.#completed(recordedStep);
        if (completed !== undefined) {
            return this.#answer(call, completed);
        }

        // make a call preparing nothing in one transaction with its step's record, exactly once
        const now = this.#store.clock.now();
        const method = ObjectType.find(this.#store.objects, call)?.method(call.method);
        try {
            if (method?.prepare === undefined) {
                return await Transaction.run(this.#store.database, async (transaction) => {
                    const [made] = await this.#store.mutate(
                        this.#caller,
                        { id: RequestId.create(now), calls: [call] },
                        { site: CallSite.store, transaction },
                    );
                    await this.#complete(name, { output: recorded(made) }, transaction);

                    return made;
                });
            }

            // make a preparing call under a request its started step derives, which a retry repeats
            const started = recordedStep?.createdAt ?? now;
            if (recordedStep === undefined) {
                await this.#wait(name, undefined, recordedStep);
            }
            const id = await RequestId.derive(started, `${this.#run.id}:${name}`);
            const [made] = await this.#store.mutate(
                this.#caller,
                { id, calls: [call] },
                { site: CallSite.store },
            );
            await this.#complete(name, { output: recorded(made) });

            return made;
        } catch (error) {
            // record a final refusal as the call's answer, and let anything else retry the advance
            if (isCallError(error) && CallError.isRefusal(error.status)) {
                await this.#complete(name, { failure: Failure.of(error) });
            }

            throw error;
        }
    }

    /** Read an object query method once as the run's actor. */
    async query<Result>(name: string, call: sync.Call<Result>): Promise<Result> {
        // answer a recorded read again, else read once and record the answer
        const recordedStep = this.#take(name);
        const completed = StepContext.#completed(recordedStep);
        if (completed !== undefined) {
            return this.#answer(call, completed);
        }
        const result = await this.#store.query(this.#caller, call, CallSite.store);
        await this.#complete(name, { output: recorded(result) });

        return result;
    }

    /** Run local code once, recording its output. */
    async run<Output>(
        name: string,
        output: schema.Schema<Output>,
        work: () => Promise<Output>,
    ): Promise<Output> {
        // answer recorded output again by its schema, else do the work once and record it
        const completed = StepContext.#completed(this.#take(name));
        if (completed !== undefined) {
            return output.parse(StepContext.#replay(completed));
        }
        const done = output.parse(await work());
        await this.#complete(name, { output: recorded(done) });

        return done;
    }

    /** Wait for a duration from the step's first advance. */
    async sleep(name: string, duration: Duration): Promise<void> {
        const at =
            this.#recorded.get(name)?.wakeAt ??
            this.#store.clock.now() + Duration.milliseconds(duration);
        await this.sleepUntil(name, at);
    }

    /** Wait until a time. */
    async sleepUntil(name: string, at: number): Promise<void> {
        // pass a completed sleep, and complete one whose time came
        const recordedStep = this.#take(name);
        if (StepContext.#completed(recordedStep) !== undefined) {
            return;
        } else if (at <= this.#store.clock.now()) {
            await this.#complete(name, { output: null });

            return;
        }

        // wait until the time
        await this.#wait(name, at, recordedStep);
        throw new Suspension(at, undefined);
    }

    /** Wait for a listing's first object as the run's actor lists it. */
    async waitFor<Item>(
        name: string,
        listing: sync.Call<{ readonly items: readonly Item[] }>,
        options: { readonly timeout?: Duration } = {},
    ): Promise<Item | undefined> {
        // require a type the workflow declares it waits for
        const waitType = this.#waitType(listing.type);

        // answer a recorded wait again: its listing's first object, or none past its timeout
        const recordedStep = this.#take(name);
        const first = (page: JsonValue): Item | undefined =>
            MethodContext.results(this.#store.objects, [listing], [page])[0].items[0];
        const completed = StepContext.#completed(recordedStep);
        if (completed !== undefined) {
            const replayed = StepContext.#replay(completed);

            return replayed === null ? undefined : first(replayed);
        }

        // answer the listing's first object, recording its page
        const page = recorded(await this.#store.query(this.#caller, listing, CallSite.store));
        const found = first(page);
        if (found !== undefined) {
            await this.#complete(name, { output: page });

            return found;
        }

        // answer nothing once the timeout passed, else wait for a change of the type or the timeout
        const now = this.#store.clock.now();
        const at =
            recordedStep?.wakeAt ??
            (options.timeout === undefined
                ? undefined
                : now + Duration.milliseconds(options.timeout));
        if (at !== undefined && at <= now) {
            await this.#complete(name, { output: null });

            return undefined;
        }
        await this.#wait(name, at, recordedStep);
        throw new Suspension(at, waitType);
    }

    /** Ask approvers to decide on an object, answering the decision, requested past a timeout. */
    async approve(
        name: string,
        subject: ObjectReference,
        request: ApprovalRequest,
    ): Promise<ApprovalState> {
        // answer a recorded decision again
        const waitType = this.#waitType(approval.typeReference);
        const recordedStep = this.#take(name);
        const completed = StepContext.#completed(recordedStep);
        if (completed !== undefined) {
            return schema
                .enum(["requested", "approved", "rejected"])
                .parse(StepContext.#replay(completed));
        }

        // ask each approver once with the waiting step, asking again those who decided before
        const now = this.#store.clock.now();
        const { approvers, count = 1, timeout } = request;
        const at =
            recordedStep?.wakeAt ??
            (timeout === undefined ? undefined : now + Duration.milliseconds(timeout));
        if (recordedStep === undefined) {
            await Transaction.run(this.#store.database, async (transaction) => {
                // ask the new approvers and again those who decided, recording the waiting step
                const asked = await this.#asked(subject, approvers);
                const calls = asked.flatMap(({ approver, row }) =>
                    StepContext.#ask(subject, approver, row),
                );
                if (calls.length > 0) {
                    await this.#store.mutate(
                        this.#caller,
                        { id: RequestId.create(now), calls },
                        { site: CallSite.store, transaction },
                    );
                }
                await this.#wait(name, at, recordedStep, transaction);
            });
        }

        // answer the approvers' decision, requested once the timeout passed, else wait for approvals
        const decided = Approval.decision(
            (await this.#asked(subject, approvers)).flatMap(({ row }) =>
                row === undefined ? [] : [row],
            ),
            count,
        );
        if (decided !== "requested" || (at !== undefined && at <= now)) {
            await this.#complete(name, { output: decided });

            return decided;
        }
        throw new Suspension(at, waitType);
    }

    /** Start a run of a workflow the store serves once, answering its output once it ends. */
    async invoke<Input, Output>(
        name: string,
        workflow: InvokedWorkflow<Input, Output>,
        input: Input,
    ): Promise<Output> {
        // refuse a workflow answering no output, and answer a recorded run's output again
        const waitType = this.#waitType(workflow.run.typeReference);
        const output = workflow.output;
        if (output === undefined) {
            throw new TypeError(
                `${this.#workflow.name} invokes ${workflow.name}, which declares no output`,
            );
        }
        const recordedStep = this.#take(name);
        const completed = StepContext.#completed(recordedStep);
        if (completed !== undefined) {
            return output.parse(StepContext.#replay(completed));
        }

        // start the run as this run's actor under a key the step derives, with the waiting step
        const key = `${this.#run.id}/${name}`;
        if (recordedStep === undefined) {
            await Transaction.run(this.#store.database, async (transaction) => {
                await this.#store.mutate(
                    this.#store.self,
                    {
                        id: RequestId.create(this.#store.clock.now()),
                        calls: [workflow.start(input, { key, actor: this.#actor })],
                    },
                    { scope: this.#run.scope, transaction },
                );
                await this.#wait(name, undefined, recordedStep, transaction);
            });
            throw new Suspension(undefined, waitType);
        }

        // answer an ended run's output or failure, else wait for it to end
        const table = workflow.run.table;
        const [invoked] = await this.#store.database
            .select()
            .from(table)
            .where(eq(table[TABLE].column("key"), key));
        const ended = InvokedRun.parse(invoked);
        if (ended.state === "completed") {
            await this.#complete(name, { output: ended.output ?? null });

            return output.parse(ended.output ?? null);
        } else if (ended.state === "failed" || ended.state === "cancelled") {
            const failure =
                ended.failure ??
                Failure.of(
                    new CallError("CLIENT_CLOSED_REQUEST", {
                        message: `run ${ended.id} was cancelled`,
                    }),
                );
            await this.#complete(name, { failure });
            throw errorOf(failure);
        }
        throw new Suspension(undefined, waitType);
    }

    /** Read a type's reference key, refusing a type the workflow does not declare it waits for. */
    #waitType(type: sync.ObjectTypeReference): string {
        const waitType = ObjectTypeReference.key(type);
        if (
            !this.#workflow.waits.some(
                (waited) => ObjectTypeReference.key(waited.typeReference) === waitType,
            )
        ) {
            throw new TypeError(
                `${this.#workflow.name} waits for ${waitType}, which it does not declare`,
            );
        }

        return waitType;
    }

    /** Read each approver's approval of an object, absent for one never asked. */
    async #asked(
        subject: ObjectReference,
        approvers: readonly Subject[],
    ): Promise<{ readonly approver: Subject; readonly row: Approval | undefined }[]> {
        const table = approval.table;
        const rows = await this.#store.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table.parentPackageId, subject.packageId),
                    eq(table.parentType, subject.type),
                    eq(table.parentId, subject.id),
                ),
            );

        return approvers.map((approver) => ({
            approver,
            row: rows.find((row) =>
                Subject.same(ObjectReference.at("approver", row), Subject.parse(approver)),
            ),
        }));
    }

    /** Build the call asking an approver: a new approval, or one asking again after a decision. */
    static #ask(
        subject: ObjectReference,
        approver: Subject,
        row: Approval | undefined,
    ): readonly sync.Call[] {
        // ask a new approver
        if (row === undefined) {
            return [
                approval.call("create", {
                    ...approval.in(subject.scope),
                    parent: { packageId: subject.packageId, type: subject.type, id: subject.id },
                    ...ObjectReference.columns("approver", approver),
                }),
            ];
        }
        // ask again an approver who decided
        else if (row.state !== "requested") {
            return [approval.call("request", approval.target(row))];
        }
        // leave a pending approval as it is
        else {
            return [];
        }
    }

    /** Take a step's name for this advance, refusing a repeated name and a replay that diverges. */
    #take(name: string): StepRecord | undefined {
        // refuse a second step of the same name
        if (this.#taken.includes(name)) {
            throw new TypeError(`run ${this.#run.id} takes two steps named ${name}`);
        }
        this.#taken.push(name);

        // refuse a step at another place than the run recorded, as the function took other steps
        const position = this.#taken.length - 1;
        const recordedStep = this.#recorded.get(name);
        const recordedAt = this.#positions.get(position);
        if (
            (recordedStep !== undefined && recordedStep.position !== position) ||
            (recordedAt !== undefined && recordedAt !== name)
        ) {
            throw new Error(
                `run ${this.#run.id} diverged from its recorded steps: it takes ${name} where it recorded ${recordedAt ?? "nothing"}, as its function is not deterministic between steps`,
            );
        }

        return recordedStep;
    }

    /** Answer a recorded call or read again by its method's output, its failure thrown again. */
    #answer<Result>(call: sync.Call<Result>, recordedStep: StepRecord): Result {
        const [result] = MethodContext.results(
            this.#store.objects,
            [call],
            [StepContext.#replay(recordedStep)],
        );

        return result;
    }

    /** Record a step's output or failure as it completes, in a transaction when given. */
    async #complete(
        name: string,
        answer: { readonly output: JsonValue } | { readonly failure: Failure },
        transaction?: Transaction,
    ): Promise<void> {
        // update a waiting or started step, else create the completed one
        const existing = this.#recorded.get(name) ?? (await this.#started(name));
        const values = {
            ...("output" in answer ? { output: answer.output } : { failure: answer.failure }),
            completedAt: this.#store.clock.now(),
        };
        const call =
            existing === undefined
                ? this.#workflow.step.call("create", {
                      ...this.#workflow.step.in(this.#run.scope),
                      parentId: this.#run.id,
                      name,
                      position: this.#position(name),
                      ...values,
                  })
                : this.#workflow.step.call("update", {
                      ...this.#workflow.step.target({
                          scope: this.#run.scope,
                          id: existing.id,
                      }),
                      ...values,
                  });
        await this.#record(call, transaction);
    }

    /** Record a step waiting until a time or for an object, in a transaction when given. */
    async #wait(
        name: string,
        at: number | undefined,
        existing: StepRecord | undefined,
        transaction?: Transaction,
    ): Promise<void> {
        // record a new wait once, keeping its first wake time across advances
        if (existing !== undefined) {
            return;
        }
        await this.#record(
            this.#workflow.step.call("create", {
                ...this.#workflow.step.in(this.#run.scope),
                parentId: this.#run.id,
                name,
                position: this.#position(name),
                ...(at === undefined ? {} : { wakeAt: at }),
            }),
            transaction,
        );
    }

    /** Write a step record as the store, the run's lease making it the only writer. */
    async #record(call: sync.Call, transaction?: Transaction): Promise<void> {
        await this.#store.mutate(
            this.#store.self,
            { id: RequestId.create(this.#store.clock.now()), calls: [call] },
            transaction === undefined ? {} : { transaction },
        );
    }

    /** Read a step this advance started, absent before. */
    async #started(name: string): Promise<StepRecord | undefined> {
        const table = this.#workflow.step.table;
        const [row] = await this.#store.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table[TABLE].column("parentId"), this.#run.id),
                    eq(table[TABLE].column("name"), name),
                ),
            );

        return row === undefined ? undefined : StepRecord.parse(row);
    }

    /** Read a step's place in its run: the order this advance took it in. */
    #position(name: string): number {
        return this.#taken.indexOf(name);
    }

    /** Read whether a recorded step completed. */
    static #completed(recordedStep: StepRecord | undefined): StepRecord | undefined {
        return recordedStep?.completedAt === null ? undefined : recordedStep;
    }

    /** Answer a recorded step: its output, or its failure thrown again. */
    static #replay(recordedStep: StepRecord): JsonValue {
        if (recordedStep.failure !== null) {
            throw errorOf(recordedStep.failure);
        }

        return recordedStep.output;
    }
}
