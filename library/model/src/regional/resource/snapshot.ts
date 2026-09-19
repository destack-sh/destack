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
    unique,
} from "@destack/db";
import { resource } from "./resource.ts";

/** An immutable capture of one resource's committed state. */
export const snapshot = table("snapshot", {
    ...recordColumns("snapshot"),
    /** The space retaining the recovery point. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The resource whose committed state was captured. */
    resourceId: identifier("resource_id", "resource").notNull().references(() => resource.id),
    /** The provider retaining the recovery point. */
    providerCode: text("provider").notNull(),
    /** The snapshot storage location. */
    regionId: identifier("region_id", "region"),
    /** The provider's immutable recovery reference. */
    reference: text("reference").notNull(),
    /** The recovery format interpreted by the controller. */
    format: text("format").notNull(),
    /** The committed source position represented by the snapshot. */
    position: text("position").notNull(),
    /** The integrity digest, when supplied by the format. */
    digest: text("digest"),
    /** Completion of the integrity and completeness checks. */
    verifiedAt: integer("verified_at"),
    /** The earliest time retention permits removal. */
    retainUntil: integer("retain_until").notNull(),
    /** The confirmed removal time. */
    deletedAt: integer("deleted_at"),
}, (snapshot) => [
    unique("snapshot_space_id").on(snapshot.spaceId, snapshot.id),
    foreignKey({
        columns: [snapshot.spaceId, snapshot.resourceId],
        foreignColumns: [resource.spaceId, resource.id],
    }).onDelete("restrict"),
    unique("snapshot_resource_id").on(snapshot.resourceId, snapshot.id),
    check(
        "snapshot_retention",
        sql`${snapshot.retainUntil} >= ${snapshot.createdAt} AND (${snapshot.deletedAt} IS NULL OR ${snapshot.deletedAt} >= ${snapshot.retainUntil})`,
    ),
    check(
        "snapshot_verification",
        sql`${snapshot.verifiedAt} IS NULL OR ${snapshot.verifiedAt} >= ${snapshot.createdAt}`,
    ),
]);

/** A retained resource snapshot. */
export type Snapshot = Select<typeof snapshot>;
