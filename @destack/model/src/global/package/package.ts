import {
    foreignKey,
    identifier,
    recordColumns,
    type Select,
    table,
    text,
    unique,
} from "@destack/db";
import { repository } from "./repository.ts";

/** A package name reserved globally within its publishing account. */
export const packageTable = table(
    "package",
    {
        ...recordColumns("package"),
        /** The publishing account. */
        accountId: identifier("account_id", "account").notNull(),
        /** The package name within the account scope. */
        name: text("name").notNull(),
        /** The repository authority storing package and release records. */
        repositoryId: identifier("repository_id", "repository").notNull(),
    },
    (entry) => [
        unique("package_name").on(entry.accountId, entry.name),
        foreignKey({
            columns: [entry.accountId, entry.repositoryId],
            foreignColumns: [repository.accountId, repository.id],
        }).onDelete("restrict"),
    ],
);

/** A globally registered package name. */
export type Package = Select<typeof packageTable>;
