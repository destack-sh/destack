import { type Resource, appliedStates } from "./resource.ts";
import { AccessContext, Caller, principal } from "@destack/access";
import { Condition, Lease, lt, type DatabaseConnection, Snapshot, Transaction } from "@destack/db";
import { type ObjectReconciliation } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import { Plan, type ContentStore } from "@destack/resource";
import { Recipient } from "@destack/identity";
import { type Digest, type Identifier, present } from "@destack/schema";
import { Scope } from "@destack/sync";
import { Restore, restore, snapshot } from "../../object/snapshot.ts";
import { SpaceTransfer, type TransferOptions } from "../host/transfer.ts";
import type { Transfer } from "../../object/index.ts";
import type { Handoff } from "../host/handoff.ts";
import { RequestId } from "@destack/runtime/request";
import { Clock } from "@destack/environment";

/** How long a snapshot no move retires stays in the store: a day. */
const SNAPSHOT_RETENTION_MILLISECONDS = 24 * 60 * 60 * 1000;

/** The first wait before a target asks again for a snapshot not taken yet, in milliseconds. */
const SNAPSHOT_POLL_INITIAL_MILLISECONDS = 10;

/** The longest wait between a target's asks, doubling up to a second for snapshots of minutes. */
const SNAPSHOT_POLL_MILLISECONDS = 1000;

/** The lease a space's content store sweep takes, which its snapshots and restores wait out. */
export const SWEEP_LEASE = { name: "snapshot-sweep", key: "store" };

/** How long the sweep's lease lasts, renewed every third while the sweep deletes: a minute. */
const SWEEP_LEASE_MILLISECONDS = 60 * 1000;

/** The snapshots not taken yet, which write blobs into the store. */
const TAKING: Condition = { takenAt: { isNull: true } };

/** The restores not restored yet or not caught up once fenced, which write blobs into the store. */
const RESTORING: Condition = {
    OR: [
        { restoredAt: { isNull: true } },
        { fencedAt: { isNotNull: true }, caughtUpAt: { isNull: true } },
    ],
};

/** Delete a space's snapshots once its move ended, as the space. */
export async function retireSnapshots(
    transaction: Transaction,
    store: Pick<ObjectStore, "mutate">,
    spaceId: Identifier<"space">,
    now: number,
): Promise<void> {
    const retired = await transaction.read(snapshot.table).findMany({ where: { spaceId } });
    await store.mutate(
        AccessContext.of(Caller.of(principal.space.reference(Scope.universe.id, spaceId)), now),
        {
            id: RequestId.create(now),
            calls: retired.map(({ id }) => snapshot.call("delete", { id })),
        },
        { transaction, scope: Scope.universe.id },
    );
}

/** Implement snapshots: take each pending one into the space's content store. */
export function implementSnapshot(options: TransferOptions) {
    return snapshot.implement({
        controller: {
            pending: TAKING,
            reconcile: async (reconciliation) => {
                // wait out a sweep of the store before writing into it
                const swept = await sweepWait(reconciliation.store.database, reconciliation.now);
                if (swept !== undefined) {
                    return swept;
                }

                // take each pending snapshot
                for (const row of reconciliation.rows) {
                    // find the resource and the provider keeping it here
                    const { spaceId } = row;
                    const { object, row: resource } = present(
                        await options.providers.find(
                            Snapshot.live(reconciliation.store.database),
                            spaceId,
                            row.resourceId,
                        ),
                        `resource ${row.resourceId}`,
                    );
                    const { kind, snapshot: snapshotter } = options.providers.snapshotting(
                        object,
                        resource.provider,
                    );

                    // copy its content into the store and record the copy with its blobs as the space
                    const { store, blobs } = recording(options.store);
                    const digest = await snapshotter.snapshot(
                        kind.record(resource),
                        kind.states(await appliedStates(reconciliation.store.database, resource)),
                        store,
                        Recipient.of(row.recipient),
                    );
                    await reconciliation.store.mutate(
                        AccessContext.of(
                            Caller.of(principal.space.reference(Scope.universe.id, spaceId)),
                            reconciliation.now,
                        ),
                        {
                            id: RequestId.create(reconciliation.now),
                            calls: [
                                snapshot.call("update", {
                                    ...snapshot.target(row),
                                    digest,
                                    takenAt: reconciliation.now,
                                    blobs: [...blobs, digest],
                                }),
                            ],
                        },
                        { component: snapshot.name },
                    );
                }

                return undefined;
            },
        },
    });
}

