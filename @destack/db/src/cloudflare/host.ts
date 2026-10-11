import { defineSchema, Digest, schema } from "@destack/schema";
import { type KindState, Plan, ResourceId, type ResourceRecord } from "@destack/resource";
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
import { type SqliteConnectOptions, SqliteDatabase } from "../sqlite/database.ts";
import {
    type DurableObjectCursor,
    DurableObjectClient,
    type DurableObjectStorage,
} from "./client.ts";
import { DurableObjectContentStore } from "./content.ts";

/** The provider code of databases in Durable Objects' SQLite storage. */
export const DURABLE_OBJECT_PROVIDER = "durable-object";

/** The path below which an object answers operations on the databases it keeps. */
export const DATABASE_PATH = "/.destack/database";

/** A database resource as an operation names it. */
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
        /** Copy the database's rows into the content the sender keeps, answering the copy's digest. */
        operation: schema.literal("snapshot"),
        /** The database resource. */
        record: DatabaseRecord,
        /** The desired states. */
        desired: schema.array(DatabaseState),
    }),
    schema.object({
        /** Restore a copy the sender keeps into the database, onto the base copy it holds already when given. */
        operation: schema.literal("restore"),
        /** The database resource. */
        record: DatabaseRecord,
        /** The desired states. */
        desired: schema.array(DatabaseState),
        /** The copy's digest. */
        digest: Digest,
        /** The digest of the copy the database holds already, absent for an empty database. */
        base: Digest.exactOptional(),
    }),
    schema.object({
        /** Count the bytes of every database the object keeps, and take the rows they read and wrote. */
        operation: schema.literal("measure"),
    }),
]);
/** An operation on the databases an object keeps, as another object sends it. */
export type DurableObjectDatabaseOperation = schema.Infer<typeof DurableObjectDatabaseOperation>;

/** The bytes an object's databases keep, and the rows they read and wrote since it was last measured. */
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
/** The bytes an object's databases keep, and the rows they read and wrote since it was last measured. */
export type DurableObjectMeasurement = schema.Infer<typeof DurableObjectMeasurement>;

/** The options of a database in a Durable Object's SQLite storage. */
export interface DurableObjectConnectOptions extends SqliteConnectOptions {
    /** The database's namespace in the storage several databases share, such as its identifier; absent for the storage's only database. */
    readonly namespace?: string;
}

/** The prefix of the references naming databases in an object's storage. */
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

/** Keep databases in one Durable Object's SQLite storage, each in a namespace named by its resource. */
export class DurableObjectDatabaseHost implements DatabaseHost {
    /** The provider code of databases in Durable Objects. */
    readonly provider = DURABLE_OBJECT_PROVIDER;
    /** The object's SQLite storage. */
    readonly storage: DurableObjectStorage;
    /** The work queue the storage's one connection runs its databases' work in, one transaction at a time. */
    readonly queue = new SqliteWorkQueue();
    /** The rows the databases read and wrote since the last measure. */
    #rows = { rowsRead: 0, rowsWritten: 0 };

    /** Keep databases in an object's storage. */
    constructor(storage: DurableObjectStorage) {
        this.storage = storage;
    }

    /** Open a database in the storage, the object being its sole writer, its work behind the storage's queue. */
    connect<Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>>(
        tables: declaration.Database<Models> | readonly Table[] = [],
        options: DurableObjectConnectOptions = {},
    ): SqliteDatabase<DurableObjectClient, Models> {
        return this.#open(tables, options);
    }

    /** Run work in a storage transaction behind the queue, the transaction closing once the work settles. */
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

    /** Name a database in the storage of the object keeping it: `durable-object:<resource>`. */
    static reference(resource: ResourceId): string {
        return `${REFERENCE_PREFIX}${resource}`;
    }

    /** Read the namespace of a database a reference names, refusing a reference of another provider. */
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
        await this.drop(record.id);
    }

    /** Drop every relation of a namespace, such as a database a runtime keeps only while an instance runs. */
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
        const connect = (tables: readonly Table[]) =>
            this.connect(tables, { namespace: record.id });

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
        const connect = (tables: readonly Table[]) =>
            this.connect(tables, { namespace: record.id });
        const states = desired.map((state) => state.tables.sqlite);

        return DatabaseHandle.apply(connect, states, digest, record.id);
    }

    /** Open the tables the desired states describe, and a replica's local tables beside them once migrated. */
    open(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<DatabaseHandle> {
        const connect = (tables: readonly Table[]) =>
            this.connect(tables, { namespace: record.id });

        return DatabaseHandle.open(
            connect,
            desired.map((state) => state.tables.sqlite),
        );
    }

    /** Copy a database's rows into a store as content-addressed pages, answering the copy's digest. */
    snapshot(...copy: Parameters<DatabaseHost["snapshot"]>): Promise<Digest> {
        return DatabaseHandle.snapshotter(this).snapshot(...copy);
    }

    /** Restore the rows a copy keeps into a database, onto the base copy it holds already when given. */
    restore(...copy: Parameters<DatabaseHost["restore"]>): Promise<void> {
        return DatabaseHandle.snapshotter(this).restore(...copy);
    }

    /** Count the bytes of every database the object keeps: its whole SQLite storage, as Cloudflare bills it. */
    bytes(): number {
        return this.storage.sql.databaseSize;
    }

    /** Count the bytes of every database the object keeps, and take the rows they read and wrote since the last measure. */
    measure(): DurableObjectMeasurement {
        const rows = this.#rows;
        this.#rows = { rowsRead: 0, rowsWritten: 0 };

        return { bytes: this.bytes(), ...rows };
    }

    /** Answer an operation another object sends on the databases this object keeps, copying through the content the object keeping a resource's space keeps under the request's credential. */
    async answer(
        request: Request,
        keeper: (scope: string) => DurableObjectStub,
    ): Promise<Response> {
        // run the operation the body names
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
        }
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
    fetch(request: Request): Promise<Response>;
}

/** Where a residency's objects and storage stay: a Cloudflare jurisdiction, the European Union or the United States, or local for workerd, which enforces none. */
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
    idFromName(name: string): unknown;
    /** Get an object's stub, hinting where a new one starts. */
    get(id: unknown, options?: { readonly locationHint?: string }): DurableObjectStub;
    /** Narrow the namespace to the objects kept in a jurisdiction. */
    jurisdiction(name: Exclude<Jurisdiction, "local">): DurableObjectNamespace;
}

/** The bindings a database's object reads: the partitions' objects keeping the content its copies read and write. */
export interface DurableObjectDatabaseEnvironment {
    /** The partitions' objects, a space's own named by its space. */
    readonly PARTITION: DurableObjectNamespace;
    /** The jurisdiction keeping the partitions' objects. */
    readonly DESTACK_JURISDICTION: Jurisdiction;
}

/** A Durable Object answering operations on the databases it keeps, such as an instance's object before its workload first starts, copying through its databases' partitions' objects. */
export class DurableObjectDatabase {
    /** The databases the object keeps. */
    readonly host: DurableObjectDatabaseHost;
    /** The partitions' objects, in their jurisdiction. */
    readonly #partitions: DurableObjectNamespace;

    /** Keep databases in the object's storage, reaching the partitions' objects through the Worker's bindings. */
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

    /** Answer an operation on a database the object keeps, copying through the object of the database's partition. */
    fetch(request: Request): Promise<Response> {
        return this.host.answer(request, (scope) =>
            this.#partitions.get(this.#partitions.idFromName(scope)),
        );
    }
}

/** Keep the parts of a plan its reviewers see, which its digest covers. */
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
