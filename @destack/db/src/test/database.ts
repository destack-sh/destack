import { copyFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import postgres from "postgres";
import { v7 } from "uuid";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Model } from "../query/model.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { Table } from "../table/table.ts";
import type * as declaration from "../declare/database.ts";
import { declareState } from "../migration/state.ts";
import * as sqlite from "../bun/connection.ts";
import * as postgresql from "../postgres/connection.ts";
import { PostgresDatabase } from "../postgres/database.ts";
import { LOG_EPOCH, LOG_HORIZON, LOG_TABLES } from "../log/schema.ts";
import { createLog } from "../log/trigger.ts";
import type { Channel } from "../channel/channel.ts";
import { channelHub } from "./channel.ts";
import { readState, STATE, type TableState } from "../migration/state.ts";

/** The test dialects: SQLite, and PostgreSQL when DESTACK_TEST_POSTGRES has a server address. */
export const TEST_DIALECTS: readonly Dialect[] = [
    "sqlite",
    ...((process.env["DESTACK_TEST_POSTGRES"] ?? "") === "" ? [] : (["postgresql"] as const)),
];

/** The connections each test database pools: ten workers stay within a server's hundred. */
const TEST_POOL_CONNECTIONS = 4;

/** The migrated SQLite templates, by declared state. */
const templates = new Map<string, Promise<string>>();

/**
 * How long an unclaimed test schema stays for reuse, in milliseconds.
 *
 * Every claim from the server restamps a schema, so only states no test used for an hour expire.
 * Test runs create 50 to 230 schemas of about 175 catalog relations an hour, so an hour keeps up to 40k relations.
 * Each connection's type read and each catalog scan grows with them: 186k relations cost 65 ms and 20 ms.
 */
const SCHEMA_RETENTION_MILLISECONDS = 60 * 60 * 1000;

/**
 * The most expired schemas one test process drops when it starts.
 *
 * A drop of about 100 relations measured 116 ms on a loaded machine, so two add at most about 250 ms to a first test.
 * The tens of processes of one suite run still drop about a hundred schemas, above the hourly creation rate.
 */
const EXPIRED_SCHEMA_DROPS = 2;

/** The PostgreSQL server of this process's tests, opened once. */
let testServer: Promise<TestServer> | undefined;

/**
 * An isolated database for one test: a SQLite memory or file database, or a PostgreSQL schema.
 *
 * Migrated PostgreSQL schemas stay on the server by declared state, claimed through advisory locks and reset per test.
 */
export class TestDatabase<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> {
    /** The first connection. */
    readonly database: TestConnection<Models>;
    /** Open another connection, absent for SQLite in memory. */
    readonly #open: TestConnector | undefined;
    /** Remove the database. */
    readonly #remove: () => Promise<void>;
    /** The other open connections. */
    readonly #connections = new Set<TestConnection>();

    /** Create the test database. */
    private constructor(
        database: TestConnection<Models>,
        open: TestConnector | undefined,
        remove: () => Promise<void>,
    ) {
        this.database = database;
        this.#open = open;
        this.#remove = remove;
    }

