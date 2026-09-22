import {
    check,
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
import { reconciliationChecks, reconciliationColumns } from "../../record/index.ts";
import { space } from "./space.ts";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";

/** Installation records. */
export const installation = table(
    "installation",
    {
        ...recordColumns("installation"),
        ...provenanceColumns(),
        /** The space containing the installation. */
        spaceId: identifier("space_id", "space")
            .notNull()
            .references(() => space.id, {
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
        state: text("state", { enum: ["enabled", "suspended"] })
            .notNull()
            .default("enabled"),
        /** The space-local address for this installation. */
        alias: text("alias").notNull(),

        ...reconciliationColumns(),
    },
    (installation) => [
        ...provenanceChecks("installation", installation),
        ...reconciliationChecks("installation", installation),
        unique("installation_space_alias").on(installation.spaceId, installation.alias),
        unique("installation_space_id").on(installation.spaceId, installation.id),
        unique("installation_space_package").on(
            installation.spaceId,
            installation.id,
            installation.packageId,
        ),
        index("installation_release").on(installation.packageId, installation.version),
        check("installation_state", sql`${installation.state} IN ('enabled', 'suspended')`),
        check(
            "installation_applied_generation",
            sql`${installation.appliedGeneration} BETWEEN 0 AND ${installation.generation} AND ((${installation.appliedVersion} IS NULL AND ${installation.appliedGeneration} = 0) OR (${installation.appliedVersion} IS NOT NULL AND ${installation.appliedGeneration} > 0))`,
        ),
    ],
);

/** A persisted installation record. */
export type Installation = Select<typeof installation>;
