import { type DatabaseConnection, relation, TABLE, Table } from "@destack/db";
import {
    DATABASE_PATH,
    DurableObjectDatabaseHost,
    DurableObjectReader,
    type DurableObjectNamespace,
    type DurableObjectStorage,
    type DurableObjectStub,
} from "@destack/db/cloudflare";
import type { ObjectType } from "@destack/object";
import type { ExternalStorage } from "@destack/object/server";
import type { BuildReader } from "@destack/package/manifest";
import { Digest, type Identifier, schema } from "@destack/schema";
import { catalog, GraphIndex, module, symbol } from "../object/graph.ts";

/** The origin of the requests to an index object, which its stub routes by path alone. */
const OBJECT_ORIGIN = "https://graph.invalid";

/** The path an index object answers its own operations at, beside the database reads. */
const GRAPH_PATH = "/.destack/graph";

/** The most parameters Durable Object SQLite binds to a statement, as Cloudflare limits it. */
const OBJECT_PARAMETERS = 100;

/** The table marking a manifest's index complete, which one write seals. */
const SEALED_TABLE = "destack_graph_sealed";

/** A build's place in its manifest's index, as an operation carries it. */
const IndexedBuild = schema.object({
    /** The build reading the index. */
    build: schema.identifier("build"),
    /** The space keeping the build. */
    space: schema.identifier("space"),
    /** The digest of the build's manifest. */
    manifest: Digest,
    /** When the build finished. */
    finishedAt: schema.number().int(),
});

/** An operation on a manifest's index object. */
const GraphOperation = schema.discriminatedUnion("operation", [
    schema.object({
        /** Store a manifest's rows once, sealing the index, and skip them once sealed. */
        operation: schema.literal("index"),
        /** The build whose rows they are. */
        index: IndexedBuild,
        /** The module rows in JSON form. */
        modules: schema.array(schema.jsonObject()),
        /** The symbol rows in JSON form. */
        symbols: schema.array(schema.jsonObject()),
        /** The catalog rows in JSON form. */
        catalogs: schema.array(schema.jsonObject()),
    }),
    schema.object({
        /** Show a build the index's rows in its own namespace, answering whether it is sealed. */
        operation: schema.literal("view"),
        /** The build. */
        index: IndexedBuild,
    }),
]);
/** An operation on a manifest's index object. */
type GraphOperation = schema.Infer<typeof GraphOperation>;

/** The answer to a view: whether the manifest's index is sealed, and the build shown it. */
const Viewed = schema.object({ isSealed: schema.boolean() });

/** The graph of each build as external objects, one index object per manifest. */
export class DurableObjectGraphStorage implements ExternalStorage {
    /** The graph's object types. */
    readonly objects: readonly ObjectType[] = [module, symbol, catalog];
    /** The database with the builds. */
    readonly #database: DatabaseConnection;
    /** Read a build a space stores by its manifest's digest. */
    readonly #builds: (spaceId: Identifier<"space">, manifest: string) => Promise<BuildReader>;
    /** The index objects, one per manifest. */
    readonly #indexes: DurableObjectNamespace;
    /** The opened builds' views, by build. */
    readonly #opened = new Map<string, Promise<DatabaseConnection>>();

    /** Index the builds a database records and their spaces store, in one object per manifest. */
    constructor(
        database: DatabaseConnection,
        builds: (spaceId: Identifier<"space">, manifest: string) => Promise<BuildReader>,
        indexes: DurableObjectNamespace,
    ) {
        this.#database = database;
        this.#builds = builds;
        this.#indexes = indexes;
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

        return opened;
    }

    /** Show a build its manifest's index, writing the index first when it is not sealed. */
    async #open(scope: string): Promise<DatabaseConnection> {
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
        if (!viewed.isSealed) {
            const { modules, symbols, catalogs } = await index.rows(
                await this.#builds(index.space, index.manifest),
            );
            await send(object, {
                operation: "index",
                index: fields,
                modules: modules.map((row) => module.table[TABLE].encode(row)),
                symbols: symbols.map((row) => symbol.table[TABLE].encode(row)),
                catalogs: catalogs.map((row) => catalog.table[TABLE].encode(row)),
            });
            await send(object, { operation: "view", index: fields });
        }

        return DurableObjectReader.connect(object, GraphIndex.tables, index.build);
    }
}

/** One manifest's index: its rows stored once, shown to each build in the build's namespace. */
export class GraphIndexObject {
    /** The object's database. */
    readonly #host: DurableObjectDatabaseHost;

    /** Keep the index in the object's storage. */
    constructor(state: { readonly storage: DurableObjectStorage }) {
        this.#host = new DurableObjectDatabaseHost(state.storage);
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
        } else {
            return Response.json(this.#view(new GraphIndex(body.index)));
        }
    }

    /** Store the manifest's rows in one transaction and seal the index, once. */
    async #index(body: Extract<GraphOperation, { operation: "index" }>): Promise<void> {
        // skip a sealed index
        if (this.#isSealed()) {
            return;
        }

        // read the rows from their JSON form
        const modules = body.modules.map((row) =>
            Table.complete(module.table, Table.decode(module.table, row)),
        );
        const symbols = body.symbols.map((row) =>
            Table.complete(symbol.table, Table.decode(symbol.table, row)),
        );
        const catalogs = body.catalogs.map((row) =>
            Table.complete(catalog.table, Table.decode(catalog.table, row)),
        );

        // migrate the tables, then insert the rows and the seal in one transaction
        const database = this.#host.connect(GraphIndex.tables);
        await database.migrate(GraphIndex.tables);
        await database.transaction(async (transaction) => {
            // insert the rows, then seal the index
            await GraphIndex.write(transaction, { modules, symbols, catalogs }, OBJECT_PARAMETERS);
            this.#host.storage.sql.exec(
                `CREATE TABLE IF NOT EXISTS "${SEALED_TABLE}" (manifest TEXT PRIMARY KEY)`,
            );
            this.#host.storage.sql.exec(
                `INSERT INTO "${SEALED_TABLE}" (manifest) VALUES (?)`,
                body.index.manifest,
            );
        });
    }

    /** Create a build's views of the sealed index once, answering whether it is sealed. */
    #view(index: GraphIndex): schema.Infer<typeof Viewed> {
        // show nothing before the index is sealed
        if (!this.#isSealed()) {
            return { isSealed: false };
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

        return { isSealed: true };
    }

    /** Report whether a write sealed the index. */
    #isSealed(): boolean {
        const rows = this.#host.storage.sql
            .exec("SELECT 1 FROM sqlite_schema WHERE type = 'table' AND name = ?", SEALED_TABLE)
            .toArray();

        return rows.length > 0;
    }

    /** List the stored relations outside the builds' namespaces, but SQLite's and the seal. */
    #relations(): string[] {
        const rows = this.#host.storage.sql
            .exec(
                `SELECT name FROM sqlite_schema WHERE type = 'table' AND instr(name, '.') = 0
                    AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_cf_%' AND name <> ?`,
                SEALED_TABLE,
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
            `the index object answered ${response.status}: ${await response.text()}`,
        );
    }

    return await response.json();
}
