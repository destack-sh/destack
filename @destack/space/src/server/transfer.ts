import { and, eq, isNull } from "@destack/db";
import type { LogPosition } from "@destack/db/log";
import { Condition } from "@destack/db/query";
import type { Directory, Zone } from "@destack/directory";
import type { ObjectType } from "@destack/object";
import type { ObjectServer } from "@destack/object/server";
import { zone } from "@destack/account/object";
import {
    type Copy,
    Plan,
    Recipient,
    type ResourceKind,
    Provider,
    type Provisioning,
    type Copying,
} from "@destack/resource";
import type { Identifier } from "@destack/schema";
import { ServiceError } from "@destack/service/error";
import { Replica, type QueryPage, Scope } from "@destack/sync";
import type { connect } from "../client/client.ts";
import {
    resource,
    type Resource,
    resourceTransfer,
    space,
    SpaceCell,
    transfer,
} from "../object/index.ts";
import { appliedStates, type ProviderIndex } from "./resource.ts";
import { requireCreation } from "./space.ts";
import { ZoneTransfer } from "./zone.ts";

/** The spaces a cell receives at once: four copies streaming side by side keep a link of 100 MB/s busy at 25 MB/s each. */
const TRANSFER_CONCURRENCY = 4;
/** A source's relay of its spaces' zones, as a target reaches it. */
export type ZoneRelay = ReturnType<typeof connect>["relay"];

/** What a cell moves spaces with: the directory, the tables with a space's rows, its sources' relays and its providers. */
export interface TransferOptions {
    /** The host or region serving the spaces. */
    readonly cell: SpaceCell;
    /** The global directory delegating the spaces' zones. */
    readonly directory: Directory;
    /** Reach a source's relay by its cell, proving this cell's identity. */
    source(cell: string): Promise<ZoneRelay>;
    /** The cell's providers. */
    readonly providers: ProviderIndex;
    /** The object server of the cell's space database. */
    server(): Pick<ObjectServer, "database" | "invoke">;
}

/** Serve transfers: record their source, and announce each transfer from this cell in the directory until the source fences the space. */
export function serveTransfers(options: TransferOptions) {
    return transfer
        .handle({
            create: {
                authorize: requireCreation(transfer),
                prepare: async (call) => {
                    // transfer only a space this cell serves
                    const zone = await options.directory.locate(call.scope);
                    if (zone === undefined || zone.cell !== SpaceCell.id(options.cell)) {
                        throw new ServiceError("CONFLICT", {
                            message: `${call.scope} is served by another host or region`,
                        });
                    }

                    // move it only to a cell of its account or a region, known to the directory
                    const target = call.input.target as string;
                    const cell = await options.directory.cell(target);
                    const isAdmitted =
                        cell !== undefined &&
                        (cell.scope === zone.scope || cell.scope === Scope.universe.id);
                    if (!isAdmitted) {
                        throw new ServiceError("BAD_REQUEST", {
                            message: `${target} is no host of the space's account nor a region`,
                        });
                    }

                    return zone;
                },
                effect: (call, next) => {
                    // move the space from this cell at its epoch
                    const zone = call.prepared as Zone;

                    return next(
                        call.with({
                            input: {
                                ...call.input,
                                source: zone.cell,
                                sourceEpoch: zone.epoch,
                                requestedBy: call.caller,
                            },
                        }),
                    );
                },
            },
        })
        .control({
            // TODO #Incomplete: remove the fenced source's copy of the space once the target activates it and retention allows
            pending: Condition.all(
                Condition.missing("completedAt"),
                Condition.eq("source", SpaceCell.id(options.cell)),
            ),
            reconcile: async (reconciliation) => {
                // act on a transfer of a space this cell serves still
                const record = reconciliation.rows[0]!;
                const [served] = await reconciliation.database
                    .select({ accountId: space.table.scope })
                    .from(space.table)
                    .where(and(eq(space.table.id, record.scope), SpaceCell.served()));
                if (served === undefined) {
                    return undefined;
                }

                // mark the zone as moving to the target at the epoch the source serves it at
                await options.directory.move(
                    {
                        id: record.scope,
                        scope: served.accountId,
                        cell: record.source,
                        epoch: record.sourceEpoch,
                    },
                    record.target,
                );

                return undefined;
            },
        });
}

