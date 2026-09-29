import { mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import {
    Plan,
    type ResourceRecord,
    type Provider,
    type Provisioning,
    type Copying,
} from "@destack/resource";
import type { DatabaseConnection } from "../database/connection.ts";
import { Database } from "../declare/database.ts";
import { connect } from "./bun/connection.ts";
import type { SqliteDatabase } from "./database.ts";
import type { Table } from "../table/table.ts";
import { DatabaseError } from "../error/error.ts";
import { Replication } from "../replication/replication.ts";
import type { DatabaseKind } from "../declare/database.ts";

/** Provide databases as SQLite files, one folder per space. */
export function sqliteProvider(
    root: URL,
): Provider<DatabaseConnection, typeof DatabaseKind> & Provisioning & Copying {
    // require a directory URL
    if (root.protocol !== "file:" || !root.pathname.endsWith("/")) {
        throw new TypeError(`sqlite provider root must be a file directory URL: ${root.href}`);
    }

    return {
        kind: "database",
        code: "sqlite",
        provision: async (record) => {
            // create the space folder and the database file
            const space = new URL(`${record.scope}/`, root);
            const file = new URL(`${record.id}.db`, space);
            await mkdir(space, { recursive: true });
            const connection = await connect(fileURLToPath(file));
            await connection.close();

            return { reference: file.href };
        },
        plan: async (record, desired) => {
            // diff the applied state against the desired states
            const connection = await open(record, []);
            try {
                return await connection.plan(desired);
            } finally {
                await connection.close();
            }
        },
        apply: async (record, desired, digest) => {
            // apply the reviewed plan
            const connection = await open(record, []);
            try {
                const plan = await connection.plan(desired);
                if ((await Plan.digest(plan)) !== digest) {
                    throw new DatabaseError(
                        "PLAN_CHANGED",
                        `plan of ${record.id} changed since review`,
                    );
                }
                await connection.apply(plan);
            } finally {
                await connection.close();
            }
        },
        connect: async (record, declaration) => {
            // refuse a database lacking its tables
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
        export: async function* (copy, after, signal) {
            // read the source file's tables
            const replication = Replication.of(copy.desired, "sqlite");
            const connection = await open(copy.record, replication.tables);
            try {
                yield* replication.export(
                    connection,
                    `copy:${copy.record.id}`,
                    copy.stage,
                    after,
                    signal,
                );
            } finally {
                await connection.close();
            }
        },
        import: async (copy, chunk) => {
            // write the chunk into the target
            const replication = Replication.of(copy.desired, "sqlite");
            const connection = await open(copy.record, replication.tables);
            try {
                await replication.import(connection, chunk);
            } finally {
                await connection.close();
            }
        },
        destroy: async (record) => {
            // remove the file with its WAL and shared memory
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

/** Open a provisioned database file. */
function open(record: ResourceRecord, tables: readonly Table[]): Promise<SqliteDatabase> {
    return connect(fileURLToPath(requireReference(record.reference)), tables);
}
