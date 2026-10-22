import {
    appliedStates,
    type Resource,
    type ProviderIndex,
    type SpaceProvider,
} from "../resource/resource.ts";
import {
    RequestId,
    Replica,
    type Call,
    ObjectReference,
    replica,
    Scope,
    type Subject,
    type Uplink,
} from "@destack/sync";
import { placement, Space, Placement } from "@destack/account/object";
import { SpaceReplica } from "./replica.ts";
import { machineBound, type HandoffOptions, type Handoff, SourceHandoff } from "./handoff.ts";
import type { Runtime } from "../../runtime/index.ts";
import { nth, type Identifier, present, schema } from "@destack/schema";
import {
    and,
    Change,
    type DatabaseConnection,
    desc,
    eq,
    inArray,
    isNotNull,
    isNull,
    type Select,
    sql,
    type SQL,
    Transaction,
    Condition,
    type RowImage,
    Snapshot,
    type Table,
    TABLE,
    DatabaseHandle,
    DatabaseState,
} from "@destack/db";
import type { ResourceState } from "@destack/package/declare";
import { type IdentityKeystore } from "@destack/account/identity";
import { DesiredState, type SettlementContext } from "@destack/object";
import type { ObjectStore } from "@destack/object/server";
import type { ResourceId, ResourceRecord, ContentStore } from "@destack/resource";
import { Recipient } from "@destack/identity";
import { CallError } from "@destack/object";
import type { Controller, Reconciliation } from "@destack/db";
import { AccessContext, Caller, principal } from "@destack/access";
import {
    Database,
    deployment,
    build,
    instance,
    restore,
    Restore,
    snapshot,
    SPACE_PROVIDER,
    transfer,
    type Transfer,
    type Fence,
} from "../../object/index.ts";
import type { BuildStore } from "../installation/build.ts";
import { retireSnapshots } from "../resource/snapshot.ts";
import type { Clock } from "@destack/environment";
import type { SpaceRole } from "./space.ts";

/** The spaces a machine receives at once. */
const TRANSFER_CONCURRENCY = 4;

/** What a machine moves spaces with: the sources' handoffs, providers and store. */
export interface TransferOptions {
    /** The space's role on the machine: only a standby removes a moved space's fenced replica. */
    readonly role: SpaceRole;
    /** The machine serving the spaces. */
    readonly machine: Identifier<"machine">;
    /** The space's database, which a move fences and copies. */
    readonly database: DatabaseConnection;
    /** The universe's uplink, which places the spaces as this machine. */
    readonly publisher: Uplink;
    /** Connect to a source's handoff by its machine. */
    readonly source: (
        machine: Identifier<"machine">,
        spaceId: Identifier<"space">,
    ) => Promise<Handoff>;
    /** The providers of the space's resources, from the stack and the other modules. */
    readonly providers: ProviderIndex;
    /** The space's own content-addressed store of its resources' snapshots. */
    readonly store: ContentStore;
    /** The machine's store of every build. */
    readonly builds: BuildStore;
    /** The keystore of the machine's spaces. */
    readonly keystore: IdentityKeystore;
    /** The runtimes running the spaces' instances. */
    readonly runtimes: readonly Runtime[];
}

/** The space's object store a move runs its calls through as its principal. */
export type TransferStore = Pick<ObjectStore, "database" | "serving" | "mutate" | "clock">;

/** A resource of the space whose provider fences its content, with its record. */
interface ResourceFence {
    /** The provider's fence. */
    readonly fence: NonNullable<SpaceProvider["fence"]>;
    /** The resource as its provider reads it. */
    readonly record: ResourceRecord;
}

