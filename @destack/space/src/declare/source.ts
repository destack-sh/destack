import { defineSchema, identifier, schema } from "@destack/schema";
import { Digest, PackagePath } from "@destack/package/file";
import { Entrypoint } from "@destack/package/workload";

/** The package export and build used to evaluate a configuration. */
const SOURCE = schema.object({
    /** The registered source repository. */
    repository: identifier("repository"),
    /** The source package directory, or the repository root. */
    directory: schema.union([schema.literal("."), PackagePath]),
    /** The public package entrypoint containing the definition. */
    entrypoint: Entrypoint,
    /** The exported configuration definition or parameterised definition function. */
    export: schema.string().min(1),
    /** The exported input schema, absent for an unparameterised definition. */
    parameterSchema: schema.string().min(1).optional(),
    /** The immutable manifest digest covering source, dependencies, and outputs. */
    build: Digest,
});

/** A committed source or an exact local checkout snapshot. */
export const SpaceSource = defineSchema(
    schema.union([
        SOURCE.extend({
            /** Source retained in repository history. */
            kind: schema.literal("commit"),
            /** The complete Git commit object identifier. */
            commit: schema.string().regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/),
        }),
        SOURCE.extend({
            /** Unpublished source retained by the local build. */
            kind: schema.literal("checkout"),
            /** The registered working directory. */
            checkout: identifier("checkout"),
            /** The parent commit, absent for an unborn branch. */
            commit: schema
                .string()
                .regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/)
                .optional(),
        }),
    ]),
);
/** Exact source used to evaluate a configuration. */
export type SpaceSource = schema.Infer<typeof SpaceSource>;
