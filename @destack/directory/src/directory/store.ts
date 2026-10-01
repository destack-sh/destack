import { and, eq, gt, inArray, lte, min, ne, or, sql, type DatabaseConnection } from "@destack/db";
import { ReadCache, type Change } from "@destack/db/log";
import { CHAIN_TERMS } from "@destack/db/query";
import { ServiceError } from "@destack/service/error";
import {
    claimTable,
    RESERVATION_MILLISECONDS,
    type Claim,
    type Expiry,
    type ObjectClaims,
} from "../claim/claim.ts";
import { cellTable, ZONE_SCOPE, zoneTable, type Cell, type Zone } from "../zone/zone.ts";
import { Directory } from "./directory.ts";

/** The most reads of each kind the store keeps: about 3 MiB at 200 bytes an entry. */
const CAPACITY = 16_384;

/** The global tables of the kept reads, with logged changes that invalidate them. */
const CACHED_TABLES = [claimTable, zoneTable, cellTable];

/** The columns of a zone. */
const ZONE_COLUMNS = {
    id: zoneTable.id,
    scope: zoneTable.parent,
    cell: zoneTable.cell,
    epoch: zoneTable.epoch,
};

/** The directory itself, in the global database, keeping its reads while it follows the log. */
export class DirectoryStore extends Directory {
    /** The global database. */
    readonly database: DatabaseConnection;
    /** Claim owners by index and key. */
    readonly #owners = new ReadCache<Pick<Claim, "objectId" | "scope"> | undefined>(CAPACITY);
    /** Zones by scope. */
    readonly #zones = new ReadCache<Zone | undefined>(CAPACITY);
    /** Cells by identifier. */
    readonly #cells = new ReadCache<Cell | undefined>(CAPACITY);
    /** Whether the log is followed, which keeps the reads current. */
    #isFollowing = false;

    /** Keep the zones, cells and claims in the global database. */
    constructor(database: DatabaseConnection) {
        super();
        this.database = database;
    }