/** Implement transfers: mark each move at the universe as it is created. */
export function implementTransfer(options: TransferOptions) {
    return transfer.implement({
        methods: {
            create: {
                prepare: (context) => mark(context, options),
                settle: async (context, settled) => {
                    // unmark a move whose creation failed
                    if (settled.status === "rolledBack") {
                        await unmark(context, options);
                    }
                },
                handle: async (context, next) => {
                    // move the space from this machine at its epoch
                    const located = context.prepared;

                    return next(
                        context.with({
                            input: {
                                ...context.input,
                                sourceId: located.machine,
                                sourceEpoch: located.epoch,
                                ...ObjectReference.columns(
                                    "requestedBy",
                                    requester(context.caller.subject, context.scope),
                                ),
                            },
                        }),
                    );
                },
            },
            fence: {
                // fence the space and its resources before the call's transaction, stopping its instances
                prepare: async (context) => {
                    const moving = await SpaceTransfer.plan(options, context.target);

                    return moving.fence(options, context.now);
                },
                handle: async (context) => context.prepared,
            },
            snapshot: async (call) => {
                // request the resource's snapshot once, answering its digest once the machine took it
                const record = call.target;
                const spaceId = schema.identifier("space").parse(record.scope);

                return SourceHandoff.snapshot(options.providers, call, { spaceId, ...call.input });
            },
            cancel: {
                // place the space at the next epoch to refuse every later step of a running move
                prepare: async (context) => {
                    // refuse an ended transfer
                    const record = context.target;
                    if (record.completedAt !== null) {
                        throw new CallError("CONFLICT", {
                            message: `transfer ${record.id} has ended`,
                        });
                    }

                    // place the space at the next epoch
                    const moving = await SpaceTransfer.plan(options, record);
                    const next = { ...moving.placement, epoch: moving.placement.epoch + 1 };
                    await options.publisher.receive({
                        id: RequestId.create(context.now),
                        calls: [SpaceTransfer.placing(next)],
                    });

                    return moving.placement;
                },
                // serve the space again, end the transfer and retire its snapshots
                handle: async (context) => {
                    // unfence the space as the canceller
                    const located = context.prepared;
                    await context.mutate(context.caller, [
                        transfer.call("unfenceScope", transfer.target(context.target)),
                    ]);

                    // retire its snapshots as the space
                    const retired = await context
                        .read(snapshot.table)
                        .findMany({ where: { spaceId: located.id } });
                    await context.mutate(
                        AccessContext.of(
                            Caller.of(principal.space.reference(Scope.universe.id, located.id)),
                            context.now,
                        ),
                        retired.map(({ id }) => snapshot.call("delete", { id })),
                    );

                    return context.update({ completedAt: context.now, outcome: "cancelled" });
                },
                // lift the fences of the space's resources once cancelled
                settle: async (context, settled) => {
                    if (settled.status === "committed") {
                        const id = schema.identifier("transfer").parse(context.id);
                        const record = await SpaceTransfer.read(context.database, id);
                        const cancelled = await SpaceTransfer.of(
                            context.database,
                            settled.prepared,
                            record.targetId,
                        );
                        await cancelled.lift(options.providers);
                    }
                },
            },
        },
        controller: {
            pending: { completedAt: { isNull: true }, sourceId: options.machine },
            watches: [
                Space.watch((spaceId, database) =>
                    database
                        .select({ id: transfer.table.id })
                        .from(transfer.table)
                        .where(
                            and(
                                eq(transfer.table.scope, spaceId),
                                isNull(transfer.table.completedAt),
                            ),
                        ),
                ),
            ],
            reconcile: (reconciliation) =>
                advance(nth(reconciliation.rows, 0), reconciliation, options),
        },
    });
}

/** Read who requests a transfer: its caller, or the space for a move its scheduler starts. */
function requester(caller: Subject | undefined, spaceId: string): Subject {
    return caller === undefined ? principal.space.reference(Scope.universe.id, spaceId) : caller;
}

/** Mark a space this machine serves as moving to a target at the universe. */
async function mark(
    call: Pick<Snapshot, "read"> & {
        readonly scope: string;
        readonly now: number;
        readonly input: { readonly targetId: string };
        readonly id: string;
    },
    options: TransferOptions,
): Promise<Placement> {
    // transfer only a space this machine serves, under the identifier its move records
    const { scope, input } = call;
    const [located] = await Placement.find(call, [schema.identifier("space").parse(scope)]);
    if (located === undefined || located.machine !== options.machine) {
        throw new CallError("CONFLICT", {
            message: `${scope} is served by another machine`,
        });
    }

    // mark the move at the epoch this machine serves the space at
    const move = { accountId: located.scope, id: located.id, epoch: located.epoch };
    const transferId = schema.identifier("transfer").parse(call.id);
    await options.publisher.receive({
        id: RequestId.create(call.now),
        calls: [
            placement.call("move", {
                ...move,
                target: input.targetId,
                transferId,
            }),
        ],
    });

    return located;
}

/** Clear the move a failed transfer creation marked, unless another transfer of the space runs. */
async function unmark(context: SettlementContext, options: TransferOptions): Promise<void> {
    // leave a space another machine serves now
    const spaceId = schema.identifier("space").parse(context.scope);
    const [located] = await Placement.find(Snapshot.live(context.database), [spaceId]);
    if (located === undefined || located.machine !== options.machine) {
        return;
    }

    // leave the move of a transfer still running
    const [running] = await context.database
        .select({ id: transfer.table.id })
        .from(transfer.table)
        .where(and(eq(transfer.table.scope, spaceId), isNull(transfer.table.completedAt)));
    if (running !== undefined) {
        return;
    }

    // serve the space at its epoch again, which clears the move
    const placed = { accountId: located.scope, id: spaceId, epoch: located.epoch };
    await options.publisher.receive({
        id: RequestId.create(context.now),
        calls: [placement.call("place", placed)],
    });
}

/** Advance a transfer from this machine. */
async function advance(
    record: Select<typeof transfer.table>,
    reconciliation: {
        readonly database: DatabaseConnection;
        readonly now: number;
        readonly store: TransferStore;
    },
    options: TransferOptions,
): Promise<undefined> {
    // remove the fenced replica of a space its target serves now, once this machine stands by
    const [located] = await Placement.find(Snapshot.live(reconciliation.database), [record.scope]);
    const isServedThere =
        located?.machine === record.targetId && located.epoch > record.sourceEpoch;
    if (options.role === "standby" && located !== undefined && isServedThere) {
        const machine = record.sourceId;
        const source = { ...located, machine, epoch: record.sourceEpoch };
        const target = record.targetId;
        const moved = await SpaceTransfer.of(reconciliation.database, source, target);
        await moved.remove(options, reconciliation.store, reconciliation.now);
    }

    return undefined;
}

