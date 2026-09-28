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
import * as turso from "../sqlite/turso/connection.ts";
import * as postgresql from "../postgres/connection.ts";
import { LOG_EPOCH } from "../log/schema.ts";
import { planMigration } from "../migration/database.ts";
import { readState, STATE, type TableState } from "../migration/state.ts";

/** The dialects tests run on: SQLite, and PostgreSQL when DESTACK_TEST_POSTGRES names a server. */
export const TEST_DIALECTS: readonly Dialect[] = [
    "sqlite",
    ...(process.env.DESTACK_TEST_POSTGRES ? (["postgresql"] as const) : []),
];

/** The migrated SQLite templates of this process, by declared state. */
const templates = new Map<string, Promise<string>>();

/** How long an unclaimed migrated test schema stays for reuse, in milliseconds: a day, past which its declared state has likely changed. */
const SCHEMA_RETENTION_MILLISECONDS = 24 * 60 * 60 * 1000;

/** The server connection creating, resetting and dropping test schemas, opened once per process. */
let administration: Promise<postgres.Sql> | undefined;

/**
 * An isolated database for one test: in memory or a temporary file for SQLite, a schema for PostgreSQL.
 *
 * A schema takes milliseconds where a database takes a hundred, and copying a migrated template takes a millisecond where migrating takes tens.
 * Migrating a PostgreSQL schema runs about a hundred DDL statements at nearly a millisecond each, so migrated schemas stay on the server, named by their declared state.
 * A test claims one through an advisory lock a claim connection holds until the test database closes or its process ends, and resets it to what migrating left.
 */
export class TestDatabase {
    /** The first connection, bound to the tables given at creation. */
    readonly database: DatabaseConnection & { close(): Promise<void> };
    /** Open another connection to the same database, absent for SQLite in memory. */
    readonly #open: TestConnector | undefined;
    /** Remove the database once its connections closed. */
    readonly #remove: () => Promise<void>;
    /** The further connections still open, which closing the database closes first. */
    readonly #connections = new Set<TestConnection>();

    /** Retain the first connection, how to reach the database again, and how to remove it. */
    private constructor(
        database: TestDatabase["database"],
        open: TestConnector | undefined,
        remove: () => Promise<void>,
    ) {
        this.database = database;
        this.#open = open;
        this.#remove = remove;
    }

