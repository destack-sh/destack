import { none, relation, through, union } from "@destack/access";
import { space } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { schema } from "@destack/schema";
import { Call, ObjectReference, Subject } from "@destack/sync";
import { Failure } from "@destack/error";
import { approval } from "./approval.ts";

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
            // the object's approvers, the requester of its delegate's call and the space's managers approve it
            approve: union(
                through("parent", "approve"),
                relation("requester"),
                through("scope", "manage"),
            ),
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
);
