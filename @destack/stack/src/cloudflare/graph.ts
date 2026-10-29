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
import type { ObjectType } from "@destack/object";
import type { PackageStore } from "@destack/build/store";
import type { ExternalStorage } from "@destack/object/server";
import { Digest, schema } from "@destack/schema";
import { catalog, declaration, GraphIndex, module, symbol } from "../object/graph.ts";

/** The origin of the requests to an index object, which its stub routes by path alone. */
const OBJECT_ORIGIN = "https://graph.invalid";

/** The path an index object answers its own operations at, next to the database reads. */
const GRAPH_PATH = "/.destack/graph";

/** The most parameters Durable Object SQLite binds to a statement, as Cloudflare limits it. */
const OBJECT_PARAMETERS = 100;

/** How long an index object keeps an index past its last view: a week, as a build cache does. */
export const GRAPH_LEASE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often a view renews an index's lease: a seventh of the lease, daily for a week. */
const RENEW_FRACTION = 7;

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

/** A build's opened view of its manifest's index, and when the build last viewed it. */
interface OpenedView {
    /** The build's view of the index's rows. */
    readonly database: Promise<DatabaseConnection>;
    /** When the build last viewed the index, renewing its lease. */
    readonly viewedAt: number;
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
]);
/** An operation on a manifest's index object. */
type GraphOperation = schema.Infer<typeof GraphOperation>;

/** The answer to a view: whether the manifest's index is complete, and the build shown it. */
const Viewed = schema.object({ isComplete: schema.boolean() });

/** The graph of each build as external objects, one index object per manifest. */
export class DurableObjectGraphStorage implements ExternalStorage {
    /** The graph's object types. */
    readonly objects: readonly ObjectType[] = [module, declaration, symbol, catalog];
    /** The database with the builds. */
    readonly #database: DatabaseConnection;
    /** The builds and code indexes the spaces store. */
    readonly #builds: Pick<PackageStore, "contains" | "contents" | "referrer" | "file">;
    /** The index objects, one per manifest. */
    readonly #indexes: DurableObjectNamespace;
    /** How often a build views its index again, renewing the index's lease. */
    readonly #renewMilliseconds: number;
    /** The opened builds' views, by build. */
    readonly #opened = new Map<string, OpenedView>();

    /** Index the builds a database records and their spaces store, in one object per manifest. */
    constructor(
        database: DatabaseConnection,
        builds: Pick<PackageStore, "contains" | "contents" | "referrer" | "file">,
        indexes: DurableObjectNamespace,
        leaseMilliseconds = GRAPH_LEASE_MILLISECONDS,
    ) {
        // store the database, the builds, the index objects and how often a view renews
        this.#database = database;
        this.#builds = builds;
        this.#indexes = indexes;
        this.#renewMilliseconds = leaseMilliseconds / RENEW_FRACTION;
    }

    /** Open a build's graph, indexing it on first use and viewing it again once renewal is due. */
    async open(scope: string): Promise<DatabaseConnection> {
        // reuse a recent view, else view the index again, renewing its lease and indexing it after a lapse
        const now = this.#database.state.environment.clock.now();
        const known = this.#opened.get(scope);
        if (known !== undefined && now - known.viewedAt < this.#renewMilliseconds) {
            return await known.database;
        }
        const database = this.#open(scope);
        this.#opened.set(scope, { database, viewedAt: now });
        database.catch(() => this.#opened.delete(scope));

        return await database;
    }

    /** Show a build its manifest's index, writing the index first when it is not complete. */
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
        if (!viewed.isComplete) {
            // store its rows
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

        return DurableObjectReader.connect(
            object,
            GraphIndex.tables,
            this.#database.state.environment,
            index.build,
        );
    }
}

/** One manifest's index: its rows stored once, shown to each build in the build's namespace. */
export class GraphIndexObject {
    /** The object's database. */
    readonly #host: DurableObjectDatabaseHost;
    /** The object's storage, whose alarm ends the index's lease. */
    readonly #storage: IndexStorage;
    /** How long the object keeps the index past its last view. */
    readonly #leaseMilliseconds: number;

    /** Open the index in the object's storage, its lease the environment's or a week. */
    constructor(
        state: { readonly storage: IndexStorage },
        environment: { readonly DESTACK_GRAPH_LEASE?: string } = {},
    ) {
        this.#host = new DurableObjectDatabaseHost(state.storage, Environment.system);
        this.#storage = state.storage;
        this.#leaseMilliseconds =
            environment.DESTACK_GRAPH_LEASE === undefined
                ? GRAPH_LEASE_MILLISECONDS
                : schema.number().int().positive().parse(Number(environment.DESTACK_GRAPH_LEASE));
    }

    /** Delete the index once its lease ends unviewed. */
    async alarm(): Promise<void> {
        await this.#storage.deleteAll();
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
        // show a build its view, renewing the index's lease
        else {
            await this.#renew();

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

    /** Move the lease's end a lease past now, once a renewal is due. */
    async #renew(): Promise<void> {
        // read the lease's end
        const now = Environment.system.clock.now();
        const until = now + this.#leaseMilliseconds;
        const alarm = await this.#storage.getAlarm();

        // set the alarm a lease from now, unless a recent view set it
        if (alarm === null || until - alarm >= this.#leaseMilliseconds / RENEW_FRACTION) {
            await this.#storage.setAlarm(until);
        }
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

/** The storage an index object keeps its rows in and its lease's alarm on. */
interface IndexStorage extends DurableObjectStorage {
    /** Read when the alarm wakes the object, null for none. */
    readonly getAlarm: () => Promise<number | null>;
    /** Wake the object at a time. */
    readonly setAlarm: (at: number) => Promise<void>;
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
