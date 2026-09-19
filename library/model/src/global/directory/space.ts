import {
    check,
    foreignKey,
    identifier,
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

/** The globally reserved space name and regional database location. */
export const spaceDirectory = table("space_directory", {
    ...recordColumns("space"),
    ...reconciliationColumns(),
    /** The jurisdiction required for storage and processing. */
    residency: text("residency", { enum: RESIDENCIES }).notNull(),
    /** The region storing this registration’s private records. */
    regionId: identifier("region_id", "region").notNull(),
    /** The account owning this space. */
    accountId: identifier("account_id", "account").notNull().references(() => account.id, {
        onDelete: "restrict",
    }),
    /** The account-local address name. */
    name: text("name").notNull(),
}, (entry) => [
    unique("space_directory_name").on(entry.accountId, entry.name),
    unique("space_directory_account").on(entry.accountId, entry.id),
    foreignKey({
        columns: [entry.regionId, entry.residency],
        foreignColumns: [region.id, region.residency],
    }).onDelete("restrict"),
    ...reconciliationChecks("space_directory", entry),
    check(
        "space_directory_name_value",
        sql`length(${entry.name}) BETWEEN 1 AND 63 AND ${entry.name} NOT GLOB '*[^a-z0-9-]*' AND ${entry.name} NOT LIKE '-%' AND ${entry.name} NOT LIKE '%-'`,
    ),
]);

/** The global space registration. */
export type SpaceDirectory = Select<typeof spaceDirectory>;
