import { quote } from "../dialect/quote.ts";
import type { DatabaseEnvironment } from "../database/connection.ts";
import { Digest, schema, Uuid } from "@destack/schema";
import { Plan, ResourceId, ResourceRecord } from "@destack/resource";
import { DatabaseHandle } from "../database/handle.ts";
import { type DatabaseKind, DatabaseSpec } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import { DatabaseState } from "../migration/state.ts";
import type { DatabaseHost } from "../provider/provider.ts";
import { relation } from "../table/namespace.ts";
import { Table } from "../table/table.ts";
import type * as declaration from "../declare/database.ts";
import type { Model } from "../query/model.ts";
import { SqliteWorkQueue } from "../sqlite/client.ts";
import { SQLStatement, SQLValue } from "../sql/json.ts";
import { type SqliteConnectOptions, SqliteDatabase } from "../sqlite/database.ts";
import {
    type DurableObjectCursor,
    DurableObjectClient,
    DurableObjectQuery,
    type DurableObjectStorage,
} from "./client.ts";
import { DurableObjectContentStore } from "./content.ts";

/** The provider code of databases in Durable Objects' SQLite storage. */
export const DURABLE_OBJECT_PROVIDER = "durable-object";

/** The pool name of an object's storage queue, its one connection. */
const DURABLE_OBJECT_POOL = "durable_object";

/** The name of an object's database without a namespace, its storage's own. */
export const STORAGE_NAME = "storage";

/** How long the host holds a reader's wait for a commit: 25 s, below common 30 s idle timeouts. */
const COMMIT_WAIT_MILLISECONDS = 25_000;

/** The path below which an object answers operations on its databases. */
export const DATABASE_PATH = "/.destack/database";

/** A database resource as an operation refers to it. */
const DatabaseRecord = schema.object({
    /** The resource. */
    id: ResourceId,
    /** The resource's space. */
    scope: schema.string().min(1),
    /** The resource's specification. */
    spec: DatabaseSpec,
    /** The provider's reference once provisioned. */
    reference: schema.string().min(1).nullable(),
});

/** A position in a database's commits: the host's connection to it and its commit count there. */
export const CommitPosition = schema.object({
    /** The id of the connection the host opened, new at each opening. */
    connection: schema.string(),
    /** The commits the connection made since it opened. */
    generation: schema.number().int(),
});
/** A position in a database's commits. */
export type CommitPosition = schema.Infer<typeof CommitPosition>;

/** A commit wait's answer: the position then, and the tables committed since the reader's. */
export const CommitWait = schema.object({
    /** The position of the database's commits once the wait ends. */
    position: CommitPosition,
    /** The SQL names of the tables committed during the wait, or every table for a new reader. */
    tables: schema.array(schema.string()),
});
/** A commit wait's answer. */
export type CommitWait = schema.Infer<typeof CommitWait>;

/** A connection the host opened to one of its databases, with the id of that opening. */
interface HostConnection {
    /** The id of the opening, which a restart or a reopening changes. */
    readonly id: string;
    /** The connection. */
    readonly database: SqliteDatabase<DurableObjectClient>;
}

/** An operation on an object's databases, as another object sends it. */
const DurableObjectDatabaseOperation = schema.discriminatedUnion("operation", [
    schema.object({
        /** Create the database's namespace and log, or confirm them. */
        operation: schema.literal("provision"),
        /** The database resource. */
        record: DatabaseRecord,
    }),
    schema.object({
        /** Drop the database's namespace. */
        operation: schema.literal("destroy"),
        /** The database resource. */
        record: DatabaseRecord,
    }),
    schema.object({
        /** Plan the database's tables toward desired states. */
        operation: schema.literal("plan"),
        /** The database resource. */
        record: DatabaseRecord,
        /** The desired states. */
        desired: schema.array(DatabaseState),
    }),
    schema.object({
        /** Apply the reviewed plan of the database's tables toward desired states. */
        operation: schema.literal("apply"),
        /** The database resource. */
        record: DatabaseRecord,
        /** The desired states. */
        desired: schema.array(DatabaseState),
        /** The reviewed plan's digest. */
        digest: Digest,
    }),
    schema.object({
        /** Copy the database's rows into the sender's content, returning the digest. */
        operation: schema.literal("snapshot"),
        /** The database resource. */
        record: DatabaseRecord,
        /** The desired states. */
        desired: schema.array(DatabaseState),
    }),
    schema.object({
        /** Restore a sender's copy into the database, onto a base copy it has when given. */
        operation: schema.literal("restore"),
        /** The database resource. */
        record: DatabaseRecord,
        /** The desired states. */
        desired: schema.array(DatabaseState),
        /** The copy's digest. */
        digest: Digest,
        /** The digest of the copy the database has already, absent for an empty database. */
        base: Digest.exactOptional(),
    }),
    schema.object({
        /** Count the object's database bytes, and take the rows they read and wrote. */
        operation: schema.literal("measure"),
    }),
    schema.object({
        /** Wait for a database's commit past a position, answering the position then. */
        operation: schema.literal("wait"),
        /** The database's name in the object's storage. */
        name: schema.string().min(1),
        /** The commit position the reader saw last, absent before the first. */
        after: CommitPosition.exactOptional(),
    }),
    SQLStatement.extend({
        /** Read rows of the object's storage, refusing a statement that writes. */
        operation: schema.literal("read"),
        /** Whether the rows come as arrays of values or as objects by column name. */
        rows: schema.enum(["values", "objects"]),
    }),
]);
/** An operation on an object's databases, as another object sends it. */
export type DurableObjectDatabaseOperation = schema.Infer<typeof DurableObjectDatabaseOperation>;

