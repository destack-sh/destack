import { defineSchema, Digest, schema } from "@destack/schema";
import { type KindState, Plan, ResourceId, ResourceRecord } from "@destack/resource";
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

/** The path below which an object answers operations on the databases it keeps. */
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

/** An operation on the databases an object keeps, as another object sends it. */
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
    SQLStatement.extend({
        /** Read rows of the object's storage, refusing a statement that writes. */
        operation: schema.literal("read"),
        /** Whether the rows come as arrays of values or as objects by column name. */
        rows: schema.enum(["values", "objects"]),
    }),
]);
/** An operation on the databases an object keeps, as another object sends it. */
export type DurableObjectDatabaseOperation = schema.Infer<typeof DurableObjectDatabaseOperation>;

/** The bytes of an object's databases, and the rows they read and wrote since last measured. */
export const DurableObjectMeasurement = defineSchema(
    schema.object({
        /** The bytes of the object's SQLite storage. */
        bytes: schema.number().int().nonnegative(),
        /** The rows read since the last measure. */
        rowsRead: schema.number().int().nonnegative(),
        /** The rows written since the last measure. */
        rowsWritten: schema.number().int().nonnegative(),
    }),
);
/** The bytes of an object's databases, and the rows they read and wrote since last measured. */
export type DurableObjectMeasurement = schema.Infer<typeof DurableObjectMeasurement>;

/** The options of a database in a Durable Object's SQLite storage. */
export interface DurableObjectConnectOptions extends SqliteConnectOptions {
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
                  new DatabaseError("TRANSACTION_REQUIRED", "the storage transaction closed"),
              );
    }

    /** Close the transaction once its work settled. */
    close(): void {
        this.#isOpen = false;
    }
}

/** Keep databases in one Durable Object's SQLite storage, each in its resource's namespace. */
export class DurableObjectDatabaseHost implements DatabaseHost {
    /** The provider code of databases in Durable Objects. */
    readonly provider = DURABLE_OBJECT_PROVIDER;
    /** The object's SQLite storage. */
    readonly storage: DurableObjectStorage;
    /** The work queue of the storage's one connection, one transaction at a time. */
    readonly queue = new SqliteWorkQueue();
    /** The rows the databases read and wrote since the last measure. */
    #rows = { rowsRead: 0, rowsWritten: 0 };

    /** Keep databases in an object's storage. */
    constructor(storage: DurableObjectStorage) {
        this.storage = storage;
    }

