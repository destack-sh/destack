import { mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import {
    Plan,
    type Opener,
    type Provider,
    type Provisioner,
    type Reconciler,
} from "@destack/resource";
import { connect } from "./bun/connection.ts";
import { open, requireReference } from "./connector.ts";
import { DatabaseError } from "../error/error.ts";
import { DatabaseKind } from "../declare/database.ts";
import { mergeStates } from "../migration/merge.ts";
import type { DatabaseHandle } from "../blob/handle.ts";
import { Table } from "../table/table.ts";

/** Database providers by backend. */
export const databaseProvider = {
    /** Provide databases as SQLite files, one folder per space. */
    sqlite,
};

/** Provide databases as SQLite files, one folder per space. */
function sqlite<Object>(
    root: URL,
    object: Object,
): Provider<typeof DatabaseKind, Object, DatabaseHandle> & {
    readonly provision: Provisioner<typeof DatabaseKind>;
    readonly open: Opener<typeof DatabaseKind, DatabaseHandle>;
} {
    // require a directory URL
    if (root.protocol !== "file:" || !root.pathname.endsWith("/")) {
        throw new TypeError(`sqlite provider root must be a file directory URL: ${root.href}`);
    }

    return {
        kind: DatabaseKind,
        code: "sqlite",
        object,
        provision: sqliteProvision(root),
        reconcile: sqliteReconcile(),
        open: sqliteOpen(),
    };
}

/** Create and destroy SQLite database files. */
function sqliteProvision(root: URL): Provisioner<typeof DatabaseKind> {
    return {
        provision: async (record) => {
            // create the space folder and the database file with its log under the space's scope
            const space = new URL(`${record.scope}/`, root);
            const file = new URL(`${record.id}.db`, space);
            await mkdir(space, { recursive: true });
            await using connection = await connect(fileURLToPath(file));
            await connection.log.create(record.scope);

            return { reference: file.href };
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

/** Plan and apply a SQLite database's migration to the desired states. */
function sqliteReconcile(): Reconciler<typeof DatabaseKind> {
    return {
        plan: async (record, desired) => {
            // diff the applied state against the desired states
            await using connection = await open(record, []);

            return await connection.plan(mergeStates(desired.map((state) => state.tables.sqlite)));
        },
        apply: async (record, desired, digest) => {
            // apply the reviewed plan
            await using connection = await open(record, []);
            const plan = await connection.plan(
                mergeStates(desired.map((state) => state.tables.sqlite)),
            );
            if ((await Plan.digest(plan)) !== digest) {
                throw new DatabaseError(
                    "PLAN_CHANGED",
                    `plan of ${record.id} changed since review`,
                );
            }
            await connection.apply(plan);
        },
    };
}

/** Open a SQLite database's desired tables. */
function sqliteOpen(): Opener<typeof DatabaseKind, DatabaseHandle> {
    return {
        open: async (record, desired) => {
            // open the tables the desired states describe, and a replica's local tables beside them once migrated
            const states = desired.map((state) => state.tables.sqlite);
            const described = mergeStates(states).declared.map((state) => Table.describe(state));
            let database = await open(record, described);
            const handle: DatabaseHandle = {
                get database() {
                    return database;
                },
                migrate: async (beside) => {
                    await database.migrate(beside, { states });
                    await database.close();
                    database = await open(record, [...described, ...beside]);
                },
                close: () => database.close(),
            };

            return handle;
        },
    };
}
