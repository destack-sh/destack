import { mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { Plan, type Open, type Provider, type Provision } from "@destack/resource";
import { connect } from "./bun/connection.ts";
import { open, requireReference } from "./connector.ts";
import { DatabaseError } from "../error/error.ts";
import { DatabaseKind } from "../declare/database.ts";
import { mergeStates } from "../migration/merge.ts";
import type { DatabaseHandle } from "../blob/handle.ts";
import { Table } from "../table/table.ts";

/** Provide databases as SQLite files, one folder per space. */
export function sqliteProvider<Object>(
    root: URL,
    object: Object,
): Provider<typeof DatabaseKind, Object> &
    Provision<typeof DatabaseKind> &
    Open<typeof DatabaseKind, DatabaseHandle> {
    // require a directory URL
    if (root.protocol !== "file:" || !root.pathname.endsWith("/")) {
        throw new TypeError(`sqlite provider root must be a file directory URL: ${root.href}`);
    }

    return {
        kind: DatabaseKind,
        code: "sqlite",
        object,
        provision: async (record) => {
            // create the space folder and the database file with its log under the space's scope
            const space = new URL(`${record.scope}/`, root);
            const file = new URL(`${record.id}.db`, space);
            await mkdir(space, { recursive: true });
            await using connection = await connect(fileURLToPath(file));
            await connection.log.create(record.scope);

            return { reference: file.href };
        },
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
        open: async (record, desired) => {
            // open the tables the desired states describe, and a replica's own tables beside them once migrated
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
        destroy: async (record) => {
            // remove the file with its WAL and shared memory
            const path = fileURLToPath(requireReference(record.reference));
            for (const suffix of ["", "-wal", "-shm"]) {
                await rm(`${path}${suffix}`, { force: true });
            }
        },
    };
}