    /** Keep the reads, forgetting those each logged change affects, until the signal aborts. */
    async follow(signal: AbortSignal): Promise<void> {
        this.#isFollowing = true;
        try {
            await this.database.log.invalidate(
                CACHED_TABLES,
                { clear: () => this.#clear(), forget: (change) => this.#forget(change) },
                signal,
            );
        } finally {
            this.#isFollowing = false;
            this.#clear();
        }
    }

    // zones and cells

    /** Place a zone in its cell at an epoch and refuse an earlier epoch. */
    async place(zone: Zone): Promise<void> {
        // move the zone to a later epoch, or keep the same placement
        const placed = await this.database
            .insert(zoneTable)
            .values({
                id: zone.id,
                scope: ZONE_SCOPE,
                parent: zone.scope,
                cell: zone.cell,
                epoch: zone.epoch,
            })
            .onConflictDoUpdate({
                target: zoneTable.id,
                set: { parent: zone.scope, cell: zone.cell, epoch: zone.epoch, target: null },
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
    locate(scope: string): Promise<Zone | undefined> {
        return this.#read(this.#zones, scope, async () => {
            const [zone] = await this.database
                .select(ZONE_COLUMNS)
                .from(zoneTable)
                .where(eq(zoneTable.id, scope));

            return zone;
        });
    }

    /** List the zones a scope contains, in identity order. */
    async list(scope: string): Promise<readonly Zone[]> {
        return this.database
            .select(ZONE_COLUMNS)
            .from(zoneTable)
            .where(eq(zoneTable.parent, scope))
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
    cell(id: string): Promise<Cell | undefined> {
        return this.#read(this.#cells, id, async () => {
            const [found] = await this.database
                .select({ id: cellTable.id, scope: cellTable.scope, endpoint: cellTable.endpoint })
                .from(cellTable)
                .where(eq(cellTable.id, id));

            return found;
        });
    }

    // claims

    /** Reserve a request's claims in chunks until its write commits, returning the names other objects own. */
    async claim(claims: readonly Claim[], requestId: string): Promise<readonly Claim[]> {
        // reserve each chunk for a minute, collecting the names other objects own
        const expiresAt = Date.now() + RESERVATION_MILLISECONDS;
        const refused: Claim[] = [];
        for (let start = 0; start < claims.length; start += CHAIN_TERMS) {
            const chunk = claims.slice(start, start + CHAIN_TERMS);
            refused.push(...(await this.#reserve(chunk, requestId, expiresAt)));
        }

        return refused;
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

    /** Replace an object's claims after a write that reserved none, unless other objects own some of its names. */
    async replace(owned: ObjectClaims, requestId: string): Promise<readonly Claim[]> {
        // read the owners of the object's names, a chunk at a time
        const chunks: { claims: readonly Claim[]; owners: Map<string, string> }[] = [];
        for (let start = 0; start < owned.claims.length; start += CHAIN_TERMS) {
            const claims = owned.claims.slice(start, start + CHAIN_TERMS);
            chunks.push({ claims, owners: await this.#owned(claims) });
        }

        // refuse the names other objects own before writing any
        const refused = chunks.flatMap(({ claims, owners }) => taken(claims, owners));
        if (refused.length > 0) {
            return refused;
        }

        // confirm the names the object owns already and insert the new ones
        const now = Date.now();
        for (const { claims, owners } of chunks) {
            const held = claims.filter((entry) => owners.has(nameKey(entry)));
            const fresh = claims.filter((entry) => !owners.has(nameKey(entry)));
            if (held.length > 0) {
                await this.database
                    .update(claimTable)
                    .set({ state: "confirmed" })
                    .where(or(...held.map(matches)));
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
        }

        // release the names the object no longer claims
        await this.#release([owned]);

        return [];
    }

    /** Find the object owning a confirmed name. */
    owner(index: string, key: string): Promise<Pick<Claim, "objectId" | "scope"> | undefined> {
        return this.#read(this.#owners, nameKey({ index, key }), async () => {
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
        });
    }

    /** List the expired reservations of some indexes, and the next deadline, by the directory's clock. */
    async expired(indexes: readonly string[]): Promise<Expiry> {
        // read the expired reservations of the indexes
        const now = Date.now();
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

        return next!.expiresAt === null ? { claims } : { claims, next: next!.expiresAt };
    }

    /** Match a zone as its cell serves it at its epoch. */
    #serving(zone: Zone) {
        return and(
            eq(zoneTable.id, zone.id),
            eq(zoneTable.cell, zone.cell),
            eq(zoneTable.epoch, zone.epoch),
        );
    }

    /** Reserve a chunk of claims in one insert, returning the names other objects own. */
    async #reserve(
        claims: readonly Claim[],
        requestId: string,
        expiresAt: number,
    ): Promise<readonly Claim[]> {
        // insert, skipping held names
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
        const reserved = new Set(inserted.map((entry) => nameKey(entry)));
        const held = claims.filter((entry) => !reserved.has(nameKey(entry)));

        // keep the held names this object owns already
        return held.length === 0 ? [] : taken(held, await this.#owned(held));
    }

    /** Read the objects owning some names, by name key. */
    async #owned(claims: readonly Claim[]): Promise<Map<string, string>> {
        const rows = await this.database
            .select({
                index: claimTable.index,
                key: claimTable.key,
                objectId: claimTable.objectId,
            })
            .from(claimTable)
            .where(or(...claims.map(matches)));

        return new Map(rows.map((row) => [nameKey(row), row.objectId]));
    }

    /** Read through a cache while the log is followed, straight from the database otherwise. */
    #read<Value>(cache: ReadCache<Value>, key: string, read: () => Promise<Value>): Promise<Value> {
        return this.#isFollowing ? cache.get(key, read) : read();
    }

    /** Forget every read. */
    #clear(): void {
        this.#owners.clear();
        this.#zones.clear();
        this.#cells.clear();
    }

    /** Forget the reads one change affects, before and after it. */
    #forget(change: Change): void {
        const images = [change.before, change.after].filter(
            (image) => image !== undefined,
        ) as Readonly<Record<string, unknown>>[];
        for (const image of images) {
            // forget the read the row's table keeps it under
            if (change.table === claimTable) {
                this.#owners.forget(
                    nameKey({ index: String(image.index), key: String(image.key) }),
                );
            } else if (change.table === zoneTable) {
                this.#zones.forget(String(image.id));
            } else {
                this.#cells.forget(String(image.id));
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

/** Key a claimed name by its index and key. */
function nameKey(entry: { readonly index: string; readonly key: string }): string {
    return JSON.stringify([entry.index, entry.key]);
}

/** List the claims whose names another object owns. */
function taken(claims: readonly Claim[], owners: ReadonlyMap<string, string>): Claim[] {
    return claims.filter((entry) => {
        const owner = owners.get(nameKey(entry));

        return owner !== undefined && owner !== entry.objectId;
    });
}
