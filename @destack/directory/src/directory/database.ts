import { and, eq, gt, inArray, lte, min, ne, or, sql, type DatabaseConnection } from "@destack/db";
import { CHAIN_TERMS } from "@destack/db/query";
import { canonicalize } from "@destack/schema/json";
import { ServiceError } from "@destack/service/error";
import {
    claimTable,
    RESERVATION_MILLISECONDS,
    type Claim,
    type ClaimOwner,
    type Expiry,
    type ObjectClaims,
} from "../claim/claim.ts";
import { cellTable, zoneTable, type Cell, type Zone } from "../zone/zone.ts";
import { Directory } from "./directory.ts";

/** The columns of a zone. */
const ZONE_COLUMNS = {
    id: zoneTable.id,
    scope: zoneTable.scope,
    cell: zoneTable.cell,
    epoch: zoneTable.epoch,
};

/** The directory in the global database. */
export class DirectoryDatabase extends Directory {
    /** The global database. */
    readonly database: DatabaseConnection;

    /** Keep the zones, cells and claims in the global database. */
    constructor(database: DatabaseConnection) {
        super();
        this.database = database;
    }

    // zones and cells

    /** Place a zone in its cell at an epoch and refuse an earlier epoch. */
    async place(zone: Zone): Promise<void> {
        // move the zone to a later epoch, or keep the same placement
        const placed = await this.database
            .insert(zoneTable)
            .values(zone)
            .onConflictDoUpdate({
                target: zoneTable.id,
                set: { scope: zone.scope, cell: zone.cell, epoch: zone.epoch, target: null },
                setWhere: sql`${zoneTable.epoch} < ${zone.epoch} OR (${zoneTable.epoch} = ${zone.epoch} AND ${zoneTable.cell} = ${zone.cell})`,
            })
            .returning({ id: zoneTable.id });
        if (placed.length === 0) {
            throw new ServiceError("CONFLICT", {
                message: `${zone.id} is placed at a later epoch or in another cell`,
            });
        }
    }

