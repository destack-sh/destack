import { through } from "@destack/access";
import { Catalog, LocaleTag } from "@destack/locale";
import { defineObject, field } from "@destack/object";
import { graph, PackageId } from "@destack/package";
import { SourceRange } from "@destack/package/source";
import { type DatabaseConnection, eq, type Insert, TABLE, type Table } from "@destack/db";
import { PackageManifest } from "@destack/package/manifest";
import { Digest, type Identifier, present, schema } from "@destack/schema";
import { build, Build } from "./build.ts";
import type { PackageStore } from "@destack/build/store";
import type { PackageFile } from "@destack/package/file";

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

/** One declaration of a build's graph, keyed by its moniker. */
export const declaration = defineObject({
    name: "declaration",
    plural: "declarations",
    scope: build,
    storage: "external",
    key: graph.Moniker,
    fields: {
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
    permissions: { read: through("build", "read") },
});

/** One code symbol of a build's graph, from its code index, keyed by its moniker. */
export const symbol = defineObject({
    name: "symbol",
    plural: "symbols",
    scope: build,
    storage: "external",
    key: graph.Moniker,
    fields: {
        /** The module declaring the symbol. */
        module: field.reference(module),
        /** The code kind, such as `function`. */
        kind: field.string(schema.string().min(1)),
        /** The qualified code name. */
        name: field.string(schema.string().min(1)),
        /** Whether a module of the package exports the symbol. */
        isExported: field.boolean(),
        /** The symbol's first source range. */
        source: field.json(SourceRange),
        /** The symbol's printed declaration. */
        signature: field.string(),
        /** The first sentence of the documentation, absent without documentation. */
        comment: field.string().optional(),
        /** The relationships of its code to other symbols, by kind and target. */
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

/** The path a build's code index is attached to its manifest at. */
const CODE_INDEX_PATH = "code.json";

/** A build's code index: one document per module, attached to its manifest. */
export const CodeIndex = Object.assign(schema.array(graph.Document), {
    /** Describe a stored code index as the file attached to its manifest. */
    file(stored: Pick<PackageFile, "digest" | "size">): PackageFile {
        return { ...stored, path: CODE_INDEX_PATH, mediaType: "application/json" };
    },

    /** Read the code index attached to a build, empty for a release pulled without one. */
    async read(
        packages: Pick<PackageStore, "referrer" | "file">,
        manifest: string,
    ): Promise<readonly graph.Document[]> {
        // read no documents of a build without a code index
        const attached = await packages.referrer(manifest, CODE_INDEX_PATH);
        if (attached === undefined) {
            return [];
        }

        // read and verify the attached file
        const bytes = await packages.file(attached);

        return CodeIndex.parse(JSON.parse(new TextDecoder().decode(bytes)));
    },
});

/** A build's view of its manifest's graph index: the build, its manifest and when it finished. */
export class GraphIndex {
    /** The index tables, one per graph object type. */
    static readonly tables = [
        module.table,
        declaration.table,
        symbol.table,
        catalog.table,
    ] as const;

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
            [declaration.table, rows.declarations],
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

    /** Derive the rows of the build and its code index, stored once for its manifest. */
    async rows(
        packages: Pick<PackageStore, "contains" | "contents" | "referrer" | "file">,
    ): Promise<{
        modules: Insert<typeof module.table>[];
        declarations: Insert<typeof declaration.table>[];
        symbols: Insert<typeof symbol.table>[];
        catalogs: Insert<typeof catalog.table>[];
    }> {
        // read the build and its code index
        const reader = await packages.contents(this.manifest);
        const code = await CodeIndex.read(packages, this.manifest);

        // derive the module and declaration objects of every module file
        const root = await reader.graph();
        const packageId = PackageManifest.package(reader.manifest).id;
        const record = {
            scope: this.build,
            createdAt: this.finishedAt,
            updatedAt: this.finishedAt,
        };
        const modules: Insert<typeof module.table>[] = [];
        const declarations: Insert<typeof declaration.table>[] = [];
        for (const digest of Object.values(root.modules)) {
            const indexed = indexModule(await reader.module(digest), digest, packageId, record);
            modules.push(indexed.module);
            declarations.push(...indexed.declarations);
        }

        // derive the code symbols of the code index
        const symbols = code.flatMap((file) => indexCode(file, packageId, record));

        // keep the catalogs it ships: its own and its unpublished dependencies'
        const catalogs = (await Catalog.read(reader)).map((shipped) => ({
            ...record,
            id: `${shipped.package}/${shipped.locale}`,
            packageId: shipped.package,
            locale: shipped.locale,
            messages: shipped.messages,
            ...(shipped.drafts === undefined ? {} : { drafts: shipped.drafts }),
        }));

        return { modules, declarations, symbols, catalogs };
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

/** The scope and times every row of a build's index has. */
type IndexRecord = Pick<Insert<typeof module.table>, "scope" | "createdAt" | "updatedAt">;

/** Derive one module file's module object and its declaration objects. */
function indexModule(
    file: graph.Module,
    digest: string,
    packageId: string,
    record: IndexRecord,
): { module: Insert<typeof module.table>; declarations: Insert<typeof declaration.table>[] } {
    // group the module's edges by source
    const moniker = graph.Moniker.of({ packageId, module: file.path });
    const relationships = Map.groupBy(file.edges, (edge) => edge.from);
    const outgoing = (from: string): Relationship[] =>
        (relationships.get(from) ?? []).map(({ kind, to }) => ({ kind, to }));

    // keep the declarations at the module's symbols or the module, exported where their symbol is
    const exported = new Set(file.exports.map((entry) => entry.symbol));
    const declarations = file.declarations.map((each) => ({
        ...record,
        id: each.moniker,
        moduleId: moniker,
        kind: each.kind,
        name: each.name,
        enclosing: each.symbol,
        kindPackageId: each.kindPackageId,
        isExported: exported.has(each.symbol),
        description: each.description,
        vocabulary: each.vocabulary ?? null,
        relationships: outgoing(each.moniker),
    }));

    // refuse an edge from neither the module nor one of its declarations
    const sources = new Set<string>([moniker, ...declarations.map((entry) => entry.id)]);
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
        declarations,
    };
}

/** Derive the symbol objects of one module's code, with the relationships of their code. */
function indexCode(
    file: graph.Document,
    packageId: string,
    record: IndexRecord,
): Insert<typeof symbol.table>[] {
    // group the module's code edges by source
    const moniker = graph.Moniker.of({ packageId, module: file.path });
    const relationships = Map.groupBy(file.edges, (edge) => edge.from);

    return file.symbols.map((entry) => ({
        ...record,
        id: entry.moniker,
        moduleId: moniker,
        kind: entry.kind,
        name: present(graph.Moniker.name(entry.moniker), `the name of ${entry.moniker}`),
        isExported: entry.isExported,
        source: entry.source,
        signature: entry.signature,
        comment: entry.comment ?? null,
        relationships: (relationships.get(entry.moniker) ?? []).map(({ kind, to }) => ({
            kind,
            to,
        })),
    }));
}
