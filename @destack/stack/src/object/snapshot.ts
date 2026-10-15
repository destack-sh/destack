import { permission, principal, relation } from "@destack/access";
import { type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { ResourceId } from "@destack/resource";
import { Digest, schema } from "@destack/schema";

/** A copy of a resource's content at a point in time in its machine's content store. */
export const snapshot = defineObject({
    name: "snapshot",
    plural: "snapshots",
    scope: "universe",
    fields: {
        /** The space keeping the resource. */
        space: field.reference(principal.space),
        /** The resource whose content the snapshot copies. */
        resourceId: field.string(ResourceId),
        /** The recipient the content's machine-bound values are encrypted to. */
        recipient: field.string(),

        // the replica the controller took
        /** The digest of the replica in the machine's content store, absent until taken. */
        digest: field.string(Digest).optional(),
        /** The time the replica was taken, in UTC epoch milliseconds. */
        takenAt: field.time().optional(),
        /** The digests of the store's blobs the replica reads. */
        blobs: field.json(schema.array(Digest)).default([]),
    },
    // the space a record serves reads and writes it
    permissions: { read: permission("record"), record: relation("space") },
    constraints: {
        recipient: { kind: "unique", on: ["resourceId", "recipient"] },
        taken: {
            kind: "check",
            condition: {
                OR: [
                    { digest: { isNull: true }, takenAt: { isNull: true } },
                    { digest: { isNotNull: true }, takenAt: { isNotNull: true } },
                ],
            },
        },
    },
}).methods((method) => ({
    create: method.create("record", {
        isInternal: true,
        fields: ["spaceId", "resourceId", "recipient"],
    }),
    update: method.update("record", {
        isInternal: true,
        fields: ["digest", "takenAt", "blobs"],
    }),
    delete: method.delete("record", { isInternal: true }),
}));

/** A restore of a snapshot's content into a resource provisioned here. */
export const restore = defineObject({
    name: "restore",
    plural: "restores",
    scope: "universe",
    fields: {
        /** The space keeping the resource. */
        space: field.reference(principal.space),
        /** The resource whose content the restore receives. */
        resourceId: field.string(ResourceId),
        /** The machine whose content store keeps the snapshot. */
        source: field.reference(principal.machine),
        /** The epoch the source served the space at when the restore was requested. */
        sourceEpoch: field.integer(),

        // the resource the controller restored
        /** The digest of the restored replica in the source's content store. */
        digest: field.string(Digest).optional(),
        /** The provider keeping the restored resource here. */
        provider: field.string().optional(),
        /** The provider's reference of the restored resource. */
        reference: field.string().optional(),
        /** The provider's location of the restored resource. */
        location: field.string().optional(),
        /** The specification the resource was provisioned at. */
        spec: field.json(schema.json()).optional(),
        /** The time the content was restored, in UTC epoch milliseconds. */
        restoredAt: field.time().optional(),

        // the catch-up from the fenced source
        /** The time the source fenced the space. */
        fencedAt: field.time().optional(),
        /** The time the content caught up with the fenced source, in UTC epoch milliseconds. */
        caughtUpAt: field.time().optional(),
    },
    // the space a record serves reads and writes it
    permissions: { read: permission("record"), record: relation("space") },
    constraints: {
        source: { kind: "unique", on: ["resourceId", "sourceId", "sourceEpoch"] },
        restored: {
            kind: "check",
            condition: {
                OR: [
                    {
                        restoredAt: { isNull: true },
                        digest: { isNull: true },
                        provider: { isNull: true },
                        reference: { isNull: true },
                        spec: { isNull: true },
                    },
                    {
                        restoredAt: { isNotNull: true },
                        digest: { isNotNull: true },
                        provider: { isNotNull: true },
                        reference: { isNotNull: true },
                        spec: { isNotNull: true },
                    },
                ],
            },
        },
        caughtUp: {
            kind: "check",
            condition: {
                OR: [
                    { caughtUpAt: { isNull: true } },
                    {
                        fencedAt: { isNotNull: true },
                        restoredAt: { isNotNull: true },
                        caughtUpAt: { gte: { field: "restoredAt" } },
                    },
                ],
            },
        },
    },
}).methods((method) => ({
    create: method.create("record", {
        isInternal: true,
        fields: ["spaceId", "resourceId", "sourceId", "sourceEpoch", "fencedAt"],
    }),
    update: method.update("record", {
        isInternal: true,
        fields: [
            "digest",
            "provider",
            "reference",
            "location",
            "spec",
            "restoredAt",
            "fencedAt",
            "caughtUpAt",
        ],
    }),
}));

/** A resource snapshot. */
export type Snapshot = Select<typeof snapshot.table>;
/** A resource restore. */
export type Restore = Select<typeof restore.table>;

/** The resource a restore restored: the snapshot it read and where the provider keeps it here. */
export interface RestoredResource {
    /** The digest of the restored snapshot. */
    readonly digest: string;
    /** The provider keeping the restored resource here. */
    readonly provider: string;
    /** The provider's reference of the restored resource. */
    readonly reference: string;
    /** The provider's location of the restored resource, absent for a provider without one. */
    readonly location: string | null;
    /** The specification the resource was provisioned at. */
    readonly spec: NonNullable<Restore["spec"]>;
}

/** Reads of restores. */
export const Restore = {
    /** Read a restore's restored replica, failing on one the controller has not restored. */
    restored(row: Restore): RestoredResource {
        const { digest, provider, reference, location, spec } = row;
        if (digest === null || provider === null || reference === null || spec === null) {
            throw new TypeError(`restore ${row.id} is not restored`);
        }

        return { digest, provider, reference, location, spec };
    },
};
