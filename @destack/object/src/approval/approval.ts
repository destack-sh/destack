import {
    accepts,
    AccessContext,
    AccessError,
    AuthenticationAssurance,
    Caller,
    intersection,
    principal,
    relation,
    StepUp,
    through,
} from "@destack/access";
import type { Select } from "@destack/db";
import { schema } from "@destack/schema";
import { ObjectReference, Subject } from "@destack/sync";
import { ObjectError } from "../error/error.ts";
import { field } from "../field/field.ts";
import type { MethodContext } from "../method/context.ts";
import { defineObject } from "../object/object.ts";
import { nested } from "../trait/index.ts";
import { argument, plural, t } from "@destack/locale";

/** The longest comment an approver writes with a decision, a review summary's worth. */
const COMMENT_LENGTH = 4000;

/** The states of an approval: asked, approved or rejected. */
export const APPROVAL_STATES = ["requested", "approved", "rejected"] as const;

/** The state of an approval, and the decision of several. */
export type ApprovalState = (typeof APPROVAL_STATES)[number];

/** A decision asked of a principal or a subject set on an object, such as a change request. */
export const approval = defineObject(
    {
        name: "approval",
        plural: "approvals",
        title: t`${plural(argument("count"), { one: "Approval", other: "Approvals" })}`,
        scope: [principal.space],
        fields: {
            /** The principal or subject set asked to decide, as GitHub asks a person or a team. */
            approver: field.subject().open(),
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
            /** The authentication deciding asks of the approver, as the call it approves needs. */
            stepUp: field.json(StepUp.schema).immutable().optional(),
            /** How strongly and how recently the approver authenticated when deciding. */
            assurance: field.json(AuthenticationAssurance).optional(),
        },
        constraints: {
            approver: {
                kind: "unique",
                on: ["parentPackageId", "parentType", "parentId", "approverId"],
            },
        },
        access: {
            permissions: {
                // the parent's readers read approvals, its approvers request them, and the asked or a member of the asked set decides
                read: through("parent", "read"),
                request: through("parent", "approve"),
                decide: intersection(relation("approver"), through("parent", "approve")),
            },
        },
        traits: [nested({ in: "any", add: "approve" })],
    },
    (method) => ({
        create: method.create("request", {
            fields: [
                "approverPackageId",
                "approverType",
                "approverScope",
                "approverId",
                "approverRelation",
                "stepUp",
            ],
        }),
        list: method.list("read"),
        comment: method.update("decide", { fields: ["comment"] }),
    }),
).implement({
    methods: {
        create: async (context, next) => {
            // refuse asking a subject the approver relation does not accept, contributed sets among them
            const asked = Approval.asked(context.input);
            const { authorizer } = context;
            if (!accepts(authorizer.relation(context.object.policy, "approver"), asked)) {
                const subject =
                    asked.relation === undefined ? asked.type : `${asked.type}#${asked.relation}`;
                throw AccessError.of(
                    "SUBJECT_INVALID",
                    `approver '${subject}' is invalid: approvals cannot ask this subject`,
                );
            }

            return await next();
        },
        approve: async (context, next) => {
            // require the approver acting directly, authenticated as the approval asks
            Approval.requireDecider(context);
            const assurance = AccessContext.assurance(context.caller);
            Approval.requireAssured(context.target, assurance, context.now);

            // record how the approver authenticated, which the approved call runs with
            const approved = await next();

            return assurance === undefined
                ? approved
                : await context.at(approved).update({ assurance });
        },
        reject: async (context, next) => {
            Approval.requireDecider(context);

            return await next();
        },
    },
});

/** An approval as its table stores it. */
export type Approval = Select<typeof approval.table>;

/** Decide who decides approvals. */
export const Approval = {
    /** Refuse a decision made through a delegate, as only the asked may decide. */
    requireDecider(context: Pick<MethodContext, "caller">): void {
        if (Caller.isDelegated(context.caller)) {
            throw ObjectError.of(
                "DECIDER_DENIED",
                "approval decision is forbidden: an approval is decided by its approver, acting directly",
            );
        }
    },

    /** Refuse an approver authenticated less strongly or less recently than the approval asks. */
    requireAssured(
        row: Pick<Approval, "stepUp">,
        assurance: AuthenticationAssurance | undefined,
        now: number,
    ): void {
        // pass an approval asking no step-up
        const { stepUp } = row;
        if (stepUp === null || stepUp === undefined) {
            return;
        }

        // refuse an approver below the level, or authenticated before the age allows
        const isStale =
            stepUp.maxAge !== undefined &&
            assurance !== undefined &&
            now - assurance.authenticatedAt >= stepUp.maxAge;
        if (assurance === undefined || assurance.level < stepUp.assurance || isStale) {
            throw AccessError.of(
                "ASSURANCE_UNAUTHENTICATED",
                "authentication required: authentication assurance is too low",
                { data: stepUp },
            );
        }
    },

    /** Read who an approval or its input asks: a principal, or a subject set by its relation. */
    asked(
        row: Parameters<typeof ObjectReference.at<"approver">>[1] & {
            readonly approverRelation?: string | null;
        },
    ): Subject {
        const asked = ObjectReference.at("approver", row);
        const set = row.approverRelation;

        return set === undefined || set === null ? asked : { ...asked, relation: set };
    },

    /** Read who decided an approval last: its approver, or the member of its set who did. */
    decider(row: Approval): Subject {
        return ObjectReference.at("updatedBy", row);
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
                "QUORUM_INVALID",
                `approval quorum is invalid: ${count} approvals need at least ${count} approvers, and it asks ${approvers} approvers`,
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