/** The bytes of an object's databases, and the rows they read and wrote since last measured. */
export const DurableObjectMeasurement = schema.object({
    /** The bytes of the object's SQLite storage. */
    bytes: schema.number().int().nonnegative(),
    /** The rows read since the last measure. */
    rowsRead: schema.number().int().nonnegative(),
    /** The rows written since the last measure. */
    rowsWritten: schema.number().int().nonnegative(),
});
/** The bytes of an object's databases, and the rows they read and wrote since last measured. */
export type DurableObjectMeasurement = schema.Infer<typeof DurableObjectMeasurement>;

/** The options of a database in a Durable Object's SQLite storage. */
export interface DurableObjectConnectOptions extends Omit<SqliteConnectOptions, "environment"> {
    /** The database's namespace in a shared storage, such as its identifier, if any. */
    readonly namespace?: string;
}

/** The prefix of the database references in an object's storage. */
const REFERENCE_PREFIX = `${DURABLE_OBJECT_PROVIDER}:`;

/** One storage transaction of a host, open while its work runs. */
export class StorageTransaction {
    /** Whether the transaction's work still runs. */
    #isOpen = true;

    /** Run work within the transaction, refusing it once the transaction closed. */
    within<Value>(work: () => Promise<Value>): Promise<Value> {
        return this.#isOpen
            ? work()
            : Promise.reject(
                  DatabaseError.of("TRANSACTION_BROKEN", "the storage transaction closed"),
              );
    }

    /** Close the transaction once its work settled. */
    close(): void {
        this.#isOpen = false;
    }
}

/** Store databases in one Durable Object's SQLite storage, each in its resource's namespace. */
export class DurableObjectDatabaseHost implements DatabaseHost {
    /** The provider code of databases in Durable Objects. */
    readonly provider = DURABLE_OBJECT_PROVIDER;
    /** The object's SQLite storage. */
    readonly storage: DurableObjectStorage;
    /** The work queue of the storage's one connection, one transaction at a time. */
    readonly queue: SqliteWorkQueue;
    /** The clock the databases' logs stamp changes at and the randomness they draw from. */
    readonly environment: DatabaseEnvironment;
    /** The rows the databases read and wrote since the last measure. */
    #rows = { rowsRead: 0, rowsWritten: 0 };
    /** The connections the host opened, by database name, whose commits other objects wait for. */
    readonly #connections = new Map<string, HostConnection>();

    /** Store databases in an object's storage, in the object's environment. */
    constructor(storage: DurableObjectStorage, environment: DatabaseEnvironment) {
        this.storage = storage;
        this.environment = environment;
        this.queue = new SqliteWorkQueue(environment.telemetry, DURABLE_OBJECT_POOL);
    }

    /** Open a database in the storage in the host's environment, the object its sole writer. */
    connect<Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>>(
        tables: declaration.Database<Models> | readonly Table[],
        options: DurableObjectConnectOptions = {},
    ): SqliteDatabase<DurableObjectClient, Models> {
        return this.#open(tables, options);
    }

