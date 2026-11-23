import { through } from "@destack/access";
import { argument, LocaleTag, plural, t } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { graph, PackageId } from "@destack/package";
import { Digest, schema } from "@destack/schema";
import { build } from "./build.ts";
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
    title: t`${plural(argument("count"), { one: "Module", other: "Modules" })}`,
    scope: build,
    storage: "external",
    key: "moniker",
    fields: {
        /** The module's moniker, the same in every build of its manifest. */
        moniker: field.string(graph.Moniker).immutable(),
        /** The package-relative module path. */
        path: field.string(),
        /** The digest of the source bytes. */
        digest: field.string(Digest),
        /** The relationships of the module's code outside its symbols, by kind and target. */
        relationships: field.json(schema.array(Relationship)),
    },
    access: {
        permissions: { read: through("build", "read") },
    },
});

/** One declaration of a build's graph, keyed by its moniker. */
export const declaration = defineObject({
    name: "declaration",
    plural: "declarations",
    title: t`${plural(argument("count"), { one: "Declaration", other: "Declarations" })}`,
    scope: build,
    storage: "external",
    key: "moniker",
    fields: {
        /** The declaration's moniker. */
        moniker: field.string(graph.Moniker).immutable(),
        /** The module declaring it. */
        module: field.reference(module),
        /** The declaration kind, such as `package`. */
        kind: field.string(schema.string().min(1)),
        /** The declaration's name within its kind. */
        name: field.string(schema.string().min(1)),
        /** The code symbol or module it is declared at. */
        enclosing: field.string(graph.Moniker),
        /** The package declaring its kind. */
        kindPackageId: field.string(PackageId),
        /** Whether a module of the package exports its symbol. */
        isExported: field.boolean(),
        /** Its description, parsed by its kind's schema at read. */
        description: field.json(schema.jsonObject()),
        /** Its terms with the digest of each definition, absent without terms. */
        vocabulary: field.json(schema.record(schema.string(), Digest)).optional(),
        /** The relationships to other symbols, by kind and target. */
        relationships: field.json(schema.array(Relationship)),
    },
    access: {
        permissions: { read: through("build", "read") },
    },
});

/** One symbol of a build's graph, keyed by its moniker. */
export const symbol = defineObject({
    name: "symbol",
    plural: "symbols",
    title: t`${plural(argument("count"), { one: "Symbol", other: "Symbols" })}`,
    scope: build,
    storage: "external",
    key: "moniker",
    fields: {
        /** The symbol's moniker. */
        moniker: field.string(graph.Moniker).immutable(),
        /** The module declaring the symbol. */
        module: field.reference(module),
        /** The qualified code name. */
        name: field.string(schema.string().min(1)),
        /** Whether a module of the package exports the symbol. */
        isExported: field.boolean(),
        /** Its declarations in source order, each with its kind, range and signature. */
        declarations: field.json(schema.array(graph.SymbolDeclaration).min(1)),
        /** The first sentence of the documentation, absent without documentation. */
        comment: field.string().optional(),
        /** The relationships of its code to other symbols, by kind and target. */
        relationships: field.json(schema.array(Relationship)),
    },
    access: {
        permissions: { read: through("build", "read") },
    },
});

/** One package's messages translated into one locale, as a build ships them. */
export const catalog = defineObject({
    name: "catalog",
    plural: "catalogs",
    title: t`${plural(argument("count"), { one: "Catalog", other: "Catalogs" })}`,
    scope: build,
    storage: "external",
    key: "name",
    fields: {
        /** The catalog's name: its package and locale, as `<package>/<locale>`. */
        name: field.string(schema.string().min(1)).immutable(),
        /** The package whose messages the catalog translates. */
        packageId: field.string(PackageId),
        /** The locale the catalog translates into. */
        locale: field.string(LocaleTag),
        /** The translated source of each message, by identifier. */
        messages: field.json(schema.record(schema.string().min(1), schema.string())),
        /** The machine-translated messages no person reviewed yet, absent for none. */
        drafts: field.json(schema.array(schema.string().min(1))).optional(),
    },
    access: {
        permissions: { read: through("build", "read") },
    },
});