    /** Create an isolated database of a dialect, empty or migrated to its tables, in a file when further connections need it. */
    static async create(
        dialect: Dialect,
        tables: declaration.Database | readonly Table[],
        options: TestDatabaseOptions = {},
    ): Promise<TestDatabase> {
        const declared = "tables" in tables ? tables.tables : tables;

        // claim or create a schema on the test server
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
                            connection: { search_path: schema },
                            onnotice: () => {},
                        }),
                        connected,
                    );

            // claim a migrated schema of the same declared state, else migrate a new one kept for later tests
            if (options.isMigrated) {
                const state = declareState(declared, "postgresql", {
                    isReplica: options.isReplica ?? false,
                });
                const claimed = await claim(address, server, state, connector, tables);

                return new TestDatabase(claimed.database, connector(claimed.schema), () =>
                    claimed.claim.end(),
                );
            }

            // create an empty schema, dropped with the test
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
                await turso.connect(":memory:", tables),
                undefined,
                async () => {},
            );
        }
        // keep SQLite in a temporary file, copied from the process's migrated template when asked
        else {
            const directory = await mkdtemp(join(tmpdir(), "destack-test-"));
            const file = join(directory, "test.db");
            if (options.isMigrated) {
                await copyFile(await sqliteTemplate(declared, options.isReplica ?? false), file);
            }

            return new TestDatabase(
                await turso.connect(file, tables),
                (connected) => turso.connect(file, connected),
                () => rm(directory, { recursive: true }),
            );
        }
    }

    /** Open another connection to the same database. */
    async connect(
        tables: declaration.Database | readonly Table[],
    ): Promise<DatabaseConnection & { close(): Promise<void> }> {
        // require a database other connections can reach
        if (this.#open === undefined) {
            throw new TypeError("an in-memory SQLite database has no further connections");
        }

        // track the connection until it closes, closing it once however often it is closed
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

    /** Close the further connections still open, then the first, and remove the database, whose claim ends with the first. */
    async close(): Promise<void> {
        await Promise.all([...this.#connections].map((connection) => connection.close()));
        await this.database.close();
        await this.#remove();
    }
}

/** How to create a test database. */
export interface TestDatabaseOptions {
    /** Where empty SQLite lives: in memory, or in a file further connections reach; migrated SQLite lives in a file. */
    readonly storage?: "memory" | "file";
    /** Whether the database starts migrated to its tables. */
    readonly isMigrated?: boolean;
    /** Whether it migrates as a copy of another database's tables: without their trees and references. */
    readonly isReplica?: boolean;
}

/** Migrate a SQLite template file of some tables once per process, as a source or a copy, and return its path. */
function sqliteTemplate(tables: readonly Table[], isReplica: boolean): Promise<string> {
    // reuse the template of the same declared state
    const key = JSON.stringify(declareState(tables, "sqlite", { isReplica }));
    const known = templates.get(key);
    if (known) {
        return known;
    }

    // migrate a new file and fold its write-ahead log in, so copying the file copies everything
    const created = (async () => {
        // migrate the template in its own directory
        const directory = await mkdtemp(join(tmpdir(), "destack-template-"));
        const file = join(directory, "template.db");
        const database = await turso.connect(file, tables);
        await database.migrate(tables, { isReplica });
        await database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
        await database.close();

        return file;
    })();
    templates.set(key, created);

    return created;
}

/** Open the process's administration connection, first dropping the migrated test schemas nobody claimed for a day. */
function administer(address: string): Promise<postgres.Sql> {
    administration ??= (async () => {
        // drop each unclaimed schema past its retention, skipping the ones a test holds
        const server = postgres(address, { max: 1, onnotice: () => {} });
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

/** Claim a migrated schema of a declared state, reset to what migrating left, else migrate a new one, under a claim connection the caller ends last. */
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
    // name the schemas of the declared state by its digest
    const digest = new Uint8Array(
        await crypto.subtle.digest("SHA-256", new TextEncoder().encode(JSON.stringify(state))),
    );
    const prefix = `test_s${digest.subarray(0, 8).toHex()}_`;

    // claim the first schema no other test holds, reusing it once reset
    const schemas = await server<{ name: string }[]>`
        SELECT nspname AS name FROM pg_namespace WHERE starts_with(nspname, ${prefix})`;
    for (const { name } of schemas) {
        // hold the schema on a connection of its own, which outlives every connection the test opens
        const held = postgres(address, { max: 1, onnotice: () => {} });
        if (await hold(held, name)) {
            const database = await connector(name)(tables);
            if (await reset(server, database, name, state)) {
                return { schema: name, database, claim: held };
            }
            await database.close();
        }
        await held.end();
    }

    // create a new schema with its claim time in one statement, so that no cleanup finds it unclaimed, then migrate it under the claim
    const schema = `${prefix}${crypto.randomUUID().replaceAll("-", "")}`;
    await server.unsafe(
        `CREATE SCHEMA "${schema}"; COMMENT ON SCHEMA "${schema}" IS '${Date.now()}';`,
    );
    const held = postgres(address, { max: 1, onnotice: () => {} });
    await hold(held, schema);
    const database = await connector(schema)(tables);
    await database.apply(await planMigration(database, state));

    return { schema, database, claim: held };
}

/** Take a schema's advisory lock on a claim connection, which holds it until the connection ends, reporting whether it was free. */
async function hold(claim: postgres.Sql, schema: string): Promise<boolean> {
    const [held] = await claim<{ isHeld: boolean }[]>`
        SELECT pg_try_advisory_lock(hashtext(${schema})) AS "isHeld"`;

    return held!.isHeld;
}

/**
 * Reset a claimed schema to what migrating left, reporting false for one a test changed, which is dropped instead.
 *
 * It deletes every row but the applied state with triggers off, restarts the sequences, starts a new log epoch, and requires the migration engine to find nothing to apply.
 */
async function reset(
    server: postgres.Sql,
    database: TestConnection,
    schema: string,
    state: readonly TableState[],
): Promise<boolean> {
    // refuse a schema holding tables beside its managed ones and the log's
    const managed = new Set((await readState(database)).map((applied) => applied.table.name));
    const tables = await server<{ name: string }[]>`
        SELECT tablename AS name FROM pg_tables WHERE schemaname = ${schema}`;
    const isForeign = tables.some(
        (table) => !managed.has(table.name) && !table.name.startsWith("__destack_"),
    );
    const sequences = await server<{ name: string }[]>`
        SELECT sequencename AS name FROM pg_sequences WHERE schemaname = ${schema}`;

    // delete every row but the applied state and the epoch, restart the sequences and start a new epoch
    const quoted = (name: string) => `"${schema}"."${name}"`;
    const emptied = tables.filter((table) => table.name !== STATE && table.name !== LOG_EPOCH);
    if (!isForeign) {
        await server.begin((transaction) =>
            transaction.unsafe(`SET LOCAL session_replication_role = replica;
                ${emptied.map((table) => `DELETE FROM ${quoted(table.name)};`).join("\n")}
                ${sequences.map((sequence) => `ALTER SEQUENCE ${quoted(sequence.name)} RESTART;`).join("\n")}
                UPDATE ${quoted(LOG_EPOCH)} SET epoch = '${v7()}' WHERE slot = 1;
                COMMENT ON SCHEMA "${schema}" IS '${Date.now()}';`),
        );
    }

    // reuse the schema only when the migration engine finds nothing to apply
    const isReset = !isForeign && (await planMigration(database, state)).steps.length === 0;
    if (!isReset) {
        await server.unsafe(`DROP SCHEMA IF EXISTS "${schema}" CASCADE`);
    }

    return isReset;
}

/** A connection to a test database. */
type TestConnection = DatabaseConnection & { close(): Promise<void> };

/** Open a connection to a test database with the tables its queries use. */
type TestConnector = (tables: declaration.Database | readonly Table[]) => Promise<TestConnection>;
