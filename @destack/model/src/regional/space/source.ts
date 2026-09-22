import {
    check,
    type Column,
    dialectSQL,
    foreignKey,
    type Identifier,
    identifier,
    integer,
    json,
    type Select,
    sql,
    table,
    text,
    unique,
} from "@destack/db";
import { SpaceDefinition, StackSource } from "@destack/space";
import { PackageRelease } from "@destack/package";
import { ResourceName } from "@destack/resource";
import { schema } from "@destack/schema";
import { sourceChecks } from "../../source/index.ts";
import { space } from "./space.ts";

/** The repository export a space follows for its declared records. */
export const spaceSource = table(
    "space_source",
    {
        /** The configured space; source management is optional and selects one source. */
        spaceId: identifier("space_id", "space")
            .primaryKey()
            .notNull()
            .references(() => space.id),
        /** The repository registered in the global database. */
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
        /** The last revision fully applied to the space's records. */
        appliedRevisionId: identifier("applied_revision_id", "space-revision"),
    },
    (entry) => [
        foreignKey({
            columns: [entry.spaceId, entry.appliedRevisionId],
            foreignColumns: [spaceRevision.spaceId, spaceRevision.id],
        }).onDelete("restrict"),
        ...sourceChecks("space_source", entry),
    ],
);

/** An immutable evaluation of a space source. */
export const spaceRevision = table(
    "space_revision",
    {
        /** The retained evaluation. */
        id: identifier("id", "space-revision").primaryKey().notNull(),
        /** The destination space. */
        spaceId: identifier("space_id", "space")
            .notNull()
            .references((): Column<Identifier<"space">> => space.id),
        /** The source selection generation evaluated by this build. */
        sourceGeneration: integer("source_generation").notNull(),
        /** The exact committed source or local checkout build. */
        source: json("source", StackSource).notNull(),
        /** The validated parameters supplied during evaluation. */
        parameters: json("parameters", schema.record(schema.string(), schema.json())).notNull(),
        /** The source-managed objects after composition and parameter evaluation. */
        definition: json("definition", SpaceDefinition).notNull(),
        /** Exact releases selected for each source-managed installation. */
        releases: json("releases", schema.record(ResourceName, PackageRelease)).notNull(),
        /** The SHA-256 digest of the canonical source, parameters, definition, and releases. */
        digest: text("digest").notNull(),
        /** Creation time in UTC epoch milliseconds. */
        createdAt: integer("created_at").notNull(),
    },
    (entry) => [
        unique("space_revision_space_id").on(entry.spaceId, entry.id),
        check("space_revision_generation", sql`${entry.sourceGeneration} > 0`),
        check(
            "space_revision_digest",
            dialectSQL({
                sqlite: sql`length(${entry.digest}) = 64 AND ${entry.digest} NOT GLOB '*[^a-f0-9]*'`,
                postgresql: sql`(${entry.digest} COLLATE "C") ~ '^[a-f0-9]{64}$'`,
            }),
        ),
    ],
);

/** The repository export and arguments followed by a space. */
export type SpaceSource = Select<typeof spaceSource>;
/** An immutable evaluation applied to space records. */
export type SpaceRevision = Select<typeof spaceRevision>;
