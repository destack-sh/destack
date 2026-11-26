import type { Opener, Provider, Provisioner, Reconciler, Snapshotter } from "@destack/resource";
import { DatabaseKind } from "../declare/database.ts";
import type { DatabaseHandle } from "../database/handle.ts";
import type { DatabaseConnection } from "../database/connection.ts";
import type { Table } from "../table/table.ts";
import type { DatabaseSchema } from "../declare/database.ts";

/** Where a backend stores databases: creating, migrating, opening, copying and destroying them. */
export interface DatabaseHost
    extends
        Provisioner<typeof DatabaseKind>,
        Reconciler<typeof DatabaseKind>,
        Opener<typeof DatabaseKind, DatabaseHandle>,
        Snapshotter<typeof DatabaseKind> {
    /** The provider code of the backend, such as `sqlite`. */
    readonly provider: string;
    /** Resolve a reference to where a workload on this host reaches a local database file. */
    readonly resolve?: (reference: string) => string;
}

/** A host keeping many databases by reference, as SQLite files or a cluster's schemas. */
export interface DatabaseCluster extends DatabaseHost, AsyncDisposable {
    /** Name the database of an identifier in a scope. */
    readonly reference: (scope: string, id: string) => string;
    /** Create an empty database unless it exists. */
    readonly create: (reference: string) => Promise<void>;
    /** Connect to a database with a schema, its replicated tables included, or some tables. */
    readonly connect: (
        reference: string,
        tables: DatabaseSchema | readonly Table[],
    ) => Promise<DatabaseConnection & AsyncDisposable & { readonly close: () => Promise<void> }>;
    /** Count a database's bytes. */
    readonly bytes: (reference: string) => Promise<number>;
    /** Drop a database with its contents. */
    readonly drop: (reference: string) => Promise<void>;
    /** List the scopes whose own database the cluster keeps, by a prefix of their identifiers. */
    readonly scopes: (prefix: string) => Promise<string[]>;
}

/** A database provider: provisioning, migrating, opening and snapshotting one host's databases. */
export type DatabaseProvider<Object> = Provider<typeof DatabaseKind, Object, DatabaseHandle> & {
    readonly provision: Provisioner<typeof DatabaseKind>;
    readonly reconcile: Reconciler<typeof DatabaseKind>;
    readonly open: Opener<typeof DatabaseKind, DatabaseHandle>;
    readonly snapshot: Snapshotter<typeof DatabaseKind>;
};

/** Provide a host's databases under its provider code, as an object, copied where stored. */
export function databaseProvider<Object>(
    host: DatabaseHost,
    object: Object,
): DatabaseProvider<Object> {
    return {
        kind: DatabaseKind,
        code: host.provider,
        object,
        provision: host,
        reconcile: host,
        open: host,
        snapshot: host,
        ...(host.resolve === undefined ? {} : { resolve: host.resolve }),
    };
}