    /** Create an isolated database, empty or migrated. */
    static async create<
        Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
    >(
        dialect: Dialect,
        tables: declaration.Database<Models> | readonly Table[],
        options: TestDatabaseOptions = {},
    ): Promise<TestDatabase<Models>> {
        const declared = "tables" in tables ? tables.tables : tables;

        // claim or create a PostgreSQL schema
        if (dialect === "postgresql") {
            const address = process.env["DESTACK_TEST_POSTGRES"];
            if (address === undefined || address === "") {
                throw new TypeError("DESTACK_TEST_POSTGRES has no PostgreSQL server address");
            }
            testServer ??= TestServer.open(address);
            const server = await testServer;

            // claim a migrated schema, kept by this process across tests
            if (options.isMigrated === true) {
                const state = declareState(declared, "postgresql", {
                    isReplica: options.isReplica ?? false,
                });
                const schema = await server.claim(state, tables);
                const open: TestConnector = (connected) => schema.connect(connected);

                return new TestDatabase(await open(tables), open, async () => {
                    schema.isIdle = true;
                });
            }

            // create an empty schema
            const schema = `test_${crypto.randomUUID().replaceAll("-", "")}`;
            await server.administration.unsafe(`CREATE SCHEMA "${schema}"`);
            const open: TestConnector = (connected) =>
                postgresql.connect(server.connect(schema), connected);

            return new TestDatabase(await open(tables), open, async () => {
                await server.administration.unsafe(`DROP SCHEMA "${schema}" CASCADE`);
            });
        }
        // keep empty SQLite in memory
        else if (options.isMigrated !== true && (options.storage ?? "memory") === "memory") {
            return new TestDatabase(
                await sqlite.connect(":memory:", tables),
                undefined,
                async () => {},
            );
        }
        // copy SQLite from the migrated template into a temporary file
        else {
            const directory = await mkdtemp(join(tmpdir(), "destack-test-"));
            const file = join(directory, "test.db");
            if (options.isMigrated === true) {
                await copyFile(await sqliteTemplate(declared, options.isReplica ?? false), file);
            }

            // share each named channel through one hub
            const hubs = new Map<string, () => Channel<unknown>>();
            const openChannel = (name: string) => {
                const party = hubs.get(name) ?? channelHub<unknown>();
                hubs.set(name, party);

                return party();
            };
            const open: TestConnector = (connected) =>
                sqlite.connect(file, connected, { openChannel });

            return new TestDatabase(await open(tables), open, () =>
                rm(directory, { recursive: true }),
            );
        }
    }

    /** Open another connection to the same database. */
    async connect<Other extends Readonly<Record<string, Model>>>(
        tables: declaration.Database<Other> | readonly Table[],
    ): Promise<TestConnection<Other>> {
        // require a reachable database
        if (this.#open === undefined) {
            throw new TypeError("an in-memory SQLite database has no further connections");
        }

        // track the connection, which closes once however often it is closed
        const connection = await this.#open(tables);
        this.#connections.add(connection);

        return connection;
    }