    /** Run work in a storage transaction behind the queue, closing it once the work settles. */
    transaction<Value>(work: (transaction: StorageTransaction) => Promise<Value>): Promise<Value> {
        return this.queue.run(async () => {
            const transaction = new StorageTransaction();
            try {
                return await this.storage.transaction(() => work(transaction));
            } finally {
                transaction.close();
            }
        }, this.environment);
    }

    /** Open a database of the storage whose statements run in a mode. */
    #open<Models extends Readonly<Record<string, Model>>>(
        tables: declaration.Database<Models> | readonly Table[],
        options: DurableObjectConnectOptions,
    ): SqliteDatabase<DurableObjectClient, Models> {
        // open the database, recorded by name for the readers waiting for its commits
        const name = options.namespace ?? STORAGE_NAME;
        const database = new SqliteDatabase(
            new DurableObjectClient(this, { kind: "queued" }),
            tables,
            "embedded",
            "writer",
            { ...options, environment: this.environment, name },
        );
        this.#connections.set(name, { id: Uuid.v4(this.environment.random), database });

        return database;
    }

    /** Add the rows a statement read and wrote, once its rows are read. */
    count(cursor: DurableObjectCursor): void {
        this.#rows.rowsRead += cursor.rowsRead;
        this.#rows.rowsWritten += cursor.rowsWritten;
    }

    /** Write a database's reference by its storage namespace: `durable-object:<namespace>`. */
    static reference(namespace: string): string {
        return `${REFERENCE_PREFIX}${namespace}`;
    }

    /** Read the namespace of a database reference, refusing other providers' references. */
    static namespace(reference: string): string {
        if (
            !reference.startsWith(REFERENCE_PREFIX) ||
            reference.length === REFERENCE_PREFIX.length
        ) {
            throw DatabaseError.of(
                "RECORD_INVALID",
                `database reference '${reference}' is invalid: there is no Durable Object database of that name`,
            );
        }

        return reference.slice(REFERENCE_PREFIX.length);
    }

    /** Create the database's log in its namespace once, in its resource's scope. */
    async provision(record: ResourceRecord<typeof DatabaseKind>): Promise<{ reference: string }> {
        // start the log unless an earlier provisioning started it
        const reference = DurableObjectDatabaseHost.reference(record.id);
        await using database = this.connect([], { namespace: record.id });
        const isLogged = (await this.relations(record.id)).length > 0;
        if (!isLogged) {
            await database.log.create(record.scope);
        }

        return { reference };
    }

    /** Drop every relation of the database's namespace. */
    async destroy(record: ResourceRecord<typeof DatabaseKind>): Promise<void> {
        await this.drop(DurableObjectDatabaseHost.namespace(ResourceRecord.reference(record)));
    }

    /** Drop a namespace's relations, such as a database living only while an instance runs. */
    async drop(namespace: string): Promise<void> {
        // read and drop the relations behind the storage's queue, in one transaction
        await this.queue.run(async () => {
            await this.storage.transaction(async () => {
                // drop the tables' triggers first, which read tables dropped before theirs
                const tables = await this.relations(namespace);
                const triggers = this.storage.sql
                    .exec("SELECT name, tbl_name FROM sqlite_schema WHERE type = 'trigger'")
                    .toArray()
                    .filter((row) => tables.includes(schema.string().parse(row["tbl_name"])))
                    .map((row) => schema.string().parse(row["name"]));
                for (const name of triggers) {
                    this.storage.sql.exec(`DROP TRIGGER IF EXISTS ${quote(name)}`);
                }

                // check the references between the tables once every one is dropped, whatever their order
                this.storage.sql.exec("PRAGMA defer_foreign_keys = ON");
                for (const name of tables) {
                    this.storage.sql.exec(`DROP TABLE IF EXISTS ${quote(name)}`);
                }
            });
        }, this.environment);
    }

    /** Diff the applied state against the desired states. */
    plan(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
    ): Promise<Plan> {
        const namespace = DurableObjectDatabaseHost.namespace(ResourceRecord.reference(record));
        const connect = (tables: readonly Table[]) => this.connect(tables, { namespace });

        return DatabaseHandle.plan(
            connect,
            desired.map((state) => state.tables.sqlite),
            record.id,
        );
    }

    /** Apply the reviewed plan, refusing one that changed since review. */
    apply(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
        digest: Digest,
    ): Promise<void> {
        // apply the plan in the database's namespace
        const namespace = DurableObjectDatabaseHost.namespace(ResourceRecord.reference(record));
        const connect = (tables: readonly Table[]) => this.connect(tables, { namespace });
        const states = desired.map((state) => state.tables.sqlite);

        return DatabaseHandle.apply(connect, states, digest, record.id);
    }

    /** Open the tables of the desired states, and a replica's local tables once migrated. */
    open(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
    ): Promise<DatabaseHandle> {
        const namespace = DurableObjectDatabaseHost.namespace(ResourceRecord.reference(record));
        const connect = (tables: readonly Table[]) => this.connect(tables, { namespace });

        return DatabaseHandle.open(
            connect,
            desired.map((state) => state.tables.sqlite),
            record.id,
        );
    }

    /** Copy a database's rows into a store as content-addressed pages, returning the digest. */
    snapshot(...copy: Parameters<DatabaseHost["snapshot"]>): Promise<Digest> {
        return DatabaseHandle.snapshotter(this).snapshot(...copy);
    }

    /** Restore a copy's rows into a database, onto a base copy it already has when given. */
    restore(...copy: Parameters<DatabaseHost["restore"]>): Promise<void> {
        return DatabaseHandle.snapshotter(this).restore(...copy);
    }

    /** Count the object's database bytes: its whole SQLite storage, as Cloudflare bills it. */
    bytes(): number {
        return this.storage.sql.databaseSize;
    }

    /** Count the object's database bytes, and take the rows read and written since last time. */
    measure(): DurableObjectMeasurement {
        const rows = this.#rows;
        this.#rows = { rowsRead: 0, rowsWritten: 0 };

        return { bytes: this.bytes(), ...rows };
    }

    /** Answer an operation another object sends on this object's databases. */
    async answer(
        request: Request,
        keeper: (scope: string) => DurableObjectStub,
    ): Promise<Response> {
        // run the operation in the body
        const body = DurableObjectDatabaseOperation.parse(await request.json());
        const content = (scope: string) => {
            const authorization = request.headers.get("authorization");
            if (authorization === null) {
                throw DatabaseError.of(
                    "CONTENT_UNAVAILABLE",
                    "copy content is unavailable: the copy is missing the sender's credential",
                );
            }

            return new DurableObjectContentStore(
                keeper(scope),
                authorization,
                this.environment.crypto,
            );
        };
        switch (body.operation) {
            case "provision":
                return Response.json(await this.provision(body.record));
            case "destroy":
                await this.destroy(body.record);

                return Response.json({});
            case "plan":
                return Response.json(reviewed(await this.plan(body.record, body.desired)));
            case "apply":
                await this.apply(body.record, body.desired, body.digest);

                return Response.json({});
            case "snapshot": {
                await using opened = await this.#opened(body.record, body.desired);
                const store = content(body.record.scope);

                return Response.json({ digest: await DatabaseHandle.snapshot(opened, store) });
            }
            case "restore": {
                const { record, desired, digest, base } = body;
                await using opened = await this.#opened(record, desired);
                const store = content(record.scope);
                await DatabaseHandle.restore(opened, digest, store, undefined, base);

                return Response.json({});
            }
            case "measure":
                return Response.json(this.measure());
            case "wait":
                return Response.json(await this.#waitCommit(body.name, body.after));
            case "read":
                return Response.json({ rows: await this.#read(body) });
        }
    }

    /** Answer a read or a commit wait another object sends, refusing every other operation. */
    async read(request: Request): Promise<Response> {
        // wait for a database's next commit as a reader asks
        const body = DurableObjectDatabaseOperation.parse(await request.json());
        if (body.operation === "wait") {
            return Response.json(await this.#waitCommit(body.name, body.after));
        }
        // refuse anything but a read
        else if (body.operation !== "read") {
            throw DatabaseError.of(
                "QUERY_INVALID",
                `operation '${body.operation}' is invalid: this object only accepts reads`,
            );
        }

        return Response.json({ rows: await this.#read(body) });
    }

    /** Wait for a database's commit past a position, for a wait's time at most. */
    async #waitCommit(name: string, after: CommitPosition | undefined): Promise<CommitWait> {
        // find the connection the host opened to the database
        const connection = this.#connections.get(name);
        if (connection === undefined) {
            throw DatabaseError.of(
                "CONNECTION_UNAVAILABLE",
                `connection to database '${name}' is unavailable: this object has not opened it`,
            );
        }

        // wait for a commit past the reader's generation, at once for another connection's
        const { id, database } = connection;
        const generation = after?.connection === id ? after.generation : undefined;
        const signal = this.environment.clock.timeout(COMMIT_WAIT_MILLISECONDS);
        let committed: string[] = [];
        await database.log
            .waitUntil(
                async (tables) => {
                    committed = [...tables];

                    return tables.size > 0;
                },
                signal,
                generation,
            )
            .catch((error: unknown) => {
                // end quietly at the wait's time, passing any other failure on
                if (!signal.aborted) {
                    throw error;
                }
            });

        // answer the position reached and the tables committed on the way
        const position = { connection: id, generation: database.state.commits.generation };

        return { position, tables: committed };
    }

    /** Read a statement's rows in JSON form, undoing and refusing a statement that writes. */
    #read(statement: SQLStatement & { readonly rows: "values" | "objects" }): Promise<unknown[]> {
        const parameters = (statement.parameters ?? []).map((value) => SQLValue.toDriver(value));

        return this.transaction(async (transaction) => {
            // read the rows in the open transaction, which the refusal undoes
            const query = new DurableObjectQuery(this, { kind: "joined", transaction });
            const read = await query.read(statement.sql, parameters, statement.rows);
            if (read.rowsWritten > 0) {
                throw DatabaseError.of(
                    "QUERY_INVALID",
                    "read is invalid: a read of another object wrote rows",
                );
            }

            return read.rows.map(encoded);
        });
    }

    /** Open a database for one copy, closing it when disposed. */
    async #opened(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
    ): Promise<DatabaseHandle & AsyncDisposable> {
        const handle = await this.open(record, desired);

        return Object.assign(handle, { [Symbol.asyncDispose]: () => handle.close() });
    }

    /** List the relations of a database's namespace. */
    async relations(namespace: string): Promise<string[]> {
        const prefix = relation("", namespace);
        const rows = this.storage.sql
            .exec(
                "SELECT name FROM sqlite_schema WHERE type = 'table' AND substr(name, 1, ?) = ?",
                prefix.length,
                prefix,
            )
            .toArray();

        return rows.map((row) => schema.string().parse(row["name"]));
    }
}

/** A Durable Object's stub, which sends requests to the object. */
export interface DurableObjectStub {
    /** Send the object a request. */
    readonly fetch: (request: Request) => Promise<Response>;
}

/** Where a residency's objects and storage stay: a Cloudflare jurisdiction, or none in workerd. */
export type Jurisdiction = "eu" | "us" | "none";

/** The jurisdictions a namespace narrows to. */
export const Jurisdiction = {
    /** Narrow a namespace to a jurisdiction's objects, the whole namespace for none. */
    narrow(namespace: DurableObjectNamespace, jurisdiction: Jurisdiction): DurableObjectNamespace {
        return jurisdiction === "none" ? namespace : namespace.jurisdiction(jurisdiction);
    },
};

/** The Durable Objects of a binding, as a Worker addresses them. */
export interface DurableObjectNamespace {
    /** Derive the identifier of the object of a name. */
    readonly idFromName: (name: string) => unknown;
    /** Get an object's stub, hinting where a new one starts. */
    readonly get: (id: unknown, options?: { readonly locationHint?: string }) => DurableObjectStub;
    /** Narrow the namespace to the objects in a jurisdiction. */
    readonly jurisdiction: (name: Exclude<Jurisdiction, "none">) => DurableObjectNamespace;
}

/** The bindings of a database's object: the partition objects that store its copies' content. */
export interface DurableObjectDatabaseBindings {
    /** The partitions' objects, a space's keyed by its space. */
    readonly PARTITION: DurableObjectNamespace;
    /** The jurisdiction that stores the partitions' objects. */
    readonly DESTACK_JURISDICTION: Jurisdiction;
}

/** Write a row of values, or of columns by name, in JSON form. */
function encoded(row: unknown): SQLValue[] | Record<string, SQLValue> {
    return Array.isArray(row)
        ? row.map((value) => SQLValue.fromDriver(value))
        : Object.fromEntries(
              Object.entries(schema.record(schema.string(), schema.unknown()).parse(row)).map(
                  ([column, value]) => [column, SQLValue.fromDriver(value)],
              ),
          );
}

/** Select the plan fields its reviewers see, which its digest covers. */
function reviewed(plan: Plan): Plan {
    const { steps, deferred } = plan;

    return {
        steps: steps.map(({ action, target, risk, detail, fields }) => ({
            action,
            target,
            risk,
            detail,
            ...(fields === undefined ? {} : { fields }),
        })),
        ...(deferred === undefined ? {} : { deferred }),
    };
}
