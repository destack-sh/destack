import { Caller, intersection, principal, relation, through } from "@destack/access";
import type { Select } from "@destack/db";
import { schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import { field } from "../field/field.ts";
import type { MethodContext } from "../method/context.ts";
import { defineObject } from "../object/object.ts";

/** The longest comment an approver writes with a decision, a review summary's worth. */
const COMMENT_LENGTH = 4000;

/** The states of an approval: asked, approved or rejected. */
export const APPROVAL_STATES = ["requested", "approved", "rejected"] as const;

/** The state of an approval, and the decision of several. */
export type ApprovalState = (typeof APPROVAL_STATES)[number];

/** A principal's decision on an object awaiting approval, such as a change request. */
export const approval = defineObject(
    {
        name: "approval",
        plural: "approvals",
        scope: [principal.space],
        nested: { in: "any", receive: "approve" },
        fields: {
            /** The principal asked to decide. */
            approver: field.subject(),
            /** Whether the approver has yet to decide, approved, or rejected. */
            state: field.state({
                initial: "requested",
                transitions: {
                    approve: {
                        from: ["requested", "rejected"],
                        to: "approved",
                        permission: "decide",
                    },
                    reject: {
                        from: ["requested", "approved"],
                        to: "rejected",
                        permission: "decide",
                    },
                    request: {
                        from: ["approved", "rejected"],
                        to: "requested",
                        permission: "request",
                    },
                },
            }),
            /** What the approver wrote with the decision. */
            comment: field.string(schema.string().max(COMMENT_LENGTH)).optional(),
        },
        permissions: {
            // the parent's readers read approvals, its approvers request them, and each decides its own
            read: through("parent", "read"),
            request: through("parent", "approve"),
            decide: intersection(relation("approver"), through("parent", "approve")),
        },
        constraints: {
            approver: {
                kind: "unique",
                on: ["parentPackageId", "parentType", "parentId", "approverId"],
            },
        },
    },
    (method) => ({
        create: method.create("request", {
            fields: ["approverPackageId", "approverType", "approverScope", "approverId"],
        }),
        list: method.list("read"),
        comment: method.update("decide", { fields: ["comment"] }),
    }),
).implement({
    methods: {
        approve: async (context, next) => {
            Approval.requireDecider(context, context.target);

            return await next();
        },
        reject: async (context, next) => {
            Approval.requireDecider(context, context.target);

            return await next();
        },
    },
});

/** An approval as its table stores it. */
export type Approval = Select<typeof approval.table>;

/** Decide who decides approvals. */
export const Approval = {
    /** Refuse a decision by anyone but the approver itself, acting directly. */
    requireDecider(context: Pick<MethodContext, "caller">, row: Approval): void {
        const { caller } = context;
        const approver = ObjectReference.at("approver", row);
        if (Caller.isDelegated(caller) || !Subject.same(caller.subject, approver)) {
            throw ObjectError.of(
                "DECIDER_REFUSED",
                "an approval is decided by its approver, acting directly",
            );
        }
    },
};

/** The approvals an object needs, any rejection rejecting it as a review blocks a merge. */
export class ApprovalQuorum {
    /** The approvals that approve the object. */
    readonly count: number;

    /** Need a count of approvals. */
    constructor(count: number) {
        this.count = count;
    }

    /** Refuse asking more approvals than the approvers asked can give, one decision each. */
    static require(count: number, approvers: number): void {
        if (count > approvers) {
            throw ObjectError.of(
                "QUORUM_UNREACHABLE",
                `${count} approvals need at least ${count} approvers, not ${approvers}`,
            );
        }
    }

    /** Decide on approvals: rejected by any rejection, approved by enough, else requested. */
    decide(approvals: readonly Pick<Approval, "state">[]): ApprovalState {
        const approved = approvals.filter((each) => each.state === "approved").length;
        // reject on any rejection
        if (approvals.some((each) => each.state === "rejected")) {
            return "rejected";
        }
        // approve with enough approvals
        else if (approved >= this.count) {
            return "approved";
        }
        // wait for the rest
        else {
            return "requested";
        }
    }
}
