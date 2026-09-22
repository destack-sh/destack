import {
    check,
    dialectSQL,
    identifier,
    recordColumns,
    sql,
    table,
    text,
    unique,
    type Select,
} from "@destack/db";
import { provenanceChecks, provenanceColumns } from "../../source/index.ts";
import { account } from "./account.ts";

/** An account-defined classification shared by its spaces. */
export const environment = table(
    "environment",
    {
        ...recordColumns("environment"),
        ...provenanceColumns(),
        /** The account defining this environment. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references(() => account.id, { onDelete: "restrict" }),
        /** The account-local environment name. */
        name: text("name").notNull(),
    },
    (entry) => [
        unique("environment_account_name").on(entry.accountId, entry.name),
        unique("environment_account_id").on(entry.accountId, entry.id),
        check(
            "environment_name",
            dialectSQL({
                sqlite: sql`length(${entry.name}) > 0 AND substr(${entry.name}, 1, 1) GLOB '[a-z]' AND ${entry.name} NOT GLOB '*[^a-z0-9-]*'`,
                postgresql: sql`(${entry.name} COLLATE "C") ~ '^[a-z][a-z0-9-]*$' AND (${entry.name} COLLATE "C") !~ '[^a-z0-9-]'`,
            }),
        ),
        ...provenanceChecks("environment", entry),
    ],
);

/** A persisted account environment. */
export type Environment = Select<typeof environment>;
