import { defineSchema, identifier, schema } from "@destack/schema";
import { Digest, PackagePath } from "@destack/package/file";
import { Entrypoint } from "@destack/package/workload";

/** The package export and build used to evaluate a configuration. */
const SOURCE = schema.object({
    /** The source package directory, or the repository root. */
    directory: schema.union([schema.literal("."), PackagePath]),
    /** The public package entrypoint containing the definition. */
    entrypoint: Entrypoint,
    /** The exported configuration definition or parameterised definition function. */
    export: schema.string().min(1),
    /** The immutable manifest digest covering source, dependencies, and outputs. */
    build: Digest,
});

/** A repository ref or a working directory followed for configuration changes. */
export const StackSelection = defineSchema(
    schema.discriminatedUnion("kind", [
        schema.object({
            /** Follow committed repository history. */
            kind: schema.literal("repository"),
            /** The registered repository. */
            repository: identifier("repository"),
            /** The full branch or tag ref. */
            reference: schema.string().regex(/^refs\/(heads|tags)\/.+$/),
        }),
        schema.object({
            /** Follow a local working directory, including unpublished changes. */
            kind: schema.literal("checkout"),
            /** The host administering the checkout. */
            host: identifier("host"),
            /** The checkout registered on that host. */
            checkout: identifier("checkout"),
        }),
    ]),
);
/** A configuration source selection. */
export type StackSelection = schema.Infer<typeof StackSelection>;

/** A committed source or an exact local checkout snapshot. */
export const StackSource = defineSchema(
    schema.union([
        SOURCE.extend({
            /** Source retained in repository history. */
            kind: schema.literal("commit"),
            /** The registered repository retaining the commit. */
            repository: identifier("repository"),
            /** The complete Git commit object identifier. */
            commit: schema.string().regex(/^(?:[a-f0-9]{40}|[a-f0-9]{64})$/),
        }),
        SOURCE.extend({
            /** Unpublished source retained by the local build. */
            kind: schema.literal("checkout"),
            /** The host retaining the checkout and build snapshot. */
            host: identifier("host"),
            /** The associated global repository, absent for standalone checkouts. */
            repository: identifier("repository").optional(),
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
export type StackSource = schema.Infer<typeof StackSource>;