/** The move of one space to another machine. */
export class SpaceTransfer {
    /** The space as its source serves it. */
    readonly placement: Placement;
    /** The machine receiving the space. */
    readonly target: Identifier<"machine">;
    /** The replica of the space's rows. */
    readonly space: SpaceReplica;

    /** The space moving. */
    get spaceId(): Identifier<"space"> {
        return this.placement.id;
    }

    /** Plan a space's move with its main database's tables, by default those applied here. */
    static async of(
        database: DatabaseConnection,
        placed: Placement,
        target: Identifier<"machine">,
        tables?: readonly Table[],
    ): Promise<SpaceTransfer> {
        const added =
            tables ??
            SpaceTransfer.tables(database, await SpaceTransfer.states(database, placed.id));

        return new SpaceTransfer({
            placement: placed,
            target,
            space: await SpaceReplica.of(database, placed.id, added),
        });
    }

    /** Read the states the space's main database applied, none while absent or unprovisioned. */
    static async states(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
    ): Promise<ResourceState[]> {
        const view = Snapshot.live(database);
        const main = await Database.mainMaybe(view, spaceId);

        return main === undefined || main.reference === null
            ? []
            : DesiredState.read(view, main.scope, main.appliedStates);
    }

    /** Describe the tables some main database states declare beside a database's own. */
    static tables(database: DatabaseConnection, states: readonly ResourceState[]): Table[] {
        // describe the states' tables in the database's dialect, leaving the ones it declares
        const own = new Set(database.tables.map((table) => table[TABLE].sqlName));
        const described = DatabaseHandle.tables(
            states.map((state) => DatabaseState.parse(state).tables[database.dialect]),
        );

        return described.filter((table) => !own.has(table[TABLE].sqlName));
    }

    /** Plan the steps of a transfer while its source still serves its space. */
    static async plan(
        options: Pick<HandoffOptions, "database">,
        record: Transfer,
    ): Promise<SpaceTransfer> {
        // refuse a space placed elsewhere now
        const [located] = await Placement.find(Snapshot.live(options.database), [record.scope]);
        if (located === undefined || located.machine !== record.sourceId) {
            throw new CallError("CONFLICT", {
                message: `${record.scope} is no longer placed on ${record.sourceId}`,
            });
        }

        // plan its move with its main database's tables, refusing a plan waiting for approval
        const main = await Database.mainMaybe(Snapshot.live(options.database), located.id);
        const states =
            main === undefined || main.reference === null
                ? []
                : await appliedStates(options.database, main);

        return SpaceTransfer.of(
            options.database,
            located,
            record.targetId,
            SpaceTransfer.tables(options.database, states),
        );
    }

    /** Read a space's active transfer, refusing a space without one. */
    static async readActive(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
    ): Promise<Transfer> {
        const [record] = await database
            .select()
            .from(transfer.table)
            .where(and(eq(transfer.table.scope, spaceId), isNull(transfer.table.completedAt)));
        if (record === undefined) {
            throw new CallError("NOT_FOUND", { message: `${spaceId} has no active transfer` });
        }

        return record;
    }

    /** Read a transfer by its identifier, refusing one this database keeps no record of. */
    static async read(database: DatabaseConnection, id: Identifier<"transfer">): Promise<Transfer> {
        const [record] = await database
            .select()
            .from(transfer.table)
            .where(eq(transfer.table.id, id));
        if (record === undefined) {
            throw new CallError("NOT_FOUND", { message: `no transfer ${id}` });
        }

        return record;
    }

    /** Plan a space's transfer to a target through its replica, refusing its own machine. */
    constructor(options: {
        readonly placement: Placement;
        readonly target: Identifier<"machine">;
        readonly space: SpaceReplica;
    }) {
        // refuse handing a space to its own machine
        if (options.target === options.placement.machine) {
            throw new CallError("CONFLICT", {
                message: `${options.placement.id} is served by ${options.target} already`,
            });
        }

        // keep the placement, the space, the target and the replica
        this.placement = options.placement;
        this.target = options.target;
        this.space = options.space;
    }

    /** Fence the space against writes unless fenced already, then its resources. */
    async fence(
        options: Pick<HandoffOptions, "providers" | "runtimes">,
        now: number,
    ): Promise<Fence> {
        // fence the space once, reading the database's position and the digest of the space's rows at it
        await Transaction.run(this.space.database, async (transaction) => {
            const chain = await Scope.chain(transaction.view(), this.placement.id);
            if (chain.every((link) => link.movedTo === undefined)) {
                await Scope.fence(transaction, this.placement.id, this.target, now);
            }
        });

        // read the space's latest change, its rows' digest and its subtree
        const fenced = await this.space.database.transaction(async (database) => {
            const scopes = await Scope.subtree(Snapshot.live(database), this.placement.id);

            return {
                position: await database.log.changed(this.space.tables, scopes),
                digest: await SpaceReplica.digest(database, this.space.digestTables(), scopes),
                scopes,
            };
        });

        // fence each resource's content once its writes in flight finish
        const fences = await this.#fences(options.providers);
        await Promise.all(fences.map((resource) => resource.fence.fence(resource.record)));

        // stop the space's instances running here before their resources are captured
        await this.#stop(options);

        return fenced;
    }

