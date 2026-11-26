import type { DatabaseEnvironment } from "../database/connection.ts";
import type { Digest } from "@destack/schema";
import { schema } from "@destack/schema";
import { Plan, ResourceRecord } from "@destack/resource";
import { DatabaseHandle } from "../database/handle.ts";
import type { DatabaseKind, DatabaseSchema } from "../declare/database.ts";
import { quote } from "../dialect/quote.ts";
import type { DatabaseCluster, DatabaseHost } from "../provider/provider.ts";
import { sql } from "../sql/index.ts";
import { Table } from "../table/table.ts";
import { connectPostgres } from "./connection.ts";
import { PostgresPool } from "./pool.ts";
import { PostgresConnector } from "./connector.ts";
import type { DatabaseState } from "../migration/state.ts";

/** Store databases in one PostgreSQL cluster database, one schema per database, over one pool. */
export class PostgresDatabaseHost implements DatabaseCluster {
    /** The provider code of PostgreSQL databases. */
    readonly provider = "postgresql";
    /** Provisioning creates the database's schema, so it is not pure. */
    readonly isPure = false;
    /** The connector to the cluster database, as a role able to create and drop schemas. */
    readonly connector: PostgresConnector;

    /** The machine's clock and randomness, which the databases stamp changes at and draw from. */
    readonly environment: DatabaseEnvironment;
    /** The pool every schema's database runs on, with one connection listening for all. */
    readonly pool: PostgresPool;

    /** Store databases in the cluster a PostgreSQL URL selects, in the machine's environment. */
    constructor(cluster: URL, environment: DatabaseEnvironment) {
        this.connector = new PostgresConnector(cluster);
        this.environment = environment;
        this.pool = new PostgresPool(cluster.href, environment);
    }

    /** Close the pool at the end of the host's scope. */
    [Symbol.asyncDispose](): Promise<void> {
        return this.close();
    }

    /** Close the pool after its databases closed. */
    close(): Promise<void> {
        return this.pool.end();
    }

    /** Create the database's schema for the resource, and start the scope's log in it once. */
    async provision(record: ResourceRecord<typeof DatabaseKind>): Promise<{ reference: string }> {
        // create the schema, then start the log in one with no tables yet
        const reference = this.reference(record.scope, record.id);
        await this.create(reference);
        await using connection = await this.connect(reference);
        const [found] = await connection.execute(
            sql`SELECT count(*)::int AS tables FROM pg_class WHERE relnamespace = ${reference}::regnamespace AND relkind = 'r'`,
            schema.object({ tables: schema.number().int().nonnegative() }),
        );
        if (found?.tables === 0) {
            await connection.log.create(record.scope);
        }

        return { reference };
    }

    /** Name the database of an identifier in a scope: its schema, named by the identifier. */
    reference(_scope: string, id: string): string {
        return id;
    }

    /** List the scopes whose own schema the cluster keeps, by a prefix of their identifiers. */
    scopes(prefix: string): Promise<string[]> {
        return this.schemas(prefix);
    }

    /** Create an empty database schema unless it exists. */
    async create(reference: string): Promise<void> {
        await using connection = await this.connect(reference);
        await connection.executeScript(`CREATE SCHEMA IF NOT EXISTS ${quote(reference)}`);
    }

    /** List the schemas of the cluster whose names start with a prefix, by name. */
    async schemas(prefix: string): Promise<string[]> {
        // read the names from the catalog through the public schema
        await using connection = await this.connect("public");
        const rows = await connection.execute(
            sql`SELECT nspname AS name FROM pg_namespace WHERE starts_with(nspname, ${prefix}) ORDER BY nspname`,
            schema.object({ name: schema.string() }),
        );

        return rows.map((row) => row.name);
    }

    /** Count a database's bytes: its schema's tables with their indexes and TOAST. */
    async bytes(reference: string): Promise<number> {
        // sum the total size of each table in the schema, refusing a missing schema
        await using connection = await this.connect(reference);
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
        await this.drop(ResourceRecord.reference(record));
    }

    /** Drop a database's schema with its contents. */
    async drop(reference: string): Promise<void> {
        await using connection = await this.connect(reference);
        await connection.executeScript(`DROP SCHEMA ${quote(reference)} CASCADE`);
    }

    /** Diff the applied state against the desired states. */
    plan(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
    ): Promise<Plan> {
        const connect = (tables: readonly Table[]) =>
            this.connect(ResourceRecord.reference(record), tables);

        return DatabaseHandle.plan(
            connect,
            desired.map((state) => state.tables.postgresql),
            record.id,
        );
    }

    /** Apply the reviewed plan, refusing one that changed since review. */
    apply(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
        digest: Digest,
    ): Promise<void> {
        const connect = (tables: readonly Table[]) =>
            this.connect(ResourceRecord.reference(record), tables);
        const states = desired.map((state) => state.tables.postgresql);

        return DatabaseHandle.apply(connect, states, digest, record.id);
    }

    /** Open the tables of the desired states, and a replica's local tables once migrated. */
    open(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
    ): Promise<DatabaseHandle> {
        const connect = (tables: readonly Table[]) =>
            this.connect(ResourceRecord.reference(record), tables);

        return DatabaseHandle.open(
            connect,
            desired.map((state) => state.tables.postgresql),
            record.id,
        );
    }

    /** Connect to one schema of the cluster on the shared pool. */
    connect(reference: string, tables: DatabaseSchema | readonly Table[] = []) {
        return connectPostgres(this.pool, tables, {
            environment: this.environment,
            schema: reference,
        });
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
