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
import { repository } from "./repository.ts";

/** Published package records. */
export const packageTable = table(
    "package",
    {
        ...recordColumns("package"),
        /** The publishing account. */
        accountId: identifier("account_id", "account").notNull(),
        /** Who can discover and download the package. */
        visibility: text("visibility", { enum: ["public", "unlisted", "private"] }).notNull(),
        /** The repository containing the source. */
        repositoryId: identifier("repository_id", "repository").notNull(),
        /** The package directory relative to the repository root. */
        directory: text("directory").notNull(),
    },
    (packageTable) => [
        unique("package_repository_id").on(packageTable.repositoryId, packageTable.id),
        foreignKey({
            columns: [packageTable.accountId, packageTable.repositoryId],
            foreignColumns: [repository.accountId, repository.id],
        }).onDelete("restrict"),
        check(
            "package_visibility",
            sql`${packageTable.visibility} IN ('public', 'unlisted', 'private')`,
        ),
        check(
            "package_directory",
            sql`length(${packageTable.directory}) > 0 AND substr(${packageTable.directory}, 1, 1) <> '/' AND ${packageTable.directory} NOT LIKE '../%' AND ${packageTable.directory} NOT LIKE '%/../%' AND ${packageTable.directory} <> '..' AND ${packageTable.directory} NOT LIKE '%/..'`,
        ),
    ],
);

/** A persisted package record. */
export type Package = Select<typeof packageTable>;