    /** Close every connection and remove the database. */
    async close(): Promise<void> {
        await Promise.all([...this.#connections].map((connection) => connection.close()));
        await this.database.close();
        await this.#remove();
    }
}

/** The options of a test database. */
export interface TestDatabaseOptions {
    /** Where empty SQLite lives. */
    readonly storage?: "memory" | "file";
    /** Whether the database starts migrated to its tables. */
    readonly isMigrated?: boolean;
    /** Whether it migrates as a copy, without trees and references. */
    readonly isReplica?: boolean;
}

/**
 * The PostgreSQL server of one test process, keeping the migrated schemas it claimed until the process exits.
 *
 * The administration connection takes every claim's advisory lock, so the server releases them when the process ends.
 */
class TestServer {
    /** The server address. */
    readonly address: string;
    /** The one connection that manages schemas and locks their claims. */
    readonly administration: postgres.Sql;
    /** The schema names this process locked, including claims still verifying. */
    readonly #locked = new Set<string>();
    /** The adopted schemas this process locked, idle or in use. */
    readonly #schemas: TestSchema[] = [];

    /** Create the server of a connection. */
    private constructor(address: string, administration: postgres.Sql) {
        this.address = address;
        this.administration = administration;
    }

    /** Connect to the server, dropping a few stale unclaimed schemas. */
    static async open(address: string): Promise<TestServer> {
        // skip the array type read, which scans every catalog type
        const server = new TestServer(
            address,
            postgres(address, { max: 1, fetch_types: false, onnotice: postgresql.reportNotice }),
        );

        // drop the oldest unclaimed schemas past retention
        const expired = Date.now() - SCHEMA_RETENTION_MILLISECONDS;
        const schemas = await server.administration<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace
            WHERE starts_with(nspname, 'test_s') AND coalesce(obj_description(oid, 'pg_namespace'), '0')::bigint < ${expired}
            ORDER BY coalesce(obj_description(oid, 'pg_namespace'), '0')::bigint
            LIMIT ${EXPIRED_SCHEMA_DROPS}`;
        for (const { name } of schemas) {
            if (await server.#lockListed(name)) {
                await server.#drop(name, []);
            }
        }

        return server;
    }

    /** Open a connection pool whose queries resolve in a schema. */
    connect(schema: string): postgres.Sql {
        return postgres(this.address, {
            max: TEST_POOL_CONNECTIONS,
            connection: { search_path: schema },
            onnotice: postgresql.reportNotice,
        });
    }

    /** Claim a reset migrated schema: an idle one of this process, a free retained one, or a new one. */
    async claim(
        state: readonly TableState[],
        tables: declaration.Database | readonly Table[],
    ): Promise<TestSchema> {
        // derive the schema names from the digest of the state and the log it migrates beside
        const layout = JSON.stringify([state, createLog("postgresql", "")]);
        const digest = new Uint8Array(
            await crypto.subtle.digest("SHA-256", new TextEncoder().encode(layout)),
        );
        const prefix = `test_s${digest.subarray(0, 8).toHex()}_`;

        // reuse an idle schema this process locked unless a test changed its shape
        for (const schema of this.#schemas.filter((locked) => locked.name.startsWith(prefix))) {
            if (schema.isIdle) {
                schema.isIdle = false;
                const shape = await this.#readShape(schema.name);
                if (shape.fingerprint === schema.shape.fingerprint) {
                    await this.#reset(schema);

                    return schema;
                }
                this.#schemas.splice(this.#schemas.indexOf(schema), 1);
                await this.#drop(schema.name, schema.pools);
            }
        }

        // claim a free schema another process retained
        const listed = await this.administration<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace WHERE starts_with(nspname, ${prefix})`;
        for (const { name } of listed) {
            if (!this.#locked.has(name) && (await this.#lockListed(name))) {
                const schema = await this.#verify(name, state, tables);
                if (schema !== undefined) {
                    await this.#reset(schema);

                    return schema;
                }
            }
        }

        // create a new schema with its claim time
        const name = `${prefix}${crypto.randomUUID().replaceAll("-", "")}`;
        await this.#lock(name);
        await this.administration.unsafe(
            `CREATE SCHEMA "${name}"; COMMENT ON SCHEMA "${name}" IS '${Date.now()}';`,
        );

        // migrate it on a pool the schema keeps
        const pools: postgres.Sql[] = [];
        const database = new PoolDatabase(this.connect(name), tables, pools);
        await database.apply(await database.plan({ declared: state }));
        await database.close();

        return this.#adopt(name, pools);
    }

    /** Take a schema's advisory lock, registered before it is awaited, reporting whether it was free. */
    async #lock(name: string): Promise<boolean> {
        // register the name so concurrent claims of this session skip it
        this.#locked.add(name);
        const isLocked: unknown = (
            await this.administration`
            SELECT pg_try_advisory_lock(hashtext(${name}))`.values()
        )[0]?.[0];
        if (typeof isLocked !== "boolean") {
            throw new TypeError("the advisory lock returned no boolean");
        } else if (!isLocked) {
            this.#locked.delete(name);
        }

        return isLocked;
    }

    /** Take a listed schema's advisory lock, reporting whether it was free and the schema still exists. */
    async #lockListed(name: string): Promise<boolean> {
        if (!(await this.#lock(name))) {
            return false;
        }

        // release a name another process dropped after the listing
        const isPresent: unknown = (
            await this.administration`
            SELECT to_regnamespace(${name}) IS NOT NULL`.values()
        )[0]?.[0];
        if (typeof isPresent !== "boolean") {
            throw new TypeError("the schema lookup returned no boolean");
        } else if (!isPresent) {
            await this.administration`SELECT pg_advisory_unlock(hashtext(${name}))`;
            this.#locked.delete(name);
        }

        return isPresent;
    }

    /**
     * Adopt a newly locked schema that migrates to nothing, or drop it.
     *
     * A schema without state, without a log, with foreign tables or with a pending plan is dropped.
     */
    async #verify(
        name: string,
        state: readonly TableState[],
        tables: declaration.Database | readonly Table[],
    ): Promise<TestSchema | undefined> {
        // refuse a schema without state, without a log, or with foreign tables
        const pools: postgres.Sql[] = [];
        const database = new PoolDatabase(this.connect(name), tables, pools);
        const names = (await this.#readRelations(name))
            .filter((relation) => relation.kind !== "S")
            .map((relation) => relation.name);
        const managed = names.includes(STATE)
            ? new Set((await readState(database)).map((applied) => applied.table.name))
            : undefined;
        const isUsable =
            managed !== undefined &&
            LOG_TABLES.every((table) => names.includes(table)) &&
            names.every((table) => managed.has(table) || table.startsWith("__destack_"));

        // adopt the schema only with an empty plan
        const isCurrent = isUsable && (await database.plan({ declared: state })).steps.length === 0;
        await database.close();
        if (!isCurrent) {
            await this.#drop(name, pools);

            return undefined;
        }

        // stamp the claim time for retention
        await this.administration.unsafe(`COMMENT ON SCHEMA "${name}" IS '${Date.now()}'`);

        return await this.#adopt(name, pools);
    }

    /** Record a locked schema's shape and keep it with its pools. */
    async #adopt(name: string, pools: postgres.Sql[]): Promise<TestSchema> {
        const schema = new TestSchema(this, name, await this.#readShape(name), pools);
        this.#schemas.push(schema);

        return schema;
    }

    /** Delete the rows, restart the sequences and start a new epoch, in one statement batch. */
    async #reset(schema: TestSchema): Promise<void> {
        // pick the tables to empty and the sequences to restart
        const quoted = (name: string) => `"${schema.name}"."${name}"`;
        const emptied = schema.shape.relations.filter(
            (relation) =>
                relation.kind !== "S" &&
                relation.name !== STATE &&
                relation.name !== LOG_EPOCH &&
                relation.name !== LOG_HORIZON,
        );
        const sequences = schema.shape.relations.filter((relation) => relation.kind === "S");

        // run the batch as one implicit transaction without triggers
        await this.administration.unsafe(`SET LOCAL session_replication_role = replica;
            ${emptied.map((table) => `DELETE FROM ${quoted(table.name)};`).join("\n")}
            ${sequences.map((sequence) => `ALTER SEQUENCE ${quoted(sequence.name)} RESTART;`).join("\n")}
            UPDATE ${quoted(LOG_EPOCH)} SET epoch = '${v7()}' WHERE slot = 1;
            UPDATE ${quoted(LOG_HORIZON)} SET sequence = 0 WHERE slot = 1;`);
    }