    /** Lift the fences of the space's resources here, once its move was cancelled. */
    async lift(providers: ProviderIndex): Promise<void> {
        const fences = await this.#fences(providers);
        await Promise.all(fences.map((resource) => resource.fence.lift(resource.record)));
    }

    /** Remove the source's replica of a space its target serves now. */
    async remove(
        options: Pick<HandoffOptions, "providers" | "builds">,
        store: Pick<ObjectStore, "mutate">,
        now: number,
    ): Promise<void> {
        // destroy each provisioned resource of the space kept here
        for (const { row } of await this.#provisioned(options.providers)) {
            await options.providers.destroy(
                present(row.provider, `the provider of ${row.id}`),
                row,
            );
        }

        // retire the space's snapshots
        await Transaction.run(this.space.database, (transaction) =>
            retireSnapshots(transaction, store, this.spaceId, now),
        );

        // delete the space's rows and its builds' root
        await this.space.delete();
        await options.builds.packages.root(this.spaceId, new Set());
    }

    /** Cut over to this machine. */
    async cutover(
        source: Handoff,
        signal: AbortSignal,
        options: TransferOptions,
        store: TransferStore,
    ): Promise<void> {
        // follow the source's rows up to its fence, restoring resources and builds meanwhile
        const fenced = await this.#followToFence(source, signal, options, store);

        // capture the space's rows and copy the builds they added
        await this.#captureRows(source, fenced, options, store.clock);
        await this.#copyBuilds(source, options.builds, signal);

        // catch each resource up on what changed since its snapshot
        await this.#catchUp(signal, options, store, store.clock.now());

        // take over the space's rows and point each resource here
        await this.#takeOver(store);
        await this.settle(options, store);
    }

