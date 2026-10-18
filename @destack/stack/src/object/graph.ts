import { through } from "@destack/access";
import { Catalog, LocaleTag } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { graph, PackageId } from "@destack/package";
import { SourceRange } from "@destack/package/source";
import { type DatabaseConnection, eq, type Insert, TABLE, type Table } from "@destack/db";
import type { BuildReader } from "@destack/package/manifest";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import { build, Build } from "./build.ts";

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
});

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
        /** The code symbol or module a declaration is declared at, absent for a code symbol. */
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
});

/** One package's messages translated into one locale, as a build ships them. */
export const catalog = defineObject({
    name: "catalog",
    plural: "catalogs",
    scope: build,
    storage: "external",
    key: schema.string().min(1),
    fields: {
        /** The package whose messages the catalog translates. */
        packageId: field.string(PackageId),
        /** The locale the catalog translates into. */
        locale: field.string(LocaleTag),
        /** The translated source of each message, by identifier. */
        messages: field.json(schema.record(schema.string().min(1), schema.string())),
        /** The machine-translated messages no person reviewed yet, absent for none. */
        drafts: field.json(schema.array(schema.string().min(1))).optional(),
    },
    permissions: { read: through("build", "read") },
});

/** A build's view of its manifest's graph index: the build, its manifest and when it finished. */
export class GraphIndex {
    /** The index tables, one per graph object type. */
    static readonly tables = [module.table, symbol.table, catalog.table] as const;

    /** The build reading the index. */
    readonly build: Identifier<"build">;
    /** The space keeping the build. */
    readonly space: Identifier<"space">;
    /** The digest of the build's manifest, which keys the index. */
    readonly manifest: string;
    /** When the build finished, which its rows are made at. */
    readonly finishedAt: number;

    /** Keep a build's place in its manifest's index. */
    constructor(fields: Pick<GraphIndex, "build" | "space" | "manifest" | "finishedAt">) {
        // keep the build, its space, its manifest and its finish
        this.build = fields.build;
        this.space = fields.space;
        this.manifest = fields.manifest;
        this.finishedAt = fields.finishedAt;
    }

    /** Insert an index's rows table by table, each statement within a storage's parameters. */
    static async write(
        transaction: Pick<DatabaseConnection, "insert">,
        rows: Awaited<ReturnType<GraphIndex["rows"]>>,
        parameters: number,
    ): Promise<void> {
        const tables = [
            [module.table, rows.modules],
            [symbol.table, rows.symbols],
            [catalog.table, rows.catalogs],
        ] as const;
        for (const [table, inserted] of tables) {
            // bind each statement's values, a row taking one parameter per column
            const columns = Object.keys(table[TABLE].columns).length;
            const batch = Math.max(1, Math.floor(parameters / columns));
            for (let start = 0; start < inserted.length; start += batch) {
                await transaction.insert(table).values(inserted.slice(start, start + batch));
            }
        }
    }

    /** Read the succeeded build an index scope names, refusing one without its files. */
    static async of(database: DatabaseConnection, scope: string): Promise<GraphIndex> {
        // read the build's space, manifest and finish
        const id = schema.identifier("build").parse(scope);
        const [found] = await database
            .select({
                id: build.table.id,
                scope: build.table.scope,
                state: build.table.state,
                manifest: build.table.manifest,
                failure: build.table.failure,
                expiredAt: build.table.expiredAt,
                finishedAt: build.table.finishedAt,
            })
            .from(build.table)
            .where(eq(build.table.id, id));
        const record = present(found, `build ${scope}`);
        const finishedAt = present(record.finishedAt, "a succeeded build's finish");

        return new GraphIndex({
            build: id,
            space: record.scope,
            manifest: Build.manifest(record),
            finishedAt,
        });
    }

