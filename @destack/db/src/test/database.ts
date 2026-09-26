import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { sql } from "drizzle-orm";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Dialect } from "../dialect/dialect.ts";
import type { Table } from "../table/table.ts";
import type * as declaration from "../declare/database.ts";
import * as turso from "../sqlite/turso/connection.ts";
import * as postgres from "../postgres/connection.ts";

/** The dialects tests run on: SQLite, and PostgreSQL when DESTACK_TEST_POSTGRES names a server. */
export const TEST_DIALECTS: readonly Dialect[] = [
    "sqlite",
    ...(process.env.DESTACK_TEST_POSTGRES ? (["postgresql"] as const) : []),
];

/** An isolated database for one test: in memory or a temporary file for SQLite, a new database for PostgreSQL. */
export class TestDatabase {
    /** The first connection, bound to the tables given at creation. */
    readonly database: DatabaseConnection & { close(): Promise<void> };
    /** The SQLite file or PostgreSQL URL further connections open, absent for SQLite in memory. */
    readonly location?: string;
    /** Remove the database once its connections closed. */
    readonly #remove: () => Promise<void>;

    /** Retain the first connection and how to remove the database. */
    private constructor(
        database: TestDatabase["database"],
        location: string | undefined,
        remove: () => Promise<void>,
    ) {
        this.database = database;
        this.location = location;
        this.#remove = remove;
    }

    /** Create an empty isolated database of a dialect, kept in a file when further connections need it. */
    static async create(
        dialect: Dialect,
        tables: declaration.Database | readonly Table[],
        storage: "memory" | "file" = "memory",
    ): Promise<TestDatabase> {
        // create a new PostgreSQL database on the test server
        if (dialect === "postgresql") {
            const address = process.env.DESTACK_TEST_POSTGRES;
            if (!address) {
                throw new TypeError("DESTACK_TEST_POSTGRES names no PostgreSQL server");
            }
            const administration = await postgres.connect(address);
            const name = `test_${crypto.randomUUID().replaceAll("-", "")}`;
            await administration.execute(sql`CREATE DATABASE ${sql.identifier(name)}`);
            const url = new URL(address);
            url.pathname = `/${name}`;

            return new TestDatabase(
                await postgres.connect(url.href, tables),
                url.href,
                async () => {
                    await administration.execute(sql`DROP DATABASE ${sql.identifier(name)}`);
                    await administration.close();
                },
            );
        }
        // keep SQLite in memory
        else if (storage === "memory") {
            return new TestDatabase(
                await turso.connect(":memory:", tables),
                undefined,
                async () => {},
            );
        }
        // keep SQLite in a temporary file
        else {
            const directory = await mkdtemp(join(tmpdir(), "destack-test-"));
            const file = join(directory, "test.db");

            return new TestDatabase(await turso.connect(file, tables), file, () =>
                rm(directory, { recursive: true }),
            );
        }
    }

    /** Open another connection to the same database. */
    async connect(
        tables: declaration.Database | readonly Table[],
    ): Promise<DatabaseConnection & { close(): Promise<void> }> {
        // require a database other connections can reach
        if (this.location === undefined) {
            throw new TypeError("an in-memory SQLite database has no further connections");
        }

        return this.location.startsWith("postgres")
            ? await postgres.connect(this.location, tables)
            : await turso.connect(this.location, tables);
    }

    /** Close the first connection and remove the database. */
    async close(): Promise<void> {
        await this.database.close();
        await this.#remove();
    }
}
