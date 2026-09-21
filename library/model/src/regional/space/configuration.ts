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
import { SpaceDefinition, SpaceSource } from "@destack/space";
import { PackageRelease } from "@destack/package";
import { ResourceName } from "@destack/resource";
import { schema } from "@destack/schema";
import { space } from "./space.ts";

/** The selected source and parameters for a space configuration. */
export const spaceConfiguration = table(
    "space_configuration",
    {
        /** The configured space; source management is optional and selects one source. */
        spaceId: identifier("space_id", "space")
            .primaryKey()
            .notNull()
            .references(() => space.id),
        /** The registered repository, resolved through the account directory. */
        repositoryId: identifier("repository_id", "repository").notNull(),
        /** The tracked branch or tag, including its refs prefix. */
        reference: text("reference").notNull(),
        /** The package directory within the repository. */
        directory: text("directory").notNull(),
        /** The public package entrypoint. */
        entrypoint: text("entrypoint").notNull(),
        /** The exported stack definition or parameterised function. */
        export: text("export").notNull(),
        /** The exported input schema, required when the selected export takes parameters. */
        parameterSchema: text("parameter_schema"),
        /** Explicit inputs validated against the selected stack parameter schema. */
        parameters: json("parameters", schema.record(schema.string(), schema.json())).notNull(),
        /** Incremented when the selected source or parameters change. */
        generation: integer("generation").notNull().default(1),
        /** The fully applied immutable configuration. */
        appliedRevisionId: identifier("applied_revision_id", "stack-revision"),
    },
    (entry) => [
        foreignKey({
            columns: [entry.spaceId, entry.appliedRevisionId],
            foreignColumns: [stackRevision.spaceId, stackRevision.id],
        }).onDelete("restrict"),
        check("space_configuration_generation", sql`${entry.generation} > 0`),
        check(
            "space_configuration_reference",
            sql`${entry.reference} LIKE 'refs/heads/%' OR ${entry.reference} LIKE 'refs/tags/%'`,
        ),
        check(
            "space_configuration_directory",
            sql`length(${entry.directory}) > 0 AND substr(${entry.directory}, 1, 1) <> '/' AND ${entry.directory} <> '..' AND ${entry.directory} NOT LIKE '../%' AND ${entry.directory} NOT LIKE '%/../%' AND ${entry.directory} NOT LIKE '%/..'`,
        ),
        check(
            "space_configuration_export",
            sql`length(${entry.export}) > 0 AND (${entry.entrypoint} = '.' OR ${entry.entrypoint} LIKE './%')`,
        ),
    ],
);

/** An immutable stack evaluation, including the exact source and parameter values. */
export const stackRevision = table(
    "stack_revision",
    {
        /** The immutable configuration identifier. */
        id: identifier("id", "stack-revision").primaryKey().notNull(),
        /** The destination space. */
        spaceId: identifier("space_id", "space")
            .notNull()
            .references((): Column<Identifier<"space">> => space.id),
        /** The source configuration generation evaluated by this build. */
        sourceGeneration: integer("source_generation").notNull(),
        /** The exact committed source or local checkout build. */
        source: json("source", SpaceSource).notNull(),
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
        unique("stack_revision_space_id").on(entry.spaceId, entry.id),
        unique("stack_revision_source_generation").on(
            entry.spaceId,
            entry.id,
            entry.sourceGeneration,
        ),
        check("stack_revision_generation", sql`${entry.sourceGeneration} > 0`),
        check(
            "stack_revision_digest",
            dialectSQL({
                sqlite: sql`length(${entry.digest}) = 64 AND ${entry.digest} NOT GLOB '*[^a-f0-9]*'`,
                postgresql: sql`length(${entry.digest}) = 64 AND (${entry.digest} COLLATE "C") !~ '[^a-f0-9]'`,
            }),
        ),
    ],
);

/** A space's authoritative configuration source. */
export type SpaceConfiguration = Select<typeof spaceConfiguration>;
/** An immutable evaluated stack. */
export type StackRevision = Select<typeof stackRevision>;
