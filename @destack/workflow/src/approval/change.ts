import { relation, through } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { isCallError, CallError } from "@destack/runtime/error";
import { schema } from "@destack/schema";
import { Call, Failure, Subject } from "@destack/sync";
import type { RunContext, Step } from "../step/step.ts";
import { defineWorkflow } from "../workflow/workflow.ts";
import { approval } from "./approval.ts";

/** A call one principal asks others to approve, made as that principal once they approve it. */
export const changeRequest = defineObject(
    {
        name: "change-request",
        plural: "change-requests",
        scope: space,
        nested: { in: "any", receive: "approve" },
        fields: {
            /** The principal asking for the change, whom the held call runs as. */
            requester: field.subject().caller(),
            /** The call the change makes once approved. */
            call: field.json(Call).immutable(),
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
            /** The refusal the held call answered once approved. */
            failure: field.json(Failure).optional(),
        },
        permissions: {
            // the object's readers read the change, its approvers open and approve it, and its requester advances it
            read: through("parent", "read"),
            approve: through("parent", "approve"),
            open: through("parent", "approve"),
            advance: relation("requester"),
            withdraw: relation("requester"),
        },
        attachments: [approval.attach({ by: "approve" })],
        constraints: { count: { kind: "check", condition: { count: { gte: 1 } } } },
    },
    (method) => ({
        create: method.create("open", {
            fields: ["call", "count"],
            input: schema.object({
                /** The principals asked to approve the change. */
                approvers: schema.array(Subject).min(1),
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

        // make the held call once approved, recording the refusal it answers
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
