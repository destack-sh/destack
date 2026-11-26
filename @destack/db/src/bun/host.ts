import { type Environment, FileSystemError } from "@destack/environment";
import { dirname } from "node:path";
import { fileURLToPath } from "node:url";
import type { Digest } from "@destack/schema";
import { Plan, ResourceRecord } from "@destack/resource";
import { openBunSqlite } from "./sqlite.ts";
import type { DatabaseKind, DatabaseSchema } from "../declare/database.ts";
import type { DatabaseCluster, DatabaseHost } from "../provider/provider.ts";
import { DatabaseHandle } from "../database/handle.ts";
import { Table } from "../table/table.ts";
import type { DatabaseState } from "../migration/state.ts";

/** Store databases as SQLite files below a directory, one folder per space. */
export class BunSqliteDatabaseHost implements DatabaseCluster {
    /** The provider code of SQLite files. */
    readonly provider: string = "sqlite";
    /** Provisioning creates the database's file, so it is not pure. */
    readonly isPure = false;
    /** The directory URL with a folder per space. */
    readonly root: URL;
    /** The machine's environment the databases run in, with its files and servers. */
    readonly environment: Environment<"files" | "server">;

    /** Store databases below a file directory URL, in the machine's environment. */
    constructor(root: URL, environment: Environment<"files" | "server">) {
        // require a directory URL
        if (root.protocol !== "file:" || !root.pathname.endsWith("/")) {
            throw new TypeError(
                `sqlite database root must be a file directory URL: '${root.href}'`,
            );
        }
        this.root = root;
        this.environment = environment;
    }

    /** Resolve a database's reference below the root to its file's URL. */
    readonly resolve = (reference: string): string => new URL(reference, this.root).href;

    /** Create a scope's database as a file below the root, and start the scope's log in it. */
    async provision(record: ResourceRecord<typeof DatabaseKind>): Promise<{ reference: string }> {
        // create the file in the scope's folder, then start its log
        const reference = this.reference(record.scope, record.id);
        await this.create(reference);
        await using connection = await this.connect(reference, []);
        await connection.log.create(record.scope);

        return { reference };
    }

    /** Name the database of an identifier in a scope: its file in the scope's folder. */
    reference(scope: string, id: string): string {
        return `${scope}/${id}.db`;
    }

    /** Create an empty database file unless it exists, with its folder. */
    async create(reference: string): Promise<void> {
        // make the folder, then open the file once, which creates it
        const file = fileURLToPath(this.resolve(reference));
        await this.environment.files.mkdir(dirname(file), { recursive: true });
        const connection = await this.connect(reference, []);
        await connection.close();
    }

    /** Connect to a database's file at its resolved reference, with a schema or some tables. */
    connect(reference: string, tables: DatabaseSchema | readonly Table[]) {
        return openBunSqlite({ reference: this.resolve(reference) }, tables, this.environment);
    }

    /** List the scopes whose own database file the root keeps, by a prefix of their identifiers. */
    async scopes(prefix: string): Promise<string[]> {
        // take each folder of the prefix holding its scope's own file
        const { files } = this.environment;
        const root = fileURLToPath(this.root);
        const entries = (await files.exists(root)) ? await files.readDirectory(root) : [];
        const named = entries.map((entry) => entry.name).filter((name) => name.startsWith(prefix));
        const kept = await Promise.all(
            named.map(async (scope) =>
                (await files.exists(fileURLToPath(this.resolve(this.reference(scope, scope)))))
                    ? [scope]
                    : [],
            ),
        );

        return kept.flat().toSorted((left, right) => left.localeCompare(right));
    }

    /** Count a database's bytes: its file and write-ahead log, if a checkpoint left one. */
    async bytes(reference: string): Promise<number> {
        // read the file's size, and its log's
        const { files } = this.environment;
        const path = fileURLToPath(this.resolve(reference));
        const file = await files.stat(path);
        const log = await files.stat(`${path}-wal`).catch((error: unknown) => {
            // count no log once a checkpoint removed it
            if (FileSystemError.is(error, "FILE_NOT_FOUND")) {
                return { size: 0 };
            }

            throw error;
        });

        return file.size + log.size;
    }

    /** Close nothing, as each database's file connects on its own. */
    async [Symbol.asyncDispose](): Promise<void> {}

    /** Remove the database's file with its WAL and shared memory. */
    async destroy(record: ResourceRecord<typeof DatabaseKind>): Promise<void> {
        await this.drop(ResourceRecord.reference(record));
    }

    /** Remove a database's file with its WAL and shared memory. */
    async drop(reference: string): Promise<void> {
        const path = fileURLToPath(this.resolve(reference));
        const { files } = this.environment;
        for (const suffix of ["", "-wal", "-shm"]) {
            await files.remove(`${path}${suffix}`, { force: true });
        }
    }

    /** Diff the applied state against the desired states. */
    plan(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
    ): Promise<Plan> {
        const connect = (tables: readonly Table[]) => this.#open(record, tables);

        return DatabaseHandle.plan(
            connect,
            desired.map((state) => state.tables.sqlite),
            record.id,
        );
    }

    /** Apply the reviewed plan, refusing one that changed since review. */
    apply(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
        digest: Digest,
    ): Promise<void> {
        const connect = (tables: readonly Table[]) => this.#open(record, tables);
        const states = desired.map((state) => state.tables.sqlite);

        return DatabaseHandle.apply(connect, states, digest, record.id);
    }

    /** Open the tables of the desired states, and a replica's local tables once migrated. */
    open(
        record: ResourceRecord<typeof DatabaseKind>,
        desired: readonly DatabaseState[],
    ): Promise<DatabaseHandle> {
        const connect = (tables: readonly Table[]) => this.#open(record, tables);

        return DatabaseHandle.open(
            connect,
            desired.map((state) => state.tables.sqlite),
            record.id,
        );
    }

    /** Open a database's file by its record, with some tables. */
    #open(record: ResourceRecord<typeof DatabaseKind>, tables: readonly Table[]) {
        return this.connect(ResourceRecord.reference(record), tables);
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
