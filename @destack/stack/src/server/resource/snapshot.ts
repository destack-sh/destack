import { appliedStates } from "./resource.ts";
import { asc, type BlobStore, Condition, isNotNull, lt, Snapshot, Transaction } from "@destack/db";
import { AccessContext, Caller, principal } from "@destack/access";
import { type ObjectReconciliation, type Resource, type ResourceType } from "@destack/object";
import type { space } from "@destack/account/object";
import type { ObjectStore } from "@destack/object/server";
import { Plan, type BlobStorage } from "@destack/resource";
import { Recipient } from "@destack/identity";
import { Digest, type Identifier, present } from "@destack/schema";
import { Scope, RequestId } from "@destack/sync";
import { Restore, restore, snapshot } from "../../object/snapshot.ts";
import { SpaceTransfer, type TransferOptions } from "../host/transfer.ts";
import type { Handoff } from "../host/handoff.ts";

/** How long a snapshot no move retires stays in the store: a day. */
const SNAPSHOT_RETENTION_MILLISECONDS = 24 * 60 * 60 * 1000;

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
    store: Pick<ObjectStore, "mutate" | "environment">,
    spaceId: Identifier<"space">,
    now: number,
): Promise<void> {
    const retired = await transaction.read(snapshot.table).findMany({ where: { spaceId } });
    await store.mutate(
        AccessContext.of(Caller.of(principal.space.reference(Scope.universe.id, spaceId)), now),
        {
            id: RequestId.create(store.environment),
            calls: retired.map(({ id }) => snapshot.call("delete", { id })),
        },
        { transaction, scope: Scope.universe.id },
    );
}

/** Implement snapshots: take each pending one into the space's blobs. */
export function implementSnapshot(options: TransferOptions) {
    return snapshot.implement({
        controller: {
            pending: TAKING,
            reconcile: async (reconciliation) => {
                // take each pending snapshot
                for (const row of reconciliation.rows) {
                    // find the resource and the provider that stores it here
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

                    // copy its content into the space's blobs and record the copy with them as the space
                    const { store, blobs } = recording(options.blobs);
                    const digest = await snapshotter.snapshot(
                        kind.record(resource),
                        kind.states(await appliedStates(reconciliation.store.blobs, resource)),
                        store,
                        Recipient.of(row.recipient, reconciliation.store.environment),
                    );
                    await reconciliation.store.mutate(
                        AccessContext.of(
                            Caller.of(principal.space.reference(Scope.universe.id, spaceId)),
                            reconciliation.now,
                        ),
                        {
                            id: RequestId.create(reconciliation.store.environment),
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
    readonly store: Pick<BlobStorage, "read" | "batch" | "write" | "fetch">;
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
    const recipient = await Recipient.generate(reconciliation.store.environment);
    const source = await options.source(row.sourceId, spaceId);
    const moving = await SpaceTransfer.readActive(reconciliation.store.database, spaceId);
    const requested = { resourceId: row.resourceId, recipient: recipient.key };
    const digest = await source.snapshot(moving, requested, { signal: reconciliation.signal });
    const copy: Copy = {
        digest,
        store: relayed(options.blobs, source, spaceId),
        recipient,
        desired: await appliedStates(reconciliation.store.blobs, found.row),
    };

    // provision and restore a resource or catch the restored one up
    await (row.restoredAt === null
        ? restoreFirst(options, reconciliation, row, found.object, found.row, copy)
        : catchUp(options, reconciliation, row, found.object, found.row, copy));
}

/** Provision a resource and restore a copy into it. */
async function restoreFirst(
    options: TransferOptions,
    reconciliation: RestoreReconciliation,
    row: RestoreReconciliation["rows"][number],
    object: ResourceType<typeof space>,
    resource: Resource,
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
            await reconcile.apply(
                record,
                states,
                await Plan.digest(plan, reconciliation.store.environment.crypto),
            );
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
                id: RequestId.create(reconciliation.store.environment),
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
    object: ResourceType<typeof space>,
    resource: Resource,
    copy: Copy,
): Promise<void> {
    // restore the later copy onto the earlier one with the provider that stores it
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
            id: RequestId.create(reconciliation.store.environment),
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

/** Delete a space's expired snapshots, which lets their blobs go, answering the next expiry. */
export async function expireSnapshots(
    objects: Pick<ObjectStore, "database" | "mutate" | "clock" | "environment">,
): Promise<number | undefined> {
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
                    id: RequestId.create(objects.environment),
                    calls: rows.map(({ id }) => snapshot.call("delete", { id })),
                },
                { transaction, scope: Scope.universe.id },
            );
        }
    });

    // read when the next snapshot expires
    const [oldest] = await database
        .select({ takenAt: snapshot.table.takenAt })
        .from(snapshot.table)
        .where(isNotNull(snapshot.table.takenAt))
        .orderBy(asc(snapshot.table.takenAt))
        .limit(1);

    return oldest?.takenAt === null || oldest === undefined
        ? undefined
        : oldest.takenAt + SNAPSHOT_RETENTION_MILLISECONDS;
}

/** Write into a space's blobs, leasing each for a day, collecting what it writes or fetches. */
function recording(space: BlobStore): {
    readonly store: Pick<BlobStorage, "read" | "batch" | "write" | "fetch">;
    readonly blobs: Set<Digest>;
} {
    const blobs = new Set<Digest>();
    const until = () => space.environment.clock.now() + SNAPSHOT_RETENTION_MILLISECONDS;

    return {
        blobs,
        store: {
            read: (digest) => space.storage.read(digest),
            batch: (digests) => space.storage.batch(digests),
            write: async (body, expected) => {
                // lease the bytes' digest before keeping them, so no collection takes them meanwhile
                const chunked = await Array.fromAsync(body);
                const bytes = new Uint8Array(
                    await new Blob(chunked.map((chunk) => new Uint8Array(chunk))).arrayBuffer(),
                );
                const digest = expected ?? (await Digest.of(bytes, space.environment.crypto));
                await space.lease([digest], until());
                await space.storage.write(
                    (async function* () {
                        yield bytes;
                    })(),
                    digest,
                );
                blobs.add(digest);

                return digest;
            },
            fetch: async (digests, from) => {
                await space.lease(digests, until());
                await space.storage.fetch(digests, from);
                for (const digest of digests) {
                    blobs.add(digest);
                }
            },
        },
    };
}

/** Read through a space's blobs from another machine's, leasing what it fetches for a day. */
function relayed(
    space: BlobStore,
    source: Handoff,
    spaceId: Identifier<"space">,
): Pick<BlobStorage, "read" | "batch" | "write" | "fetch"> {
    // read blobs from the source machine's store through its relay, in batches
    const remote = {
        batch: (digests: readonly Digest[]) => source.batch(spaceId, digests),
    };
    const { store } = recording(space);

    return {
        read: async function* (digest) {
            await store.fetch([digest], remote);
            yield* space.storage.read(digest);
        },
        batch: async function* (digests) {
            await store.fetch(digests, remote);
            yield* space.storage.batch(digests);
        },
        write: (body, expected) => store.write(body, expected),
        fetch: (digests, from) => store.fetch(digests, from),
    };
}
