import { type SQL, sql } from "drizzle-orm";
import type { Migration } from "./migration.ts";
import type { MigrationDescription } from "../inspect/migration.ts";
import { DatabaseError } from "../error/index.ts";

/** Compare committed migrations with the database's ordered history. */
export class MigrationHistory {
    /** The SQL name of the history table. */
    readonly name: string;
    /** The history table identifier. */
    readonly table: ReturnType<typeof sql.identifier>;
    /** The complete committed migration sequence. */
    readonly migrations: readonly Migration[];

    /** Select the history table and committed migrations. */
    constructor(migrations: readonly Migration[], name: string) {
        this.name = `__destack_migrations_${name}`;
        this.table = sql.identifier(this.name);
        this.migrations = migrations;
    }

    /** Create the schema's ordered migration history. */
    create(): SQL {
        return sql`CREATE TABLE IF NOT EXISTS ${this.table} (
            id INTEGER PRIMARY KEY,
            name TEXT NOT NULL UNIQUE,
            checksum TEXT NOT NULL,
            applied_at TEXT NOT NULL
        )`;
    }

    /** Acquire the transaction's write lock before reading history. */
    lock(): SQL {
        return sql`UPDATE ${this.table} SET id = id WHERE 0`;
    }

    /** Read migrations in their application order. */
    select(): SQL {
        return sql`SELECT name, checksum FROM ${this.table} ORDER BY id`;
    }

    /** Record a migration in the transaction that applies its SQL. */
    insert(migration: Migration, position: number): SQL {
        return sql`INSERT INTO ${this.table} (id, name, checksum, applied_at)
            VALUES (${position}, ${migration.name}, ${migration.checksum},
                ${new Date().toISOString()})`;
    }

    /** Reject changed or missing history and select the unapplied suffix. */
    pending(applied: readonly MigrationDescription[]): readonly Migration[] {
        // require applied migrations to remain an exact prefix of the source history
        for (const [index, previous] of applied.entries()) {
            const migration = this.migrations[index];
            if (
                !migration ||
                previous.name !== migration.name ||
                previous.checksum !== migration.checksum
            ) {
                throw new DatabaseError(
                    "MIGRATION_HISTORY",
                    `Database migration history differs at ${previous.name}.`,
                );
            }
        }

        return this.migrations.slice(applied.length);
    }
}
