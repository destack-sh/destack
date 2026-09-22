import {
    check,
    dialectSQL,
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

/** The globally reserved repository name and regional database location. */
export const repositoryDirectory = table(
    "repository_directory",
    {
        ...recordColumns("repository"),
        ...reconciliationColumns(),
        /** The jurisdiction required for storage and processing. */
        residency: text("residency", { enum: RESIDENCIES }).notNull(),
        /** The region storing this registration’s private records. */
        regionId: identifier("region_id", "region").notNull(),
        /** The account owning this repository. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references(() => account.id, {
                onDelete: "restrict",
            }),
        /** The account-local address name. */
        name: text("name").notNull(),
    },
    (entry) => [
        unique("repository_directory_name").on(entry.accountId, entry.name),
        unique("repository_directory_account").on(entry.accountId, entry.id),
        foreignKey({
            columns: [entry.regionId, entry.residency],
            foreignColumns: [region.id, region.residency],
        }).onDelete("restrict"),
        ...reconciliationChecks("repository_directory", entry),
        check(
            "repository_directory_name_value",
            dialectSQL({
                sqlite: sql`length(${entry.name}) BETWEEN 1 AND 63 AND ${entry.name} NOT GLOB '*[^a-z0-9-]*' AND ${entry.name} NOT LIKE '-%' AND ${entry.name} NOT LIKE '%-'`,
                postgresql: sql`length(${entry.name}) BETWEEN 1 AND 63 AND (${entry.name} COLLATE "C") !~ '[^a-z0-9-]' AND ${entry.name} NOT LIKE '-%' AND ${entry.name} NOT LIKE '%-'`,
            }),
        ),
    ],
);

/** The global repository registration. */
export type RepositoryDirectory = Select<typeof repositoryDirectory>;
