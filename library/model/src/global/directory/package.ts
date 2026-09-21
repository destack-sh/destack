import {
    foreignKey,
    identifier,
    recordColumns,
    type Select,
    table,
    text,
    unique,
} from "@destack/db";
import { repositoryDirectory } from "./repository.ts";

/** A package name reserved globally within its publishing account. */
export const packageDirectory = table(
    "package_directory",
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
        unique("package_directory_name").on(entry.accountId, entry.name),
        foreignKey({
            columns: [entry.accountId, entry.repositoryId],
            foreignColumns: [repositoryDirectory.accountId, repositoryDirectory.id],
        }).onDelete("restrict"),
    ],
);

/** A globally registered package name. */
export type PackageDirectory = Select<typeof packageDirectory>;