/** Implement restores: restore each pending one into a resource provisioned here. */
export function implementRestore(options: TransferOptions) {
    return restore.implement({
        controller: {
            pending: RESTORING,
            reconcile: async (reconciliation) => {
                // wait out a sweep of the store before writing into it
                const swept = await sweepWait(reconciliation.store.database, reconciliation.now);
                if (swept !== undefined) {
                    return swept;
                }

                // restore each pending restore
                for (const row of reconciliation.rows) {
                    await restoreRow(options, reconciliation, row);
                }

                return undefined;
            },
        },
    });
}

/** A restore of a resource, as the machine's object store reconciles it. */
type RestoreReconciliation = ObjectReconciliation<typeof restore>;

/** A copy of a resource restored from. */
interface Copy {
    /** The snapshot's digest. */
    readonly digest: Digest;
    /** The space's store, reading blobs through the source's handoff. */
    readonly store: Pick<ContentStore, "read" | "write" | "fetch">;
    /** The recipient the source encrypted the machine-bound values to. */
    readonly recipient: Recipient;
    /** The resource's applied states. */
    readonly desired: Awaited<ReturnType<typeof appliedStates>>;
}

/** Restore one pending restore from a snapshot its source machine takes for it. */
async function restoreRow(
    options: TransferOptions,
    reconciliation: RestoreReconciliation,
    row: RestoreReconciliation["rows"][number],
): Promise<void> {
    // find the resource in this machine's copy of the space
    const { spaceId } = row;
    const found = present(
        await options.providers.find(
            Snapshot.live(reconciliation.store.database),
            spaceId,
            row.resourceId,
        ),
        `resource ${row.resourceId}`,
    );

    // have the source snapshot the resource for a recipient of this attempt
    const recipient = await Recipient.generate();
    const source = await options.source(row.sourceId, spaceId);
    const moving = await SpaceTransfer.readActive(reconciliation.store.database, spaceId);
    const requested = { resourceId: row.resourceId, recipient: recipient.key };
    const digest = await snapshotOf(source, moving, requested, reconciliation.signal);
    const copy: Copy = {
        digest,
        store: relayed(options.store, source, spaceId),
        recipient,
        desired: await appliedStates(reconciliation.store.database, found.row),
    };

    // provision and restore a resource or catch the restored one up
    await (row.restoredAt === null
        ? restoreFirst(options, reconciliation, row, found, copy)
        : catchUp(options, reconciliation, row, found, copy));
}

/** Request a snapshot from the source until it took it, answering its digest. */
async function snapshotOf(
    source: Handoff,
    moving: Transfer,
    requested: Parameters<Handoff["snapshot"]>[1],
    signal: AbortSignal,
): Promise<Digest> {
    for (let interval = SNAPSHOT_POLL_INITIAL_MILLISECONDS; ; interval *= 2) {
        // answer the digest of a taken snapshot, else ask again after a doubling wait
        const { digest } = await source.snapshot(moving, requested);
        if (digest !== null) {
            return digest;
        }
        await Clock.system.sleep(Math.min(interval, SNAPSHOT_POLL_MILLISECONDS), signal);
    }
}

