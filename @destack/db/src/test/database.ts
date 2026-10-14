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
import { ColumnCipher } from "../database/cipher.ts";
import { LOG_EPOCH, LOG_HORIZON, LOG_TABLES } from "../log/schema.ts";
import { createLog } from "../log/trigger.ts";
import { sql } from "../sql/index.ts";
import type { Channel } from "../channel/channel.ts";
import { channelHub } from "../channel/channel.ts";
import { readState, STATE, type TableState } from "../migration/state.ts";
import { upsertPredefined } from "../migration/plan.ts";

/** The test dialects: SQLite, and PostgreSQL when DESTACK_TEST_POSTGRES has a server address. */
export const TEST_DIALECTS: readonly Dialect[] = [
    "sqlite",
    ...((process.env["DESTACK_TEST_POSTGRES"] ?? "") === "" ? [] : (["postgresql"] as const)),
];

/** The cipher of every test database, a fixed key so schemas reused across test processes open. */
export const TEST_CIPHER = new ColumnCipher(new Uint8Array(32).fill(7));

/** The connections each test database pools: ten workers stay within a server's hundred. */
const TEST_POOL_CONNECTIONS = 4;

/** The seconds a test pool keeps an idle connection. */
const TEST_IDLE_SECONDS = 1;

/** The migrated SQLite templates, by declared state. */
const templates = new Map<string, Promise<string>>();

/** How long an unclaimed test schema stays for reuse, in milliseconds. */
const SCHEMA_RETENTION_MILLISECONDS = 60 * 60 * 1000;

/** The most expired schemas one test process drops when it starts. */
const EXPIRED_SCHEMA_DROPS = 2;

/** The PostgreSQL cluster of this process's tests, opened once. */
let testCluster: Promise<TestCluster> | undefined;

