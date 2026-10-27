import { Environment } from "@destack/environment";
import { type DatabaseConnection, relation, TABLE, Table } from "@destack/db";
import {
    DATABASE_PATH,
    DurableObjectDatabaseHost,
    DurableObjectReader,
    type DurableObjectNamespace,
    type DurableObjectStorage,
    type DurableObjectStub,
} from "@destack/db/cloudflare";
import type { Bucket } from "@destack/bucket";
import type { ObjectType } from "@destack/object";
import type { PackageStore } from "@destack/build/store";
import type { ExternalGraphStorage } from "../server/resource/retention.ts";
import { Digest, schema, zip } from "@destack/schema";
import { catalog, declaration, GraphIndex, module, symbol } from "../object/graph.ts";

/** The origin of the requests to an index object, which its stub routes by path alone. */
const OBJECT_ORIGIN = "https://graph.invalid";

/** The path an index object answers its own operations at, next to the database reads. */
const GRAPH_PATH = "/.destack/graph";

/** The most parameters Durable Object SQLite binds to a statement, as Cloudflare limits it. */
const OBJECT_PARAMETERS = 100;

/** The prefix of the keys marking each manifest an index object stores, which sweeps list. */
const INDEXED_PREFIX = "graphs/";

/** The table marking a manifest's index complete, written with its rows. */
const COMPLETE_TABLE = "destack_graph_complete";

/** A build's place in its manifest's index, as an operation sends it. */
const IndexedBuild = schema.object({
    /** The build reading the index. */
    build: schema.identifier("build"),
    /** The space of the build. */
    space: schema.identifier("space"),
    /** The digest of the build's manifest. */
    manifest: Digest,
    /** When the build finished. */
    finishedAt: schema.number().int(),
});

/** A build's opened view of its manifest's index. */
interface OpenedView {
    /** The digest of the build's manifest. */
    readonly manifest: Digest;
    /** The build's view of the index's rows. */
    readonly database: DatabaseConnection;
}

/** An operation on a manifest's index object. */
const GraphOperation = schema.discriminatedUnion("operation", [
    schema.object({
        /** Store a manifest's rows once, completing the index, and skip them once complete. */
        operation: schema.literal("index"),
        /** The build whose rows they are. */
        index: IndexedBuild,
        /** The module rows in JSON form. */
        modules: schema.array(schema.jsonObject()),
        /** The declaration rows in JSON form. */
        declarations: schema.array(schema.jsonObject()),
        /** The symbol rows in JSON form. */
        symbols: schema.array(schema.jsonObject()),
        /** The catalog rows in JSON form. */
        catalogs: schema.array(schema.jsonObject()),
    }),
    schema.object({
        /** Show a build the index's rows in its own namespace, answering whether it is complete. */
        operation: schema.literal("view"),
        /** The build. */
        index: IndexedBuild,
    }),
    schema.object({
        /** Delete the index, once retention drops its manifest. */
        operation: schema.literal("delete"),
    }),
]);
/** An operation on a manifest's index object. */
type GraphOperation = schema.Infer<typeof GraphOperation>;

/** The answer to a view: whether the manifest's index is complete, and the build shown it. */
const Viewed = schema.object({ isComplete: schema.boolean() });

/** The graph of each build as external objects, one index object per manifest. */
export class DurableObjectGraphStorage implements ExternalGraphStorage {
    /** The graph's object types. */
    readonly objects: readonly ObjectType[] = [module, declaration, symbol, catalog];
    /** The database with the builds. */
    readonly #database: DatabaseConnection;
    /** The builds and code indexes the spaces store. */
    readonly #builds: Pick<PackageStore, "contains" | "contents" | "referrer" | "file">;
    /** The index objects, one per manifest. */
    readonly #indexes: DurableObjectNamespace;
    /** The bucket marking each manifest an index object stores, which sweeps list. */
    readonly #indexed: Pick<Bucket, "put" | "list" | "delete">;
    /** The opened builds' views with their manifests, by build. */
    readonly #opened = new Map<string, Promise<OpenedView>>();

