import type { DatabaseConnection } from "../database/connection.ts";
import type { Table } from "../table/table.ts";
import type { BlobStore } from "./blob.ts";

/** A database a provider opened for another party: its connection, its blob store, and how it migrates itself. */
export interface DatabaseHandle {
    /** The connection. */
    readonly database: DatabaseConnection;
    /** The store of the content its blob columns reference, absent for a database without blob columns. */
    readonly blobs?: BlobStore;
    /** Migrate the database to its own tables and some tables beside them, dropping earlier ones beside. */
    migrate(beside: readonly Table[]): Promise<void>;
    /** Release the database. */
    close(): Promise<void>;
}
