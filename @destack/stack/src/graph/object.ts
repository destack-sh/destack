import { through } from "@destack/access";
import { defineObject, field } from "@destack/object";
import { graph, PackageId } from "@destack/package";
import { SourceRange } from "@destack/package/source";
import { Digest, schema } from "@destack/schema";
import { build } from "../object/index.ts";

/** A relationship of a symbol to another, after SCIP's `Relationship`, resolved to its moniker. */
export const Relationship = schema.object({
    /** The relation. */
    kind: schema.enum(graph.EDGE_KINDS),
    /** The symbol the relationship ends at, in any build. */
    to: graph.Moniker,
});
/** A relationship of a symbol to another. */
export type Relationship = schema.Infer<typeof Relationship>;

/** One module of a build's graph, keyed by its moniker. */
export const module = defineObject({
    name: "module",
    plural: "modules",
    scope: build,
    storage: "external",
    key: graph.Moniker,
    fields: {
        /** The package-relative module path. */
        path: field.string(),
        /** The digest of the source bytes. */
        digest: field.string(Digest),
        /** The digest of the module's graph file. */
        file: field.string(Digest),
        /** The relationships of the module's code outside its symbols, by kind and target. */
        relationships: field.json(schema.array(Relationship)),
    },
    permissions: { read: through("build", "read") },
}).methods();

/** One symbol of a build's graph, keyed by its moniker. */
export const symbol = defineObject({
    name: "symbol",
    plural: "symbols",
    scope: build,
    storage: "external",
    key: graph.Moniker,
    fields: {
        /** The module declaring the symbol. */
        module: field.reference(module),
        /** The kind: a code kind such as `function`, or a declaration kind such as `package`. */
        kind: field.string(schema.string().min(1)),
        /** The name: the qualified code name, or the declaration's name within its kind. */
        name: field.string(schema.string().min(1)),
        /** The code symbol a declaration is declared at, absent for a code symbol. */
        enclosing: field.string(graph.Moniker).optional(),
        /** The package declaring a declaration's kind, absent for a code symbol. */
        kindPackageId: field.string(PackageId).optional(),
        /** Whether a module of the package exports the symbol. */
        isExported: field.boolean(),
        /** The code symbol's first source range, absent for a declaration. */
        source: field.json(SourceRange).optional(),
        /** The code symbol's printed declaration, absent for a declaration. */
        signature: field.string().optional(),
        /** The first sentence of the documentation, absent without documentation. */
        comment: field.string().optional(),
        /** A declaration's description, parsed by its kind's schema at read. */
        description: field.json(schema.jsonObject()).optional(),
        /** A declaration's terms with the digest of each definition, absent without terms. */
        vocabulary: field.json(schema.record(schema.string(), Digest)).optional(),
        /** The relationships to other symbols, by kind and target. */
        relationships: field.json(schema.array(Relationship)),
    },
    permissions: { read: through("build", "read") },
}).methods();
