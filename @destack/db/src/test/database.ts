import { copyFile, mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import postgres from "postgres";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { Table } from "../table/table.ts";
import type * as declaration from "../declare/database.ts";
import { migrate } from "../migration/database.ts";
import { declareState } from "../migration/state.ts";
import * as turso from "../sqlite/turso/connection.ts";
import * as postgresql from "../postgres/connection.ts";

/** The dialects tests run on: SQLite, and PostgreSQL when DESTACK_TEST_POSTGRES names a server. */
export const TEST_DIALECTS: readonly Dialect[] = [
    "sqlite",
    ...(process.env.DESTACK_TEST_POSTGRES ? (["postgresql"] as const) : []),
];

/** The migrated SQLite templates of this process, by declared state. */
const templates = new Map<string, Promise<string>>();

/** The server connection creating and dropping test schemas, opened once per process. */
let administration: Promise<postgres.Sql> | undefined;

/**
 * An isolated database for one test: in memory or a temporary file for SQLite, a new schema for PostgreSQL.
 *
 * A schema takes milliseconds where a database takes a hundred, and copying a migrated template takes a millisecond where migrating takes tens.
 */
export class TestDatabase {
    /** The first connection, bound to the tables given at creation. */
    readonly database: DatabaseConnection & { close(): Promise<void> };
    /** Open another connection to the same database, absent for SQLite in memory. */
    readonly #open: TestConnector | undefined;
    /** Remove the database once its connections closed. */
    readonly #remove: () => Promise<void>;

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

        // create a schema on the test server
        if (dialect === "postgresql") {
            const address = process.env.DESTACK_TEST_POSTGRES;
            if (!address) {
                throw new TypeError("DESTACK_TEST_POSTGRES names no PostgreSQL server");
            }

            // create the schema through the process's one administration connection
            administration ??= Promise.resolve(postgres(address, { max: 1, onnotice: () => {} }));
            const server = await administration;
            const schema = `test_${crypto.randomUUID().replaceAll("-", "")}`;
            await server.unsafe(`CREATE SCHEMA "${schema}"`);

            // reach the schema first on every connection's search path, migrating it when asked
            const open: TestConnector = (connected) =>
                postgresql.connect(
                    postgres(address, { connection: { search_path: schema }, onnotice: () => {} }),
                    connected,
                );
            const database = await open(tables);
            if (options.isMigrated) {
                await migrate(database, declared);
            }

            return new TestDatabase(database, open, async () => {
                await server.unsafe(`DROP SCHEMA "${schema}" CASCADE`);
            });
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
                await copyFile(await sqliteTemplate(declared), file);
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

        return this.#open(tables);
    }

    /** Close the first connection and remove the database. */
    async close(): Promise<void> {
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
}

/** Migrate a SQLite template file of some tables once per process, and return its path. */
function sqliteTemplate(tables: readonly Table[]): Promise<string> {
    // reuse the template of the same declared state
    const key = JSON.stringify(declareState(tables, "sqlite"));
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
        await migrate(database, tables);
        await database.executeScript("PRAGMA wal_checkpoint(TRUNCATE)");
        await database.close();

        return file;
    })();
    templates.set(key, created);

    return created;
}

/** Open a connection to a test database with the tables its queries use. */
type TestConnector = (
    tables: declaration.Database | readonly Table[],
) => Promise<DatabaseConnection & { close(): Promise<void> }>;
