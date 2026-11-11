import {
    AccessContext,
    Caller,
    Credential,
    Grant,
    GrantPermission,
    none,
    StepUp,
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
import { Approval, approval, ApprovalQuorum, type ApprovalState } from "./approval.ts";
import { nested } from "../trait/index.ts";
import { argument, plural, t } from "@destack/locale";

/** A call one principal asks others to approve, made once approved as requester or approver. */
export const changeRequest = defineObject(
    {
        name: "change-request",
        plural: "change-requests",
        title: t`${plural(argument("count"), { one: "Change request", other: "Change requests" })}`,
        scope: [principal.space],
        fields: {
            /** The principal asking for the change. */
            requester: field.subject().caller().immutable(),
            /** The approver the call runs as, else the requester (SQL's EXECUTE AS). */
            as: field.subject().open().immutable().optional(),
            /** The call the change makes once approved. */
            call: field.json(Call).immutable(),
            /** The actors the requester called through, none for a direct call. */
            actors: field.json(schema.array(Subject)).immutable().default([]),
            /** The approvals that approve the change. */
            count: field.integer().immutable().default(1),
            /** The authentication approving asks of each approver, as the call needs it. */
            stepUp: field.json(StepUp.schema).immutable().optional(),
            /** The permission the requester's grant lacks, which allowing it from now on adds. */
            grant: field.json(GrantPermission.schema).immutable().optional(),
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
        attachments: [approval.attach({ by: "approve" })],
        constraints: { count: { kind: "check", condition: { count: { gte: 1 } } } },
        access: {
            permissions: {
                // the object's readers, the requester and the space's managers read it, its code through its role
                read: union(
                    through("parent", "read"),
                    relation("requester"),
                    through("scope", "manage"),
                ),
                // the object's approvers, the space's managers, and the approver the call runs as approve it
                approve: union(
                    through("parent", "approve"),
                    through("scope", "manage"),
                    relation("as"),
                ),
                // the object's approvers, anyone reading the space, and the approver the call runs as open it
                open: union(through("parent", "approve"), through("scope", "read"), relation("as")),
                // the change's code advances it through its role, and its requester withdraws it
                advance: none(),
                withdraw: relation("requester"),
            },
        },
        traits: [nested({ in: "any", add: "approve", optional: true })],
    },
    (method) => ({
        create: method.create("open", {
            fields: [
                "call",
                "count",
                "asPackageId",
                "asType",
                "asScope",
                "asId",
                "asRelation",
                "stepUp",
                "grant",
            ],
            input: schema.object({
                /** The principals or subject sets asked, the grant delegator when absent. */
                approvers: schema.array(Subject).min(1).exactOptional(),
            }),
        }),
        list: method.list("read"),
    }),
).implement({
    methods: {
        create: async (context, next) => {
            // ask the approvers given, the approver the call runs as, or those the caller's kind of request asks
            const { subject, actors } = context.caller;
            const as =
                context.input.asId === undefined ? undefined : ChangeRequest.as(context.input);
            const approvers =
                context.input.approvers ??
                (as === undefined ? ChangeRequest.approvers(context.caller) : [as]);
            const count = context.input.count ?? 1;
            ApprovalQuorum.require(count, approvers.length);

            // refuse a change run as anyone but its one approver
            const [only, ...others] = approvers;
            if (
                as !== undefined &&
                (only === undefined || others.length > 0 || !Subject.same(only, as))
            ) {
                throw ObjectError.of(
                    "APPROVERS_INVALID",
                    "approver set is invalid: a change run as an approver asks exactly that approver",
                );
            }

            // refuse the acting principal asking itself, as a pull request's author cannot
            const acting = actors.at(-1) ?? subject;
            if (approvers.some((each) => Subject.same(each, acting))) {
                throw ObjectError.of(
                    "SELF_APPROVAL_INVALID",
                    "approval is invalid: a requester cannot approve their own change",
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
                approvers.map((approver) => {
                    const asked = Subject.parse(approver);

                    return approval.call("create", {
                        ...approval.in(context.scope),
                        parent: { ...parent, id: row.id },
                        ...ObjectReference.columns("approver", asked),
                        ...(asked.relation === undefined
                            ? {}
                            : { approverRelation: asked.relation }),
                        ...(context.input.stepUp === undefined
                            ? {}
                            : { stepUp: context.input.stepUp }),
                    });
                }),
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
    controllers: [
        {
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
            reconcile: async (reconciliation) => {
                for (const row of reconciliation.rows) {
                    await new ChangeRequest(row).decide(reconciliation);
                }
            },
        },
    ],
});

/** A call one principal asks others to approve, as its row stores it. */
export class ChangeRequest {
    /** The change request's row. */
    readonly row: Select<typeof changeRequest.table>;

    /** Read a change request from its row. */
    constructor(row: Select<typeof changeRequest.table>) {
        this.row = row;
    }

    /** Read the approver a change runs as: a principal, or a subject set by its relation. */
    static as(
        row: Parameters<typeof ObjectReference.at<"as">>[1] & {
            readonly asRelation?: string | null;
        },
    ): Subject {
        const as = ObjectReference.at("as", row);
        const set = row.asRelation;

        return set === undefined || set === null ? as : { ...as, relation: set };
    }

    /** Ask a delegated caller's delegator, refusing a direct caller naming no approvers. */
    static approvers(caller: Caller): readonly Subject[] {
        // ask the person whose grant the delegated caller acts within
        if (Caller.isDelegated(caller)) {
            return [caller.subject];
        }
        // refuse a direct request naming no approvers, which would approve itself
        else {
            throw ObjectError.of(
                "APPROVERS_REQUIRED",
                "approver of a direct change request is required",
            );
        }
    }

    /** Apply the change once its approvers approve it, or reject it once one rejects it. */
    async decide(reconciliation: ObjectReconciliation<typeof changeRequest>): Promise<void> {
        const decision = this.#decide(
            await this.#approvals(Snapshot.live(reconciliation.database)),
        );
        const target = changeRequest.target(this.row);

        // make the call once approved, recording the refusal it answers unless the approvals changed meanwhile
        if (decision === "approved") {
            try {
                await reconciliation.mutate([changeRequest.call("apply", target)]);
            } catch (error) {
                const refusal = CallError.refusal(error);
                if (refusal === undefined) {
                    throw error;
                } else if (refusal.reason !== "APPROVALS_CONFLICT") {
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
                "APPROVALS_CONFLICT",
                `cannot apply change request '${this.row.id}': its approvals changed before it applied`,
            );
        }
    }

    /** Make the approved call as its requester with its approvers, or as its one approver alone. */
    async execute(context: MethodContext): Promise<JsonValue> {
        // run a review's call as its requester through its actors, with the subjects who approved it
        const approvals = await this.#approvals(ChangeRequest.#reader(context));
        const approved = approvals.filter((row) => row.state === "approved");
        let caller: AccessContext;
        if (this.row.asId === null) {
            const subject = ObjectReference.at("requester", this.row);
            const chain = { subject, actors: this.row.actors };
            const acting = await Grant.chain(ChangeRequest.#reader(context), chain, context.now);
            const approvers = approved.map((row) => Approval.decider(row));
            caller = AccessContext.of(acting, context.now, { approvers });
        }
        // run the call as the approver it names, acting directly, refusing approvals added since
        else {
            const [decided, ...added] = approved;
            if (
                decided === undefined ||
                added.length > 0 ||
                approvals.length > 1 ||
                !Subject.same(Approval.asked(decided), ChangeRequest.as(this.row))
            ) {
                throw ObjectError.of(
                    "APPROVERS_INVALID",
                    "approver set is invalid: a change run as an approver asks exactly that approver",
                );
            }
            const credential = Credential.approval(
                decided.id,
                decided.assurance ?? undefined,
                changeRequest.package.id,
                context.now,
            );
            caller = AccessContext.of(Caller.of(Approval.decider(decided)), context.now, {
                credential,
            });
        }

        // make the call and answer its output
        const [output] = await context.mutate(caller, [Call.parse(this.row.call)]);

        return schema.json().parse(output ?? null);
    }

    /** Decide on the approvals decided by others than the acting principal. */
    #decide(approvals: readonly Approval[]): ApprovalState {
        const acting = this.row.actors.at(-1) ?? ObjectReference.at("requester", this.row);
        const decisions = approvals.filter(
            (row) => row.state === "requested" || !Subject.same(acting, Approval.decider(row)),
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
