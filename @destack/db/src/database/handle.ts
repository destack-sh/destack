import { canonicalize, Digest, type JsonObject, schema } from "@destack/schema";
import type {
    KindState,
    Opener,
    ResourceKind,
    ResourceRecord,
    Snapshotter,
    ContentStore,
} from "@destack/resource";
import type { DatabaseConnection } from "./connection.ts";
import { Plan } from "@destack/resource";
import { DatabaseError } from "../error/error.ts";
import { mergeStates } from "../migration/merge.ts";
import type { TableState } from "../migration/state.ts";
import { Order } from "../query/order.ts";
import type { Row } from "../table/row.ts";
import { TABLE, Table } from "../table/table.ts";

/** The rows one read of a snapshot takes: 1000 rows of about 500 B make a read near 500 kB. */
const SNAPSHOT_READ_ROWS = 1000;

/** The shift that leaves a row hash's high 10 bits, which end a page when zero. */
const PAGE_BOUNDARY_SHIFT = 22;

/** The most rows of a page that no row ends sooner: 8192, missed once in e^8 pages. */
const PAGE_MAX_ROWS = 8192;

/** The FNV-1a 32-bit offset basis. */
const FNV_OFFSET = 0x811c9dc5;

/** The FNV-1a 32-bit prime. */
const FNV_PRIME = 0x01000193;

/** A snapshot of a database's rows: each table's pages of encoded rows, each page by its digest. */
const SnapshotManifest = schema.object({
    /** The digests of each table's pages of encoded rows, by the table's SQL name. */
    tables: schema.record(schema.string(), schema.array(Digest)),
});

/** A page of encoded rows. */
const SnapshotPage = schema.array(schema.jsonObject());

/** Rewrite a row of a table, such as wrapping its host-bound values for another host. */
type RowRewrite = (table: Table, row: Row) => Promise<Row>;

/** A database a provider opened for another party: its connection, and how it migrates itself. */
export interface DatabaseHandle {
    /** The connection. */
    readonly database: DatabaseConnection;
    /** Migrate the database to its tables and some beside them, dropping earlier ones beside. */
    readonly migrate: (beside: readonly Table[]) => Promise<void>;
    /** Release the database. */
    readonly close: () => Promise<void>;
}

/** Connect to a database with some tables declared, answering a connection that closes. */
type Connect = (
    tables: readonly Table[],
) =>
    | (DatabaseConnection & AsyncDisposable & { readonly close: () => Promise<void> })
    | Promise<DatabaseConnection & AsyncDisposable & { readonly close: () => Promise<void> }>;

/** Plan, apply and open a host's databases. */
export const DatabaseHandle = {
    /** Diff a database's applied state against the desired table states. */
    async plan(connect: Connect, desired: readonly (readonly TableState[])[]): Promise<Plan> {
        await using database = await connect([]);

        return await database.plan(mergeStates(desired));
    },

    /** Apply the reviewed plan of a database, refusing one that changed since review. */
    async apply(
        connect: Connect,
        desired: readonly (readonly TableState[])[],
        digest: Digest,
        id: string,
    ): Promise<void> {
        // plan again and apply the plan only while it matches the reviewed digest
        await using database = await connect([]);
        const plan = await database.plan(mergeStates(desired));
        if ((await Plan.digest(plan, database.state.environment.crypto)) !== digest) {
            throw DatabaseError.of("PLAN_CHANGED", `plan of ${id} changed since review`);
        }
        await database.apply(plan);
    },

    /** Describe the tables some desired states declare together. */
    tables(desired: readonly (readonly TableState[])[]): Table[] {
        return mergeStates(desired).declared.map((state) => Table.describe(state));
    },

    /** Open the tables of the desired states, reopening them with tables beside once migrated. */
    async open(
        connect: Connect,
        desired: readonly (readonly TableState[])[],
    ): Promise<DatabaseHandle> {
        // open the desired tables and reopen them with the tables beside once migrated
        const described = DatabaseHandle.tables(desired);
        let database = await connect(described);
        const handle: DatabaseHandle = {
            get database() {
                return database;
            },
            migrate: async (beside) => {
                await database.migrate(beside, { states: desired });
                await database.close();
                database = await connect([...described, ...beside]);
            },
            close: () => database.close(),
        };

        return handle;
    },

    /** Copy every row of the database into a store as content-addressed pages. */
    async snapshot(
        handle: Pick<DatabaseHandle, "database">,
        store: Pick<ContentStore, "write">,
        rewrite?: RowRewrite,
    ): Promise<Digest> {
        // read each table's rows in content-defined pages, in one read transaction
        const tables: Record<string, Digest[]> = {};
        await handle.database.transaction(
            async (transaction) => {
                for (const table of handle.database.tables) {
                    const pages: Digest[] = [];
                    for await (const rows of pagesOf(transaction, table)) {
                        // rewrite the page's rows and store the page under its digest
                        const rewritten =
                            rewrite === undefined
                                ? rows
                                : await Promise.all(rows.map((row) => rewrite(table, row)));
                        const encoded = rewritten.map((row) => table[TABLE].encode(row));
                        pages.push(await write(store, canonicalize(encoded)));
                    }
                    tables[table[TABLE].sqlName] = pages;
                }
            },
            { isReadOnly: true, isolationLevel: "repeatable read" },
        );

        return write(store, canonicalize({ tables }));
    },

    /** Restore a snapshot's rows into a database of the same tables, rewriting rows on the way. */
    async restore(
        handle: Pick<DatabaseHandle, "database">,
        digest: Digest,
        store: Pick<ContentStore, "read">,
        rewrite?: RowRewrite,
        base?: Digest,
    ): Promise<void> {
        // read the snapshot's pages and the base's
        const manifest = await readManifest(store, digest);
        const held = base === undefined ? { tables: {} } : await readManifest(store, base);

        // replace the rows of pages the snapshot lacks, checking references at commit
        await handle.database.transaction(
            async (transaction) => {
                for (const table of handle.database.tables) {
                    // take out the rows of the pages that left
                    const pages = manifest.tables[table[TABLE].sqlName] ?? [];
                    const previous = held.tables[table[TABLE].sqlName] ?? [];
                    for (const page of previous.filter((entry) => !pages.includes(entry))) {
                        const rows = await readPage(store, table, page);
                        if (rows.length > 0) {
                            await transaction.remove(table, rows);
                        }
                    }

                    // insert the rows of the pages that arrived
                    for (const page of pages.filter((entry) => !previous.includes(entry))) {
                        const decoded = await readPage(store, table, page);
                        const rows =
                            rewrite === undefined
                                ? decoded
                                : await Promise.all(decoded.map((row) => rewrite(table, row)));
                        if (rows.length > 0) {
                            await transaction.insert(table).values(rows);
                        }
                    }
                }
            },
            { constraints: "deferred" },
        );
    },

    /** Snapshot and restore an opener's databases, each opened for the copy and closed after. */
    snapshotter<Kind extends ResourceKind>(
        opener: Opener<Kind, DatabaseHandle>,
    ): Snapshotter<Kind> {
        return {
            snapshot: async (record, desired, store) => {
                await using opened = await openFor(opener, record, desired);

                return DatabaseHandle.snapshot(opened.handle, store);
            },
            restore: async (record, desired, digest, store, _recipient, base) => {
                await using opened = await openFor(opener, record, desired);
                await DatabaseHandle.restore(opened.handle, digest, store, undefined, base);
            },
        };
    },
};

