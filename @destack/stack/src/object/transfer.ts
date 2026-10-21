import { none, permission, principal, relation, union } from "@destack/access";
import { sudo } from "@destack/account/authentication";
import { LogPosition, type Select } from "@destack/db";
import { Placement } from "@destack/account/object";
import { defineObject, field } from "@destack/object";
import { ResourceId } from "@destack/resource";
import { Digest, schema } from "@destack/schema";
import { space } from "@destack/account/object";

/** Where a source stood once fenced: its log position and the digest of the space's rows there. */
export const Fence = schema.object({
    /** The database's log position once fenced. */
    position: LogPosition,
    /** The digest of the rows of the space and the scopes below it at the position. */
    digest: Digest,
    /** The space and the scopes below it, which the fence keeps from changing. */
    scopes: schema.array(schema.string().min(1)),
});
/** Where a source stood once fenced. */
export type Fence = schema.Infer<typeof Fence>;

/** A move of a space with its rows and resources from the machine serving it to another. */
export const transfer = defineObject(
    {
        name: "transfer",
        plural: "transfers",
        scope: space,
        fields: {
            /** The machine serving the space until the transfer activates. */
            source: field.reference(principal.machine),
            /** The machine receiving the space. */
            target: field.reference(principal.machine),
            /** The epoch the source serves the space at. */
            sourceEpoch: field.integer(),
            /** The verified identity requesting the transfer. */
            requestedBy: field.subject(),
            /** The time the transfer ended, in UTC epoch milliseconds, absent while it runs. */
            completedAt: field.time().optional(),
            /** How the transfer ended: the target took the space over or the source cancelled. */
            outcome: field.enum(["succeeded", "cancelled"]).optional(),
        },
        permissions: {
            read: none(),
            create: none(),
            finish: none(),
            receive: relation("target"),
            cancel: union(relation("source"), permission("create")),
        },
        elevated: { create: sudo },
        moving: ["receive", "cancel"],
        fencing: { by: "cancel" },
        constraints: {
            space: {
                kind: "foreignKey",
                on: ["scope"],
                references: { object: () => space, on: ["id"] },
                onDelete: "no action",
            },
            scopeId: { kind: "unique", on: ["scope", "id"] },
            holder: { kind: "check", condition: { sourceId: { ne: { field: "targetId" } } } },
            active: {
                kind: "index",
                on: ["scope"],
                unique: true,
                where: { completedAt: { isNull: true } },
            },
            epoch: { kind: "check", condition: { sourceEpoch: { gt: 0 } } },
            completion: {
                kind: "check",
                condition: {
                    OR: [
                        { completedAt: { isNull: true }, outcome: { isNull: true } },
                        { completedAt: { isNotNull: true }, outcome: { isNotNull: true } },
                    ],
                },
            },
            times: {
                kind: "check",
                condition: {
                    completedAt: { OR: [{ isNull: true }, { gte: { field: "createdAt" } }] },
                },
            },
        },
    },
    (method) => ({
        create: method.create("create", { fields: ["targetId"], prepared: Placement }),

        /** Fence the space against writes as its target, answering where its database stands. */
        fence: method.mutation({ permission: "receive", prepared: Fence, output: Fence }),

        /** Snapshot a resource of the space for the target's recipient key. */
        snapshot: method.mutation({
            permission: "receive",
            input: schema.object({
                /** The resource to snapshot. */
                resourceId: ResourceId,
                /** The recipient key the machine-bound values are encrypted to. */
                recipient: schema.base64(),
            }),
            output: schema.object({
                /** The snapshot's digest in the source's content store, null until taken. */
                digest: Digest.nullable(),
            }),
        }),

        /** Cancel the transfer as its source machine or one who may create transfers. */
        cancel: method.mutation({ permission: "cancel", prepared: Placement }),

        /** Record as the target a transfer's success once it took the space over. */
        complete: method.mutation({ permission: "finish", isInternal: true }),
    }),
).implement({
    methods: {
        complete: (call) => call.update({ completedAt: call.now, outcome: "succeeded" }),
    },
});

/** A transfer of a space between machines. */
export type Transfer = Select<typeof transfer.table>;
