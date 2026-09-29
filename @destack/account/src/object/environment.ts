import { check, dialectSQL, foreignKey, sql, unique, type Select } from "@destack/db";
import { defineObject, field } from "@destack/object";
import { EnvironmentDefinition } from "../declare/account.ts";
import { account } from "./account.ts";

/** An account-defined classification its spaces share, declared by the account's stack. */
export const environment = defineObject({
    name: "environment",
    tier: "global",
    plural: "environments",
    scope: account,
    declarable: { schema: EnvironmentDefinition },
    fields: {
        /** The account-local environment name. */
        name: field.string(),
    },
    constraints: (entry) => [
        foreignKey({ columns: [entry.scope], foreignColumns: [account.table.id] }).onDelete(
            "restrict",
        ),
        unique("environment_scope_name").on(entry.scope, entry.name),
        unique("environment_scope_id").on(entry.scope, entry.id),
        check(
            "environment_name",
            dialectSQL({
                sqlite: sql`length(${entry.name}) > 0 AND substr(${entry.name}, 1, 1) GLOB '[a-z]' AND ${entry.name} NOT GLOB '*[^a-z0-9-]*'`,
                postgresql: sql`(${entry.name} COLLATE "C") ~ '^[a-z][a-z0-9-]*$' AND (${entry.name} COLLATE "C") !~ '[^a-z0-9-]'`,
            }),
        ),
    ],
    permissions: ["read"],
});
/** A persisted account environment. */
export type Environment = Select<typeof environment.table>;
