import type { AccessContext } from "@destack/access";
import { and, eq, type Select, TABLE, Transaction } from "@destack/db";
import { CallSite, MethodContext, type ObjectCall, ObjectType } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { CallError } from "@destack/object";
import objectPackage from "@destack/object/package";
import { RequestId, ObjectReference, ObjectTypeReference, Subject } from "@destack/sync";
import { Duration, type JsonValue, present, schema } from "@destack/schema";
import { Failure } from "@destack/error";
import type * as sync from "@destack/sync";
import {
    type Approval,
    approval,
    ApprovalQuorum,
    type ApprovalState,
} from "../approval/approval.ts";
import { changeRequest } from "../approval/request.ts";
import type { Run } from "../run/run.ts";
import { type ChildRun, recorded, type StepType, type Workflow } from "../workflow/workflow.ts";
import type {} from "@destack/package/import-meta";
import { WorkflowError } from "../error/index.ts";

/** A step's row as its workflow's step type stores it. */
export type StepRecord = Select<StepType["table"]>;

/** What steps read of their workflow: its name, the types it waits for, its run and step types. */
type WorkflowObjects = Pick<Workflow, "name" | "waits" | "run" | "step">;

/** A run as its steps read it: its workflow's objects, its row and its actor. */
type StepRun = Pick<Run, "row" | "actor"> & { readonly workflow: WorkflowObjects };

/** A workflow another run invokes: its name, input, runs, how they start, and their output. */
export type InvokedWorkflow<Input, Output> = Pick<
    Workflow<Input, Output>,
    "name" | "input" | "output" | "run" | "start"
>;

/** What happens to an invoked run once its invoking run ends, as Temporal's parent close policy. */
export interface InvokeOptions {
    /** Cancel the invoked run with its parent, by default, or abandon it to run on. */
    readonly onParentClose?: "cancel" | "abandon";
}

/** Who decides on an object, how many approvals approve it, and how long the run waits. */
export interface ApprovalRequest {
    /** The principals asked to decide. */
    readonly approvers: readonly ObjectReference[];
    /** The approvals that approve the object, one by default. */
    readonly count?: number;
    /** How long the run waits for the decision, absent to wait until it comes. */
    readonly timeout?: Duration;
}

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

/** The steps a workflow's function takes, each memoised by its name across its run's advances. */
export class Step<Waits extends ObjectType = ObjectType> {
    /** The run, of its workflow and acting for its actor. */
    readonly #run: StepRun;
    /** The store the steps act through. */
    readonly #store: ObjectStore;
    /** The caller the run's calls run as. */
    readonly #caller: AccessContext;
    /** The steps recorded before this advance, by name. */
    readonly #recorded: ReadonlyMap<string, StepRecord>;
    /** The names taken in this advance in order, which no two steps share. */
    readonly #taken: string[];
    /** The recorded steps' names, by their place in the run. */
    readonly #positions: ReadonlyMap<number, string>;

    /** Take a run's steps over its recorded step objects, acting as its actor through a caller. */
    constructor(
        run: StepRun,
        caller: AccessContext,
        store: ObjectStore,
        recordedSteps: readonly StepRecord[],
    ) {
        // take the run, its caller and its steps by name
        this.#run = run;
        this.#taken = [];
        this.#caller = caller;
        this.#store = store;
        this.#recorded = new Map(recordedSteps.map((step) => [step.name, step]));
        this.#positions = new Map(recordedSteps.map((step) => [step.position, step.name]));
    }

    /** Read the steps a run recorded. */
    static async load(store: ObjectStore, run: StepRun): Promise<StepRecord[]> {
        const table = run.workflow.step.table;
        const rows = await store.database
            .select()
            .from(table)
            .where(eq(table[TABLE].column("parentId"), run.row.id));

        return rows;
    }