/** An isolated database for one test: a SQLite memory or file database, or a PostgreSQL schema. */
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

    /** Wait until a database meets a check after a commit, failing once some milliseconds pass. */
    static async settle(
        database: DatabaseConnection,
        check: () => Promise<boolean>,
        milliseconds: number,
    ): Promise<void> {
        if (!(await database.log.until(check, AbortSignal.timeout(milliseconds)))) {
            throw new TypeError(`the database never settled within ${milliseconds} ms`);
        }
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

        // claim or create a PostgreSQL schema, keep empty SQLite in memory, or copy SQLite into a file
        if (dialect === "postgresql") {
            return TestDatabase.#postgres(declared, tables, options);
        } else if (options.isMigrated !== true && (options.storage ?? "memory") === "memory") {
            return new TestDatabase(
                keyed(await sqlite.connectBunSqlite(":memory:", tables)),
                undefined,
                async () => {},
            );
        }

        return TestDatabase.#sqliteFile(declared, tables, options);
    }

    /** Create an isolated migrated database copied from a seed's template, with its value. */
    static async seeded<Models extends Readonly<Record<string, Model>>, Value>(
        dialect: Dialect,
        tables: declaration.Database<Models> | readonly Table[],
        seed: TestSeed<Value>,
    ): Promise<{ readonly storage: TestDatabase<Models>; readonly value: Value }> {
        // write the seed's template once per process for the dialect and the declared state
        const declared = "tables" in tables ? tables.tables : tables;
        const state = declareState(declared, dialect, {
            isReplica: false,
            replicated: replicatedOf(tables),
        });
        const key = JSON.stringify([dialect, state]);
        const written =
            seed.templates.get(key) ?? TestDatabase.#template(dialect, tables, state, seed);
        seed.templates.set(key, written);
        const { source, value } = await written;

        // copy the template into a claimed schema or a temporary file
        const storage =
            source.dialect === "postgresql"
                ? await TestDatabase.#postgres(declared, tables, { isMigrated: true }, source)
                : await TestDatabase.#sqliteFile(declared, tables, {}, source.file);

        return { storage, value };
    }

    /** Write a seed into a migrated database the process keeps as the seed's template. */
    static async #template<Value>(
        dialect: Dialect,
        tables: declaration.Database | readonly Table[],
        state: readonly TableState[],
        seed: TestSeed<Value>,
    ): Promise<TestTemplate<Value>> {
        // claim a migrated schema this process keeps for the template
        if (dialect === "postgresql") {
            const cluster = await TestCluster.current();
            const schema = await cluster.claim(state, tables);
            const open: TestConnector = (connected) => schema.connect(connected);
            const value = await TestDatabase.#write(
                seed,
                new TestDatabase(await open(tables), open, async () => {}),
            );

            return {
                source: {
                    dialect,
                    schema: schema.name,
                    columns: await cluster.columns(schema.name),
                },
                value,
            };
        }

        // write a migrated file the template keeps, and checkpoint its WAL into it
        const declared = "tables" in tables ? tables.tables : tables;
        const directory = await mkdtemp(join(tmpdir(), "destack-seed-"));
        const file = join(directory, "seed.db");
        await copyFile(await sqliteTemplate(declared, false, replicatedOf(tables)), file);
        const value = await TestDatabase.#write(
            seed,
            await TestDatabase.#sqliteAt(file, tables, async () => {}),
        );
        const database = await sqlite.connectBunSqlite(file, tables);
        await database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
        await database.close();

        return { source: { dialect, file }, value };
    }

    /** Write a seed into a database, closing it once written. */
    static async #write<Value>(seed: TestSeed<Value>, storage: TestDatabase): Promise<Value> {
        try {
            return await seed.write(storage);
        } finally {
            await storage.close();
        }
    }

    /** Claim a migrated PostgreSQL schema this process keeps across tests, or create one. */
    static async #postgres<Models extends Readonly<Record<string, Model>>>(
        declared: readonly Table[],
        tables: declaration.Database<Models> | readonly Table[],
        options: TestDatabaseOptions,
        template?: PostgresTemplate,
    ): Promise<TestDatabase<Models>> {
        // claim a migrated schema, kept by this process across tests, with the template's rows
        const cluster = await TestCluster.current();
        if (options.isMigrated === true) {
            const state = declareState(declared, "postgresql", {
                isReplica: options.isReplica ?? false,
                replicated: replicatedOf(tables),
            });
            const schema = await cluster.claim(state, tables, template);
            const open: TestConnector = (connected) => schema.connect(connected);

            return new TestDatabase(await open(tables), open, async () => {
                schema.isIdle = true;
            });
        }

        // create an empty schema
        const schema = `test_${crypto.randomUUID().replaceAll("-", "")}`;
        await cluster.administration.unsafe(`CREATE SCHEMA "${schema}"`);
        const open: TestConnector = (connected) =>
            postgresql.connectPostgres(cluster.connect(schema), connected).then(keyed);

        return new TestDatabase(await open(tables), open, async () => {
            await cluster.administration.unsafe(`DROP SCHEMA "${schema}" CASCADE`);
        });
    }

    /** Keep SQLite in a temporary file, copied from a seed's or the migrated template. */
    static async #sqliteFile<Models extends Readonly<Record<string, Model>>>(
        declared: readonly Table[],
        tables: declaration.Database<Models> | readonly Table[],
        options: TestDatabaseOptions,
        template?: string,
    ): Promise<TestDatabase<Models>> {
        // copy the seed's or the migrated template into a temporary file
        const directory = await mkdtemp(join(tmpdir(), "destack-test-"));
        const file = join(directory, "test.db");
        const source =
            template ??
            (options.isMigrated === true
                ? await sqliteTemplate(declared, options.isReplica ?? false, replicatedOf(tables))
                : undefined);
        if (source !== undefined) {
            await copyFile(source, file);
        }

        return TestDatabase.#sqliteAt(file, tables, () => rm(directory, { recursive: true }));
    }

    /** Keep SQLite in a file, sharing one hub, removed as given once closed. */
    static async #sqliteAt<Models extends Readonly<Record<string, Model>>>(
        file: string,
        tables: declaration.Database<Models> | readonly Table[],
        remove: () => Promise<void>,
    ): Promise<TestDatabase<Models>> {
        // share each named channel through one hub
        const hubs = new Map<string, () => Channel<unknown>>();
        const openChannel = (name: string) => {
            const party = hubs.get(name) ?? channelHub<unknown>();
            hubs.set(name, party);

            return party();
        };
        const open: TestConnector = (connected) =>
            sqlite.connectBunSqlite(file, connected, { openChannel }).then(keyed);

        // start a logged copy's own epoch as a restore does
        const database = await open(tables);
        const [log] = await database.execute(
            sql`SELECT name FROM sqlite_master WHERE name = ${LOG_EPOCH}`,
        );
        if (log !== undefined) {
            await database.log.renew();
        }

        return new TestDatabase(database, open, remove);
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
    /** Whether it migrates as a replica, without trees and references. */
    readonly isReplica?: boolean;
}

/** The rows test databases start with, written into a template once per process and copied. */
export class TestSeed<Value> {
    /** Write the rows into the template's database, returning what each copy shares. */
    readonly write: (storage: TestDatabase) => Promise<Value>;
    /** The written templates with what writing them returned, by dialect and declared state. */
    readonly templates = new Map<string, Promise<TestTemplate<Value>>>();