    /** Read a schema's tables and sequences with its applied state, which the migration plan compares. */
    async #readShape(name: string): Promise<TestSchemaShape> {
        // read the relations and digest the applied state
        const relations = await this.#readRelations(name);
        const digest: unknown = (
            await this.administration`
            SELECT md5(string_agg("table" || ':' || state, ',' ORDER BY "table"))
            FROM ${this.administration(name)}.${this.administration(STATE)}`.values()
        )[0]?.[0];
        if (typeof digest !== "string" && digest !== null) {
            throw new TypeError("the applied state digest is no text");
        }

        return { relations, fingerprint: JSON.stringify([relations, digest]) };
    }

    /** Read a schema's tables and sequences through the namespace dependency index. */
    async #readRelations(schema: string): Promise<TestRelation[]> {
        return await this.administration<TestRelation[]>`
            SELECT c.relname AS name, c.relkind AS kind
            FROM pg_depend d JOIN pg_class c ON c.oid = d.objid
            WHERE d.classid = 'pg_class'::regclass AND d.refclassid = 'pg_namespace'::regclass
                AND d.refobjid = ${schema}::regnamespace AND c.relkind IN ('r', 'p', 'S')
            ORDER BY c.relname`;
    }

    /** Drop a locked schema, close its pools and release its claim. */
    async #drop(name: string, pools: readonly postgres.Sql[]): Promise<void> {
        // close the pools before the schema goes
        await Promise.all(pools.map((pool) => pool.end()));

        // drop the schema, then release its lock
        await this.administration.unsafe(`DROP SCHEMA "${name}" CASCADE`);
        await this.administration`SELECT pg_advisory_unlock(hashtext(${name}))`;
        this.#locked.delete(name);
    }
}

