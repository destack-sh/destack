import { mkdir, readdir, rename, rm, stat } from "node:fs/promises";
import { join } from "node:path";
import { Database } from "bun:sqlite";
import { connectBunSqlite } from "@destack/db/bun";
import { eq, type DatabaseConnection, type Insert } from "@destack/db";
import type { ObjectType } from "@destack/object";
import type { ExternalStorage } from "@destack/object/server";
import { graph } from "@destack/package";
import type { BuildReader } from "@destack/package/manifest";
import { type Identifier, present, schema, zip } from "@destack/schema";
import { build, Build } from "../object/index.ts";
import { module, type Relationship, symbol } from "../graph/object.ts";

/** The rows one insert statement writes. */
const INSERT_ROWS = 500;

/** The index tables, one per graph object type. */
const TABLES = [module.table, symbol.table];

/** The graph of each build as external objects. */
export class BunGraphStorage implements ExternalStorage, AsyncDisposable {
    /** The graph's object types. */
    readonly objects: readonly ObjectType[] = [module, symbol];
    /** The database with the builds. */
    readonly #database: DatabaseConnection;
    /** Read a build a space stores by its manifest's digest. */
    readonly #builds: (spaceId: Identifier<"space">, manifest: string) => Promise<BuildReader>;
    /** The directory with one index directory per manifest. */
    readonly #directory: string;
    /** The opened indexes, by build. */
    readonly #opened = new Map<string, Promise<GraphIndex>>();

    /** Index the builds a database records and their spaces store, under a directory. */
    constructor(
        database: DatabaseConnection,
        builds: (spaceId: Identifier<"space">, manifest: string) => Promise<BuildReader>,
        directory: string,
    ) {
        this.#database = database;
        this.#builds = builds;
        this.#directory = directory;
    }

    /** Open a build's graph, indexing its module files on first use. */
    async open(scope: string): Promise<DatabaseConnection> {
        // open the build's index once and forget a failed open
        let opened = this.#opened.get(scope);
        if (opened === undefined) {
            opened = this.#open(scope);
            this.#opened.set(scope, opened);
            opened.catch(() => this.#opened.delete(scope));
        }

        return (await opened).database;
    }

    /** Delete the indexes of the manifests retention drops, closing their open databases. */
    async sweep(retained: ReadonlySet<string>): Promise<void> {
        // close the dropped builds' indexes except the failed opens
        const opened = [...this.#opened];
        const settled = await Promise.allSettled(opened.map(([, index]) => index));
        for (const [[scope], index] of zip(opened, settled)) {
            if (index.status === "fulfilled" && !retained.has(index.value.manifest)) {
                this.#opened.delete(scope);
                await index.value.database.close();
            }
        }

        // delete their directories
        const manifests = await readdir(this.#directory).catch((error: unknown) => {
            if (isMissing(error)) {
                return [];
            }

            throw error;
        });
        for (const manifest of manifests.filter((name) => !retained.has(name))) {
            await rm(join(this.#directory, manifest), { recursive: true });
        }
    }

    /** Close the open indexes. */
    async [Symbol.asyncDispose](): Promise<void> {
        // close each index whose open succeeded
        const opened = [...this.#opened.values()];
        this.#opened.clear();
        for (const index of await Promise.allSettled(opened)) {
            if (index.status === "fulfilled") {
                await index.value.database.close();
            }
        }
    }

    /** Open a succeeded build's index file, writing it from the stored graph files when absent. */
    async #open(scope: string): Promise<GraphIndex> {
        // read the build's manifest and finish
        const buildId = schema.identifier("build").parse(scope);
        const [found] = await this.#database
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
            .where(eq(build.table.id, buildId));
        const record = present(found, `build ${scope}`);
        const manifest = Build.manifest(record);
        const finishedAt = present(record.finishedAt, "a succeeded build's finish");

        // write the index once per build under its manifest
        const directory = join(this.#directory, manifest);
        const file = join(directory, `${scope}.sqlite`);
        if (!(await exists(file))) {
            await mkdir(directory, { recursive: true });
            await write(await this.#builds(record.scope, manifest), buildId, finishedAt, file);
        }

        return {
            manifest,
            database: await connectBunSqlite(new Database(file, { readonly: true }), TABLES),
        };
    }
}

/** A build's opened index. */
interface GraphIndex {
    /** The digest of the build's manifest. */
    readonly manifest: string;
    /** The read-only index database. */
    readonly database: DatabaseConnection & { readonly close: () => Promise<void> };
}

/** Write a build's module and symbol objects into a new index file, published whole when done. */
async function write(
    reader: BuildReader,
    scope: Identifier<"build">,
    finishedAt: number,
    file: string,
): Promise<void> {
    // derive the objects of every module file
    const root = await reader.graph();
    const packageId = reader.manifest.package.id;
    const modules: Insert<typeof module.table>[] = [];
    const symbols: Insert<typeof symbol.table>[] = [];
    for (const digest of Object.values(root.modules)) {
        const graphFile = await reader.module(digest);
        const indexed = indexModule(graphFile, digest, packageId, scope, finishedAt);
        modules.push(indexed.module);
        symbols.push(...indexed.symbols);
    }

    // write a private file in the rollback journal mode read-only readers open
    const pending = `${file}.${crypto.randomUUID()}`;
    const database = await connectBunSqlite(new Database(pending), TABLES);
    try {
        await database.migrate(TABLES);
        await database.transaction(async (transaction) => {
            for (let start = 0; start < modules.length; start += INSERT_ROWS) {
                await transaction
                    .insert(module.table)
                    .values(modules.slice(start, start + INSERT_ROWS));
            }
            for (let start = 0; start < symbols.length; start += INSERT_ROWS) {
                await transaction
                    .insert(symbol.table)
                    .values(symbols.slice(start, start + INSERT_ROWS));
            }
        });
    } finally {
        await database.close();
    }

    // publish the file under its final name
    await rename(pending, file);
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

    // keep the declarations at the code symbols
    const declarations = file.declarations.map((declaration) => ({
        ...record,
        id: declaration.moniker,
        moduleId: moniker,
        kind: declaration.kind,
        name: declaration.name,
        enclosing: declaration.symbol,
        kindPackageId: declaration.package,
        isExported: present(
            exported.get(declaration.symbol),
            `the symbol ${declaration.symbol} of ${declaration.moniker}`,
        ),
        description: declaration.description,
        vocabulary: declaration.vocabulary ?? null,
        relationships: outgoing(declaration.moniker),
    }));

    return [...symbols, ...declarations];
}

/** Report whether a file exists. */
async function exists(file: string): Promise<boolean> {
    try {
        await stat(file);

        return true;
    } catch (error) {
        if (isMissing(error)) {
            return false;
        }

        throw error;
    }
}

/** Report whether a file system failure is a missing file. */
function isMissing(error: unknown): boolean {
    return error instanceof Error && "code" in error && error.code === "ENOENT";
}