    /** Write a seed's rows as given. */
    constructor(write: (storage: TestDatabase) => Promise<Value>) {
        this.write = write;
    }
}

/** A seed's written template: a SQLite file or a PostgreSQL schema, with what the seed returned. */
interface TestTemplate<Value> {
    /** Where the rows are. */
    readonly source: { readonly dialect: "sqlite"; readonly file: string } | PostgresTemplate;
    /** What writing the rows returned. */
    readonly value: Value;
}

/** A PostgreSQL schema whose rows claimed schemas copy. */
interface PostgresTemplate {
    /** The dialect. */
    readonly dialect: "postgresql";
    /** The schema's name. */
    readonly schema: string;
    /** The written columns of each table, quoted and listed in declared order. */
    readonly columns: ReadonlyMap<string, string>;
}

/** The PostgreSQL cluster of one test process, keeping its claimed schemas until exit. */
class TestCluster {
    /** The server address. */
    readonly address: string;
    /** The one connection that manages schemas and locks their claims. */
    readonly administration: postgres.Sql;
    /** The schema names this process locked, including claims still verifying. */
    readonly #locked = new Set<string>();
    /** The adopted schemas this process locked, idle or in use. */
    readonly #schemas: TestSchema[] = [];

    /** Hold the cluster behind an administration connection. */
    private constructor(address: string, administration: postgres.Sql) {
        this.address = address;
        this.administration = administration;
    }

    /** Reach the cluster of this process's tests at DESTACK_TEST_POSTGRES, opened once. */
    static current(): Promise<TestCluster> {
        // require a server address
        const address = process.env["DESTACK_TEST_POSTGRES"];
        if (address === undefined || address === "") {
            throw new TypeError("DESTACK_TEST_POSTGRES has no PostgreSQL server address");
        }
        testCluster ??= TestCluster.open(address);

        return testCluster;
    }

