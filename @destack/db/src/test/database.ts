import { copyFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import postgres from "postgres";
import { v7 } from "uuid";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { Table } from "../table/table.ts";
import type * as declaration from "../declare/database.ts";
import { declareState } from "../migration/state.ts";
import * as sqlite from "../sqlite/bun/connection.ts";
import * as postgresql from "../postgres/connection.ts";
import { LOG_EPOCH } from "../log/schema.ts";
import { relayNotifier } from "../log/notifier.ts";
import { relayHub } from "./relay.ts";
import { planMigration } from "../migration/database.ts";
import { readState, STATE, type TableState } from "../migration/state.ts";

/** The test dialects: SQLite, and PostgreSQL when DESTACK_TEST_POSTGRES names a server. */
export const TEST_DIALECTS: readonly Dialect[] = [
    "sqlite",
    ...(process.env.DESTACK_TEST_POSTGRES ? (["postgresql"] as const) : []),
];

/** The connections each test database pools: ten workers stay within a server's hundred. */
const TEST_POOL_CONNECTIONS = 4;

/** The migrated SQLite templates, by declared state. */
const templates = new Map<string, Promise<string>>();

/**
 * How long an unclaimed test schema stays for reuse, in milliseconds.
 *
 * Past a day its declared state has likely changed.
 */
const SCHEMA_RETENTION_MILLISECONDS = 24 * 60 * 60 * 1000;

/** The server connection managing test schemas. */
let administration: Promise<postgres.Sql> | undefined;

/**
 * An isolated database for one test: a SQLite memory or file database, or a PostgreSQL schema.
 *
 * Migrated PostgreSQL schemas stay on the server by declared state, claimed through advisory locks and reset per test.
 */
export class TestDatabase {
    /** The first connection. */
    readonly database: DatabaseConnection & { close(): Promise<void> };
    /** Open another connection, absent for SQLite in memory. */
    readonly #open: TestConnector | undefined;
    /** Remove the database. */
    readonly #remove: () => Promise<void>;
    /** The other open connections. */
    readonly #connections = new Set<TestConnection>();

    /** Create the test database. */
    private constructor(
        database: TestDatabase["database"],
        open: TestConnector | undefined,
        remove: () => Promise<void>,
    ) {
        this.database = database;
        this.#open = open;
        this.#remove = remove;
    }

    /** Create an isolated database, empty or migrated. */
    static async create(
        dialect: Dialect,
        tables: declaration.Database | readonly Table[],
        options: TestDatabaseOptions = {},
    ): Promise<TestDatabase> {
        const declared = "tables" in tables ? tables.tables : tables;

        // claim or create a PostgreSQL schema
        if (dialect === "postgresql") {
            const address = process.env.DESTACK_TEST_POSTGRES;
            if (!address) {
                throw new TypeError("DESTACK_TEST_POSTGRES names no PostgreSQL server");
            }
            const server = await administer(address);
            const connector =
                (schema: string): TestConnector =>
                (connected) =>
                    postgresql.connect(
                        postgres(address, {
                            max: TEST_POOL_CONNECTIONS,
                            connection: { search_path: schema },
                            onnotice: postgresql.reportNotice,
                        }),
                        connected,
                    );

            // claim a migrated schema or migrate a new one
            if (options.isMigrated) {
                const state = declareState(declared, "postgresql", {
                    isReplica: options.isReplica ?? false,
                });
                const claimed = await claim(address, server, state, connector, tables);

                return new TestDatabase(claimed.database, connector(claimed.schema), () =>
                    claimed.claim.end(),
                );
            }

            // create an empty schema
            const schema = `test_${crypto.randomUUID().replaceAll("-", "")}`;
            await server.unsafe(`CREATE SCHEMA "${schema}"`);

            return new TestDatabase(
                await connector(schema)(tables),
                connector(schema),
                async () => {
                    await server.unsafe(`DROP SCHEMA "${schema}" CASCADE`);
                },
            );
        }
        // keep empty SQLite in memory
        else if (!options.isMigrated && (options.storage ?? "memory") === "memory") {
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
            if (options.isMigrated) {
                await copyFile(await sqliteTemplate(declared, options.isReplica ?? false), file);
            }

            // share commits through one relay
            const relay = relayHub<{ readonly kind: string }>();
            const open = (connected: declaration.Database | readonly Table[]) =>
                sqlite.connect(file, connected, { notifier: relayNotifier(relay()) });

            return new TestDatabase(await open(tables), open, () =>
                rm(directory, { recursive: true }),
            );
        }
    }

    /** Open another connection to the same database. */
    async connect(
        tables: declaration.Database | readonly Table[],
    ): Promise<DatabaseConnection & { close(): Promise<void> }> {
        // require a reachable database
        if (this.#open === undefined) {
            throw new TypeError("an in-memory SQLite database has no further connections");
        }

        // track the connection until it closes once
        const connection = await this.#open(tables);
        const closed = connection.close.bind(connection);
        let closing: Promise<void> | undefined;
        connection.close = () => {
            closing ??= closed().finally(() => this.#connections.delete(connection));

            return closing;
        };
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

/** Migrate a SQLite template once per process and return its path. */
function sqliteTemplate(tables: readonly Table[], isReplica: boolean): Promise<string> {
    // reuse the template of the same state
    const key = JSON.stringify(declareState(tables, "sqlite", { isReplica }));
    const known = templates.get(key);
    if (known) {
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

/** Open the administration connection, dropping stale unclaimed schemas. */
function administer(address: string): Promise<postgres.Sql> {
    administration ??= (async () => {
        // drop unclaimed schemas past retention
        const server = postgres(address, { max: 1, onnotice: postgresql.reportNotice });
        const expired = Date.now() - SCHEMA_RETENTION_MILLISECONDS;
        const schemas = await server<{ name: string }[]>`
            SELECT nspname AS name FROM pg_namespace
            WHERE starts_with(nspname, 'test_s') AND coalesce(obj_description(oid, 'pg_namespace'), '0')::bigint < ${expired}`;
        for (const { name } of schemas) {
            const [held] = await server<{ isHeld: boolean }[]>`
                SELECT pg_try_advisory_lock(hashtext(${name})) AS "isHeld"`;
            if (held!.isHeld) {
                await server.unsafe(`DROP SCHEMA "${name}" CASCADE`);
                await server`SELECT pg_advisory_unlock(hashtext(${name}))`;
            }
        }

        return server;
    })();

    return administration;
}

/** Claim and reset a migrated schema, or migrate a new one. */
async function claim(
    address: string,
    server: postgres.Sql,
    state: readonly TableState[],
    connector: (schema: string) => TestConnector,
    tables: declaration.Database | readonly Table[],
): Promise<{
    readonly schema: string;
    readonly database: TestConnection;
    readonly claim: postgres.Sql;
}> {
    // name the schemas by the state's digest
    const digest = new Uint8Array(
        await crypto.subtle.digest("SHA-256", new TextEncoder().encode(JSON.stringify(state))),
    );
    const prefix = `test_s${digest.subarray(0, 8).toHex()}_`;

    // claim the first free schema
    const schemas = await server<{ name: string }[]>`
        SELECT nspname AS name FROM pg_namespace WHERE starts_with(nspname, ${prefix})`;
    for (const { name } of schemas) {
        // hold the claim on its own connection
        const held = postgres(address, { max: 1, onnotice: postgresql.reportNotice });
        if (await hold(held, name)) {
            const database = await connector(name)(tables);
            if (await reset(server, database, name, state)) {
                return { schema: name, database, claim: held };
            }
            await database.close();
        }
        await held.end();
    }

    // name the new schema before it exists
    const schema = `${prefix}${crypto.randomUUID().replaceAll("-", "")}`;
    const held = postgres(address, { max: 1, onnotice: postgresql.reportNotice });
    await hold(held, schema);

    // create it with its claim time, then migrate it
    await server.unsafe(
        `CREATE SCHEMA "${schema}"; COMMENT ON SCHEMA "${schema}" IS '${Date.now()}';`,
    );
    const database = await connector(schema)(tables);
    await database.apply(await planMigration(database, state));

    return { schema, database, claim: held };
}

/** Take a schema's advisory lock, reporting whether it was free. */
async function hold(claim: postgres.Sql, schema: string): Promise<boolean> {
    const [held] = await claim<{ isHeld: boolean }[]>`
        SELECT pg_try_advisory_lock(hashtext(${schema})) AS "isHeld"`;

    return held!.isHeld;
}

/**
 * Reset a claimed schema to its migrated state, returning false for a changed one.
 *
 * The reset deletes rows, restarts sequences, starts a new epoch, and checks the plan is empty.
 */
async function reset(
    server: postgres.Sql,
    database: TestConnection,
    schema: string,
    state: readonly TableState[],
): Promise<boolean> {
    // refuse a schema with foreign tables or without a log
    const managed = new Set((await readState(database)).map((applied) => applied.table.name));
    const tables = await server<{ name: string }[]>`
        SELECT tablename AS name FROM pg_tables WHERE schemaname = ${schema}`;
    const isUnusable =
        !tables.some((table) => table.name === LOG_EPOCH) ||
        tables.some((table) => !managed.has(table.name) && !table.name.startsWith("__destack_"));
    const sequences = await server<{ name: string }[]>`
        SELECT sequencename AS name FROM pg_sequences WHERE schemaname = ${schema}`;

    // delete the rows, restart the sequences and start a new epoch
    const quoted = (name: string) => `"${schema}"."${name}"`;
    const emptied = tables.filter((table) => table.name !== STATE && table.name !== LOG_EPOCH);
    if (!isUnusable) {
        await server.begin((transaction) =>
            transaction.unsafe(`SET LOCAL session_replication_role = replica;
                ${emptied.map((table) => `DELETE FROM ${quoted(table.name)};`).join("\n")}
                ${sequences.map((sequence) => `ALTER SEQUENCE ${quoted(sequence.name)} RESTART;`).join("\n")}
                UPDATE ${quoted(LOG_EPOCH)} SET epoch = '${v7()}' WHERE slot = 1;
                COMMENT ON SCHEMA "${schema}" IS '${Date.now()}';`),
        );
    }

    // reuse the schema only with an empty plan
    const isReset = !isUnusable && (await planMigration(database, state)).steps.length === 0;
    if (!isReset) {
        await server.unsafe(`DROP SCHEMA IF EXISTS "${schema}" CASCADE`);
    }

    return isReset;
}

/** A connection to a test database. */
type TestConnection = DatabaseConnection & { close(): Promise<void> };

/** Open a connection to a test database. */
type TestConnector = (tables: declaration.Database | readonly Table[]) => Promise<TestConnection>;