/** Serve the zones moving to this cell, as the cell copies them: receive each space, then take it over. */
export function serveZones(options: TransferOptions) {
    return zone.control({
        pending: Condition.eq("target", SpaceCell.id(options.cell)),
        concurrency: TRANSFER_CONCURRENCY,
        reconcile: async (reconciliation) => {
            // read the zone as the directory places it, in its account
            const row = reconciliation.rows[0]!;
            const moving = { id: row.id, scope: row.parent, cell: row.cell, epoch: row.epoch };
            await receive(moving, reconciliation.signal, options);

            return undefined;
        },
    });
}

/** One resource a target copies with its provider, desired states and import cursor. */
interface ResourceCopy {
    /** The resource as the source records it. */
    readonly row: Resource;
    /** The target's provider of the resource. */
    readonly provider: Provider<ResourceKind, ObjectType> & Provisioning & Copying;
    /** The resource as the target provisioned it. */
    readonly record: Resource & { readonly providerCode: string; readonly reference: string };
    /** The desired states the source applied for the target to apply. */
    readonly desired: Copy["desired"];
    /** The cursor of the last chunk imported, absent before the first. */
    cursor: string | undefined;
}

/** Receive one space moving to this cell: copy it unless promoted already, then delegate its zone here at the next epoch. */
async function receive(zone: Zone, signal: AbortSignal, options: TransferOptions): Promise<void> {
    // find whether an earlier attempt promoted the copy already
    const database = options.server().database;
    const steps = ZoneTransfer.of(database, zone, SpaceCell.id(options.cell));
    const [copied] = await database
        .select({ id: space.table.id })
        .from(space.table)
        .where(eq(space.table.id, zone.id as Identifier<"space">));
    const isPromoted = copied !== undefined && !(await Replica.isCopied(database, zone.id));

    // copy the space unless promoted, then delegate its zone here
    if (!isPromoted) {
        await copyZone(steps, await options.source(zone.cell), signal, options);
    }
    await steps.activate(options.directory);
}

/** Copy a space and its resources from its source, have the source fence, finish both copies, verify, and take the space over. */
async function copyZone(
    steps: ZoneTransfer,
    source: ZoneRelay,
    signal: AbortSignal,
    options: TransferOptions,
): Promise<void> {
    // follow the space's rows while the source serves them
    const server = options.server();
    const zone = steps.zone;
    const following = new AbortController();
    const stopping = AbortSignal.any([signal, following.signal]);
    const followed = steps.follow(
        server.database,
        (after, stop) => watch(source, zone, after, stop),
        stopping,
    );

    // stop waiting on the copy once following fails
    followed.catch((error: unknown) => following.abort(error));
    try {
        // copy each resource's live content once the copy has the space's rows
        await server.database.log.until(() => Replica.isCopied(server.database, zone.id), stopping);
        stopping.throwIfAborted();
        const recipient = await Recipient.generate();
        const copies = new Map<string, ResourceCopy>();
        await pull(source, zone, copies, "live", recipient, signal, options);

        // have the source fence, then reach its position and verify the copy
        const fenced = await source.fence({ spaceId: zone.id as Identifier<"space"> }, { signal });
        if (!(await steps.reach(server.database, fenced, stopping))) {
            stopping.throwIfAborted();
        }
        following.abort();
        await followed;
        await steps.verify(server.database, fenced);

        // copy what changed since, and what only the fenced source keeps, then take the space over
        await pull(source, zone, copies, "fenced", recipient, signal, options);
        await takeOver(steps, [...copies.values()], options);
    } finally {
        following.abort();
        await followed;
    }
}

/** Follow a space's rows through its source's relay from a position. */
async function* watch(
    source: ZoneRelay,
    zone: Zone,
    after: LogPosition | undefined,
    signal: AbortSignal,
): AsyncGenerator<QueryPage> {
    yield* await source.watch(
        { spaceId: zone.id as Identifier<"space">, ...(after === undefined ? {} : { after }) },
        { signal },
    );
}