    /** Derive the module, symbol and catalog rows of the build, stored once for its manifest. */
    async rows(reader: BuildReader): Promise<{
        modules: Insert<typeof module.table>[];
        symbols: Insert<typeof symbol.table>[];
        catalogs: Insert<typeof catalog.table>[];
    }> {
        // derive the objects of every module file
        const root = await reader.graph();
        const packageId = reader.manifest.package.id;
        const modules: Insert<typeof module.table>[] = [];
        const symbols: Insert<typeof symbol.table>[] = [];
        for (const digest of Object.values(root.modules)) {
            const graphFile = await reader.module(digest);
            const indexed = indexModule(graphFile, digest, packageId, this.build, this.finishedAt);
            modules.push(indexed.module);
            symbols.push(...indexed.symbols);
        }

        // keep the catalogs it ships: its own and its unpublished dependencies'
        const record = {
            scope: this.build,
            createdAt: this.finishedAt,
            updatedAt: this.finishedAt,
        };
        const catalogs = (await Catalog.read(reader)).map((shipped) => ({
            ...record,
            id: `${shipped.package}/${shipped.locale}`,
            packageId: shipped.package,
            locale: shipped.locale,
            messages: shipped.messages,
            ...(shipped.drafts === undefined ? {} : { drafts: shipped.drafts }),
        }));

        return { modules, symbols, catalogs };
    }

    /** Select an index table's stored rows as the build's: in its scope, made when it finished. */
    select(table: Table, from: string): string {
        // replace the scope and the times, keeping every other column
        const replaced: Readonly<Record<string, string>> = {
            scope: `'${this.build}'`,
            created_at: String(this.finishedAt),
            updated_at: String(this.finishedAt),
        };
        const selected = Object.values(table[TABLE].columns).map(({ definition: { name } }) => {
            const value = replaced[name] ?? `"${name}"`;

            return `${value} AS "${name}"`;
        });

        return `SELECT ${selected.join(", ")} FROM ${from}`;
    }
}

/** Derive one module file's module object and its symbol objects. */
function indexModule(
    file: graph.Module,
    digest: string,
    packageId: string,
    scope: Identifier<"build">,
    finishedAt: number,
): { module: Insert<typeof module.table>; symbols: Insert<typeof symbol.table>[] } {
    // group the module's edges by source
    const moniker = graph.Moniker.of({ packageId, module: file.path });
    const relationships = Map.groupBy(file.edges, (edge) => edge.from);
    const outgoing = (from: string): Relationship[] =>
        (relationships.get(from) ?? []).map(({ kind, to }) => ({ kind, to }));
    const record = { scope, createdAt: finishedAt, updatedAt: finishedAt };

    // keep the code symbols and the declarations at them
    const symbols = indexSymbols(file, moniker, record, outgoing);

    // refuse an edge from neither the module nor one of its symbols
    const sources = new Set<string>([moniker, ...symbols.map((entry) => entry.id)]);
    const stray = file.edges.find((edge) => !sources.has(edge.from));
    if (stray !== undefined) {
        throw new TypeError(
            `module ${moniker} has an edge from ${stray.from}, which it does not declare`,
        );
    }

    return {
        module: {
            ...record,
            id: moniker,
            path: file.path,
            digest: file.digest,
            file: digest,
            relationships: outgoing(moniker),
        },
        symbols,
    };
}

/** Derive the symbol objects of one module file's code symbols and the declarations at them. */
function indexSymbols(
    file: graph.Module,
    moniker: graph.Moniker,
    record: Pick<Insert<typeof symbol.table>, "scope" | "createdAt" | "updatedAt">,
    outgoing: (from: string) => Relationship[],
): Insert<typeof symbol.table>[] {
    // keep the code symbols
    const exported = new Map(file.symbols.map((entry) => [entry.moniker, entry.isExported]));
    const symbols = file.symbols.map((entry) => ({
        ...record,
        id: entry.moniker,
        moduleId: moniker,
        kind: entry.kind,
        name: entry.moniker.slice(entry.moniker.indexOf("#") + 1),
        isExported: entry.isExported,
        source: entry.source,
        signature: entry.signature,
        comment: entry.comment ?? null,
        relationships: outgoing(entry.moniker),
    }));

    // keep the declarations at the code symbols or the module
    const declarations = file.declarations.map((declaration) => ({
        ...record,
        id: declaration.moniker,
        moduleId: moniker,
        kind: declaration.kind,
        name: declaration.name,
        enclosing: declaration.symbol,
        kindPackageId: declaration.kindPackageId,
        isExported:
            declaration.symbol === moniker
                ? false
                : present(
                      exported.get(declaration.symbol),
                      `the symbol ${declaration.symbol} of ${declaration.moniker}`,
                  ),
        description: declaration.description,
        vocabulary: declaration.vocabulary ?? null,
        relationships: outgoing(declaration.moniker),
    }));

    return [...symbols, ...declarations];
}
