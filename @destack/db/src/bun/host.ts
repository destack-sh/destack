import { type Environment, FileSystemError } from "@destack/environment";
import { fileURLToPath } from "node:url";
import type { Digest } from "@destack/schema";
import { type KindState, Plan, ResourceRecord } from "@destack/resource";
import { connectBunSqlite } from "./connection.ts";
import { openBunSqlite } from "./sqlite.ts";
import type { DatabaseKind } from "../declare/database.ts";
import type { DatabaseHost } from "../provider/provider.ts";
import { DatabaseHandle } from "../database/handle.ts";
import { Table } from "../table/table.ts";

/** Store databases as SQLite files below a directory, one folder per space. */
export class BunSqliteDatabaseHost implements DatabaseHost {
    /** The provider code of SQLite files. */
    readonly provider: string = "sqlite";
    /** The directory URL with a folder per space. */
    readonly root: URL;
    /** The machine's environment the databases run in, with its files and servers. */
    readonly environment: Environment<"files" | "server">;

    /** Store databases below a file directory URL, in the machine's environment. */
    constructor(root: URL, environment: Environment<"files" | "server">) {
        // require a directory URL
        if (root.protocol !== "file:" || !root.pathname.endsWith("/")) {
            throw new TypeError(`sqlite database root must be a file directory URL: ${root.href}`);
        }
        this.root = root;
        this.environment = environment;
    }

    /** Resolve a database's reference below the root to its file's URL. */
    readonly resolve = (reference: string): string => new URL(reference, this.root).href;

    /** Create the space folder and the database file with its log, referenced below the root. */
    async provision(record: ResourceRecord<typeof DatabaseKind>): Promise<{ reference: string }> {
        // create the folder and the file, and start its log
        const reference = `${record.scope}/${record.id}.db`;
        const file = fileURLToPath(this.resolve(reference));
        await this.environment.files.mkdir(fileURLToPath(new URL(`${record.scope}/`, this.root)), {
            recursive: true,
        });
        await using connection = await connectBunSqlite(file, [], {
            environment: this.environment,
        });
        await connection.log.create(record.scope);

        return { reference };
    }

    /** Count a database's bytes: its file and write-ahead log, if a checkpoint left one. */
    async bytes(reference: string): Promise<number> {
        // read the file's size, and its log's
        const { files } = this.environment;
        const path = fileURLToPath(this.resolve(reference));
        const file = await files.stat(path);
        const log = await files.stat(`${path}-wal`).catch((error: unknown) => {
            // count no log once a checkpoint removed it
            if (FileSystemError.is(error, "NOT_FOUND")) {
                return { size: 0 };
            }

            throw error;
        });

        return file.size + log.size;
    }

    /** Remove the file with its WAL and shared memory. */
    async destroy(record: ResourceRecord<typeof DatabaseKind>): Promise<void> {
        const path = fileURLToPath(this.resolve(ResourceRecord.reference(record)));
        const { files } = this.environment;
        for (const suffix of ["", "-wal", "-shm"]) {
            await files.remove(`${path}${suffix}`, { force: true });
        }
    }

    /** Diff the applied state against the desired states. */
    plan(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<Plan> {
        const connect = (tables: readonly Table[]) => this.#open(record, tables);

        return DatabaseHandle.plan(
            connect,
            desired.map((state) => state.tables.sqlite),
        );
    }

    /** Apply the reviewed plan, refusing one that changed since review. */
    apply(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
        digest: Digest,
    ): Promise<void> {
        const connect = (tables: readonly Table[]) => this.#open(record, tables);
        const states = desired.map((state) => state.tables.sqlite);

        return DatabaseHandle.apply(connect, states, digest, record.id);
    }

    /** Open the tables of the desired states, and a replica's local tables once migrated. */
    open(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly KindState<typeof DatabaseKind>[],
    ): Promise<DatabaseHandle> {
        const connect = (tables: readonly Table[]) => this.#open(record, tables);

        return DatabaseHandle.open(
            connect,
            desired.map((state) => state.tables.sqlite),
        );
    }

    /** Open a database's file at its resolved reference, with some tables. */
    #open(record: ResourceRecord<typeof DatabaseKind>, tables: readonly Table[]) {
        const url = this.resolve(ResourceRecord.reference(record));

        return openBunSqlite({ reference: url }, tables, this.environment);
    }

    /** Copy a database's rows into a store as content-addressed pages, returning the digest. */
    snapshot(...copy: Parameters<DatabaseHost["snapshot"]>): Promise<Digest> {
        return DatabaseHandle.snapshotter(this).snapshot(...copy);
    }

    /** Restore a copy's rows into a database, onto a base copy it already has when given. */
    restore(...copy: Parameters<DatabaseHost["restore"]>): Promise<void> {
        return DatabaseHandle.snapshotter(this).restore(...copy);
    }
}
