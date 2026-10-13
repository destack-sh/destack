import type { Digest } from "@destack/schema";
import { schema } from "@destack/schema";
import { type KindState, Plan, ResourceRecord } from "@destack/resource";
import { DatabaseHandle } from "../database/handle.ts";
import type { DatabaseKind } from "../declare/database.ts";
import { quote } from "../dialect/quote.ts";
import type { DatabaseHost } from "../provider/provider.ts";
import { sql } from "../sql/index.ts";
import { Table } from "../table/table.ts";
import { connectPostgres } from "./connection.ts";
import { PostgresConnector } from "./connector.ts";

/** Keep databases in one PostgreSQL cluster database, one schema per database resource. */
export class PostgresDatabaseHost implements DatabaseHost {
    /** The provider code of PostgreSQL databases. */
    readonly provider = "postgresql";
    /** The connector to the cluster database, as a role able to create and drop schemas. */
    readonly connector: PostgresConnector;

    /** Keep databases in the cluster database a PostgreSQL URL selects. */
    constructor(cluster: URL) {
        this.connector = new PostgresConnector(cluster);
    }

    /** Create the database's schema for the resource. */
    async provision(record: ResourceRecord<typeof DatabaseKind>): Promise<{ reference: string }> {
        // create the schema unless it exists
        await using connection = await this.#connect(record.id);
        await connection.executeScript(`CREATE SCHEMA IF NOT EXISTS ${quote(record.id)}`);

        // start the log in a schema with no tables yet
        const [found] = await connection.execute(
            sql`SELECT count(*)::int AS tables FROM pg_class WHERE relnamespace = ${record.id}::regnamespace AND relkind = 'r'`,
            schema.object({ tables: schema.number().int().nonnegative() }),
        );
        if (found?.tables === 0) {
            await connection.log.create(record.scope);
        }

        return { reference: record.id };
    }

    /** Count the bytes a database keeps: its schema's tables with their indexes and TOAST. */
    async bytes(reference: string): Promise<number> {
        // sum the total size of each table in the schema, refusing a missing schema
        await using connection = await this.#connect(reference);
        const [row] = await connection.execute(
            sql`SELECT coalesce(sum(pg_total_relation_size(oid)), 0)::float8 AS bytes FROM pg_class WHERE relnamespace = ${reference}::regnamespace AND relkind = 'r'`,
            schema.object({ bytes: schema.number().int().nonnegative() }),
        );
        if (row === undefined) {
            throw new TypeError("the size sum returned no row");
        }

        return row.bytes;
    }

    /** Drop the database's schema with its contents. */
    async destroy(record: ResourceRecord<typeof DatabaseKind>): Promise<void> {
        const reference = ResourceRecord.reference(record);
        await using connection = await this.#connect(reference);
        await connection.executeScript(`DROP SCHEMA ${quote(reference)} CASCADE`);
    }

    /** Diff the applied state against the desired states. */
    plan(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<Plan> {
        const connect = (tables: readonly Table[]) =>
            connectPostgres(this.connector.url(ResourceRecord.reference(record)), tables);

        return DatabaseHandle.plan(
            connect,
            desired.map((state) => state.tables.postgresql),
        );
    }

    /** Apply the reviewed plan, refusing one that changed since review. */
    apply(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
        digest: Digest,
    ): Promise<void> {
        const connect = (tables: readonly Table[]) =>
            connectPostgres(this.connector.url(ResourceRecord.reference(record)), tables);
        const states = desired.map((state) => state.tables.postgresql);

        return DatabaseHandle.apply(connect, states, digest, record.id);
    }

    /** Open the tables of the desired states, and a replica's local tables once migrated. */
    open(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<DatabaseHandle> {
        const connect = (tables: readonly Table[]) =>
            connectPostgres(this.connector.url(ResourceRecord.reference(record)), tables);

        return DatabaseHandle.open(
            connect,
            desired.map((state) => state.tables.postgresql),
        );
    }

    /** Connect to one schema of the cluster. */
    #connect(namespace: string) {
        return connectPostgres(this.connector.url(namespace));
    }

    /** Copy a database's rows into a store as content-addressed pages, returning the digest. */
    snapshot(...copy: Parameters<DatabaseHost["snapshot"]>): Promise<Digest> {
        return DatabaseHandle.snapshotter(this).snapshot(...copy);
    }

    /** Restore a copy's rows into a database, onto a base copy it already has when given. */
    restore(...copy: Parameters<DatabaseHost["restore"]>): Promise<void> {
        return DatabaseHandle.snapshotter(this).restore(...copy);
    }
}
