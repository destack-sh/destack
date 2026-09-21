import {
    check,
    dialectSQL,
    identifier,
    integer,
    primaryKey,
    type Select,
    sql,
    table,
    text,
} from "@destack/db";
import { repository } from "./repository.ts";

/** A Git ref last observed during repository reconciliation. */
export const repositoryReference = table(
    "repository_reference",
    {
        /** The repository containing this ref. */
        repositoryId: identifier("repository_id", "repository")
            .notNull()
            .references(() => repository.id, { onDelete: "cascade" }),
        /** The complete Git ref name, including refs/heads or refs/tags. */
        name: text("name").notNull(),
        /** The object directly named by the ref. */
        object: text("object").notNull(),
        /** The resolved commit, absent for refs that do not resolve to a commit. */
        commit: text("commit"),
        /** The local observation revision; Git object identifiers are not ordered. */
        revision: integer("revision").notNull().default(1),
        /** The last successful authoritative observation. */
        observedAt: integer("observed_at").notNull(),
        /** The time a complete ref listing confirmed deletion. */
        deletedAt: integer("deleted_at"),
    },
    (ref) => [
        primaryKey({ columns: [ref.repositoryId, ref.name] }),
        check("repository_reference_name", sql`${ref.name} LIKE 'refs/%'`),
        check(
            "repository_reference_object",
            dialectSQL({
                sqlite: sql`length(${ref.object}) IN (40, 64) AND ${ref.object} NOT GLOB '*[^0-9a-f]*'`,
                postgresql: sql`length(${ref.object}) IN (40, 64) AND (${ref.object} COLLATE "C") !~ '[^0-9a-f]'`,
            }),
        ),
        check(
            "repository_reference_commit",
            dialectSQL({
                sqlite: sql`${ref.commit} IS NULL OR (length(${ref.commit}) IN (40, 64) AND ${ref.commit} NOT GLOB '*[^0-9a-f]*')`,
                postgresql: sql`${ref.commit} IS NULL OR (length(${ref.commit}) IN (40, 64) AND (${ref.commit} COLLATE "C") !~ '[^0-9a-f]')`,
            }),
        ),
        check("repository_reference_revision", sql`${ref.revision} >= 1`),
    ],
);

/** An indexed Git ref. */
export type RepositoryReference = Select<typeof repositoryReference>;