/** Copy each resource of the copied space through the target's providers, provisioning the ones new to the target. */
async function pull(
    source: ZoneRelay,
    zone: Zone,
    copies: Map<string, ResourceCopy>,
    stage: Copy["stage"],
    recipient: Recipient,
    signal: AbortSignal,
    options: TransferOptions,
): Promise<void> {
    const rows = await options
        .server()
        .database.select()
        .from(resource.table)
        .where(eq(resource.table.scope, zone.id as Identifier<"space">));

    // destroy the copies of resources the fenced source no longer has
    if (stage === "fenced") {
        for (const [id, copy] of copies) {
            if (!rows.some((row) => row.id === id)) {
                await copy.provider.destroy(copy.provider.kind.record(copy.record));
                copies.delete(id);
            }
        }
    }

    // copy each resource the source has
    for (const row of rows) {
        // provision the resource on the target, and apply the desired states the source applied
        let copy = copies.get(row.id);
        if (copy === undefined) {
            copy = await provision(row, options);
            copies.set(row.id, copy);
        }

        // import the source's chunks from the cursor on
        const chunks = await source.export(
            {
                spaceId: zone.id as Identifier<"space">,
                resourceId: row.id,
                stage,
                recipient: recipient.key,
                ...(copy.cursor === undefined ? {} : { after: copy.cursor }),
            },
            { signal },
        );
        const imported = {
            record: copy.provider.kind.record(copy.record),
            desired: copy.desired,
            recipient,
            stage,
        };
        let isEnded = false;
        for await (const record of chunks) {
            if ("end" in record) {
                isEnded = true;
            } else {
                await copy.provider.import(imported, record.chunk);
                copy.cursor = record.chunk.cursor;
            }
        }

        // refuse an export that stopped before its end
        if (!isEnded) {
            throw new Error(`the export of ${row.id} stopped before its end`);
        }
    }
}

/** Provision a resource with the target's provider and apply the source's desired states. */
async function provision(row: Resource, options: TransferOptions): Promise<ResourceCopy> {
    // select the provider the resource requests, or the first of its kind
    const provider = options.providers.select({ ...row, providerCode: null });
    if (provider === undefined) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: `no provider of ${row.kind} receives ${row.id}`,
        });
    } else if (!Provider.provisions(provider) || !Provider.copies(provider)) {
        throw new ServiceError("PRECONDITION_FAILED", {
            message: `provider ${provider.code} of ${row.kind} hosts and imports no content`,
        });
    }

    // provision the resource, and apply the desired states the source applied
    const desired = provider.kind.states(await appliedStates(options.server().database, row));
    const provision = await provider.provision(provider.kind.record({ ...row, reference: null }));
    const record = {
        ...row,
        providerCode: provider.code,
        reference: provision.reference,
        location: provision.location ?? null,
        hostId: SpaceCell.host(options.providers.cell),
    };
    if (Provider.reconciles(provider)) {
        const plan = await provider.plan(provider.kind.record(record), desired);
        await provider.apply(provider.kind.record(record), desired, await Plan.digest(plan));
    }

    return { row, provider, record, desired, cursor: undefined };
}

/** Promote the copy and record the takeover in one transaction. */
async function takeOver(
    steps: ZoneTransfer,
    copies: readonly ResourceCopy[],
    options: TransferOptions,
): Promise<void> {
    // take the zone over in one transaction at one time
    const server = options.server();
    const zone = steps.zone;
    const scope = zone.id as Identifier<"space">;
    const now = Date.now();
    await server.database.transaction(async (transaction) => {
        // promote the copy in the same transaction as the takeover
        await steps.promote(transaction);
        const invoke = (
            object: typeof resource | typeof resourceTransfer | typeof transfer,
            name: string,
            input: Readonly<Record<string, unknown>>,
        ) => server.invoke(transaction, scope, object, name, input, now);

        // find the transfer the copy has
        const [record] = await transaction
            .select()
            .from(transfer.table)
            .where(and(eq(transfer.table.scope, scope), isNull(transfer.table.completedAt)));
        if (record === undefined) {
            throw new ServiceError("NOT_FOUND", {
                message: `the copy of ${zone.id} has no active transfer`,
            });
        }

        // point each resource at the target's provisioning, and record its part of the transfer
        for (const copy of copies) {
            const [current] = await transaction
                .select({ observedGeneration: resource.table.observedGeneration })
                .from(resource.table)
                .where(eq(resource.table.id, copy.row.id));
            await invoke(resource, "observe", {
                id: copy.row.id,
                observedGeneration: current!.observedGeneration,
                conditions: {},
                fields: {
                    providerCode: copy.record.providerCode,
                    reference: copy.record.reference,
                    location: copy.record.location,
                    hostId: copy.record.hostId,
                },
            });
            await invoke(resourceTransfer, "create", {
                transferId: record.id,
                resourceId: copy.row.id,
                sourceProviderCode: copy.row.providerCode!,
                sourceReference: copy.row.reference!,
                sourceHostId: copy.row.hostId,
                targetProviderCode: copy.record.providerCode,
                targetReference: copy.record.reference,
                targetHostId: copy.record.hostId,
                copiedAt: now,
            });
        }

        // complete the transfer
        await invoke(transfer, "complete", { id: record.id });
    });
}