    /** Make an object method call once as the run's actor, waiting for any approval it needs. */
    async call<Result>(name: string, call: sync.Call<Result>): Promise<Result> {
        // answer a recorded call again by its method's output schema, its failure thrown again
        const recordedStep = this.#take(name);
        const completed = Step.#completed(recordedStep);
        if (completed !== undefined) {
            return this.#answer(call, completed);
        }

        // answer a call awaiting approval once its change request is decided
        if (recordedStep?.changeRequestId != null) {
            return this.#approved(name, call, recordedStep.changeRequestId);
        }

        // make a served call preparing nothing in one transaction with its step's record, exactly once
        const now = this.#store.clock.now();
        const method = ObjectType.find(this.#store.objects, call)?.method(call.method);
        try {
            if (method !== undefined && method.prepare === undefined) {
                return await Transaction.run(this.#store.database, async (transaction) => {
                    const [made] = await this.#store.mutate(
                        this.#caller,
                        { id: RequestId.create(this.#store.environment), calls: [call] },
                        { site: CallSite.store, transaction },
                    );
                    await this.#complete(name, { output: recorded(made) }, transaction);

                    return made;
                });
            }

            // make a preparing call, or one its rows' home runs, under a request its started step derives
            const started = recordedStep?.createdAt ?? now;
            await this.#wait(name, undefined, recordedStep);
            const id = await RequestId.derive(
                started,
                `${this.#run.row.id}:${name}`,
                this.#store.environment.crypto,
            );
            const [made] = await this.#store.mutate(
                this.#caller,
                { id, calls: [call] },
                { site: CallSite.store },
            );
            await this.#complete(name, { output: recorded(made) });

            return made;
        } catch (error) {
            // ask for approval of a call the grant gives once approved
            const refused = CallError.of(error);
            if (refused?.reason === "APPROVAL_REQUIRED" && refused.domain === objectPackage.name) {
                return this.#request(name, call);
            }
            // record a final refusal as the call's answer, and let anything else retry the advance
            else if (CallError.refusal(error) !== undefined) {
                await this.#complete(name, { failure: Failure.of(error) });
            }

            throw error;
        }
    }