    /** Follow the source's rows up to its fence, restoring resources and builds meanwhile. */
    async #followToFence(
        source: Handoff,
        signal: AbortSignal,
        options: TransferOptions,
        store: TransferStore,
    ): Promise<Fence> {
        // follow the source's rows of the space, failing the cutover once the follow fails
        const { database } = this.space;
        const { spaceId } = this;
        const halted = new AbortController();
        const following = AbortSignal.any([signal, halted.signal]);
        const followed = this.space.replica
            .follow(
                database,
                ({ after }, stop) =>
                    source.watch(
                        { spaceId, ...(after === undefined ? {} : { after }) },
                        { signal: stop },
                    ),
                following,
                { clock: store.clock },
            )
            .catch((error: unknown) => halted.abort(error));
        try {
            // restore each resource and copy each build once the replica took its first complete run
            await database.log.until(
                async () => (await this.space.replica.position(database)) !== undefined,
                following,
            );
            following.throwIfAborted();
            await this.#prepare(signal, options, store);

            await this.#copyBuilds(source, options.builds, signal);

            // fence the source and follow up to its fence
            const record = await SpaceTransfer.readActive(database, spaceId);
            const fenced = await source.fence(record);
            await this.space.replica.wait(database, fenced.position, following);

            following.throwIfAborted();

            return fenced;
        } finally {
            // stop following
            halted.abort();
            await followed;
        }
    }

    /** Read the manifests of the builds a space has, compiled or pulled. */
    static async manifests(reader: Snapshot, spaceId: Identifier<"space">): Promise<Set<string>> {
        const built = await reader.read(build.table).findMany({ where: { scope: spaceId } });

        return new Set(built.flatMap((row) => (row.manifest === null ? [] : [row.manifest])));
    }

    /** Replicate each build the space's builds and revisions run from the source's build store. */
    async #copyBuilds(source: Handoff, builds: BuildStore, signal: AbortSignal): Promise<void> {
        // copy each build the space's rows refer to that the store lacks
        const manifests = await SpaceTransfer.manifests(
            Snapshot.live(this.space.database),
            this.spaceId,
        );
        for (const manifest of manifests) {
            const { location, fetch } = source.locate(this.spaceId, manifest);
            await builds.packages.copy(location, fetch, signal);
        }
    }

    /** List the resource kinds whose tables the replica moves. */
    #moved(providers: ProviderIndex): ReturnType<ProviderIndex["objects"]> {
        return providers.objects().filter((object) => this.space.tables.includes(object.table));
    }

    /** List the space's provisioned resources. */
    async #provisioned(providers: ProviderIndex): Promise<Resource[]> {
        const provisioned: Resource[] = [];
        for (const object of this.#moved(providers)) {
            // read the kind's resources the space's database keeps
            const rows = await this.space.database
                .select()
                .from(object.table)
                .where(and(object.inScope(this.spaceId), isNotNull(object.table.reference)));

            // leave the space's main database, which moves with the space's replica
            const moved = rows.filter((row) => row.provider !== SPACE_PROVIDER);
            provisioned.push(...moved.map((row) => ({ object, row })));
        }

        return provisioned;
    }

    /** List the space's provisioned resources whose providers here fence their content. */
    async #fences(providers: ProviderIndex): Promise<ResourceFence[]> {
        const fenced: ResourceFence[] = [];
        for (const { object, row } of await this.#provisioned(providers)) {
            const provider = providers.of(object).find((entry) => entry.code === row.provider);
            if (provider?.fence !== undefined) {
                fenced.push({ fence: provider.fence, record: provider.kind.record(row) });
            }
        }

        return fenced;
    }

    /** Capture the space's rows at a clock with their machine-bound rows opened here. */
    async #captureRows(
        source: Handoff,
        fenced: Fence,
        options: TransferOptions,
        clock: Clock,
    ): Promise<void> {
        // decrypt the machine-bound rows encrypted to a fresh recipient
        const recipient = await Recipient.generate();
        const { open } = machineBound(options, recipient);

        // capture and verify the rows of the subtree the source fenced
        const arriving = this.space.within(fenced.scopes);
        await arriving.capture(source, fenced, recipient, open, clock);
    }

    /** Restore each resource a provider here snapshots from a snapshot of the serving source. */
    async #prepare(
        signal: AbortSignal,
        options: TransferOptions,
        store: TransferStore,
    ): Promise<void> {
        const resources = await this.#restorable(options.providers);
        await this.#request(store, resources, await this.#restores(store.database), null);
        await this.#settle(store.database, signal);
    }

    /** Catch each restore up on the fenced source. */
    async #catchUp(
        signal: AbortSignal,
        options: TransferOptions,
        store: TransferStore,
        fencedAt: number,
    ): Promise<void> {
        // forget the restores of resources the fenced source no longer keeps
        const { database } = store;
        const resources = await this.#restorable(options.providers);
        const restores = await this.#restores(database);
        const deleted = restores.filter((row) => !resources.includes(row.resourceId));
        if (deleted.length > 0) {
            const ids = deleted.map((row) => row.id);
            await this.#forget(database, options.providers, inArray(restore.table.id, ids));
        }

        // request the missing restores and mark the others fenced, then wait until each caught up
        await this.#request(store, resources, restores, fencedAt);
        await this.#settle(database, signal);
    }

    /** Wait until each restore of the move restored its content, and caught up once fenced. */
    async #settle(database: DatabaseConnection, signal: AbortSignal): Promise<void> {
        const isSettled = await database.log.until(
            async () =>
                (await this.#restores(database)).every(
                    (row) =>
                        row.restoredAt !== null &&
                        (row.fencedAt === null || row.caughtUpAt !== null),
                ),
            signal,
        );
        if (!isSettled) {
            signal.throwIfAborted();
        }
    }

    /** List the resources whose provider here snapshots them. */
    async #restorable(providers: ProviderIndex): Promise<ResourceId[]> {
        const resources: ResourceId[] = [];
        for (const { object, row } of await this.#provisioned(providers)) {
            const provider = providers.select(object, { ...row, provider: null });
            // refuse a kind no provider here keeps
            if (provider === undefined) {
                throw new CallError("PRECONDITION_FAILED", {
                    message: `no provider of ${object.name} receives resource ${row.id} here`,
                });
            }
            // restore a resource its provider snapshots
            else if (provider.snapshot !== undefined) {
                resources.push(row.id);
            }
        }

        return resources;
    }

    /** Request the restores of resources without one. */
    async #request(
        store: TransferStore,
        resources: readonly ResourceId[],
        restores: readonly Restore[],
        fencedAt: number | null,
    ): Promise<void> {
        const requested = new Set(restores.map((row) => row.resourceId));
        await Transaction.run(store.database, async (transaction) => {
            // request the restores of resources without one, and mark the others fenced once the source is, as the space
            const now = store.clock.now();
            const created = resources
                .filter((id) => !requested.has(id))
                .map((resourceId) =>
                    restore.call("create", {
                        spaceId: this.spaceId,
                        resourceId,
                        sourceId: this.placement.machine,
                        sourceEpoch: this.placement.epoch,
                        ...(fencedAt === null ? {} : { fencedAt }),
                    }),
                );
            const fenced =
                fencedAt === null
                    ? []
                    : restores
                          .filter(
                              (entry) =>
                                  entry.fencedAt === null && resources.includes(entry.resourceId),
                          )
                          .map((entry) => restore.call("update", { id: entry.id, fencedAt }));
            const calls = [...created, ...fenced];
            if (calls.length > 0) {
                await store.mutate(
                    AccessContext.of(
                        Caller.of(principal.space.reference(Scope.universe.id, this.spaceId)),
                        now,
                    ),
                    { id: RequestId.create(now), calls },
                    { transaction, scope: Scope.universe.id },
                );
            }
        });
    }

    /** Read the restores of the move's resources. */
    async #restores(database: DatabaseConnection): Promise<Restore[]> {
        return database
            .select()
            .from(restore.table)
            .where(Condition.render(this.#restoring(), restore.table));
    }

    /** Match the restores of the move's resources. */
    #restoring(): Condition<RowImage<typeof restore.table>> {
        return {
            spaceId: this.spaceId,
            sourceId: this.placement.machine,
            sourceEpoch: this.placement.epoch,
        };
    }

    /** Place the space on its target at the next epoch, rekeyed, and serve its replica there. */
    async place(
        options: Pick<TransferOptions, "publisher" | "keystore">,
        database: DatabaseConnection,
        now: number,
    ): Promise<Placement> {
        // place the space here at the next epoch, and replace the keys its source kept
        const placed = { ...this.placement, machine: this.target, epoch: this.placement.epoch + 1 };
        await options.publisher.receive({
            id: RequestId.create(now),
            calls: [SpaceTransfer.placing(placed)],
        });
        await options.keystore.rekey(database, this.spaceId);

        // serve the replica and forget the restores the takeover recorded
        await Transaction.run(database, async (transaction) => {
            await Scope.unfence(transaction, this.placement.id);
            for (const row of await transaction
                .read(restore.table)
                .findMany({ where: this.#restoring() })) {
                await this.space.rewrite(transaction, restore.table, row, null);
            }
        });

        return placed;
    }

    /** Call the space's placement at an epoch as the calling machine. */
    static placing(placed: Placement): Call {
        return placement.call("place", {
            accountId: placed.scope,
            id: placed.id,
            epoch: placed.epoch,
        });
    }

    /** Abort the move as its target: forget its restores and destroy what they restored here. */
    async abort(providers: ProviderIndex, database: DatabaseConnection): Promise<void> {
        // forget the restores and destroy their resources
        await this.#forget(database, providers, Condition.render(this.#restoring(), restore.table));

        // drop a replica still following its source
        if (await this.space.replica.isRecorded(Snapshot.live(database))) {
            await Replica.drop(database, this.space.replica);
        }
        // delete the rows taken over
        else {
            await this.space.delete();
        }
    }

    /** Report whether the target took the space over, completing the move's transfer. */
    async isTakenOver(database: DatabaseConnection): Promise<boolean> {
        const [succeeded] = await database
            .select({ id: transfer.table.id })
            .from(transfer.table)
            .where(
                and(
                    eq(transfer.table.scope, this.spaceId),
                    eq(transfer.table.targetId, this.target),
                    eq(transfer.table.sourceEpoch, this.placement.epoch),
                    eq(transfer.table.outcome, "succeeded"),
                ),
            );

        return succeeded !== undefined;
    }

    /** Forget some restores and destroy the resources they restored here. */
    async #forget(
        database: DatabaseConnection,
        providers: ProviderIndex,
        restores: SQL | undefined,
    ): Promise<void> {
        // forget the restores and read what each recorded
        const forgotten = await database.delete(restore.table).where(restores).returning();

        // destroy each resource a restore recorded through its provider here
        for (const row of forgotten.filter((entry) => entry.restoredAt !== null)) {
            const restored = Restore.restored(row);
            await providers.destroy(restored.provider, {
                id: row.resourceId,
                scope: this.spaceId,
                spec: restored.spec,
                reference: restored.reference,
            });
        }
    }

    /** Own the space's rows, record each resource's move and complete the transfer. */
    async #takeOver(store: TransferStore): Promise<void> {
        const now = store.clock.now();

        // take over in one transaction on this machine's space database
        const scope = this.spaceId;
        await Transaction.run(store.database, async (transaction) => {
            // own the replicated rows and write into them past the replicated fence
            await this.space.promote(transaction);
            await Scope.unfence(transaction, this.placement.id);

            // find the transfer the replica has
            const [record] = await transaction.read(transfer.table).findMany({
                where: {
                    scope,
                    completedAt: { isNull: true },
                },
            });
            if (record === undefined) {
                throw new CallError("NOT_FOUND", {
                    message: `the replica of ${this.placement.id} has no active transfer`,
                });
            }

            // complete the transfer
            await store.mutate(
                store.serving(transfer),
                {
                    id: RequestId.create(now),
                    calls: [transfer.call("complete", { id: record.id })],
                },
                { transaction, scope },
            );

            // fence the replica again until the universe places the space here
            await Scope.fence(transaction, this.placement.id, this.target, now);
        });
    }

    /** Own the replicated rows and point each resource at its restore here. */
    async settle(
        options: Pick<TransferOptions, "providers" | "machine">,
        store: TransferStore,
    ): Promise<void> {
        const restored = await this.#restores(store.database);
        await Transaction.run(this.space.database, async (transaction) => {
            // own the replicated rows
            await this.space.promote(transaction);

            // point each resource the database keeps at its restore here
            for (const object of this.#moved(options.providers)) {
                for (const entry of restored.filter((row) => object.identifies(row.resourceId))) {
                    const copy = Restore.restored(entry);
                    const [row] = await transaction.read(object.table).findMany({
                        where: {
                            scope: this.spaceId,
                            id: entry.resourceId,
                        },
                    });
                    if (row !== undefined) {
                        await this.space.rewrite(transaction, object.table, row, {
                            ...row,
                            provider: copy.provider,
                            reference: copy.reference,
                            location: copy.location,
                            machineId: options.machine,
                        });
                    }
                }
            }

            // point the space's main database at this machine's storage of it
            await this.#repointMain(transaction, options);
        });
    }

    /** Point a provisioned main database at the space's own storage on this machine. */
    async #repointMain(
        transaction: Transaction,
        options: Pick<TransferOptions, "providers" | "machine">,
    ): Promise<void> {
        // leave a main database its space has not provisioned yet
        const main = await Database.main(transaction.view(), this.spaceId);
        if (main.reference === null) {
            return;
        }

        // provision it with this machine's space provider and record where it lives
        const kind = present(
            options.providers.objects().find((object) => object.identifies(main.id)),
            "the resource kind of the main database",
        );
        const provider = present(
            options.providers.of(kind).find((entry) => entry.code === SPACE_PROVIDER),
            "the space provider of this machine",
        );
        const provisioning = present(provider.provision, "the space provider's provisioning");
        const { reference } = await provisioning.provision(provider.kind.record(main));
        await this.space.rewrite(transaction, kind.table, main, {
            ...main,
            reference,
            machineId: options.machine,
        });
    }

    /** Stop the space's instances on this machine's runtimes, whatever their records say. */
    async #stop(options: Pick<HandoffOptions, "runtimes">): Promise<void> {
        // stop each instance of the space its runtime runs
        const instances = await this.space.database
            .select({ id: instance.table.id, runtime: deployment.table.runtime })
            .from(instance.table)
            .innerJoin(deployment.table, eq(deployment.table.id, instance.table.deploymentId))
            .where(eq(instance.table.scope, this.spaceId));
        for (const running of instances) {
            const runtime = options.runtimes.find((entry) => entry.name === running.runtime);
            if (runtime?.isRunning(running.id) === true) {
                await runtime.stop(running.id);
            }
        }
    }

    /** Match the spaces a machine keeps from a move to it whose placement is elsewhere now. */
    static abandoned(machine: Identifier<"machine">): SQL {
        return sql`EXISTS (SELECT 1 FROM ${transfer.table} WHERE ${transfer.table.scope} = ${Scope.table.scope} AND ${transfer.table.targetId} = ${machine}) AND NOT EXISTS (SELECT 1 FROM ${placement.table} WHERE ${placement.table.id} = ${Scope.table.scope} AND (${placement.table.machineId} = ${machine} OR ${placement.table.targetId} = ${machine})) AND (EXISTS (SELECT 1 FROM ${replica} WHERE ${replica.scope} = ${Scope.table.scope}) OR ${Scope.table.movedTo} IS NULL OR ${Scope.table.movedTo} = ${machine})`;
    }

    /** Match the spaces a machine took over and keeps fenced toward itself. */
    static fenced(machine: Identifier<"machine">): SQL {
        return sql`${Scope.table.movedTo} = ${machine}`;
    }

    /** Find the latest move to a machine of a space it keeps in a state. */
    static async latest(
        database: DatabaseConnection,
        spaceId: Identifier<"space">,
        machine: Identifier<"machine">,
        state: "abandoned" | "fenced",
    ): Promise<SpaceTransfer | undefined> {
        // read the latest move here of the space in that state
        const condition =
            state === "abandoned"
                ? SpaceTransfer.abandoned(machine)
                : SpaceTransfer.fenced(machine);
        const [latest] = await database
            .select({
                account: Scope.table.parent,
                source: transfer.table.sourceId,
                epoch: transfer.table.sourceEpoch,
            })
            .from(Scope.table)
            .innerJoin(
                transfer.table,
                and(
                    eq(transfer.table.scope, Scope.table.scope),
                    eq(transfer.table.targetId, machine),
                ),
            )
            .where(and(eq(Scope.table.scope, spaceId), condition))
            .orderBy(desc(transfer.table.createdAt))
            .limit(1);

        // plan it from the placement its source served
        if (latest === undefined) {
            return undefined;
        }
        const served = {
            id: spaceId,
            scope: schema.identifier("account").parse(latest.account),
            machine: latest.source,
            epoch: latest.epoch,
        };

        return SpaceTransfer.of(database, served, machine);
    }

    /** Receive one space moving to this machine. */
    static async receive(
        moving: Placement,
        signal: AbortSignal,
        options: TransferOptions,
        store: TransferStore,
    ): Promise<void> {
        // find whether this machine replicates the space still or an earlier attempt took it over at this epoch
        const { database } = store;
        const move = await SpaceTransfer.of(database, moving, options.machine);
        const [replicated] = await database
            .select({ id: Scope.table.scope })
            .from(Scope.table)
            .where(eq(Scope.table.scope, move.spaceId));
        const isReplicating =
            replicated !== undefined &&
            (await move.space.replica.isRecorded(Snapshot.live(database)));
        const isTakenOver =
            replicated !== undefined && !isReplicating && (await move.isTakenOver(database));

        // abort the takeover of a cancelled move here and refuse rows of any other earlier move
        if (replicated !== undefined && !isReplicating && !isTakenOver) {
            const earlier = await SpaceTransfer.latest(
                database,
                move.spaceId,
                options.machine,
                "fenced",
            );
            if (earlier === undefined) {
                throw new CallError("CONFLICT", {
                    message: `${moving.id} keeps rows here from an earlier move, which this move would serve stale`,
                });
            }
            await earlier.abort(options.providers, database);
        }

        // settle a move taken over already, and place the space here
        if (isTakenOver) {
            await move.settle(options, store);
            await move.place(options, database, store.clock.now());
        }
        // otherwise create the tables the source's main database applied, then move the space
        else {
            const source = await options.source(moving.machine, move.spaceId);
            const tables = SpaceTransfer.tables(database, await source.states(move.spaceId));
            await database.migrate(tables, { others: "keep" });
            const moved = await SpaceTransfer.of(database, moving, options.machine, tables);

            // cut over to this machine and place the space here
            await moved.cutover(source, signal, options, store);
            await moved.place(options, database, store.clock.now());
        }
    }
}

