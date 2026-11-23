import { eq, type Insert, TABLE, type Table } from "@destack/db";
import type { DatabaseConnection } from "@destack/db/database";
import type { BuildStore } from "@destack/build/store";
import { graph } from "@destack/package";
import { Catalog } from "@destack/locale";
import { PackageManifest } from "@destack/package/manifest";
import { Digest, type DigestCrypto, type Identifier, present, schema } from "@destack/schema";
import { build, Build } from "../object/build.ts";
import { catalog, declaration, module, Relationship, symbol } from "../object/graph.ts";

/** A build's graph as object rows: the build, its manifest and its finish. */
export class BuildGraph {
    /** The index tables, one per graph object type. */
    static get tables() {
        return [module.table, declaration.table, symbol.table, catalog.table] as const;
    }

    /** The build reading the index. */
    readonly build: Identifier<"build">;
    /** The space of the build. */
    readonly space: Identifier<"space">;
    /** The digest of the manifest of its image's root build, which keys the index. */
    readonly manifest: Digest;
    /** When the build finished, which its rows are made at. */
    readonly finishedAt: number;

    /** Record a build's place in its manifest's index. */
    constructor(fields: Pick<BuildGraph, "build" | "space" | "manifest" | "finishedAt">) {
        // store the build, its space, its manifest and its finish
        this.build = fields.build;
        this.space = fields.space;
        this.manifest = fields.manifest;
        this.finishedAt = fields.finishedAt;
    }

    /** Insert an index's rows table by table, each statement within a storage's parameters. */
    static async write(
        transaction: Pick<DatabaseConnection, "insert">,
        rows: Awaited<ReturnType<BuildGraph["rows"]>>,
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

    /** Read an index scope's succeeded build and its image's root, refusing one without files. */
    static async of(
        database: DatabaseConnection,
        scope: string,
        builds: Pick<BuildStore, "image">,
    ): Promise<BuildGraph> {
        // read the build's space
        const id = schema.identifier("build").parse(scope);
        const [found] = await database
            .select({
                id: build.table.id,
                scope: build.table.scope,
                state: build.table.state,
                image: build.table.image,
                failure: build.table.failure,
                expiredAt: build.table.expiredAt,
                finishedAt: build.table.finishedAt,
            })
            .from(build.table)
            .where(eq(build.table.id, id));
        const record = present(found, `build ${scope}`);
        const finishedAt = present(record.finishedAt, "a succeeded build's finish");

        return new BuildGraph({
            build: id,
            space: record.scope,
            manifest: (await builds.image(Build.image(record))).root,
            finishedAt,
        });
    }

    /** Derive the rows of the build and its graph, stored once for its manifest. */
    async rows(
        builds: Pick<BuildStore, "contents" | "graph">,
        crypto: DigestCrypto,
    ): Promise<{
        modules: Insert<typeof module.table>[];
        declarations: Insert<typeof declaration.table>[];
        symbols: Insert<typeof symbol.table>[];
        catalogs: Insert<typeof catalog.table>[];
    }> {
        // read the build and its graph
        const reader = await builds.contents(this.manifest);
        const files = await builds.graph(this.manifest);

        // derive the module, declaration and symbol objects of every module, each identified within the manifest
        const packageId = PackageManifest.package(reader.manifest).id;
        const index: IndexContext = {
            scope: this.build,
            createdAt: this.finishedAt,
            updatedAt: this.finishedAt,
            manifest: this.manifest,
            crypto,
        };
        const modules: Insert<typeof module.table>[] = [];
        const declarations: Insert<typeof declaration.table>[] = [];
        const symbols: Insert<typeof symbol.table>[] = [];
        for (const file of files) {
            const indexed = await indexModule(file, packageId, index);
            modules.push(indexed.module);
            declarations.push(...indexed.declarations);
            symbols.push(...indexed.symbols);
        }

        // read the catalogs it ships: its own and its unpublished dependencies'
        const record = {
            scope: index.scope,
            createdAt: index.createdAt,
            updatedAt: index.updatedAt,
        };
        const catalogs = await Promise.all(
            (await Catalog.read(reader)).map(async (shipped) => {
                const name = `${shipped.package}/${shipped.locale}`;

                return {
                    ...record,
                    id: await catalog.derive(this.manifest, name, crypto),
                    name,
                    packageId: shipped.package,
                    locale: shipped.locale,
                    messages: shipped.messages,
                    ...(shipped.drafts === undefined ? {} : { drafts: shipped.drafts }),
                };
            }),
        );

        return { modules, declarations, symbols, catalogs };
    }

    /** Select an index table's stored rows as the build's: in its scope, made when it finished. */
    select(table: Table, from: string): string {
        // replace the scope and the times, and leave every other column
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

/** The index a build's rows go into: their record, and what their identifiers derive from. */
type IndexContext = IndexRecord & { readonly manifest: Digest; readonly crypto: DigestCrypto };

/** Derive one module's module object, its declaration objects and its symbol objects. */
async function indexModule(
    file: graph.Module,
    packageId: string,
    index: IndexContext,
): Promise<{
    module: Insert<typeof module.table>;
    declarations: Insert<typeof declaration.table>[];
    symbols: Insert<typeof symbol.table>[];
}> {
    // group the module's edges by source
    const { manifest, crypto, ...record } = index;
    const moniker = graph.Moniker.of({ packageId, module: file.path });
    const moduleId = await module.derive(manifest, moniker, crypto);
    const edges = graph.Module.edges(file);
    const relationships = Map.groupBy(edges, (edge) => edge.from);
    const outgoing = (from: string): Relationship[] =>
        (relationships.get(from) ?? []).map(({ kind, to }) => ({ kind, to }));

    // add the declarations at the module's symbols or the module, exported where their symbol is
    const exported = new Set(file.exports.map((entry) => entry.symbol));
    const declarations = await Promise.all(
        file.declarations.map(async (each) => ({
            ...record,
            id: await declaration.derive(manifest, each.moniker, crypto),
            moniker: each.moniker,
            moduleId,
            kind: each.kind,
            name: each.name,
            enclosing: each.symbol,
            kindPackageId: each.kindPackageId,
            isExported: exported.has(each.symbol),
            description: each.description,
            vocabulary: each.vocabulary ?? null,
            relationships: outgoing(each.moniker),
        })),
    );

    // add the module's symbols with their relationships
    const symbols = await Promise.all(
        file.symbols.map(async (entry) => ({
            ...record,
            id: await symbol.derive(manifest, entry.moniker, crypto),
            moniker: entry.moniker,
            moduleId,
            name: present(graph.Moniker.name(entry.moniker), `the name of ${entry.moniker}`),
            isExported: entry.isExported,
            declarations: entry.declarations,
            comment: entry.comment ?? null,
            relationships: outgoing(entry.moniker),
        })),
    );

    // refuse an edge from neither the module nor one of its declarations or symbols
    const sources = new Set<string>([
        moniker,
        ...declarations.map((entry) => entry.moniker),
        ...symbols.map((entry) => entry.moniker),
    ]);
    const stray = edges.find((edge) => !sources.has(edge.from));
    if (stray !== undefined) {
        throw new TypeError(
            `module '${moniker}' has an edge from '${stray.from}', but does not declare it`,
        );
    }

    return {
        module: {
            ...record,
            id: moduleId,
            moniker,
            path: file.path,
            digest: file.digest,
            relationships: outgoing(moniker),
        },
        declarations,
        symbols,
    };
}