/** Open a database for one copy, closing it when disposed. */
async function openFor<Kind extends ResourceKind>(
    opener: Opener<Kind, DatabaseHandle>,
    record: ResourceRecord<Kind>,
    desired: readonly KindState<Kind>[],
): Promise<{ readonly handle: DatabaseHandle } & AsyncDisposable> {
    const handle = await opener.open(record, desired);

    return { handle, [Symbol.asyncDispose]: () => handle.close() };
}

/** Read a table's rows in key order, in pages ending at rows whose content hashes to a break. */
async function* pagesOf(database: DatabaseConnection, table: Table): AsyncGenerator<Row[]> {
    // seek in key order and gather rows into the current page
    const order = Order.complete([], table);
    let page: Row[] = [];
    let last: Row | undefined;
    do {
        // read the next rows in key order
        const rows: Row[] = await database
            .select()
            .from(table)
            .where(last === undefined ? undefined : Order.after(order, table, last))
            .orderBy(...Order.render(order, table))
            .limit(SNAPSHOT_READ_ROWS);
        last = rows.length === SNAPSHOT_READ_ROWS ? rows.at(-1) : undefined;

        // end a page after a page's last row or at its most rows
        for (const row of rows) {
            page.push(row);
            const hash = fnv(canonicalize(table[TABLE].encode(row)));
            if (hash >>> PAGE_BOUNDARY_SHIFT === 0 || page.length === PAGE_MAX_ROWS) {
                yield page;
                page = [];
            }
        }
    } while (last !== undefined);

    // end the last page
    if (page.length > 0) {
        yield page;
    }
}

/** Hash text with FNV-1a over its UTF-16 code units. */
function fnv(text: string): number {
    let hash = FNV_OFFSET;
    for (let index = 0; index < text.length; index++) {
        hash = Math.imul(hash ^ text.charCodeAt(index), FNV_PRIME);
    }

    return hash >>> 0;
}

/** Read a snapshot's manifest from a store. */
async function readManifest(
    store: Pick<ContentStore, "read">,
    digest: Digest,
): Promise<{ readonly tables: Record<string, Digest[]> }> {
    return SnapshotManifest.parse(JSON.parse(await read(store, digest)));
}

/** Read a page of a table's rows from a store. */
async function readPage(
    store: Pick<ContentStore, "read">,
    table: Table,
    digest: Digest,
): Promise<Row[]> {
    const encoded = SnapshotPage.parse(JSON.parse(await read(store, digest)));

    return encoded.map((row: JsonObject) => Table.decode(table, row));
}

/** Store text under its digest. */
function write(store: Pick<ContentStore, "write">, text: string): Promise<Digest> {
    return store.write(chunked(new TextEncoder().encode(text)));
}

/** Read the text stored under a digest. */
async function read(store: Pick<ContentStore, "read">, digest: Digest): Promise<string> {
    // decode the chunks as one text
    const decoder = new TextDecoder();
    let text = "";
    for await (const chunk of store.read(digest)) {
        text += decoder.decode(chunk, { stream: true });
    }

    return text + decoder.decode();
}

/** Yield bytes as one chunk. */
async function* chunked(bytes: Uint8Array): AsyncIterable<Uint8Array> {
    yield bytes;
}
