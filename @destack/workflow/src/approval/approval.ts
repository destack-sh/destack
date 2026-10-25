import { intersection, relation, through } from "@destack/access";
import { space } from "@destack/account/object";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { WorkflowError } from "../error/index.ts";

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
        scope: space,
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
            // the parent's approvers read and request approvals, and each decides its own
            read: through("parent", "approve"),
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
);

/** An approval as its table stores it. */
export type Approval = Select<typeof approval.table>;

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
            throw WorkflowError.of(
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