/** A migrated schema this process keeps, with the connection pools its tests reuse. */
class TestSchema {
    /** The server with the schema. */
    readonly server: TestServer;
    /** The schema name. */
    readonly name: string;
    /** The relations and applied state the schema keeps across tests. */
    readonly shape: TestSchemaShape;
    /** The idle connection pools, reused across tests. */
    readonly pools: postgres.Sql[];
    /** Whether no test uses the schema. */
    isIdle = false;

    /** Create a schema this process keeps. */
    constructor(server: TestServer, name: string, shape: TestSchemaShape, pools: postgres.Sql[]) {
        // bind the schema to its server
        this.server = server;
        this.name = name;

        // keep its shape and pools
        this.shape = shape;
        this.pools = pools;
    }

    /** Connect over an idle or new pool, returning it when the connection closes. */
    async connect<Models extends Readonly<Record<string, Model>>>(
        tables: declaration.Database<Models> | readonly Table[],
    ): Promise<TestConnection<Models>> {
        return new PoolDatabase(
            this.pools.pop() ?? this.server.connect(this.name),
            tables,
            this.pools,
        );
    }
}

/** A PostgreSQL database over a pool a test schema lends. */
class PoolDatabase<
    Models extends Readonly<Record<string, Model>>,
> extends PostgresDatabase<Models> {
    /** The idle pools the pool returns to. */
    readonly #idle: postgres.Sql[];

    /** Bind tables to a lent pool. */
    constructor(
        client: postgres.Sql,
        tables: declaration.Database<Models> | readonly Table[],
        idle: postgres.Sql[],
    ) {
        super(client, tables);
        this.#idle = idle;
    }

    /** Drain the operations, then return the open pool. */
    override async close(): Promise<void> {
        await this.state.close(async () => {
            this.#idle.push(this.$client);
        });
    }
}

/** The relations and applied state of a migrated test schema. */
interface TestSchemaShape {
    /** The tables and sequences by name. */
    readonly relations: readonly TestRelation[];
    /** The relations and applied state as one comparable text. */
    readonly fingerprint: string;
}

/** A table or sequence of a test schema. */
interface TestRelation {
    /** The relation name. */
    readonly name: string;
    /** The relation kind: `r` for a table, `p` for a partitioned table, `S` for a sequence. */
    readonly kind: "r" | "p" | "S";
}

/** Migrate a SQLite template once per process and return its path. */
function sqliteTemplate(tables: readonly Table[], isReplica: boolean): Promise<string> {
    // reuse the template of the same state
    const key = JSON.stringify(declareState(tables, "sqlite", { isReplica }));
    const known = templates.get(key);
    if (known !== undefined) {
        return known;
    }

    // migrate a file and checkpoint its WAL
    const created = (async () => {
        // migrate in its own directory
        const directory = await mkdtemp(join(tmpdir(), "destack-template-"));
        const file = join(directory, "template.db");
        const database = await sqlite.connect(file, tables);
        await database.migrate(tables, { isReplica });
        await database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
        await database.close();

        return file;
    })();
    templates.set(key, created);

    return created;
}

/** A connection to a test database. */
type TestConnection<
    Models extends Readonly<Record<string, Model>> = Readonly<Record<string, Model>>,
> = DatabaseConnection<Models> & { close(): Promise<void> };

/** Open a connection to a test database. */
type TestConnector = <Models extends Readonly<Record<string, Model>>>(
    tables: declaration.Database<Models> | readonly Table[],
) => Promise<TestConnection<Models>>;