/** Receive the spaces whose placements move to this machine. */
export class TransferController implements Controller {
    /** The controller's name in reports. */
    readonly name = "transfer";
    /** The replicated placements, whose changes select their spaces. */
    readonly watches: readonly Table[] = [placement.table];
    /** The spaces received at once. */
    readonly concurrency = TRANSFER_CONCURRENCY;
    /** What the machine moves spaces with. */
    readonly #options: TransferOptions;
    /** The space's object store the moves run their calls through as its principal. */
    readonly #store: TransferStore;

    /** Move spaces to a machine with its relays, providers and keys, over its store. */
    constructor(options: TransferOptions, store: TransferStore) {
        this.#options = options;
        this.#store = store;
    }

    /** List the space whose projected placement changed. */
    keys(change: Change): readonly string[] {
        return Change.of(change, placement.table) ? [Change.image(change).id] : [];
    }

    /** List the spaces moving to this machine. */
    async list(): Promise<readonly string[]> {
        // read the spaces moving here
        const { database } = this.#store;
        const machine = this.#options.machine;
        const moving = await database
            .select({ id: placement.table.id })
            .from(placement.table)
            .where(eq(placement.table.targetId, machine));

        // read the spaces placed here still fenced toward this machine
        const fenced = await database
            .select({ id: Scope.table.scope })
            .from(Scope.table)
            .innerJoin(placement.table, eq(placement.table.id, Scope.table.scope))
            .where(and(eq(placement.table.machineId, machine), SpaceTransfer.fenced(machine)));

        // read the spaces kept from moves the placements no longer target here
        const abandoned = await database
            .select({ id: Scope.table.scope })
            .from(Scope.table)
            .where(SpaceTransfer.abandoned(machine));

        return [...moving, ...fenced, ...abandoned].map((row) => row.id);
    }

