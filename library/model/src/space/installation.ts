import {
    check,
    foreignKey,
    identifier,
    index,
    integer,
    recordColumns,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { release } from "../package/release.ts";
import { reconcileChecks, reconcileColumns } from "../resource/reconcile.ts";
import { space } from "./space.ts";

/** Installation records. */
export const installation = table("installation", {
    ...recordColumns("installation"),
    /** The space containing the installation. */
    spaceId: identifier("space_id", "space").notNull().references(() => space.id, {
        onDelete: "restrict",
    }),
    /** The installed package. */
    packageId: identifier("package_id", "package").notNull(),
    /** The selected immutable release. */
    version: text("version").notNull(),
    /** The release currently active, absent before activation. */
    appliedVersion: text("applied_version"),
    /** The configuration generation currently active. */
    appliedGeneration: integer("applied_generation").notNull().default(0),
    /** Whether the installation should serve requests. */
    state: text("state", { enum: ["enabled", "suspended"] }).notNull().default("enabled"),
    /** The space-local address for this installation. */
    alias: text("alias").notNull(),

    ...reconcileColumns(),
}, (installation) => [
    ...reconcileChecks("installation", installation),
    unique("installation_space_alias").on(installation.spaceId, installation.alias),
    unique("installation_space_id").on(installation.spaceId, installation.id),
    foreignKey({
        columns: [installation.packageId, installation.version],
        foreignColumns: [release.packageId, release.version],
    }).onDelete("restrict"),
    index("installation_release").on(installation.packageId, installation.version),
    foreignKey({
        columns: [installation.packageId, installation.appliedVersion],
        foreignColumns: [release.packageId, release.version],
    }).onDelete("restrict"),
    check("installation_state", sql`${installation.state} IN ('enabled', 'suspended')`),
    check(
        "installation_applied_generation",
        sql`${installation.appliedGeneration} BETWEEN 0 AND ${installation.generation} AND ((${installation.appliedVersion} IS NULL AND ${installation.appliedGeneration} = 0) OR (${installation.appliedVersion} IS NOT NULL AND ${installation.appliedGeneration} > 0))`,
    ),
]);

/** A persisted installation record. */
export type Installation = Select<typeof installation>;
