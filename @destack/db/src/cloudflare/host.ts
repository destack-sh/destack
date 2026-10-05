import { Digest, schema } from "@destack/schema";
import { type KindState, Plan, ResourceId, type ResourceRecord } from "@destack/resource";
import type { DatabaseHandle } from "../blob/handle.ts";
import { type DatabaseKind, DatabaseSpec } from "../declare/database.ts";
import { DatabaseError } from "../error/error.ts";
import { mergeStates } from "../migration/merge.ts";
import { DatabaseState } from "../migration/state.ts";
import type { DatabaseHost } from "../provider/provider.ts";
import { relation } from "../table/namespace.ts";
import { Table } from "../table/table.ts";
import type { DurableObjectStorage } from "./client.ts";
import { connect } from "./connection.ts";

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

/** An operation on a database an object keeps, as another object sends it. */
const DatabaseOperation = schema.discriminatedUnion("operation", [
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
]);
/** An operation on a database an object keeps, as another object sends it. */
export type DatabaseOperation = schema.Infer<typeof DatabaseOperation>;

/** The prefix of the references naming databases in an object's storage. */
const REFERENCE_PREFIX = `${DURABLE_OBJECT_PROVIDER}:`;

/** Keep databases in one Durable Object's SQLite storage, each in a namespace named by its resource. */
export class DurableObjectDatabaseHost implements DatabaseHost {
    /** The provider code of databases in Durable Objects. */
    readonly provider = DURABLE_OBJECT_PROVIDER;
    /** The object's SQLite storage. */
    readonly storage: DurableObjectStorage;

    /** Keep databases in an object's storage. */
    constructor(storage: DurableObjectStorage) {
        this.storage = storage;
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
        await using database = connect(this.storage, [], { namespace: record.id });
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
        for (const name of await this.relations(namespace)) {
            this.storage.sql.exec(`DROP TABLE IF EXISTS "${name.replaceAll('"', '""')}"`);
        }
    }

    /** Diff the applied state against the desired states. */
    async plan(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<Plan> {
        await using database = connect(this.storage, [], { namespace: record.id });

        return await database.plan(mergeStates(desired.map((state) => state.tables.sqlite)));
    }

    /** Apply the reviewed plan, refusing one that changed since review. */
    async apply(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
        digest: Digest,
    ): Promise<void> {
        // plan again and apply the plan only while it matches the reviewed digest
        await using database = connect(this.storage, [], { namespace: record.id });
        const plan = await database.plan(mergeStates(desired.map((state) => state.tables.sqlite)));
        if ((await Plan.digest(plan)) !== digest) {
            throw new DatabaseError("PLAN_CHANGED", `plan of ${record.id} changed since review`);
        }
        await database.apply(plan);
    }

    /** Open the tables the desired states describe, and a replica's local tables beside them once migrated. */
    async open(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<DatabaseHandle> {
        // open the desired tables and reopen them with a replica's tables once migrated
        const states = desired.map((state) => state.tables.sqlite);
        const described = mergeStates(states).declared.map((state) => Table.describe(state));
        const namespace = record.id;
        let database = connect(this.storage, described, { namespace });

        return {
            get database() {
                return database;
            },
            migrate: async (beside) => {
                await database.migrate(beside, { states });
                await database.close();
                database = connect(this.storage, [...described, ...beside], { namespace });
            },
            close: () => database.close(),
        };
    }

    /** Answer an operation another object sends on a database this object keeps. */
    async answer(request: Request): Promise<Response> {
        // run the operation the body names
        const body = DatabaseOperation.parse(await request.json());
        const { record } = body;
        switch (body.operation) {
            case "provision":
                return Response.json(await this.provision(record));
            case "destroy":
                await this.destroy(record);

                return Response.json({});
            case "plan":
                return Response.json(reviewed(await this.plan(record, body.desired)));
            case "apply":
                await this.apply(record, body.desired, body.digest);

                return Response.json({});
        }
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

/** A Durable Object answering operations on the databases it keeps, such as an instance's object before its workload first starts. */
export class DurableObjectDatabase {
    /** The databases the object keeps. */
    readonly #databases: DurableObjectDatabaseHost;

    /** Keep databases in the object's storage. */
    constructor(state: { readonly storage: DurableObjectStorage }) {
        this.#databases = new DurableObjectDatabaseHost(state.storage);
    }

    /** Answer an operation on a database the object keeps. */
    fetch(request: Request): Promise<Response> {
        return this.#databases.answer(request);
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
