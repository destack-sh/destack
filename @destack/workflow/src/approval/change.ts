import { contained, relation, through, union } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field, type MethodContext } from "@destack/object";
import { isCallError, CallError } from "@destack/runtime/error";
import { type Identifier, present, schema } from "@destack/schema";
import { Call, Failure, ObjectReference, Subject } from "@destack/sync";
import type { RunContext, Step } from "../step/step.ts";
import { defineWorkflow } from "../workflow/workflow.ts";
import { approval } from "./approval.ts";
import { LIVE_STATES, Run } from "../run/run.ts";

/** Whose grant an approved change's call runs under: its requester's, or its approver's. */
export const CHANGE_AUTHORITIES = ["requester", "approver"] as const;

/** A call one principal asks others to approve, made once approved as requester or approver. */
export const changeRequest = defineObject(
    {
        name: "change-request",
        plural: "change-requests",
        scope: space,
        nested: { in: "any", receive: "approve", optional: true },
        fields: {
            /** The principal asking for the change. */
            requester: field.subject().caller().immutable(),
            /** Whose grant the call runs under: the requester's, or an approver's. */
            as: field.enum(CHANGE_AUTHORITIES).immutable().default("requester"),
            /** The call the change makes once approved. */
            call: field.json(Call).immutable(),
            /** The actors the call runs through, the change's own code when none. */
            actors: field.json(schema.array(Subject)).immutable().default([]),
            /** The approvals that approve the change. */
            count: field.integer().immutable().default(1),
            /** Whether the change is open, applied, failed, rejected or withdrawn. */
            state: field.state({
                initial: "open",
                transitions: {
                    apply: {
                        from: ["open"],
                        to: "applied",
                        permission: "advance",
                        isInternal: true,
                    },
                    fail: {
                        from: ["open"],
                        to: "failed",
                        permission: "advance",
                        fields: ["failure"],
                        isInternal: true,
                    },
                    reject: {
                        from: ["open"],
                        to: "rejected",
                        permission: "advance",
                        isInternal: true,
                    },
                    withdraw: { from: ["open"], to: "withdrawn", permission: "withdraw" },
                },
            }),
            /** The refusal the call answered once approved. */
            failure: field.json(Failure).optional(),
        },
        permissions: {
            // the object's readers, the requester, the space's managers and the change's own code read it
            read: union(
                through("parent", "read"),
                relation("requester"),
                through("scope", "manage"),
                contained(),
            ),
            // the object's approvers, the requester of its delegate's call, the space's managers, and its own code
            approve: union(
                through("parent", "approve"),
                relation("requester"),
                through("scope", "manage"),
                contained(),
            ),
            // the object's approvers, and anyone reading the space for a call its grant gives once approved
            open: union(through("parent", "approve"), through("scope", "read")),
            // the change's own code advances it, and its requester withdraws it
            advance: contained(),
            withdraw: relation("requester"),
        },
        attachments: [approval.attach({ by: "approve" })],
        constraints: { count: { kind: "check", condition: { count: { gte: 1 } } } },
    },
    (method) => ({
        create: method.create("open", {
            fields: ["call", "count", "as"],
            input: schema.object({
                /** The principals asked to approve, the caller's grant delegator when absent. */
                approvers: schema.array(Subject).min(1).exactOptional(),
            }),
        }),
        list: method.list("read"),
    }),
);

/** A change request as its table stores it. */
export type ChangeRequest = Select<typeof changeRequest.table>;

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

/** The change requests and the runs applying them. */
export const ChangeRequest = {
    /** Start the run of a change request as the change's own code, asking its approvers. */
    start(
        context: MethodContext,
        id: Identifier<"change-request">,
        approvers: readonly Subject[],
    ): Promise<unknown> {
        return context.mutate(context.self, [
            change.start(
                { id, approvers: [...approvers] },
                { key: id, actor: context.self.subject },
            ),
        ]);
    },

    /** Cancel the live run of a change request. */
    async cancel(context: MethodContext, id: Identifier<"change-request">): Promise<void> {
        const live = await context.read(change.run).findMany({
            where: { key: id, state: { in: [...LIVE_STATES] } },
        });
        await context.mutate(
            context.self,
            live.map((row) => change.run.call("cancel", new Run(change, row).target)),
        );
    },

    /** Read whom an approved change's call runs as: its requester, or its last approver. */
    async principal(context: MethodContext, request: ChangeRequest): Promise<Subject> {
        // run a review's call as its requester
        if (request.as === "requester") {
            return ObjectReference.at("requester", request);
        }

        // run an elevation's call as the approver whose approval decided it
        const approvals = await context.read(approval).findMany({
            where: {
                parentPackageId: changeRequest.package.id,
                parentType: changeRequest.name,
                parentId: request.id,
                state: "approved",
            },
            orderBy: { updatedAt: "desc" },
            limit: 1,
        });

        return ObjectReference.at(
            "approver",
            present(approvals[0], `the approval of ${request.id}`),
        );
    },

    /** Ask for the approvals of a change, then make its call as the requester or reject it. */
    async run(step: Step, input: ChangeInput, context: RunContext): Promise<void> {
        // read the approvals the change needs, and wait for its approvers' decision
        const target = changeRequest.target({ scope: context.scope, id: input.id });
        const requested = await step.query("read", changeRequest.call("get", target));
        const subject = {
            packageId: changeRequest.package.id,
            type: changeRequest.name,
            scope: context.scope,
            id: input.id,
        };
        const decision = await step.approve("decision", subject, {
            approvers: input.approvers,
            count: requested.count,
        });

        // make the call once approved, recording the refusal it answers
        if (decision === "approved") {
            try {
                await step.call("apply", changeRequest.call("apply", target));
            } catch (error) {
                if (!isCallError(error) || !CallError.isRefusal(error.status)) {
                    throw error;
                }
                await step.call(
                    "fail",
                    changeRequest.call("fail", { ...target, failure: Failure.of(error) }),
                );
            }
        }
        // reject the change once an approver rejects it, the decision without a timeout
        else {
            await step.call("reject", changeRequest.call("reject", target));
        }
    },
};