    /** Open a database in the storage, with the object as sole writer behind the queue. */
    connect<Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>>(
        tables: declaration.Database<Models> | readonly Table[] = [],
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
        });
    }

    /** Open a database of the storage whose statements run in a mode. */
    #open<Models extends Readonly<Record<string, Model>>>(
        tables: declaration.Database<Models> | readonly Table[],
        options: DurableObjectConnectOptions,
    ): SqliteDatabase<DurableObjectClient, Models> {
        const { openChannel, namespace } = options;

        return new SqliteDatabase(
            new DurableObjectClient(this, { kind: "queued" }),
            tables,
            "embedded",
            openChannel,
            namespace,
        );
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
            throw new DatabaseError("INVALID_RECORD", `no Durable Object database: ${reference}`);
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

    /** Drop every relation of a namespace, such as a database kept only while an instance runs. */
    async drop(namespace: string): Promise<void> {
        // read and drop each relation behind the storage's queue
        await this.queue.run(async () => {
            for (const name of await this.relations(namespace)) {
                this.storage.sql.exec(`DROP TABLE IF EXISTS "${name.replaceAll('"', '""')}"`);
            }
        });
    }

    /** Diff the applied state against the desired states. */
    plan(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<Plan> {
        const namespace = DurableObjectDatabaseHost.namespace(ResourceRecord.reference(record));
        const connect = (tables: readonly Table[]) => this.connect(tables, { namespace });

        return DatabaseHandle.plan(
            connect,
            desired.map((state) => state.tables.sqlite),
        );
    }

    /** Apply the reviewed plan, refusing one that changed since review. */
    apply(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
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
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<DatabaseHandle> {
        const namespace = DurableObjectDatabaseHost.namespace(ResourceRecord.reference(record));
        const connect = (tables: readonly Table[]) => this.connect(tables, { namespace });

        return DatabaseHandle.open(
            connect,
            desired.map((state) => state.tables.sqlite),
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
                throw new DatabaseError(
                    "CONTENT_UNAVAILABLE",
                    "a copy needs the sender's credential",
                );
            }

            return new DurableObjectContentStore(keeper(scope), authorization);
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
            case "read":
                return Response.json({ rows: await this.#read(body) });
        }
    }

    /** Answer a read another object sends, refusing every other operation. */
    async read(request: Request): Promise<Response> {
        const body = DurableObjectDatabaseOperation.parse(await request.json());
        if (body.operation !== "read") {
            throw new DatabaseError(
                "INVALID_QUERY",
                `this object answers reads alone, not ${body.operation}`,
            );
        }

        return Response.json({ rows: await this.#read(body) });
    }

    /** Read a statement's rows in JSON form, undoing and refusing a statement that writes. */
    #read(statement: SQLStatement & { readonly rows: "values" | "objects" }): Promise<unknown[]> {
        const parameters = (statement.parameters ?? []).map((value) => SQLValue.toDriver(value));

        return this.transaction(async (transaction) => {
            // read the rows in the open transaction, which the refusal undoes
            const query = new DurableObjectQuery(this, { kind: "joined", transaction });
            const read = await query.read(statement.sql, parameters, statement.rows);
            if (read.rowsWritten > 0) {
                throw new DatabaseError("INVALID_QUERY", "a read of another object wrote rows");
            }

            return read.rows.map(encoded);
        });
    }

    /** Open a database for one copy, closing it when disposed. */
    async #opened(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
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

/** A Durable Object's stub, through which requests reach the object. */
export interface DurableObjectStub {
    /** Send the object a request. */
    readonly fetch: (request: Request) => Promise<Response>;
}

/** Where a residency's objects and storage stay. */
export type Jurisdiction = "eu" | "us" | "local";

/** The jurisdictions a namespace narrows to. */
export const Jurisdiction = {
    /** Narrow a namespace to the objects a jurisdiction keeps, the whole namespace locally. */
    narrow(namespace: DurableObjectNamespace, jurisdiction: Jurisdiction): DurableObjectNamespace {
        return jurisdiction === "local" ? namespace : namespace.jurisdiction(jurisdiction);
    },
};

/** The Durable Objects of a binding, as a Worker addresses them. */
export interface DurableObjectNamespace {
    /** Derive the identifier of the object of a name. */
    readonly idFromName: (name: string) => unknown;
    /** Get an object's stub, hinting where a new one starts. */
    readonly get: (id: unknown, options?: { readonly locationHint?: string }) => DurableObjectStub;
    /** Narrow the namespace to the objects kept in a jurisdiction. */
    readonly jurisdiction: (name: Exclude<Jurisdiction, "local">) => DurableObjectNamespace;
}

/** The bindings of a database's object: the partition objects keeping its copies' content. */
export interface DurableObjectDatabaseEnvironment {
    /** The partitions' objects, a space's own keyed by its space. */
    readonly PARTITION: DurableObjectNamespace;
    /** The jurisdiction keeping the partitions' objects. */
    readonly DESTACK_JURISDICTION: Jurisdiction;
}

/** A Durable Object answering operations on its databases. */
export class DurableObjectDatabase {
    /** The databases the object keeps. */
    readonly host: DurableObjectDatabaseHost;
    /** The partitions' objects, in their jurisdiction. */
    readonly #partitions: DurableObjectNamespace;

    /** Keep databases in the object's storage, with partition objects from Worker bindings. */
    constructor(
        state: { readonly storage: DurableObjectStorage },
        environment: DurableObjectDatabaseEnvironment,
    ) {
        this.host = new DurableObjectDatabaseHost(state.storage);
        this.#partitions = Jurisdiction.narrow(
            environment.PARTITION,
            environment.DESTACK_JURISDICTION,
        );
    }

    /** Answer an operation on a kept database, copying through its partition's object. */
    fetch(request: Request): Promise<Response> {
        return this.host.answer(request, (scope) =>
            this.#partitions.get(this.#partitions.idFromName(scope)),
        );
    }
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

/** Keep the plan fields its reviewers see, which its digest covers. */
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
