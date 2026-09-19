import {
    check,
    foreignKey,
    identifier,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { snapshot } from "./snapshot.ts";
import { resource } from "./resource.ts";

/** A recoverable request to restore a snapshot into a separately provisioned resource. */
export const restoration = table("restoration", {
    ...recordColumns("restoration"),
    /** The space authorising the restoration. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The recovery point selected for restoration. */
    snapshotId: identifier("snapshot_id", "snapshot").notNull().references(() => snapshot.id),
    /** The destination resource; the controller verifies it is safe to initialize. */
    destinationResourceId: identifier("destination_resource_id", "resource").notNull().references(
        () => resource.id,
    ),
    /** The first restoration attempt. */
    startedAt: integer("started_at"),
    /** Completion time, absent while pending. */
    completedAt: integer("completed_at"),
    /** The terminal outcome. */
    outcome: text("outcome", { enum: ["succeeded", "failed", "cancelled"] }),
    /** The last reported failure. */
    error: text("error"),
}, (restoration) => [
    foreignKey({
        columns: [restoration.spaceId, restoration.snapshotId],
        foreignColumns: [snapshot.spaceId, snapshot.id],
    }).onDelete("restrict"),
    foreignKey({
        columns: [restoration.spaceId, restoration.destinationResourceId],
        foreignColumns: [resource.spaceId, resource.id],
    }).onDelete("restrict"),
    check(
        "restoration_completion",
        sql`(${restoration.completedAt} IS NULL) = (${restoration.outcome} IS NULL)`,
    ),
    check(
        "restoration_outcome",
        sql`${restoration.outcome} IS NULL OR ${restoration.outcome} IN ('succeeded', 'failed', 'cancelled')`,
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
]);

/** A resource restoration request. */
export type Restoration = Select<typeof restoration>;
