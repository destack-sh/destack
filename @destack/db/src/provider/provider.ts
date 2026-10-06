import type { Opener, Provider, Provisioner, Reconciler, Snapshotter } from "@destack/resource";
import { DatabaseKind } from "../declare/database.ts";
import { DatabaseHandle } from "../database/handle.ts";

/** Where a backend keeps databases: creating, migrating, opening and destroying each one. */
export interface DatabaseHost
    extends
        Provisioner<typeof DatabaseKind>,
        Reconciler<typeof DatabaseKind>,
        Opener<typeof DatabaseKind, DatabaseHandle> {
    /** The provider code of the backend, such as `sqlite`. */
    readonly provider: string;
}

/** A database provider: provisioning, migrating, opening and snapshotting one host's databases. */
export type DatabaseProvider<Object> = Provider<typeof DatabaseKind, Object, DatabaseHandle> & {
    readonly provision: Provisioner<typeof DatabaseKind>;
    readonly reconcile: Reconciler<typeof DatabaseKind>;
    readonly open: Opener<typeof DatabaseKind, DatabaseHandle>;
    readonly snapshot: Snapshotter<typeof DatabaseKind>;
};

/** Provide a host's databases under its provider code, serving them as an object and copying them through their handles. */
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
        snapshot: DatabaseHandle.snapshotter(host),
    };
}
