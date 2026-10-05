import { mkdir, rm } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import type { Digest } from "@destack/schema";
import { type KindState, Plan, type ResourceRecord } from "@destack/resource";
import { connect } from "./connection.ts";
import { open, requireReference } from "./sqlite.ts";
import { DatabaseError } from "../error/error.ts";
import type { DatabaseKind } from "../declare/database.ts";
import type { DatabaseHost } from "../provider/provider.ts";
import { mergeStates } from "../migration/merge.ts";
import type { DatabaseHandle } from "../blob/handle.ts";
import { Table } from "../table/table.ts";

/** Keep databases as SQLite files below a directory, one folder per space. */
export class SqliteDatabaseHost implements DatabaseHost {
    /** The provider code of SQLite files. */
    readonly provider = "sqlite";
    /** The directory URL holding a folder per space. */
    readonly root: URL;

    /** Keep databases below a file directory URL. */
    constructor(root: URL) {
        // require a directory URL
        if (root.protocol !== "file:" || !root.pathname.endsWith("/")) {
            throw new TypeError(`sqlite database root must be a file directory URL: ${root.href}`);
        }
        this.root = root;
    }

    /** Create the space folder and the database file with its log under the space's scope. */
    async provision(record: ResourceRecord<typeof DatabaseKind>): Promise<{ reference: string }> {
        // create the folder and the file, and start its log
        const space = new URL(`${record.scope}/`, this.root);
        const file = new URL(`${record.id}.db`, space);
        await mkdir(space, { recursive: true });
        await using connection = await connect(fileURLToPath(file));
        await connection.log.create(record.scope);

        return { reference: file.href };
    }

    /** Remove the file with its WAL and shared memory. */
    async destroy(record: ResourceRecord<typeof DatabaseKind>): Promise<void> {
        const path = fileURLToPath(requireReference(record.reference));
        for (const suffix of ["", "-wal", "-shm"]) {
            await rm(`${path}${suffix}`, { force: true });
        }
    }

    /** Diff the applied state against the desired states. */
    async plan(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<Plan> {
        await using connection = await open(record, []);

        return await connection.plan(mergeStates(desired.map((state) => state.tables.sqlite)));
    }

    /** Apply the reviewed plan, refusing one that changed since review. */
    async apply(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
        digest: Digest,
    ): Promise<void> {
        // plan again and apply the plan only while it matches the reviewed digest
        await using connection = await open(record, []);
        const plan = await connection.plan(
            mergeStates(desired.map((state) => state.tables.sqlite)),
        );
        if ((await Plan.digest(plan)) !== digest) {
            throw new DatabaseError("PLAN_CHANGED", `plan of ${record.id} changed since review`);
        }
        await connection.apply(plan);
    }

    /** Open the tables the desired states describe, and a replica's local tables beside them once migrated. */
    async open(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<DatabaseHandle> {
        // open the desired tables and reopen them with a replica's tables once migrated
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
    }
}