    /** Withdraw a zone its cell serves at an epoch and refuse a later epoch. */
    async withdraw(zone: Zone): Promise<void> {
        // delete the zone as its cell serves it
        const withdrawn = await this.database
            .delete(zoneTable)
            .where(this.#serving(zone))
            .returning({ id: zoneTable.id });

        // refuse a zone another cell or a later epoch serves
        const moved = withdrawn.length === 0 ? await this.locate(zone.id) : undefined;
        if (moved !== undefined) {
            throw new ServiceError("CONFLICT", {
                message: `${zone.id} is no longer placed in ${zone.cell} at epoch ${zone.epoch}`,
            });
        }
    }

    /** Find the zone of a scope's databases. */
    async locate(scope: string): Promise<Zone | undefined> {
        const [zone] = await this.database
            .select(ZONE_COLUMNS)
            .from(zoneTable)
            .where(eq(zoneTable.id, scope));

        return zone;
    }

    /** List the zones a scope contains, in identity order. */
    async list(scope: string): Promise<readonly Zone[]> {
        return this.database
            .select(ZONE_COLUMNS)
            .from(zoneTable)
            .where(eq(zoneTable.scope, scope))
            .orderBy(zoneTable.id);
    }

    /** Mark a zone as moving to a target cell. */
    async move(zone: Zone, target: string): Promise<void> {
        const moving = await this.database
            .update(zoneTable)
            .set({ target })
            .where(this.#serving(zone))
            .returning({ id: zoneTable.id });
        if (moving.length === 0) {
            throw new ServiceError("CONFLICT", {
                message: `${zone.id} is no longer placed in ${zone.cell} at epoch ${zone.epoch}`,
            });
        }
    }

    /** Follow the zones moving to a cell, now and after each change. */
    async *incoming(cell: string, signal: AbortSignal): AsyncGenerator<readonly Zone[]> {
        // read the zones moving to the cell, in identity order
        const read = () =>
            this.database
                .select(ZONE_COLUMNS)
                .from(zoneTable)
                .where(eq(zoneTable.target, cell))
                .orderBy(zoneTable.id);

        // yield them, then wait for a commit changing them
        let zones = await read();
        while (!signal.aborted) {
            yield zones;
            const shown = canonicalize(zones);
            const isChanged = await this.database.log.until(async () => {
                zones = await read();

                return canonicalize(zones) !== shown;
            }, signal);
            if (!isChanged) {
                return;
            }
        }
    }

    /** Record the URL a cell answers at, replacing the one it published before. */
    async publish(cell: string, scope: string, endpoint: string): Promise<void> {
        const now = Date.now();
        await this.database
            .insert(cellTable)
            .values({ id: cell, scope, endpoint, publishedAt: now })
            .onConflictDoUpdate({
                target: cellTable.id,
                set: { scope, endpoint, publishedAt: now },
            });
    }

    /** Read a cell, absent before it published an endpoint. */
    async cell(id: string): Promise<Cell | undefined> {
        const [found] = await this.database
            .select({ id: cellTable.id, scope: cellTable.scope, endpoint: cellTable.endpoint })
            .from(cellTable)
            .where(eq(cellTable.id, id));

        return found;
    }

    // claims

    /** Reserve a request's claims in chunks. */
    async claim(claims: readonly Claim[], requestId: string, now: number): Promise<void> {
        for (let start = 0; start < claims.length; start += CHAIN_TERMS) {
            await this.#reserve(claims.slice(start, start + CHAIN_TERMS), requestId, now);
        }
    }

    /** Confirm a request's reserved claims and release the names its objects dropped. */
    async confirm(requestId: string, owned: readonly ObjectClaims[]): Promise<void> {
        await this.database
            .update(claimTable)
            .set({ state: "confirmed" })
            .where(eq(claimTable.requestId, requestId));
        await this.#release(owned);
    }

    /** Release the reservations of a request with a failed write. */
    async release(requestId: string): Promise<void> {
        await this.database
            .delete(claimTable)
            .where(and(eq(claimTable.requestId, requestId), eq(claimTable.state, "reserved")));
    }

    /** Replace an object's claims after a write that reserved none, and release the rest. */
    async replace(owned: ObjectClaims, requestId: string, now: number): Promise<void> {
        // release every name of an object claiming none
        const { claims } = owned;
        if (claims.length === 0) {
            await this.#release([owned]);

            return;
        }

        // read the owners of the object's names and refuse names of other objects
        const held = await this.database
            .select({
                index: claimTable.index,
                key: claimTable.key,
                objectId: claimTable.objectId,
            })
            .from(claimTable)
            .where(or(...claims.map((entry) => matches(entry))));
        for (const entry of claims) {
            const owner = held.find((row) => row.index === entry.index && row.key === entry.key);
            if (owner !== undefined && owner.objectId !== entry.objectId) {
                throw new ServiceError("CONFLICT", {
                    message: `${entry.index.split("/").at(-1)} of ${entry.index.split("/").at(-2)} ${entry.objectId} is taken by ${owner.objectId}`,
                });
            }
        }

        // confirm the names the object owns already, insert the new ones and release the rest
        const fresh = claims.filter(
            (entry) => !held.some((row) => row.index === entry.index && row.key === entry.key),
        );
        if (held.length > 0) {
            await this.database
                .update(claimTable)
                .set({ state: "confirmed" })
                .where(or(...held.map((entry) => matches(entry))));
        }
        if (fresh.length > 0) {
            await this.database.insert(claimTable).values(
                fresh.map((entry) => ({
                    ...entry,
                    state: "confirmed" as const,
                    requestId,
                    expiresAt: now,
                })),
            );
        }
        await this.#release([owned]);
    }

    /** Find the object owning a confirmed name. */
    async owner(index: string, key: string): Promise<ClaimOwner | undefined> {
        const [found] = await this.database
            .select({ objectId: claimTable.objectId, scope: claimTable.scope })
            .from(claimTable)
            .where(
                and(
                    eq(claimTable.index, index),
                    eq(claimTable.key, key),
                    eq(claimTable.state, "confirmed"),
                ),
            );

        return found;
    }

    /** List the expired reservations of some indexes, and the next deadline. */
    async expired(indexes: readonly string[], now: number): Promise<Expiry> {
        // read the expired reservations of the indexes
        const reserved = and(
            eq(claimTable.state, "reserved"),
            inArray(claimTable.index, [...indexes]),
        );
        const claims = await this.database
            .select({
                index: claimTable.index,
                key: claimTable.key,
                objectId: claimTable.objectId,
                scope: claimTable.scope,
            })
            .from(claimTable)
            .where(and(reserved, lte(claimTable.expiresAt, now)));

        // find the next deadline
        const [next] = await this.database
            .select({ expiresAt: min(claimTable.expiresAt) })
            .from(claimTable)
            .where(and(reserved, gt(claimTable.expiresAt, now)));

        return next?.expiresAt == null ? { claims } : { claims, next: next.expiresAt };
    }

    /** Match a zone as its cell serves it at its epoch. */
    #serving(zone: Zone) {
        return and(
            eq(zoneTable.id, zone.id),
            eq(zoneTable.cell, zone.cell),
            eq(zoneTable.epoch, zone.epoch),
        );
    }

    /** Reserve a chunk of claims in one insert. */
    async #reserve(claims: readonly Claim[], requestId: string, now: number): Promise<void> {
        // insert, skipping taken names
        const expiresAt = now + RESERVATION_MILLISECONDS;
        const inserted = await this.database
            .insert(claimTable)
            .values(
                claims.map((entry) => ({
                    ...entry,
                    state: "reserved" as const,
                    requestId,
                    expiresAt,
                })),
            )
            .onConflictDoNothing()
            .returning({ index: claimTable.index, key: claimTable.key });
        const reserved = new Set(inserted.map((entry) => JSON.stringify([entry.index, entry.key])));
        const taken = claims.filter(
            (entry) => !reserved.has(JSON.stringify([entry.index, entry.key])),
        );
        if (taken.length === 0) {
            return;
        }

        // refuse a name another object claims
        const held = await this.database
            .select({
                index: claimTable.index,
                key: claimTable.key,
                objectId: claimTable.objectId,
            })
            .from(claimTable)
            .where(or(...taken.map((entry) => matches(entry))));
        for (const entry of taken) {
            const owner = held.find((row) => row.index === entry.index && row.key === entry.key);
            if (owner?.objectId !== entry.objectId) {
                throw new ServiceError("CONFLICT", {
                    message: `${entry.index.split("/").at(-1)} is taken`,
                });
            }
        }
    }

    /** Release the names objects no longer claim. */
    async #release(owned: readonly ObjectClaims[]): Promise<void> {
        for (let start = 0; start < owned.length; start += CHAIN_TERMS) {
            const chunk = owned.slice(start, start + CHAIN_TERMS);
            await this.database
                .delete(claimTable)
                .where(
                    or(
                        ...chunk.map(({ indexes, objectId, claims }) =>
                            and(
                                inArray(claimTable.index, [...indexes]),
                                eq(claimTable.objectId, objectId),
                                ...claims.map((entry) =>
                                    or(
                                        ne(claimTable.index, entry.index),
                                        ne(claimTable.key, entry.key),
                                    ),
                                ),
                            ),
                        ),
                    ),
                );
        }
    }
}

/** Match the row of one claimed name. */
function matches(entry: { readonly index: string; readonly key: string }) {
    return and(eq(claimTable.index, entry.index), eq(claimTable.key, entry.key));
}
