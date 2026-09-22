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
    uniqueIndex,
} from "@destack/db";

import { spaceMigration } from "../space/migration.ts";
import { space } from "../space/space.ts";
import { Conditions } from "../../record/index.ts";
import { resource } from "./resource.ts";
import { snapshot } from "./snapshot.ts";

/** A resource copy and cutover, optionally coordinated by a space migration. */
export const resourceMigration = table(
    "resource_migration",
    {
        /** The immutable operation identifier. */
        id: identifier("id", "resource-migration").primaryKey().notNull(),
        /** The containing space migration, absent for an independent resource move. */
        spaceMigrationId: identifier("space_migration_id", "migration"),
        /** The account administering the resource and storage hosts. */
        accountId: identifier("account_id", "account").notNull(),
        /** The space containing both the migration and resource. */
        spaceId: identifier("space_id", "space").notNull(),
        /** The resource retaining its identity after the move. */
        resourceId: identifier("resource_id", "resource").notNull(),
        /** The time this operation was requested. */
        createdAt: integer("created_at").notNull(),
        /** The first attempt time. */
        startedAt: integer("started_at"),
        /** The request to stop this operation safely. */
        cancellationRequestedAt: integer("cancellation_requested_at"),
        /** The terminal result, absent while work or recovery remains pending. */
        outcome: text("outcome", { enum: ["succeeded", "failed", "cancelled"] }),
        /** The time the terminal result was recorded. */
        completedAt: integer("completed_at"),
        /** The original storage provider. */
        sourceProviderCode: text("source_provider").notNull(),
        /** The original storage location. */
        sourceReference: text("source_reference").notNull(),
        /** The source storage host, absent for provider-managed storage. */
        sourceHostId: identifier("source_host_id", "host"),
        /** The requested destination provider. */
        targetProviderCode: text("target_provider").notNull(),
        /** The destination storage host, absent for provider-managed storage. */
        targetHostId: identifier("target_host_id", "host"),
        /** The allocated destination, absent before provisioning. */
        targetReference: text("target_reference"),
        /** The provider protocol used to interpret recovery positions. */
        protocol: text("protocol").notNull(),
        /** The immutable snapshot used to initialize the destination. */
        snapshotId: identifier("snapshot_id", "snapshot"),
        /** The final committed source position required at the destination. */
        sourcePosition: text("source_position"),
        /** The committed position durably applied at the destination. */
        targetPosition: text("target_position"),
        /** The time at which the destination passed the protocol's verification. */
        verifiedAt: integer("verified_at"),
        /** The time at which the destination became authoritative. */
        activatedAt: integer("activated_at"),
        /** The time at which explicit retention policy permitted source cleanup. */
        sourceRemovedAt: integer("source_removed_at"),
        /** Storage-specific progress and failure reports. */
        conditions: json("conditions", Conditions)
            .notNull()
            .default(sql`'{}'`),
    },
    (migration) => [
        uniqueIndex("resource_migration_active")
            .on(migration.resourceId)
            .where(sql`${migration.completedAt} IS NULL`),
        index("resource_migration_history").on(migration.resourceId, migration.createdAt),
        index("resource_migration_space_migration").on(migration.spaceMigrationId),
        foreignKey({
            columns: [migration.resourceId, migration.snapshotId],
            foreignColumns: [snapshot.resourceId, snapshot.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [migration.accountId, migration.spaceId],
            foreignColumns: [space.accountId, space.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [migration.spaceId, migration.spaceMigrationId],
            foreignColumns: [spaceMigration.spaceId, spaceMigration.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [migration.spaceId, migration.resourceId],
            foreignColumns: [resource.spaceId, resource.id],
        }).onDelete("restrict"),
        check(
            "resource_migration_outcome",
            sql`${migration.outcome} IS NULL OR ${migration.outcome} IN ('succeeded', 'failed', 'cancelled')`,
        ),
        check(
            "resource_migration_completion",
            sql`(${migration.outcome} IS NULL) = (${migration.completedAt} IS NULL)`,
        ),
        check(
            "resource_migration_success",
            sql`${migration.outcome} IS NULL OR ${migration.outcome} <> 'succeeded' OR ${migration.activatedAt} IS NOT NULL`,
        ),
        check(
            "resource_migration_cancel",
            sql`${migration.outcome} IS NULL OR ${migration.outcome} <> 'cancelled' OR (${migration.activatedAt} IS NULL AND ${migration.cancellationRequestedAt} IS NOT NULL)`,
        ),
        check(
            "resource_migration_time",
            sql`(${migration.startedAt} IS NULL OR ${migration.startedAt} >= ${migration.createdAt}) AND (${migration.completedAt} IS NULL OR (${migration.completedAt} >= ${migration.createdAt} AND (${migration.startedAt} IS NULL OR ${migration.completedAt} >= ${migration.startedAt}) AND (${migration.activatedAt} IS NULL OR ${migration.completedAt} >= ${migration.activatedAt})))`,
        ),
        check(
            "resource_migration_verification",
            sql`${migration.verifiedAt} IS NULL OR (${migration.targetReference} IS NOT NULL AND ${migration.startedAt} IS NOT NULL AND ${migration.verifiedAt} >= ${migration.startedAt})`,
        ),
        check(
            "resource_migration_activation",
            sql`${migration.activatedAt} IS NULL OR (${migration.verifiedAt} IS NOT NULL AND ${migration.activatedAt} >= ${migration.verifiedAt})`,
        ),
        check(
            "resource_migration_cleanup",
            sql`${migration.sourceRemovedAt} IS NULL OR (${migration.activatedAt} IS NOT NULL AND ${migration.sourceRemovedAt} >= ${migration.activatedAt})`,
        ),
    ],
);

/** A persisted resource migration. */
export type ResourceMigration = Select<typeof resourceMigration>;