/** Provision a resource and restore a copy into it. */
async function restoreFirst(
    options: TransferOptions,
    reconciliation: RestoreReconciliation,
    row: RestoreReconciliation["rows"][number],
    { object, row: resource }: Resource,
    copy: Copy,
): Promise<void> {
    // provision the resource with a provider here
    const {
        kind,
        code,
        reconcile,
        provision: provisioning,
        snapshot: snapshotter,
    } = options.providers.restoring(object, resource);
    const states = kind.states(copy.desired);
    const provisioned = await provisioning.provision(kind.record({ ...resource, reference: null }));
    const record = kind.record({ ...resource, reference: provisioned.reference });

    // apply its states, restore the copy and record the restore as the space
    try {
        if (reconcile !== undefined) {
            const plan = await reconcile.plan(record, states);
            await reconcile.apply(record, states, await Plan.digest(plan));
        }
        await snapshotter.restore(record, states, copy.digest, copy.store, copy.recipient);
        const { now } = reconciliation;
        const fields = {
            digest: copy.digest,
            provider: code,
            reference: provisioned.reference,
            location: provisioned.location ?? null,
            spec: resource.spec,
            restoredAt: now,
            ...(row.fencedAt === null ? {} : { caughtUpAt: now }),
        };
        await reconciliation.store.mutate(
            AccessContext.of(
                Caller.of(principal.space.reference(Scope.universe.id, row.spaceId)),
                now,
            ),
            {
                id: RequestId.create(now),
                calls: [restore.call("update", { ...restore.target(row), ...fields })],
            },
            { component: restore.name },
        );
    } catch (error) {
        await provisioning.destroy(record);
        throw error;
    }
}

/** Catch a restored resource up onto a later copy, from the copy it restored before. */
async function catchUp(
    options: TransferOptions,
    reconciliation: RestoreReconciliation,
    row: RestoreReconciliation["rows"][number],
    { object, row: resource }: Resource,
    copy: Copy,
): Promise<void> {
    // restore the later copy onto the earlier one with the provider keeping it
    const { kind, snapshot: snapshotter } = options.providers.snapshotting(object, row.provider);
    await snapshotter.restore(
        kind.record({ ...resource, reference: row.reference }),
        kind.states(copy.desired),
        copy.digest,
        copy.store,
        copy.recipient,
        Restore.restored(row).digest,
    );

    // record the catch-up as the space
    await reconciliation.store.mutate(
        AccessContext.of(
            Caller.of(principal.space.reference(Scope.universe.id, row.spaceId)),
            reconciliation.now,
        ),
        {
            id: RequestId.create(reconciliation.now),
            calls: [
                restore.call("update", {
                    ...restore.target(row),
                    digest: copy.digest,
                    caughtUpAt: reconciliation.now,
                }),
            ],
        },
        { component: restore.name },
    );
}

/** Retire a space's expired snapshots and sweep the blobs none reads from its own store. */
export async function sweepSnapshots(
    objects: Pick<ObjectStore, "database" | "mutate" | "clock">,
    store: ContentStore,
): Promise<void> {
    // delete the snapshots taken before their retention, each as its space
    const { database } = objects;
    const now = objects.clock.now();
    const expired = await database
        .select({ id: snapshot.table.id, spaceId: snapshot.table.spaceId })
        .from(snapshot.table)
        .where(lt(snapshot.table.takenAt, now - SNAPSHOT_RETENTION_MILLISECONDS));
    const bySpace = Map.groupBy(expired, (row) => row.spaceId);
    await Transaction.run(database, async (transaction) => {
        for (const [spaceId, rows] of bySpace) {
            await objects.mutate(
                AccessContext.of(
                    Caller.of(principal.space.reference(Scope.universe.id, spaceId)),
                    now,
                ),
                {
                    id: RequestId.create(now),
                    calls: rows.map(({ id }) => snapshot.call("delete", { id })),
                },
                { transaction, scope: Scope.universe.id },
            );
        }
    });

    // list the kept blobs before taking the lease, leaving writers unblocked while listing
    const listed = await Array.fromAsync(store.digests());
    const holder = crypto.randomUUID();
    try {
        // take the lease, and leave the store alone while a writer runs
        if (!(await acquireSweepIfIdle(objects, holder))) {
            return;
        }

        // read the blobs the snapshots read once no writer can add to them
        const snapshots = await database
            .select({ blobs: snapshot.table.blobs })
            .from(snapshot.table);
        const retained = new Set(snapshots.flatMap((row) => row.blobs));

        // renew the lease while deleting, keeping a lost lease to stop the deletes
        const finished = new AbortController();
        let lost: Error | undefined;
        const renewal = renewSweep(objects, holder, finished.signal).catch((error: unknown) => {
            lost = error instanceof Error ? error : new TypeError(String(error));
        });

        // delete every other blob, stopping once a writer starts or the lease is lost
        try {
            for (const digest of listed.filter((kept) => !retained.has(kept))) {
                if (lost !== undefined) {
                    throw lost;
                } else if (await isWriting(database)) {
                    return;
                }
                await store.delete(digest);
            }
        } finally {
            finished.abort();
            await renewal;
        }
    } finally {
        await Lease.release(database, { holder, ...SWEEP_LEASE });
    }
}

