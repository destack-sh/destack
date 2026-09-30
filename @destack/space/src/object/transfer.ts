import { sudo } from "@destack/account/object";
import { check, foreignKey, sql, unique, uniqueIndex, type Select } from "@destack/db";
import { Subject } from "@destack/access";
import { defineObject, field, method, type ObjectType } from "@destack/object";
import { identifier } from "@destack/schema";
import { resource } from "./resource.ts";
import { space } from "./space.ts";

/** Record a transfer's success once its target took the space over, as the target does. */
const complete = method({ permission: null, isSystem: true }).handle((call) =>
    call.revise({ completedAt: call.now }),
);

/** A handoff of a space with its rows and resources from the host or region serving it to another. */
export const transfer = defineObject({
    name: "transfer",
    plural: "transfers",
    scope: space,
    fields: {
        /** The host or region serving the space until the transfer activates. */
        source: field.string(),
        /** The host or region receiving the space. */
        target: field.string(),
        /** The epoch the source serves the space at. */
        sourceEpoch: field.integer(),
        /** The verified identity requesting the transfer. */
        requestedBy: field.json(Subject),
        /** The time the target took the space over, in UTC epoch milliseconds, absent while the transfer runs. */
        completedAt: field.time().optional(),
    },
    constraints: (transfer) => [
        foreignKey({ columns: [transfer.scope], foreignColumns: [space.table.id] }).onDelete(
            "restrict",
        ),
        unique("transfer_scope_id").on(transfer.scope, transfer.id),
        check("transfer_holder", sql`${transfer.source} <> ${transfer.target}`),
        uniqueIndex("transfer_active")
            .on(transfer.scope)
            .where(sql`${transfer.completedAt} IS NULL`),
        check("transfer_epoch", sql`${transfer.sourceEpoch} > 0`),
        check(
            "transfer_times",
            sql`${transfer.completedAt} IS NULL OR ${transfer.completedAt} >= ${transfer.createdAt}`,
        ),
    ],
    permissions: ["read", "create"],
    elevated: { create: sudo },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", { fields: ["target"] }),
        complete,
    },
});

/** The part of a transfer moving one resource: where its provider kept it on the source, and where the target's provider keeps it now. */
export const resourceTransfer = defineObject({
    name: "resource-transfer",
    plural: "resourceTransfers",
    scope: space,
    fields: {
        /** The transfer moving the resource. */
        transferId: field.reference<"transfer">((): ObjectType => transfer),
        /** The resource. */
        resourceId: field.reference<"resource">((): ObjectType => resource),
        /** The provider keeping the resource on the source. */
        sourceProviderCode: field.string(),
        /** The source provider's reference. */
        sourceReference: field.string(),
        /** The source host, absent for provider-managed storage. */
        sourceHostId: field.string(identifier("host")).optional(),
        /** The provider keeping the resource on the target. */
        targetProviderCode: field.string(),
        /** The target provider's reference. */
        targetReference: field.string(),
        /** The target host, absent for provider-managed storage. */
        targetHostId: field.string(identifier("host")).optional(),
        /** The time the target's provider had all of the fenced source's content. */
        copiedAt: field.time(),
    },
    constraints: (entry) => [
        unique("resource_transfer_resource").on(entry.transferId, entry.resourceId),
        foreignKey({ columns: [entry.scope], foreignColumns: [space.table.id] }).onDelete(
            "restrict",
        ),
        foreignKey({
            columns: [entry.scope, entry.transferId],
            foreignColumns: [transfer.table.scope, transfer.table.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [entry.scope, entry.resourceId],
            foreignColumns: [resource.table.scope, resource.table.id],
        }).onDelete("cascade"),
    ],
    permissions: ["read"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create(null, { isSystem: true }),
    },
});

/** A persisted administrative handoff. */
export type Transfer = Select<typeof transfer.table>;
/** A persisted part of a handoff moving one resource. */
export type ResourceTransfer = Select<typeof resourceTransfer.table>;
