import { mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { digestPlan, type Provider, type ResourceRecord } from "@destack/resource";
import type { DatabaseConnection } from "../database/connection.ts";
import { Database } from "../declare/database.ts";
import { connect } from "./turso/connection.ts";
import type { SqliteDatabase } from "./database.ts";
import { planStates } from "../migration/database.ts";
import { applyPlan } from "../migration/apply.ts";
import type { Table } from "../table/table.ts";
import { DatabaseError } from "../error/error.ts";

/** Provide databases as SQLite files under a directory URL, one per resource in a folder per space. */
export function sqliteProvider(root: URL): Provider<DatabaseConnection> {
    // require a directory, whose URL ends with a slash
    if (root.protocol !== "file:" || !root.pathname.endsWith("/")) {
        throw new TypeError(`sqlite provider root must be a file directory URL: ${root.href}`);
    }

    return {
        kind: "database",
        code: "sqlite",
        provision: async (record) => {
            // create the space folder and the database file
            const space = new URL(`${record.spaceId}/`, root);
            const file = new URL(`${record.id}.db`, space);
            await mkdir(space, { recursive: true });
            const connection = await connect(fileURLToPath(file));
            await connection.close();

            return { reference: file.href };
        },
        plan: async (record, desired) => {
            // diff the file's applied state against the union of its desired states
            const connection = await open(record, []);
            try {
                return await planStates(connection, desired);
            } finally {
                await connection.close();
            }
        },
        apply: async (record, desired, digest) => {
            // apply only the plan a review saw
            const connection = await open(record, []);
            try {
                const plan = await planStates(connection, desired);
                if ((await digestPlan(plan)) !== digest) {
                    throw new DatabaseError(
                        "PLAN_CHANGED",
                        `plan of ${record.id} changed since review`,
                    );
                }
                await applyPlan(connection, plan);
            } finally {
                await connection.close();
            }
        },
        connect: async (record, declaration) => {
            // refuse a database lacking the tables its declaration requires
            const database = requireDatabase(declaration);
            const connection = await open(record, database.tables);
            const unapplied = await database.check(connection);
            if (unapplied.length > 0) {
                await connection.close();
                throw new DatabaseError(
                    "NOT_APPLIED",
                    `database ${database.name} has not applied ${unapplied.join(", ")}`,
                );
            }

            return connection;
        },
        destroy: async (record) => {
            // remove the database file with its write-ahead log and shared memory
            const path = fileURLToPath(requireReference(record.reference));
            for (const suffix of ["", "-wal", "-shm"]) {
                await rm(`${path}${suffix}`, { force: true });
            }
        },
    };
}

/** Require a provisioned resource reference. */
function requireReference(reference: string | null): string {
    if (reference === null) {
        throw new TypeError("resource is not provisioned");
    }

    return reference;
}

/** Require a database declaration. */
function requireDatabase(declaration: unknown): Database {
    if (!(declaration instanceof Database)) {
        throw new TypeError("not a database declaration");
    }

    return declaration;
}

/** Open a provisioned database file with the tables queries use. */
function open(record: ResourceRecord, tables: readonly Table[]): Promise<SqliteDatabase> {
    return connect(fileURLToPath(requireReference(record.reference)), tables);
}
