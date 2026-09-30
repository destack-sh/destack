import { mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { Plan, type Copying, type Provider, type Provisioning } from "@destack/resource";
import { connect } from "./bun/connection.ts";
import { open, requireReference } from "./connector.ts";
import { DatabaseError } from "../error/error.ts";
import { Replication } from "../replication/replication.ts";
import { DatabaseKind } from "../declare/database.ts";
import { mergeStates } from "../migration/merge.ts";

/** Provide databases as SQLite files, one folder per space. */
export function sqliteProvider(
    root: URL,
): Provider<typeof DatabaseKind> &
    Provisioning<typeof DatabaseKind> &
    Copying<typeof DatabaseKind> {
    // require a directory URL
    if (root.protocol !== "file:" || !root.pathname.endsWith("/")) {
        throw new TypeError(`sqlite provider root must be a file directory URL: ${root.href}`);
    }

    return {
        kind: DatabaseKind,
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
                return await connection.plan(
                    mergeStates(desired.map((state) => state.tables.sqlite)),
                );
            } finally {
                await connection.close();
            }
        },
        apply: async (record, desired, digest) => {
            // apply the reviewed plan
            const connection = await open(record, []);
            try {
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
            } finally {
                await connection.close();
            }
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