    /** Connect to the server, dropping a few stale unclaimed schemas. */
    static async open(address: string): Promise<TestCluster> {
        // skip the array type read, which scans every catalog type
        const cluster = new TestCluster(
            address,
            postgres(address, { max: 1, fetch_types: false, onnotice: postgresql.reportNotice }),
        );

        // drop the oldest unclaimed schemas past retention
        const expired = Date.now() - SCHEMA_RETENTION_MILLISECONDS;
        const schemas = await cluster.administration<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace
            WHERE starts_with(nspname, 'test_s') AND coalesce(obj_description(oid, 'pg_namespace'), '0')::bigint < ${expired}
            ORDER BY coalesce(obj_description(oid, 'pg_namespace'), '0')::bigint
            LIMIT ${EXPIRED_SCHEMA_DROPS}`;
        for (const { name } of schemas) {
            if (await cluster.#lockListed(name)) {
                await cluster.#drop(name, []);
            }
        }

        return cluster;
    }

    /** Open a connection pool whose queries resolve in a schema. */
    connect(schema: string): postgres.Sql {
        return postgres(this.address, {
            max: TEST_POOL_CONNECTIONS,
            idle_timeout: TEST_IDLE_SECONDS,
            connection: { search_path: schema },
            onnotice: postgresql.reportNotice,
        });
    }

    /** Claim a migrated schema reset to a template's rows: an idle, a free or a new one. */
    async claim(
        state: readonly TableState[],
        tables: declaration.Database | readonly Table[],
        template?: PostgresTemplate,
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
                    await this.#reset(schema, state, template);

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
                    await this.#reset(schema, state, template);

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
        const schema = await this.#adopt(name, pools);
        if (template !== undefined) {
            await this.#reset(schema, state, template);
        }

        return schema;
    }

    /** Read the written columns of a schema's tables, quoted and listed in declared order. */
    async columns(schema: string): Promise<ReadonlyMap<string, string>> {
        const rows = await this.administration<{ table: string; columns: string }[]>`
            SELECT table_name AS table, string_agg(quote_ident(column_name::text), ', ' ORDER BY ordinal_position) AS columns
            FROM information_schema.columns
            WHERE table_schema = ${schema} AND is_generated = 'NEVER'
            GROUP BY table_name`;

        return new Map(rows.map((row) => [row.table, row.columns]));
    }

    /** Take a schema's advisory lock, registered before awaiting, reporting whether it was free. */
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

    /** Take a listed schema's advisory lock, reporting whether it was free and still exists. */
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

    /** Adopt a newly locked schema that migrates to nothing, or drop it. */
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

    /** Replace the rows with a template's and start a new epoch, in one statement batch. */
    async #reset(
        schema: TestSchema,
        state: readonly TableState[],
        template?: PostgresTemplate,
    ): Promise<void> {
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

        // empty the tables and restart the sequences and the horizon, then copy the template's or the predefined rows
        const copied =
            template === undefined
                ? state.flatMap((table) =>
                      upsertPredefined(table).map((statement) => `${statement};`),
                  )
                : TestCluster.#copy(schema, template);

        // run the batch as one implicit transaction without triggers, in a new epoch
        await this.administration.unsafe(`SET LOCAL session_replication_role = replica;
            SET LOCAL search_path = "${schema.name}";
            ${emptied.map((table) => `DELETE FROM ${quoted(table.name)};`).join("\n")}
            ${sequences.map((sequence) => `ALTER SEQUENCE ${quoted(sequence.name)} RESTART;`).join("\n")}
            UPDATE ${quoted(LOG_HORIZON)} SET sequence = 0 WHERE slot = 1;
            ${copied.join("\n")}
            UPDATE ${quoted(LOG_EPOCH)} SET epoch = '${v7()}' WHERE slot = 1;`);
    }

    /** Write the statements copying a template's rows, sequences and horizon into a schema. */
    static #copy(schema: TestSchema, template: PostgresTemplate): string[] {
        // copy the written columns of each table but the state and the epoch
        const quoted = (name: string) => `"${schema.name}"."${name}"`;
        const source = (name: string) => `"${template.schema}"."${name}"`;
        const rows = [...template.columns]
            .filter(([table]) => table !== STATE && table !== LOG_EPOCH && table !== LOG_HORIZON)
            .map(
                ([table, columns]) =>
                    `INSERT INTO ${quoted(table)} (${columns}) OVERRIDING SYSTEM VALUE SELECT ${columns} FROM ${source(table)};`,
            );

        // copy the sequences' positions and the horizon
        const sequences = schema.shape.relations
            .filter((relation) => relation.kind === "S")
            .map(
                (sequence) =>
                    `SELECT setval('${quoted(sequence.name)}', last_value, is_called) FROM ${source(sequence.name)};`,
            );
        const horizon = `UPDATE ${quoted(LOG_HORIZON)} SET sequence = (SELECT sequence FROM ${source(LOG_HORIZON)} WHERE slot = 1) WHERE slot = 1;`;

        return [...rows, ...sequences, horizon];
    }

    /** Read a schema's tables, sequences and applied state for the migration plan to compare. */
    async #readShape(name: string): Promise<TestSchemaShape> {
        // read the relations and digest the applied state
        const relations = await this.#readRelations(name);
        const digest: unknown = (
            await this.administration`
            SELECT md5(string_agg("table" || ':' || state, ',' ORDER BY "table"))
            FROM ${this.administration(name)}.${this.administration(STATE)}`.values()
        )[0]?.[0];
        if (typeof digest !== "string" && digest !== null) {
            throw new TypeError("the applied state digest is not text");
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
    /** The cluster with the schema. */
    readonly cluster: TestCluster;
    /** The schema name. */
    readonly name: string;
    /** The relations and applied state the schema keeps across tests. */
    readonly shape: TestSchemaShape;
    /** The idle connection pools, reused across tests. */
    readonly pools: postgres.Sql[];
    /** Whether no test uses the schema. */
    isIdle = false;

    /** Create a schema this process keeps. */
    constructor(cluster: TestCluster, name: string, shape: TestSchemaShape, pools: postgres.Sql[]) {
        // bind the schema to its cluster
        this.cluster = cluster;
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
            this.pools.pop() ?? this.cluster.connect(this.name),
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
        this.key(TEST_CIPHER);
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

/** Read the SQL names of the tables a declared database replicates, none for bare tables. */
function replicatedOf(tables: declaration.Database | readonly Table[]): ReadonlySet<string> {
    return "tables" in tables ? tables.replicated : new Set<string>();
}

/** Migrate a SQLite template once per process and return its path. */
function sqliteTemplate(
    tables: readonly Table[],
    isReplica: boolean,
    replicated: ReadonlySet<string>,
): Promise<string> {
    // reuse the template of the same state
    const key = JSON.stringify(declareState(tables, "sqlite", { isReplica, replicated }));
    const known = templates.get(key);
    if (known !== undefined) {
        return known;
    }

    // migrate a file and checkpoint its WAL
    const created = (async () => {
        // migrate in its own directory
        const directory = await mkdtemp(join(tmpdir(), "destack-template-"));
        const file = join(directory, "template.db");
        const database = await sqlite.connectBunSqlite(file, tables);
        await database.migrate(tables, { isReplica, replicated });
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

/** Key a test database's sensitive columns with the test cipher. */
function keyed<Database extends DatabaseConnection>(database: Database): Database {
    database.key(TEST_CIPHER);

    return database;
}