    /** Open a change request for a call awaiting approval, and wait on it. */
    async #request<Result>(name: string, call: sync.Call<Result>): Promise<Result> {
        await Transaction.run(this.#store.database, async (transaction) => {
            // open the change request as the run's actor, asking its grant's delegator
            const [opened] = await this.#store.mutate(
                this.#caller,
                {
                    id: RequestId.create(this.#store.environment),
                    calls: [
                        changeRequest.call("create", {
                            ...changeRequest.in(this.#run.row.scope),
                            call,
                        }),
                    ],
                },
                { site: CallSite.store, transaction },
            );

            // record the step waiting on it
            const existing = await this.#started(name);
            const values = { changeRequestId: opened.id };
            await this.#record(
                existing === undefined
                    ? this.#run.workflow.step.call("create", {
                          ...this.#run.workflow.step.in(this.#run.row.scope),
                          parentId: this.#run.row.id,
                          name,
                          position: this.#position(name),
                          ...values,
                      })
                    : this.#run.workflow.step.call("update", {
                          ...this.#run.workflow.step.target({
                              scope: this.#run.row.scope,
                              id: existing.id,
                          }),
                          ...values,
                      }),
                transaction,
            );
        });

        throw new Suspension(undefined, this.#waitType(changeRequest.typeReference));
    }

    /** Answer a call awaiting approval once its change request is decided. */
    async #approved<Result>(
        name: string,
        call: sync.Call<Result>,
        changeRequestId: string,
    ): Promise<Result> {
        // read the change request
        const table = changeRequest.table;
        const [row] = await this.#store.database
            .select()
            .from(table)
            .where(eq(table[TABLE].column("id"), changeRequestId));
        const request = present(row, `change request ${changeRequestId}`);

        // answer the applied call's output, or the refusal it answered
        if (request.state === "applied") {
            const output = request.output ?? null;
            await this.#complete(name, { output });
            const [result] = MethodContext.results(this.#store.objects, [call], [output]);

            return result;
        } else if (request.state === "failed") {
            const failure = present(request.failure, `the failure of ${changeRequestId}`);
            await this.#complete(name, { failure });
            throw Failure.error(failure);
        }
        // refuse a call its approvers rejected or its requester withdrew
        else if (request.state === "rejected" || request.state === "withdrawn") {
            const failure = Failure.of(
                WorkflowError.of(
                    "CHANGE_REQUEST_CLOSED",
                    `change request ${changeRequestId} was ${request.state}`,
                ),
            );
            await this.#complete(name, { failure });
            throw Failure.error(failure);
        }

        // wait for the decision
        throw new Suspension(undefined, this.#waitType(changeRequest.typeReference));
    }

    /** Read an object query method once as the run's actor. */
    async query<Result>(name: string, call: sync.Call<Result>): Promise<Result> {
        // answer a recorded read again, else read once and record the answer
        const recordedStep = this.#take(name);
        const completed = Step.#completed(recordedStep);
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
        const completed = Step.#completed(this.#take(name));
        if (completed !== undefined) {
            return output.parse(Step.#replay(completed));
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
        if (Step.#completed(recordedStep) !== undefined) {
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
        listing: ObjectCall<Waits, { readonly items: readonly Item[] }>,
        options: { readonly timeout?: Duration } = {},
    ): Promise<Item | undefined> {
        // require a type the workflow declares it waits for
        const waitType = this.#waitType(listing.type);

        // answer a recorded wait again: its listing's first object, or none past its timeout
        const recordedStep = this.#take(name);
        const first = (page: JsonValue): Item | undefined =>
            MethodContext.results(this.#store.objects, [listing], [page])[0].items[0];
        const completed = Step.#completed(recordedStep);
        if (completed !== undefined) {
            const replayed = Step.#replay(completed);

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
        ..._undeclared: typeof approval extends Waits ? [] : [never]
    ): Promise<ApprovalState> {
        // answer a recorded decision again
        const waitType = this.#waitType(approval.typeReference);
        const recordedStep = this.#take(name);
        const completed = Step.#completed(recordedStep);
        if (completed !== undefined) {
            return schema
                .enum(["requested", "approved", "rejected"])
                .parse(Step.#replay(completed));
        }

        // ask each approver once with the waiting step, asking again those who decided before
        const now = this.#store.clock.now();
        const { approvers, count = 1, timeout } = request;
        ApprovalQuorum.require(count, approvers.length);
        const quorum = new ApprovalQuorum(count);
        const at =
            recordedStep?.wakeAt ??
            (timeout === undefined ? undefined : now + Duration.milliseconds(timeout));
        if (recordedStep === undefined) {
            await Transaction.run(this.#store.database, async (transaction) => {
                // ask the new approvers and again those who decided, recording the waiting step
                const asked = await this.#asked(subject, approvers);
                const calls = asked.flatMap(({ approver, row }) =>
                    Step.#ask(subject, approver, row),
                );
                if (calls.length > 0) {
                    await this.#store.mutate(
                        this.#caller,
                        { id: RequestId.create(this.#store.environment), calls },
                        { site: CallSite.store, transaction },
                    );
                }
                await this.#wait(name, at, recordedStep, transaction);
            });
        }

        // answer the approvers' decision, requested once the timeout passed, else wait for approvals
        const decided = quorum.decide(
            (await this.#asked(subject, approvers)).flatMap(({ row }) =>
                row === undefined ? [] : [row],
            ),
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
        workflow: InvokedWorkflow<Input, Output> & { readonly run: Waits },
        input: Input,
        options: InvokeOptions = {},
    ): Promise<Output> {
        // answer a recorded run's output again
        const waitType = this.#waitType(workflow.run.typeReference);
        const output = workflow.output;
        const recordedStep = this.#take(name);
        const completed = Step.#completed(recordedStep);
        if (completed !== undefined) {
            return output.parse(Step.#replay(completed));
        }

        // start the run as this run's actor under a key the step derives, recording it with the waiting step
        if (recordedStep === undefined) {
            const key = `${this.#run.row.id}/${name}`;
            await Transaction.run(this.#store.database, async (transaction) => {
                const [started] = await this.#store.mutate(
                    this.#store.as(workflow.run),
                    {
                        id: RequestId.create(this.#store.environment),
                        calls: [workflow.start(input, { key, actor: this.#run.actor })],
                    },
                    { scope: this.#run.row.scope, transaction },
                );
                const child = {
                    type: workflow.run.typeReference,
                    id: schema.looseObject({ id: schema.string() }).parse(started).id,
                    onParentClose: options.onParentClose ?? "cancel",
                };
                await this.#wait(name, undefined, recordedStep, transaction, child);
            });
            throw new Suspension(undefined, waitType);
        }

        // answer the child run the step recorded once it ends
        const child = present(recordedStep.child, `the child run of step ${name}`);
        const table = workflow.run.table;
        const [invoked] = await this.#store.database
            .select()
            .from(table)
            .where(eq(table[TABLE].column("id"), child.id));
        const run = present(invoked, `child run ${child.id}`);
        if (run.state === "completed") {
            const answered = present(run.output, `the output of completed run ${run.id}`);
            await this.#complete(name, { output: answered });

            return output.parse(answered);
        } else if (run.state === "failed") {
            const failure = present(run.failure, `the failure of failed run ${run.id}`);
            await this.#complete(name, { failure });
            throw Failure.error(failure);
        } else if (run.state === "cancelled") {
            const failure = Failure.of(
                WorkflowError.of("RUN_CANCELLED", `run ${run.id} was cancelled`),
            );
            await this.#complete(name, { failure });
            throw Failure.error(failure);
        }
        throw new Suspension(undefined, waitType);
    }

    /** Read a type's reference key, refusing a type the workflow does not declare it waits for. */
    #waitType(type: sync.ObjectTypeReference): string {
        const waitType = ObjectTypeReference.key(type);
        if (
            !this.#run.workflow.waits.some(
                (waited) => ObjectTypeReference.key(waited.typeReference) === waitType,
            )
        ) {
            throw new TypeError(
                `${this.#run.workflow.name} waits for ${waitType}, which it does not declare`,
            );
        }

        return waitType;
    }

    /** Read each approver's approval of an object, absent for one never asked. */
    async #asked(
        subject: ObjectReference,
        approvers: readonly ObjectReference[],
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
            throw WorkflowError.of(
                "STEP_DUPLICATE",
                `run ${this.#run.row.id} has two steps named ${name}`,
            );
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
            throw WorkflowError.of(
                "RUN_NONDETERMINISTIC",
                `run ${this.#run.row.id} takes step ${name} where it recorded ${recordedAt ?? "nothing"}`,
            );
        }

        return recordedStep;
    }

    /** Answer a recorded call or read again by its method's output, its failure thrown again. */
    #answer<Result>(call: sync.Call<Result>, recordedStep: StepRecord): Result {
        const [result] = MethodContext.results(
            this.#store.objects,
            [call],
            [Step.#replay(recordedStep)],
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
                ? this.#run.workflow.step.call("create", {
                      ...this.#run.workflow.step.in(this.#run.row.scope),
                      parentId: this.#run.row.id,
                      name,
                      position: this.#position(name),
                      ...values,
                  })
                : this.#run.workflow.step.call("update", {
                      ...this.#run.workflow.step.target({
                          scope: this.#run.row.scope,
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
        child?: ChildRun,
    ): Promise<void> {
        // record a new wait once
        if (existing !== undefined) {
            return;
        }
        await this.#record(
            this.#run.workflow.step.call("create", {
                ...this.#run.workflow.step.in(this.#run.row.scope),
                parentId: this.#run.row.id,
                name,
                position: this.#position(name),
                ...(at === undefined ? {} : { wakeAt: at }),
                ...(child === undefined ? {} : { child }),
            }),
            transaction,
        );
    }

    /** Write a step record as the store, the run's lease making it the only writer. */
    async #record(call: sync.Call, transaction?: Transaction): Promise<void> {
        await this.#store.mutate(
            this.#store.as(this.#run.workflow.step),
            { id: RequestId.create(this.#store.environment), calls: [call] },
            transaction === undefined ? {} : { transaction },
        );
    }

    /** Read a step this advance started, absent before. */
    async #started(name: string): Promise<StepRecord | undefined> {
        const table = this.#run.workflow.step.table;
        const [row] = await this.#store.database
            .select()
            .from(table)
            .where(
                and(
                    eq(table[TABLE].column("parentId"), this.#run.row.id),
                    eq(table[TABLE].column("name"), name),
                ),
            );

        return row;
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
            throw Failure.error(recordedStep.failure);
        }

        return recordedStep.output;
    }
}