    /** Index the builds a database records and their spaces store, in one object per manifest. */
    constructor(
        database: DatabaseConnection,
        builds: Pick<PackageStore, "contains" | "contents" | "referrer" | "file">,
        indexes: DurableObjectNamespace,
        indexed: Pick<Bucket, "put" | "list" | "delete">,
    ) {
        // store the database, the builds, the index objects and their marks
        this.#database = database;
        this.#builds = builds;
        this.#indexes = indexes;
        this.#indexed = indexed;
    }

    /** Delete the index objects and marks of the manifests the build store no longer has. */
    async sweep(): Promise<void> {
        let cursor: string | undefined;
        do {
            // list a page of marked manifests
            const page = await this.#indexed.list({
                prefix: INDEXED_PREFIX,
                ...(cursor === undefined ? {} : { cursor }),
            });
            cursor = page.cursor;

            // forget each dropped manifest's views, delete its object, then its mark
            for (const { key } of page.files) {
                const manifest = Digest.parse(key.slice(INDEXED_PREFIX.length));
                if (!(await this.#builds.contains(manifest))) {
                    await this.#forget(manifest);
                    const object = this.#indexes.get(this.#indexes.idFromName(manifest));
                    await send(object, { operation: "delete" });
                    await this.#indexed.delete(key);
                }
            }
        } while (cursor !== undefined);
    }

    /** Open a build's graph, indexing its manifest on first use. */
    async open(scope: string): Promise<DatabaseConnection> {
        // open the build's view once and forget a failed open
        let opened = this.#opened.get(scope);
        if (opened === undefined) {
            opened = this.#open(scope);
            this.#opened.set(scope, opened);
            opened.catch(() => this.#opened.delete(scope));
        }

        return (await opened).database;
    }

    /** Forget the opened views of a manifest's builds, except the failed opens. */
    async #forget(manifest: Digest): Promise<void> {
        const opened = [...this.#opened];
        const settled = await Promise.allSettled(opened.map(([, view]) => view));
        for (const [[scope], view] of zip(opened, settled)) {
            if (view.status === "fulfilled" && view.value.manifest === manifest) {
                this.#opened.delete(scope);
            }
        }
    }

    /** Show a build its manifest's index, writing the index first when it is not complete. */
    async #open(scope: string): Promise<OpenedView> {
        // read the build's place, and its manifest's object
        const index = await GraphIndex.of(this.#database, scope);
        const object = this.#indexes.get(this.#indexes.idFromName(index.manifest));
        const fields = {
            build: index.build,
            space: index.space,
            manifest: index.manifest,
            finishedAt: index.finishedAt,
        };

        // index the manifest from the build when no build did, then show the build its rows
        const viewed = Viewed.parse(await send(object, { operation: "view", index: fields }));
        if (!viewed.isComplete) {
            // mark the manifest for sweeps, then store its rows
            await this.#indexed.put(`${INDEXED_PREFIX}${index.manifest}`, null);
            const { modules, declarations, symbols, catalogs } = await index.rows(this.#builds);
            await send(object, {
                operation: "index",
                index: fields,
                modules: modules.map((row) => module.table[TABLE].encode(row)),
                declarations: declarations.map((row) => declaration.table[TABLE].encode(row)),
                symbols: symbols.map((row) => symbol.table[TABLE].encode(row)),
                catalogs: catalogs.map((row) => catalog.table[TABLE].encode(row)),
            });
            await send(object, { operation: "view", index: fields });
        }

        return {
            manifest: index.manifest,
            database: DurableObjectReader.connect(
                object,
                GraphIndex.tables,
                this.#database.state.environment,
                index.build,
            ),
        };
    }
}

/** One manifest's index: its rows stored once, shown to each build in the build's namespace. */
export class GraphIndexObject {
    /** The object's database. */
    readonly #host: DurableObjectDatabaseHost;

    /** Open the index in the object's storage. */
    constructor(state: { readonly storage: DurableObjectStorage }) {
        this.#host = new DurableObjectDatabaseHost(state.storage, Environment.system);
    }

    /** Answer an index operation, or a build's read of its view. */
    async fetch(request: Request): Promise<Response> {
        // answer a build's reads of its namespace
        const { pathname } = new URL(request.url);
        if (pathname === DATABASE_PATH) {
            return this.#host.read(request);
        }
        // refuse every other path
        else if (pathname !== GRAPH_PATH) {
            return new Response(`no index operation at ${pathname}`, { status: 404 });
        }

        // store the rows, or show a build its view
        const body = GraphOperation.parse(await request.json());
        if (body.operation === "index") {
            await this.#index(body);

            return Response.json({});
        }
        // delete every row, view and the completion mark
        else if (body.operation === "delete") {
            await this.#host.storage.deleteAll();

            return Response.json({});
        }
        // show a build its view
        else {
            return Response.json(this.#view(new GraphIndex(body.index)));
        }
    }

    /** Store the manifest's rows and the completion mark in one transaction, once. */
    async #index(body: Extract<GraphOperation, { operation: "index" }>): Promise<void> {
        // skip a complete index
        if (this.#isComplete()) {
            return;
        }

        // read the rows from their JSON form
        const modules = body.modules.map((row) =>
            Table.complete(module.table, Table.decode(module.table, row)),
        );
        const declarations = body.declarations.map((row) =>
            Table.complete(declaration.table, Table.decode(declaration.table, row)),
        );
        const symbols = body.symbols.map((row) =>
            Table.complete(symbol.table, Table.decode(symbol.table, row)),
        );
        const catalogs = body.catalogs.map((row) =>
            Table.complete(catalog.table, Table.decode(catalog.table, row)),
        );

        // migrate the tables, then insert the rows and the completion mark in one transaction
        const database = this.#host.connect(GraphIndex.tables);
        await database.migrate(GraphIndex.tables);
        await database.transaction(async (transaction) => {
            // insert the rows, then mark the index complete
            await GraphIndex.write(
                transaction,
                { modules, declarations, symbols, catalogs },
                OBJECT_PARAMETERS,
            );
            this.#host.storage.sql.exec(
                `CREATE TABLE IF NOT EXISTS "${COMPLETE_TABLE}" (manifest TEXT PRIMARY KEY)`,
            );
            this.#host.storage.sql.exec(
                `INSERT INTO "${COMPLETE_TABLE}" (manifest) VALUES (?)`,
                body.index.manifest,
            );
        });
    }

    /** Create a build's views of the complete index once, answering whether it is complete. */
    #view(index: GraphIndex): schema.Infer<typeof Viewed> {
        // show nothing before the index is complete
        if (!this.#isComplete()) {
            return { isComplete: false };
        }

        // show every stored relation in the build's namespace, the graph's rows as the build's
        const graph = new Map<string, (typeof GraphIndex.tables)[number]>(
            GraphIndex.tables.map((table) => [table[TABLE].sqlName, table]),
        );
        for (const name of this.#relations()) {
            const table = graph.get(name);
            const selected =
                table === undefined ? `SELECT * FROM "${name}"` : index.select(table, `"${name}"`);
            this.#host.storage.sql.exec(
                `CREATE VIEW IF NOT EXISTS "${relation(name, index.build)}" AS ${selected}`,
            );
        }

        return { isComplete: true };
    }

    /** Report whether a write completed the index. */
    #isComplete(): boolean {
        const rows = this.#host.storage.sql
            .exec("SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?", COMPLETE_TABLE)
            .toArray();

        return rows.length > 0;
    }

    /** List the stored relations outside the builds' namespaces, but SQLite's and the mark. */
    #relations(): string[] {
        const rows = this.#host.storage.sql
            .exec(
                `SELECT name FROM sqlite_schema WHERE type = 'table' AND instr(name, '.') = 0
                    AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%' AND name <> ?`,
                COMPLETE_TABLE,
            )
            .toArray();

        return rows.map((row) => schema.string().parse(row["name"]));
    }
}

/** Send an index object an operation, refusing one it fails. */
async function send(object: DurableObjectStub, operation: GraphOperation): Promise<unknown> {
    const response = await object.fetch(
        new Request(`${OBJECT_ORIGIN}${GRAPH_PATH}`, {
            method: "POST",
            body: JSON.stringify(operation),
        }),
    );
    if (!response.ok) {
        throw new TypeError(
            `the index object responded with ${response.status}: ${await response.text()}`,
        );
    }

    return await response.json();
}
