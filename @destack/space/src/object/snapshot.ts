import { check, foreignKey, sql, unique, type Select } from "@destack/db";
import { defineObject, field, method, type ObjectType } from "@destack/object";
import { resource } from "./resource.ts";
import { space } from "./space.ts";

/** A retained recovery point of a resource's committed state. */
export const snapshot = defineObject({
    name: "snapshot",
    plural: "snapshots",
    scope: space,
    controlled: true,
    fields: {
        /** The resource the snapshot captures. */
        resourceId: field.reference<"resource">((): ObjectType => resource),
        /** The earliest time retention permits removal. */
        retainUntil: field.time(),

        // the recovery point the controller records after taking it
        /** The time the recovery point became usable, absent while the controller takes it. */
        readyAt: field.time().optional(),
        /** The provider retaining the recovery point. */
        providerCode: field.string().optional(),
        /** The provider location retaining the snapshot. */
        location: field.string().optional(),
        /** The provider's immutable recovery reference. */
        reference: field.string().optional(),
        /** The recovery format interpreted by the controller. */
        format: field.string().optional(),
        /** The committed source position represented by the snapshot. */
        position: field.string().optional(),
        /** The integrity digest, when supplied by the format. */
        digest: field.string().optional(),
        /** Completion of the integrity and completeness checks. */
        verifiedAt: field.time().optional(),
    },
    constraints: (snapshot) => [
        unique("snapshot_scope_id").on(snapshot.scope, snapshot.id),
        foreignKey({
            columns: [snapshot.scope, snapshot.resourceId],
            foreignColumns: [resource.table.scope, resource.table.id],
        }).onDelete("restrict"),
        unique("snapshot_resource_id").on(snapshot.resourceId, snapshot.id),
        check(
            "snapshot_retention",
            sql`${snapshot.retainUntil} >= ${snapshot.createdAt} AND (${snapshot.deletionRequestedAt} IS NULL OR ${snapshot.deletionRequestedAt} >= ${snapshot.retainUntil})`,
        ),
        check(
            "snapshot_ready",
            sql`(${snapshot.readyAt} IS NULL) = (${snapshot.providerCode} IS NULL) AND (${snapshot.readyAt} IS NULL) = (${snapshot.reference} IS NULL) AND (${snapshot.readyAt} IS NULL) = (${snapshot.format} IS NULL) AND (${snapshot.readyAt} IS NULL) = (${snapshot.position} IS NULL) AND (${snapshot.readyAt} IS NOT NULL OR (${snapshot.location} IS NULL AND ${snapshot.digest} IS NULL)) AND (${snapshot.readyAt} IS NULL OR ${snapshot.readyAt} >= ${snapshot.createdAt})`,
        ),
        check(
            "snapshot_verification",
            sql`${snapshot.verifiedAt} IS NULL OR (${snapshot.readyAt} IS NOT NULL AND ${snapshot.verifiedAt} >= ${snapshot.readyAt})`,
        ),
    ],
    permissions: ["read", "create", "delete"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", { fields: ["resourceId", "retainUntil"] }),
        delete: method.delete("delete"),
    },
});

/** A restoration of a snapshot into a resource. */
export const restoration = defineObject({
    name: "restoration",
    plural: "restorations",
    scope: space,
    fields: {
        /** The recovery point selected for restoration. */
        snapshotId: field.reference<"snapshot">((): ObjectType => snapshot),
        /** The destination resource; the controller verifies it is safe to initialize. */
        destinationResourceId: field.reference<"resource">((): ObjectType => resource),
        /** The first restoration attempt. */
        startedAt: field.time().optional(),
        /** Completion time, absent while pending. */
        completedAt: field.time().optional(),
        /** The terminal outcome. */
        outcome: field.enum(["succeeded", "failed", "cancelled"]).optional(),
        /** The last reported failure. */
        error: field.string().optional(),
    },
    constraints: (restoration) => [
        foreignKey({
            columns: [restoration.scope, restoration.snapshotId],
            foreignColumns: [snapshot.table.scope, snapshot.table.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [restoration.scope, restoration.destinationResourceId],
            foreignColumns: [resource.table.scope, resource.table.id],
        }).onDelete("restrict"),
        check(
            "restoration_completion",
            sql`(${restoration.completedAt} IS NULL) = (${restoration.outcome} IS NULL)`,
        ),
        check(
            "restoration_success",
            sql`${restoration.outcome} IS NULL OR ${restoration.outcome} <> 'succeeded' OR (${restoration.startedAt} IS NOT NULL AND ${restoration.error} IS NULL)`,
        ),
        check(
            "restoration_finish",
            sql`${restoration.completedAt} IS NULL OR ${restoration.startedAt} IS NULL OR ${restoration.completedAt} >= ${restoration.startedAt}`,
        ),
        check(
            "restoration_time",
            sql`(${restoration.startedAt} IS NULL OR ${restoration.startedAt} >= ${restoration.createdAt}) AND (${restoration.completedAt} IS NULL OR ${restoration.completedAt} >= ${restoration.createdAt})`,
        ),
    ],
    permissions: ["read", "create"],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("create", { fields: ["snapshotId", "destinationResourceId"] }),
    },
});

/** A retained resource snapshot. */
export type Snapshot = Select<typeof snapshot.table>;
/** A resource restoration. */
export type Restoration = Select<typeof restoration.table>;
