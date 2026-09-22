import {
    check,
    type Column,
    dialectSQL,
    foreignKey,
    identifier,
    type Identifier,
    integer,
    json,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { StackSource } from "@destack/space";
import { schema } from "@destack/schema";
import { AccountDefinition } from "../../declare/account.ts";
import { sourceChecks } from "../../source/index.ts";
import { account } from "./account.ts";
import { repository } from "../package/repository.ts";

/** The repository export an account follows for its declared records. */
export const accountSource = table(
    "account_source",
    {
        /** The configured account. */
        accountId: identifier("account_id", "account")
            .primaryKey()
            .notNull()
            .references(() => account.id),
        /** The source repository registered to this account. */
        repositoryId: identifier("repository_id", "repository").notNull(),
        /** The complete branch or tag reference. */
        reference: text("reference").notNull(),
        /** The package directory within the repository. */
        directory: text("directory").notNull(),
        /** The public package entrypoint. */
        entrypoint: text("entrypoint").notNull(),
        /** The exported definition or parameterised function. */
        export: text("export").notNull(),
        /** Arguments supplied to the selected definition function. */
        parameters: json("parameters", schema.record(schema.string(), schema.json())).notNull(),
        /** Incremented when the source selection or arguments change. */
        generation: integer("generation").notNull().default(1),
        /** The last revision fully applied to the account's records. */
        appliedRevisionId: identifier("applied_revision_id", "account-revision"),
    },
    (source) => [
        foreignKey({
            columns: [source.accountId, source.repositoryId],
            foreignColumns: [repository.accountId, repository.id],
        }).onDelete("restrict"),
        foreignKey({
            columns: [source.accountId, source.appliedRevisionId],
            foreignColumns: [accountRevision.accountId, accountRevision.id],
        }).onDelete("restrict"),
        ...sourceChecks("account_source", source),
    ],
);

/** An immutable evaluation of an account source. */
export const accountRevision = table(
    "account_revision",
    {
        /** The retained evaluation. */
        id: identifier("id", "account-revision").primaryKey().notNull(),
        /** The destination account. */
        accountId: identifier("account_id", "account")
            .notNull()
            .references((): Column<Identifier<"account">> => account.id),
        /** The source generation evaluated by this build. */
        sourceGeneration: integer("source_generation").notNull(),
        /** The exact committed source or retained checkout. */
        source: json("source", StackSource).notNull(),
        /** The validated inputs supplied during evaluation. */
        parameters: json("parameters", schema.record(schema.string(), schema.json())).notNull(),
        /** The declared account records. */
        definition: json("definition", AccountDefinition).notNull(),
        /** The SHA-256 digest of the canonical source, parameters, and definition. */
        digest: text("digest").notNull(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
    },
    (revision) => [
        unique("account_revision_account_id").on(revision.accountId, revision.id),
        check("account_revision_generation", sql`${revision.sourceGeneration} > 0`),
        check(
            "account_revision_digest",
            dialectSQL({
                sqlite: sql`length(${revision.digest}) = 64 AND ${revision.digest} NOT GLOB '*[^a-f0-9]*'`,
                postgresql: sql`(${revision.digest} COLLATE "C") ~ '^[a-f0-9]{64}$'`,
            }),
        ),
    ],
);

/** The repository export and arguments followed by an account. */
export type AccountSource = Select<typeof accountSource>;
/** An immutable evaluation applied to account records. */
export type AccountRevision = Select<typeof accountRevision>;
