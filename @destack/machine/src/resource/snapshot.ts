import { appliedDigests } from "./resource.ts";
import { type BlobStore, Condition, now, Snapshot, Transaction } from "@destack/db";
import { AccessContext, Caller, principal } from "@destack/access";
import { type ObjectReconciliation, type Resource, type ResourceType } from "@destack/object";
import type { space } from "@destack/account/object";
import type { ObjectStore } from "@destack/object/server";
import { Plan, type ContentStore, ResourceRecord } from "@destack/resource";
import { Recipient } from "@destack/identity";
import { Digest, type Identifier, nth, present } from "@destack/schema";
import { Scope, RequestId } from "@destack/sync";
import { Restore, restore, snapshot } from "@destack/stack/object";
import { SpaceTransfer, type TransferOptions } from "../transfer/transfer.ts";
import type { Handoff } from "../transfer/handoff.ts";

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
    time: number,
): Promise<void> {
    const retired = await transaction.read(snapshot.table).findMany({ where: { spaceId } });
    await store.mutate(
        AccessContext.of(Caller.of(principal.space.reference(Scope.universe.id, spaceId)), time),
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
        controllers: [
            {
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
                            ResourceRecord.of(kind, resource),
                            await Promise.all(
                                appliedDigests(resource).map((applied) =>
                                    reconciliation.store.blobs.readJson(
                                        applied,
                                        kind.provisioned.state,
                                    ),
                                ),
                            ),
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
            {
                // delete each space's snapshots past their retention, as the space
                name: "expiry",
                pending: {
                    takenAt: { lte: now.minus({ milliseconds: SNAPSHOT_RETENTION_MILLISECONDS }) },
                },
                key: ["spaceId"],
                reconcile: async ({ rows, store, now: at }) => {
                    const { spaceId } = nth(rows, 0);
                    const caller = Caller.of(principal.space.reference(Scope.universe.id, spaceId));
                    await store.mutate(
                        AccessContext.of(caller, at),
                        {
                            id: RequestId.create(store.environment),
                            calls: rows.map(({ id }) => snapshot.call("delete", { id })),
                        },
                        { scope: Scope.universe.id },
                    );
                },
            },
        ],
    });
}

/** Implement restores: restore each pending one into a resource provisioned here. */
export function implementRestore(options: TransferOptions) {
    return restore.implement({
        controllers: [
            {
                pending: RESTORING,
                reconcile: async (reconciliation) => {
                    // restore each pending restore
                    for (const row of reconciliation.rows) {
                        await restoreRow(options, reconciliation, row);
                    }

                    return undefined;
                },
            },
        ],
    });
}

/** A restore of a resource, as the machine's object store reconciles it. */
type RestoreReconciliation = ObjectReconciliation<typeof restore>;

/** A copy of a resource restored from. */
interface Copy {
    /** The snapshot's digest. */
    readonly digest: Digest;
    /** The space's store, reading blobs through the source's handoff. */
    readonly store: Pick<ContentStore, "read" | "batch" | "write" | "fetch">;
    /** The recipient the source encrypted the machine-bound values to. */
    readonly recipient: Recipient;
    /** The digests of the resource's applied states. */
    readonly desired: readonly Digest[];
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
        desired: appliedDigests(found.row),
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
    const states = await Promise.all(
        copy.desired.map((digest) =>
            reconciliation.store.blobs.readJson(digest, kind.provisioned.state),
        ),
    );
    const provisioned = await provisioning.provision(
        ResourceRecord.of(kind, { ...resource, reference: null }),
    );
    const record = ResourceRecord.of(kind, { ...resource, reference: provisioned.reference });

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
        const { now: time } = reconciliation;
        const fields = {
            digest: copy.digest,
            provider: code,
            reference: provisioned.reference,
            location: provisioned.location ?? null,
            spec: resource.spec,
            restoredAt: time,
            ...(row.fencedAt === null ? {} : { caughtUpAt: time }),
        };
        await reconciliation.store.mutate(
            AccessContext.of(
                Caller.of(principal.space.reference(Scope.universe.id, row.spaceId)),
                time,
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
        ResourceRecord.of(kind, { ...resource, reference: row.reference }),
        await Promise.all(
            copy.desired.map((digest) =>
                reconciliation.store.blobs.readJson(digest, kind.provisioned.state),
            ),
        ),
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

/** Write into a space's blobs, leasing each for a day, collecting what it writes or fetches. */
function recording(space: BlobStore): {
    readonly store: Pick<ContentStore, "read" | "batch" | "write" | "fetch">;
    readonly blobs: Set<Digest>;
} {
    const blobs = new Set<Digest>();
    const until = () => space.environment.clock.now() + SNAPSHOT_RETENTION_MILLISECONDS;

    return {
        blobs,
        store: {
            read: (digest) => space.read(digest),
            batch: (digests) => space.batch(digests),
            write: async (body, expected) => {
                // store the bytes, leased for the retention past their write's lease
                const digest = await space.write(body, expected);
                await space.lease([digest], until());
                blobs.add(digest);

                return digest;
            },
            fetch: async (digests, from) => {
                await space.fetch(digests, from);
                await space.lease(digests, until());
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
): Pick<ContentStore, "read" | "batch" | "write" | "fetch"> {
    // read blobs from the source machine's store through its relay, in batches
    const remote = {
        batch: (digests: readonly Digest[]) => source.batch(spaceId, digests),
    };
    const { store } = recording(space);

    return {
        read: async function* (digest) {
            await store.fetch([digest], remote);
            yield* space.read(digest);
        },
        batch: async function* (digests) {
            await store.fetch(digests, remote);
            yield* space.batch(digests);
        },
        write: (body, expected) => store.write(body, expected),
        fetch: (digests, from) => store.fetch(digests, from),
    };
}
