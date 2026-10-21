import { AccessContext, Grant } from "@destack/access";
import type { Select, Table } from "@destack/db";
import type { MethodContext } from "@destack/object";
import { CallError } from "@destack/object";
import { type JsonValue, present, schema } from "@destack/schema";
import { Call, ObjectReference, Subject } from "@destack/sync";
import { Failure } from "@destack/error";
import type { RunContext, Step } from "../step/step.ts";
import { defineWorkflow } from "../workflow/workflow.ts";
import { approval } from "./approval.ts";
import { changeRequest } from "./request.ts";
import { LIVE_STATES, Run } from "../run/run.ts";

/** The input of a change's run: its change request and the principals asked to approve it. */
const ChangeInput = schema.object({
    /** The change request. */
    id: schema.identifier("change-request"),
    /** The principals asked to approve the change. */
    approvers: schema.array(Subject).min(1),
});
/** The input of a change's run. */
export type ChangeInput = schema.Infer<typeof ChangeInput>;

/** Apply a change request once its approvers approve it, or reject it once one rejects it. */
export const change = defineWorkflow({ name: "change", input: ChangeInput, waits: [approval] });

/** A call one principal asks others to approve, as its row stores it. */
export class ChangeRequest {
    /** The change request's row. */
    readonly row: Select<typeof changeRequest.table>;

    /** Read a change request from its row. */
    constructor(row: Select<typeof changeRequest.table>) {
        this.row = row;
    }

    /** Start the change's run as the change's own code, asking its approvers. */
    async start(context: MethodContext, approvers: readonly Subject[]): Promise<void> {
        const { id } = this.row;
        const start = change.start(
            { id, approvers: [...approvers] },
            { key: id, actor: context.self.subject },
        );
        await context.mutate(context.self, [start]);
    }

    /** Cancel the change's live run. */
    async cancel(context: MethodContext): Promise<void> {
        const live = await context.read(change.run).findMany({
            where: { key: this.row.id, state: { in: [...LIVE_STATES] } },
        });
        await context.mutate(
            context.self,
            live.map((row) => change.run.call("cancel", new Run(change, row).target)),
        );
    }

    /** Read whom the approved call runs as: the requester, or the last approver. */
    async principal(context: MethodContext): Promise<Subject> {
        // run a review's call as its requester
        if (this.row.as === "requester") {
            return ObjectReference.at("requester", this.row);
        }

        // run an elevation's call as the approver whose approval decided it
        const approvals = await context.read(approval).findMany({
            where: {
                parentPackageId: changeRequest.package.id,
                parentType: changeRequest.name,
                parentId: this.row.id,
                state: "approved",
            },
            orderBy: { updatedAt: "desc" },
            limit: 1,
        });

        return ObjectReference.at(
            "approver",
            present(approvals[0], `the approval of ${this.row.id}`),
        );
    }

    /** Make the approved call as its principal through the recorded actors' grants. */
    async execute(context: MethodContext): Promise<JsonValue> {
        // act as the principal through the actors, the change's own code when none were recorded
        const subject = await this.principal(context);
        const reader = { read: <Read extends Table>(table: Read) => context.read(table) };
        const recorded = this.row.actors;
        const actors = recorded.length === 0 ? [context.self.subject] : recorded;
        const acting = await Grant.chain(reader, { subject, actors }, context.now);

        // make the call under the change's approval
        const caller = AccessContext.of(acting, context.now, { approval: this.row.id });
        const [output] = await context.mutate(caller, [Call.parse(this.row.call)]);

        return schema.json().parse(output ?? null);
    }
}

/** A change's run: ask for approvals, then make the call or reject the change. */
export class Change {
    /** The run's steps. */
    readonly #step: Step;
    /** The run's input: the change request and its approvers. */
    readonly #input: ChangeInput;
    /** The scope the change lives in. */
    readonly #scope: string;

    /** Run a change in a run's steps. */
    constructor(step: Step, input: ChangeInput, context: RunContext) {
        // take the steps, the input and the change's scope
        this.#step = step;
        this.#input = input;
        this.#scope = context.scope;
    }

    /** Wait for the approvers' decision, then make the call as its principal or reject it. */
    async run(): Promise<void> {
        // read the approvals the change needs, and wait for its approvers' decision
        const target = changeRequest.target({ scope: this.#scope, id: this.#input.id });
        const requested = await this.#step.query("read", changeRequest.call("get", target));
        const subject = {
            packageId: changeRequest.package.id,
            type: changeRequest.name,
            scope: this.#scope,
            id: this.#input.id,
        };
        const decision = await this.#step.approve("decision", subject, {
            approvers: this.#input.approvers,
            count: requested.count,
        });

        // make the call once approved, recording the refusal it answers
        if (decision === "approved") {
            try {
                await this.#step.call("execute", changeRequest.call("execute", target));
                await this.#step.call("apply", changeRequest.call("apply", target));
            } catch (error) {
                if (CallError.refusal(error) === undefined) {
                    throw error;
                }
                const failure = Failure.of(error);
                await this.#step.call("fail", changeRequest.call("fail", { ...target, failure }));
            }
        }
        // reject the change once an approver rejects it, the decision without a timeout
        else {
            await this.#step.call("reject", changeRequest.call("reject", target));
        }
    }
}
