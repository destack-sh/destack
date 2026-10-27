import {
    AccessContext,
    Caller,
    Grant,
    none,
    principal,
    relation,
    through,
    union,
} from "@destack/access";
import { type Select, Snapshot, type Table } from "@destack/db";
import { Failure } from "@destack/error";
import { type JsonValue, schema } from "@destack/schema";
import { Call, ObjectReference, Subject } from "@destack/sync";
import { CallError } from "../error/call.ts";
import { ObjectError } from "../error/error.ts";
import { field } from "../field/field.ts";
import type { MethodContext } from "../method/context.ts";
import { ObjectWatch, type ObjectReconciliation } from "../object/controller.ts";
import { defineObject } from "../object/object.ts";
import { type Approval, approval, ApprovalQuorum, type ApprovalState } from "./approval.ts";

/** Whose grant an approved change's call runs under: its requester's, or its approver's. */
export const CHANGE_AUTHORITIES = ["requester", "approver"] as const;

/** A call one principal asks others to approve, made once approved as requester or approver. */
export const changeRequest = defineObject(
    {
        name: "change-request",
        plural: "change-requests",
        scope: [principal.space],
        nested: { in: "any", receive: "approve", optional: true },
        fields: {
            /** The principal asking for the change. */
            requester: field.subject().caller().immutable(),
            /** Whose grant the call runs under: the requester's, or an approver's. */
            as: field.enum(CHANGE_AUTHORITIES).immutable().default("requester"),
            /** The call the change makes once approved. */
            call: field.json(Call).immutable(),
            /** The actors the requester called through, none for a direct call. */
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
                        visibility: "package",
                    },
                    fail: {
                        from: ["open"],
                        to: "failed",
                        permission: "advance",
                        fields: ["failure"],
                        visibility: "package",
                    },
                    reject: {
                        from: ["open"],
                        to: "rejected",
                        permission: "advance",
                        visibility: "package",
                    },
                    withdraw: { from: ["open"], to: "withdrawn", permission: "withdraw" },
                },
            }),
            /** The call's output once applied. */
            output: field.json(schema.json()).optional(),
            /** The refusal the call answered once approved. */
            failure: field.json(Failure).optional(),
        },
        permissions: {
            // the object's readers, the requester and the space's managers read it, its code through its role
            read: union(
                through("parent", "read"),
                relation("requester"),
                through("scope", "manage"),
            ),
            // the object's approvers and the space's managers approve it
            approve: union(through("parent", "approve"), through("scope", "manage")),
            // the object's approvers, and anyone reading the space for a call its grant gives once approved
            open: union(through("parent", "approve"), through("scope", "read")),
            // the change's code advances it through its role, and its requester withdraws it
            advance: none(),
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
                approvers: schema.array(ObjectReference.strict()).min(1).exactOptional(),
            }),
        }),
        list: method.list("read"),
    }),
).implement({
    methods: {
        create: async (context, next) => {
            // refuse the acting principal asking itself, as a pull request's author cannot
            const { subject, actors } = context.caller;
            const acting = actors.at(-1) ?? subject;
            const given = context.input.approvers;
            if (given?.some((each) => Subject.same(each, acting)) === true) {
                throw ObjectError.of(
                    "SELF_APPROVAL",
                    "a requester cannot approve their own change",
                );
            }

            // ask the approvers given, or those the caller's kind of request asks
            const approvers = given ?? ChangeRequest.approvers(context.caller);
            const count = context.input.count ?? 1;
            ApprovalQuorum.require(count, approvers.length);
            if (context.input.as === "approver" && approvers.length !== 1) {
                throw ObjectError.of(
                    "APPROVERS_INVALID",
                    "a change run as its approver asks exactly one approver",
                );
            }

            // record the caller's subject and actors, then ask each approver
            const row = await next(
                context.with({
                    input: {
                        ...context.input,
                        count,
                        actors: [...actors],
                        ...ObjectReference.columns("requester", subject),
                    },
                }),
            );
            const parent = { packageId: changeRequest.package.id, type: changeRequest.name };
            await context.mutate(
                context.self,
                approvers.map((approver) =>
                    approval.call("create", {
                        ...approval.in(context.scope),
                        parent: { ...parent, id: row.id },
                        ...ObjectReference.columns("approver", Subject.parse(approver)),
                    }),
                ),
            );

            return row;
        },
        apply: async (context, next) => {
            // refuse approvals that changed since the decision, then make the approved call as its principal
            const change = new ChangeRequest(context.target);
            await change.requireApproved(context);
            const output = await change.execute(context);
            const applied = await next();

            return await context.at(applied).update({ output });
        },
    },
    controller: {
        pending: { state: "open" },
        // decide a change again as each of its approvals changes
        watches: [
            ObjectWatch.of(approval.table, (row): { readonly id: string }[] =>
                row.parentPackageId === changeRequest.package.id &&
                row.parentType === changeRequest.name
                    ? [{ id: row.parentId }]
                    : [],
            ),
        ],
        reconcile: async (reconciliation): Promise<undefined> => {
            for (const row of reconciliation.rows) {
                await new ChangeRequest(row).decide(reconciliation);
            }

            return undefined;
        },
    },
});