/** Renew the sweep's lease every third of it until a signal ends the renewals. */
async function renewSweep(
    objects: Pick<ObjectStore, "database" | "clock">,
    holder: string,
    signal: AbortSignal,
): Promise<void> {
    while (!signal.aborted) {
        // wait a third of the lease, ending quietly once the sweep finished
        try {
            await objects.clock.sleep(SWEEP_LEASE_MILLISECONDS / 3, signal);
        } catch (error) {
            if (signal.aborted) {
                return;
            }
            throw error;
        }

        // renew it, refusing a lease another holder took
        const acquired = await Lease.acquire(objects.database, {
            ...SWEEP_LEASE,
            holder,
            duration: SWEEP_LEASE_MILLISECONDS,
            now: objects.clock.now(),
        });
        if (acquired.kind !== "acquired") {
            throw new TypeError("the snapshot sweep lost its lease while deleting blobs");
        }
    }
}

/** Take or renew the sweep's lease while no snapshot or restore writes, answering if it may. */
async function acquireSweepIfIdle(
    objects: Pick<ObjectStore, "database" | "clock">,
    holder: string,
): Promise<boolean> {
    const acquired = await Lease.acquire(objects.database, {
        ...SWEEP_LEASE,
        holder,
        duration: SWEEP_LEASE_MILLISECONDS,
        now: objects.clock.now(),
    });

    return acquired.kind === "acquired" && !(await isWriting(objects.database));
}

/** Read how long a writer waits before looking at a running sweep again, none without one. */
async function sweepWait(database: DatabaseConnection, now: number): Promise<number | undefined> {
    const lapsesAt = await Lease.lapsesAt(database, { ...SWEEP_LEASE, now });

    return lapsesAt === undefined
        ? undefined
        : Math.min(lapsesAt - now, SNAPSHOT_POLL_MILLISECONDS);
}

/** Report whether a snapshot or restore of the machine is writing blobs into its store. */
async function isWriting(database: DatabaseConnection): Promise<boolean> {
    const [taking] = await database
        .select({ id: snapshot.table.id })
        .from(snapshot.table)
        .where(Condition.render(TAKING, snapshot.table))
        .limit(1);
    const [restoring] = await database
        .select({ id: restore.table.id })
        .from(restore.table)
        .where(Condition.render(RESTORING, restore.table))
        .limit(1);

    return taking !== undefined || restoring !== undefined;
}

/** Write into a store, collecting the digests of the blobs written or fetched. */
function recording(store: ContentStore): {
    readonly store: Pick<ContentStore, "read" | "write" | "fetch">;
    readonly blobs: Set<Digest>;
} {
    const blobs = new Set<Digest>();

    return {
        blobs,
        store: {
            read: (digest) => store.read(digest),
            write: async (body, expected) => {
                const digest = await store.write(body, expected);
                blobs.add(digest);

                return digest;
            },
            fetch: async (digests, from) => {
                await store.fetch(digests, from);
                for (const digest of digests) {
                    blobs.add(digest);
                }
            },
        },
    };
}

/** Read through a machine's content store from another machine's store. */
function relayed(
    store: ContentStore,
    source: Handoff,
    spaceId: Identifier<"space">,
): Pick<ContentStore, "read" | "write" | "fetch"> {
    // read a blob from the source machine's store through its relay
    const remote = {
        read: (digest: Digest) => source.read(spaceId, digest),
    };

    return {
        read: async function* (digest) {
            await store.fetch([digest], remote);
            yield* store.read(digest);
        },
        write: (body, expected) => store.write(body, expected),
        fetch: (digests, from) => store.fetch(digests, from),
    };
}
