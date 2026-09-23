import {
    check,
    dialectSQL,
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
import { account } from "../account/account.ts";
import { region } from "../host/region.ts";
import { RESIDENCIES } from "../host/residency.ts";
import { reconciliationChecks, reconciliationColumns } from "../../record/index.ts";
import { environment } from "../account/environment.ts";
import { host } from "../host/host.ts";

/** The globally reserved space name and regional database location. */
export const space = table(
    "space",
    {
        ...recordColumns("space"),
        ...reconciliationColumns(),
        /** The jurisdiction required for storage and processing. */
        residency: text("residency", { enum: RESIDENCIES }).notNull(),
        /** The region coordinating this registered space within its residency. */
        regionId: identifier("region_id", "region").notNull(),
        /** The administrative host, absent when the region administers this space. */
        authorityHostId: identifier("authority_host_id", "host"),
        /** The current administration epoch, matched by the authoritative space records. */
        authorityEpoch: integer("authority_epoch").notNull(),
        /** The account owning this space. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references(() => account.id, {
                onDelete: "restrict",
            }),
        /** The account-local address name. */
        name: text("name").notNull(),
        /** The optional account-defined environment. */
        environmentId: identifier("environment_id", "environment"),
    },
    (entry) => [
        foreignKey({
            columns: [entry.accountId, entry.environmentId],
            foreignColumns: [environment.accountId, environment.id],
        }).onDelete("restrict"),
        unique("space_name").on(entry.accountId, entry.name),
        unique("space_account").on(entry.accountId, entry.id),
        foreignKey({
            columns: [entry.accountId, entry.authorityHostId],
            foreignColumns: [host.accountId, host.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [entry.regionId, entry.residency],
            foreignColumns: [region.id, region.residency],
        }).onDelete("restrict"),
        ...reconciliationChecks("space", entry),
        check("space_authority_epoch", sql`${entry.authorityEpoch} > 0`),
        check(
            "space_name_value",
            dialectSQL({
                sqlite: sql`length(${entry.name}) BETWEEN 1 AND 63 AND ${entry.name} NOT GLOB '*[^a-z0-9-]*' AND ${entry.name} NOT LIKE '-%' AND ${entry.name} NOT LIKE '%-'`,
                postgresql: sql`length(${entry.name}) BETWEEN 1 AND 63 AND (${entry.name} COLLATE "C") !~ '[^a-z0-9-]' AND ${entry.name} NOT LIKE '-%' AND ${entry.name} NOT LIKE '%-'`,
            }),
        ),
    ],
);

/** A globally registered space. */
export type Space = Select<typeof space>;