/** A change request as its table stores it. */
export type ChangeRequestRow = Select<typeof changeRequest.table>;

/** A call one principal asks others to approve, as its row stores it. */
export class ChangeRequest {
    /** The change request's row. */
    readonly row: ChangeRequestRow;

    /** Read a change request from its row. */
    constructor(row: ChangeRequestRow) {
        this.row = row;
    }

    /** Ask a delegated caller's delegator, refusing a direct caller naming no approvers. */
    static approvers(caller: Caller): readonly ObjectReference[] {
        // ask the person whose grant the delegated caller acts within
        if (Caller.isDelegated(caller)) {
            return [ObjectReference.parse(caller.subject)];
        }
        // refuse a direct request naming no approvers, which would approve itself
        else {
            throw ObjectError.of("APPROVERS_MISSING", "a direct change request names no approvers");
        }
    }

    /** Apply the change once its approvers approve it, or reject it once one rejects it. */
    async decide(reconciliation: ObjectReconciliation<typeof changeRequest>): Promise<void> {
        const decision = this.#decide(await this.#approvals(Snapshot.live(reconciliation.database)));
        const target = changeRequest.target(this.row);

        // make the call once approved, recording the refusal it answers unless the approvals changed meanwhile
        if (decision === "approved") {
            try {
                await reconciliation.mutate([changeRequest.call("apply", target)]);
            } catch (error) {
                const refusal = CallError.refusal(error);
                if (refusal === undefined) {
                    throw error;
                } else if (refusal.reason !== "APPROVALS_CHANGED") {
                    const failure = Failure.of(error);
                    await reconciliation.mutate([
                        changeRequest.call("fail", { ...target, failure }),
                    ]);
                }
            }
        }
        // reject it once an approver rejects it
        else if (decision === "rejected") {
            await reconciliation.mutate([changeRequest.call("reject", target)]);
        }
    }

    /** Refuse applying a change whose approvals no longer approve it. */
    async requireApproved(context: Pick<MethodContext, "read">): Promise<void> {
        const approvals = await this.#approvals(ChangeRequest.#reader(context));
        if (this.#decide(approvals) !== "approved") {
            throw ObjectError.of(
                "APPROVALS_CHANGED",
                `the approvals of ${this.row.id} changed before it applied`,
            );
        }
    }

    /** Make the approved call as its requester with its approvers, or as its one approver alone. */
    async execute(context: MethodContext): Promise<JsonValue> {
        // run a review's call as its requester through its actors, with the subjects who approved it
        const approvals = await this.#approvals(ChangeRequest.#reader(context));
        const approved = approvals.filter((row) => row.state === "approved");
        let caller: AccessContext;
        if (this.row.as === "requester") {
            const subject = ObjectReference.at("requester", this.row);
            const chain = { subject, actors: this.row.actors };
            const acting = await Grant.chain(ChangeRequest.#reader(context), chain, context.now);
            const approvers = approved.map((row) => ObjectReference.at("approver", row));
            caller = AccessContext.of(acting, context.now, { approvers });
        }
        // run an elevation's call as its one approver, acting directly, refusing approvals added since
        else {
            const [decided, ...added] = approved;
            if (decided === undefined || added.length > 0 || approvals.length > 1) {
                throw ObjectError.of(
                    "APPROVERS_INVALID",
                    "a change run as its approver asks exactly one approver",
                );
            }
            caller = AccessContext.of(Caller.of(ObjectReference.at("approver", decided)), context.now);
        }

        // make the call and answer its output
        const [output] = await context.mutate(caller, [Call.parse(this.row.call)]);

        return schema.json().parse(output ?? null);
    }

    /** Decide on the approvals of the approvers other than the acting principal. */
    #decide(approvals: readonly Approval[]): ApprovalState {
        const acting = this.row.actors.at(-1) ?? ObjectReference.at("requester", this.row);
        const decisions = approvals.filter(
            (row) => !Subject.same(acting, ObjectReference.at("approver", row)),
        );

        return new ApprovalQuorum(this.row.count).decide(decisions);
    }

    /** Read the change's approvals. */
    async #approvals(reader: Pick<Snapshot, "read">): Promise<Approval[]> {
        return reader.read(approval.table).findMany({
            where: {
                parentPackageId: changeRequest.package.id,
                parentType: changeRequest.name,
                parentId: this.row.id,
            },
        });
    }

    /** Read a call's tables as a snapshot does. */
    static #reader(context: Pick<MethodContext, "read">): Pick<Snapshot, "read"> {
        return { read: <Read extends Table>(table: Read) => context.read(table) };
    }
}