    /** Receive a space moving here, or place one whose replica here is still fenced. */
    async reconcile(key: string, { signal }: Reconciliation): Promise<undefined> {
        // read the replicated placement
        const { database } = this.#store;
        const { machine, providers } = this.#options;
        const spaceId = schema.identifier("space").parse(key);
        const [row] = await database
            .select()
            .from(placement.table)
            .where(eq(placement.table.id, spaceId));

        // receive a space moving here on a standby
        const { role } = this.#options;
        if (role === "standby" && row?.targetId === machine) {
            const { id, scope, epoch } = row;
            await this.#receiveWhileTargeted({ id, scope, machine: row.machineId, epoch }, signal);
        }
        // place a space whose replica here is fenced still, once primary
        else if (role === "primary" && row?.machineId === machine) {
            const move = await SpaceTransfer.latest(database, spaceId, machine, "fenced");
            await move?.place(this.#options, database, this.#store.clock.now());
        }
        // abort the latest move here the placement no longer targets, on a standby
        else if (role === "standby") {
            const move = await SpaceTransfer.latest(database, spaceId, machine, "abandoned");
            await move?.abort(providers, database);
        }

        return undefined;
    }

    /** Receive a space moving here until its placement no longer targets this machine. */
    async #receiveWhileTargeted(moving: Placement, signal: AbortSignal): Promise<void> {
        // stop the receipt once the projected placement targets another machine or none
        const { database } = this.#store;
        const machine = this.#options.machine;
        const left = new AbortController();
        const watched = new AbortController();
        const isLeft = async () => {
            const [row] = await database
                .select({ target: placement.table.targetId })
                .from(placement.table)
                .where(eq(placement.table.id, moving.id));

            return row?.target !== machine;
        };
        const watching = database.log
            .until(isLeft, AbortSignal.any([signal, watched.signal]))
            .then((isMoved) => {
                if (isMoved) {
                    left.abort();
                }
            });

        // receive it, ending quietly once the move left this machine
        try {
            await SpaceTransfer.receive(
                moving,
                AbortSignal.any([signal, left.signal]),
                this.#options,
                this.#store,
            );
        } catch (error) {
            if (!left.signal.aborted) {
                throw error;
            }
        } finally {
            watched.abort();
            await watching;
        }
    }
}
