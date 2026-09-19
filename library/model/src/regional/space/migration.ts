import {
    check,
    foreignKey,
    identifier,
    index,
    integer,
    json,
    type Select,
    sql,
    table,
    text,
    unique,
    uniqueIndex,
} from "@destack/db";

import { Conditions } from "../../record/index.ts";
import { space } from "./space.ts";

/** A requested move of a space's execution and selected resources. */
export const spaceMigration = table("space_migration", {
    /** The immutable operation identifier, also used to deduplicate requests. */
    id: identifier("id", "migration").primaryKey().notNull(),
    /** The account authorising the move. */
    accountId: identifier("account_id", "account").notNull(),
    /** The space whose identity and records remain intact. */
    spaceId: identifier("space_id", "space").notNull(),
    /** The desired generation accepted by this operation. */
    generation: integer("generation").notNull(),
    /** The host authorised when the operation was accepted. */
    sourceHostId: identifier("source_host_id", "host").notNull(),
    /** The destination host granted execution authority at cutover. */
    targetHostId: identifier("target_host_id", "host").notNull(),
    /** The source authority epoch checked before cutover. */
    sourceEpoch: integer("source_epoch").notNull(),
    /** The new authority epoch committed at cutover. */
    targetEpoch: integer("target_epoch"),
    /** The requesting user, absent for automatic placement. */
    requestedBy: identifier("requested_by", "user"),
    /** The creation time in UTC epoch milliseconds. */
    createdAt: integer("created_at").notNull(),
    /** The first attempt time. */
    startedAt: integer("started_at"),
    /** The request to stop this operation safely. */
    cancellationRequestedAt: integer("cancellation_requested_at"),
    /** The time at which old execution and write authority were revoked. */
    sourceRevokedAt: integer("source_revoked_at"),
    /** The time the new authority was committed. */
    activatedAt: integer("activated_at"),
    /** The terminal result, absent while work or recovery remains pending. */
    outcome: text("outcome", { enum: ["succeeded", "failed", "cancelled"] }),
    /** The time the terminal result was recorded. */
    completedAt: integer("completed_at"),
    /** Progress and failures reported by the migration coordinator. */
    conditions: json("conditions", Conditions).notNull().default(sql`'{}'`),
}, (migration) => [
    unique("space_migration_space_id").on(migration.spaceId, migration.id),
    uniqueIndex("space_migration_active").on(migration.spaceId).where(
        sql`${migration.completedAt} IS NULL`,
    ),
    index("space_migration_history").on(migration.spaceId, migration.createdAt),
    foreignKey({
        columns: [migration.accountId, migration.spaceId],
        foreignColumns: [space.accountId, space.id],
    }).onDelete("restrict"),
    check("space_migration_generation", sql`${migration.generation} >= 1`),
    check(
        "space_migration_epoch",
        sql`${migration.sourceEpoch} >= 1 AND (${migration.targetEpoch} IS NULL OR ${migration.targetEpoch} > ${migration.sourceEpoch})`,
    ),
    check(
        "space_migration_outcome",
        sql`${migration.outcome} IS NULL OR ${migration.outcome} IN ('succeeded', 'failed', 'cancelled')`,
    ),
    check(
        "space_migration_completion",
        sql`(${migration.outcome} IS NULL) = (${migration.completedAt} IS NULL)`,
    ),
    check(
        "space_migration_activation",
        sql`(${migration.activatedAt} IS NULL) = (${migration.targetEpoch} IS NULL) AND (${migration.activatedAt} IS NULL OR (${migration.sourceRevokedAt} IS NOT NULL AND ${migration.startedAt} IS NOT NULL AND ${migration.activatedAt} >= ${migration.sourceRevokedAt}))`,
    ),
    check(
        "space_migration_success",
        sql`${migration.outcome} IS NULL OR ${migration.outcome} <> 'succeeded' OR ${migration.activatedAt} IS NOT NULL`,
    ),
    check(
        "space_migration_cancel",
        sql`${migration.outcome} IS NULL OR ${migration.outcome} <> 'cancelled' OR (${migration.activatedAt} IS NULL AND ${migration.cancellationRequestedAt} IS NOT NULL)`,
    ),
    check(
        "space_migration_time",
        sql`(${migration.startedAt} IS NULL OR ${migration.startedAt} >= ${migration.createdAt}) AND (${migration.completedAt} IS NULL OR ${migration.completedAt} >= ${migration.createdAt})`,
    ),
    check(
        "space_migration_revoke_time",
        sql`${migration.sourceRevokedAt} IS NULL OR (${migration.startedAt} IS NOT NULL AND ${migration.sourceRevokedAt} >= ${migration.startedAt})`,
    ),
    check(
        "space_migration_complete_time",
        sql`${migration.completedAt} IS NULL OR ((${migration.startedAt} IS NULL OR ${migration.completedAt} >= ${migration.startedAt}) AND (${migration.activatedAt} IS NULL OR ${migration.completedAt} >= ${migration.activatedAt}))`,
    ),
]);

/** A persisted space migration. */
export type SpaceMigration = Select<typeof spaceMigration>;
