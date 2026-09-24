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
    "repository_ref",
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
    (reference) => [
        primaryKey({ columns: [reference.repositoryId, reference.name] }),
        check("repository_ref_name", sql`${reference.name} LIKE 'refs/%'`),
        check(
            "repository_ref_object",
            dialectSQL({
                sqlite: sql`length(${reference.object}) IN (40, 64) AND ${reference.object} NOT GLOB '*[^0-9a-f]*'`,
                postgresql: sql`length(${reference.object}) IN (40, 64) AND (${reference.object} COLLATE "C") !~ '[^0-9a-f]'`,
            }),
        ),
        check(
            "repository_ref_commit",
            dialectSQL({
                sqlite: sql`${reference.commit} IS NULL OR (length(${reference.commit}) IN (40, 64) AND ${reference.commit} NOT GLOB '*[^0-9a-f]*')`,
                postgresql: sql`${reference.commit} IS NULL OR (length(${reference.commit}) IN (40, 64) AND (${reference.commit} COLLATE "C") !~ '[^0-9a-f]')`,
            }),
        ),
        check("repository_ref_revision", sql`${reference.revision} >= 1`),
    ],
);

/** An indexed Git ref. */
export type RepositoryReference = Select<typeof repositoryReference>;
