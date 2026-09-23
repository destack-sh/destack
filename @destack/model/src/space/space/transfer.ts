import {
    check,
    identifier,
    integer,
    json,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    uniqueIndex,
} from "@destack/db";
import { Subject } from "@destack/access";
import { Conditions } from "../../record/index.ts";
import { SpaceAuthority } from "./authority.ts";
import { space } from "./space.ts";

/** A durable handoff of administrative authority, independent of resource migration. */
export const spaceTransfer = table(
    "space_transfer",
    {
        ...recordColumns("space-transfer"),
        /** The space retaining its identity throughout the handoff. */
        spaceId: identifier("space_id", "space")
            .notNull()
            .references(() => space.id, { onDelete: "restrict" }),
        /** The authority responsible until it durably fences its writes. */
        source: json("source", SpaceAuthority).notNull(),
        /** The authority receiving the complete administrative state. */
        target: json("target", SpaceAuthority).notNull(),
        /** The epoch checked by the source before accepting this transfer. */
        sourceEpoch: integer("source_epoch").notNull(),
        /** The destination epoch, reserved before the source is fenced. */
        targetEpoch: integer("target_epoch").notNull(),
        /** The verified identity authorizing the transfer. */
        requestedBy: json("requested_by", Subject).notNull(),
        /** The final administrative revision exported after source writes stop. */
        sourceRevision: integer("source_revision"),
        /** The checksum of the immutable administrative snapshot. */
        snapshotDigest: text("snapshot_digest"),
        /** The time source administration was durably fenced. */
        fencedAt: integer("fenced_at"),
        /** The time the target confirmed import of the exact snapshot. */
        preparedAt: integer("prepared_at"),
        /** The time target authority was committed. */
        activatedAt: integer("activated_at"),
        /** Terminal outcome; interrupted transfers retain their state for retry. */
        outcome: text("outcome", { enum: ["succeeded", "cancelled"] }),
        /** The completion time in UTC epoch milliseconds. */
        completedAt: integer("completed_at"),
        /** Progress and failures retained while recovery remains required. */
        conditions: json("conditions", Conditions)
            .notNull()
            .default(sql`'{}'`),
    },
    (transfer) => [
        uniqueIndex("space_transfer_active")
            .on(transfer.spaceId)
            .where(sql`${transfer.completedAt} IS NULL`),
        check(
            "space_transfer_epoch",
            sql`${transfer.sourceEpoch} > 0 AND ${transfer.targetEpoch} = ${transfer.sourceEpoch} + 1`,
        ),
        check(
            "space_transfer_snapshot",
            sql`(${transfer.sourceRevision} IS NULL) = (${transfer.snapshotDigest} IS NULL) AND (${transfer.sourceRevision} IS NULL OR (${transfer.sourceRevision} > 0 AND ${transfer.fencedAt} IS NOT NULL))`,
        ),
        check(
            "space_transfer_prepare",
            sql`${transfer.preparedAt} IS NULL OR (${transfer.sourceRevision} IS NOT NULL AND ${transfer.preparedAt} >= ${transfer.fencedAt})`,
        ),
        check(
            "space_transfer_activation",
            sql`${transfer.activatedAt} IS NULL OR (${transfer.preparedAt} IS NOT NULL AND ${transfer.activatedAt} >= ${transfer.preparedAt})`,
        ),
        check(
            "space_transfer_completion",
            sql`(${transfer.outcome} IS NULL) = (${transfer.completedAt} IS NULL)`,
        ),
        check(
            "space_transfer_outcome",
            sql`${transfer.outcome} IS NULL OR (${transfer.outcome} = 'succeeded' AND ${transfer.activatedAt} IS NOT NULL) OR (${transfer.outcome} = 'cancelled' AND ${transfer.fencedAt} IS NULL)`,
        ),
        check(
            "space_transfer_times",
            sql`(${transfer.fencedAt} IS NULL OR ${transfer.fencedAt} >= ${transfer.createdAt}) AND (${transfer.completedAt} IS NULL OR (${transfer.completedAt} >= ${transfer.createdAt} AND (${transfer.activatedAt} IS NULL OR ${transfer.completedAt} >= ${transfer.activatedAt})))`,
        ),
    ],
);

/** A persisted administrative handoff. */
export type SpaceTransfer = Select<typeof spaceTransfer>;
