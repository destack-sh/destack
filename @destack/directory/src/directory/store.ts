import {
    Change,
    and,
    desc,
    eq,
    gt,
    inArray,
    lte,
    min,
    ne,
    or,
    sql,
    type DatabaseConnection,
    ReadCache,
    CHAIN_TERMS,
    DatabaseError,
    type Select,
} from "@destack/db";
import { ServiceError } from "@destack/service/error";
import {
    claimTable,
    RESERVATION_MILLISECONDS,
    type Claim,
    type Expiry,
    type ObjectClaims,
} from "../claim/claim.ts";
import {
    assignmentTable,
    cellTable,
    ZONE_SCOPE,
    zoneTable,
    type Cell,
    type Zone,
} from "../zone/zone.ts";
import { Directory } from "./directory.ts";
import {
    type Identity,
    IdentityOperation,
    type OperationClaims,
    identityOperationTable,
    RECOVERY_MILLISECONDS,
} from "../identity/identity.ts";

/** The most reads of each kind the store keeps: about 3 MiB at 200 bytes an entry. */
const CAPACITY = 16_384;

/** The tables of the kept reads, with logged changes that invalidate them. */
const CACHED_TABLES = [claimTable, zoneTable, cellTable, identityOperationTable];

/** The columns of a zone. */
const ZONE_COLUMNS = {
    id: zoneTable.id,
    scope: zoneTable.parent,
    cell: zoneTable.cell,
    epoch: zoneTable.epoch,
};

/** The directory itself, in the account service's database, keeping its reads while it follows the log. */
export class DirectoryStore extends Directory {
    /** The account service's database. */
    readonly database: DatabaseConnection;
    /** Claim owners by index and key. */
    readonly #owners = new ReadCache<Pick<Claim, "objectId" | "scope"> | undefined>(CAPACITY);
    /** Zones by scope. */
    readonly #zones = new ReadCache<Zone | undefined>(CAPACITY);
    /** Cells by identifier. */
    readonly #cells = new ReadCache<Cell | undefined>(CAPACITY);
    /** Current identities by space. */
    readonly #identities = new ReadCache<Identity | undefined>(CAPACITY);
    /** The directory's clock, in UTC epoch milliseconds. */
    readonly #clock: () => number;
    /** Whether the log is followed, which keeps the reads current. */
    #isFollowing = false;

    /** Keep the directory's tables in the account service's database. */
    constructor(database: DatabaseConnection, options: { readonly clock?: () => number } = {}) {
        super();
        this.database = database;
        this.#clock = options.clock ?? (() => Date.now());
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

    /** Place a zone in its cell: create it, keep its placement and end its move, or take it over in its move's target at the next epoch. */
    async place(zone: Zone): Promise<void> {
        // keep the placement, or take the zone over in the cell its move still targets
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
                setWhere: sql`(${zoneTable.epoch} = ${zone.epoch} AND ${zoneTable.cell} = ${zone.cell}) OR (${zoneTable.epoch} + 1 = ${zone.epoch} AND ${zoneTable.target} = ${zone.cell})`,
            })
            .returning({ id: zoneTable.id });
        if (placed.length === 0) {
            throw new ServiceError("CONFLICT", {
                message: `${zone.id} is neither placed in ${zone.cell} at epoch ${zone.epoch} nor moving there`,
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

    /** Give a cell work in a zone, as the cell serving the zone at its epoch. */
    async assign(zone: Zone, cell: string): Promise<void> {
        await this.#requireServing(zone);
        await this.database
            .insert(assignmentTable)
            .values({ zone: zone.id, cell, scope: ZONE_SCOPE, assignedAt: this.#clock() })
            .onConflictDoNothing();
    }

    /** Withdraw a cell's work in a zone, as the cell serving the zone at its epoch. */
    async unassign(zone: Zone, cell: string): Promise<void> {
        await this.#requireServing(zone);
        await this.database
            .delete(assignmentTable)
            .where(and(eq(assignmentTable.zone, zone.id), eq(assignmentTable.cell, cell)));
    }

    /** List the zones that gave a cell work, in identity order. */
    async assignments(cell: string): Promise<readonly string[]> {
        const rows = await this.database
            .select({ zone: assignmentTable.zone })
            .from(assignmentTable)
            .where(eq(assignmentTable.cell, cell))
            .orderBy(assignmentTable.zone);

        return rows.map((row) => row.zone);
    }

    /** List the cells a zone gave work, in identity order. */
    async assigned(zone: string): Promise<readonly string[]> {
        const rows = await this.database
            .select({ cell: assignmentTable.cell })
            .from(assignmentTable)
            .where(eq(assignmentTable.zone, zone))
            .orderBy(assignmentTable.cell);

        return rows.map((row) => row.cell);
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
        const now = this.#clock();
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

    // identities

    /** Apply a signed identity operation, a space's first only from the cell serving its zone. */
    async apply(operation: string, zone?: Zone): Promise<void> {
        // read the operation and the space's log
        const claims = IdentityOperation.claims(operation);
        const log = await this.database
            .select()
            .from(identityOperationTable)
            .where(eq(identityOperationTable.space, claims.space))
            .orderBy(identityOperationTable.sequence);

        // start the identity, or follow a current operation of it
        if (log.length === 0) {
            await this.#start(operation, claims, zone);
        } else {
            await this.#follow(operation, claims, log);
        }
    }

    /** Start a space's identity from the cell serving its zone, signed by one of the operation's rotation keys. */
    async #start(operation: string, claims: OperationClaims, zone?: Zone): Promise<void> {
        // require the space's zone in the cell serving it, and no operation before
        const isServed =
            zone !== undefined &&
            zone.id === claims.space &&
            claims.previous === null &&
            (
                await this.database
                    .select({ id: zoneTable.id })
                    .from(zoneTable)
                    .where(this.#serving(zone))
            ).length > 0;
        if (!isServed) {
            throw new ServiceError("FORBIDDEN", {
                message: `only the cell serving ${claims.space} starts its identity`,
            });
        }

        // append it first in the log
        await this.#append(operation, claims.space, 0, await this.#priority(operation, claims));
    }

    /** Follow a current operation of a space's identity, nullifying later ones its higher-priority key may recover. */
    async #follow(
        operation: string,
        claims: OperationClaims,
        log: readonly Select<typeof identityOperationTable>[],
    ): Promise<void> {
        // find the current operation it follows
        const current = log.filter((entry) => !entry.isNullified);
        const previous = current.find((entry) => entry.digest === claims.previous);
        if (previous === undefined) {
            throw new ServiceError("CONFLICT", {
                message: `the operation follows no current operation of ${claims.space}`,
            });
        }

        // require a rotation key of the identity it follows, outranking any operation it nullifies
        const priority = await this.#priority(
            operation,
            IdentityOperation.claims(previous.operation),
        );
        const following = current.find((entry) => entry.sequence > previous.sequence);
        const isRecoverable =
            following === undefined ||
            (priority < following.priority &&
                following.appliedAt + RECOVERY_MILLISECONDS >= this.#clock());
        if (!isRecoverable) {
            throw new ServiceError("CONFLICT", {
                message: `the operation cannot nullify the later operations of ${claims.space}`,
            });
        }

        // nullify the operations after the one it follows, and append it
        await this.database.transaction(async (transaction) => {
            await transaction
                .update(identityOperationTable)
                .set({ isNullified: true })
                .where(
                    and(
                        eq(identityOperationTable.space, claims.space),
                        gt(identityOperationTable.sequence, previous.sequence),
                    ),
                );
            await this.#append(operation, claims.space, log.length, priority, transaction);
        });
    }

    /** Read a space's current identity, absent before its first operation. */
    async identity(space: string): Promise<Identity | undefined> {
        return this.#read(this.#identities, space, async () => {
            // read the latest operation in force
            const [latest] = await this.database
                .select({
                    operation: identityOperationTable.operation,
                    digest: identityOperationTable.digest,
                })
                .from(identityOperationTable)
                .where(
                    and(
                        eq(identityOperationTable.space, space),
                        eq(identityOperationTable.isNullified, false),
                    ),
                )
                .orderBy(desc(identityOperationTable.sequence))
                .limit(1);
            if (latest === undefined) {
                return undefined;
            }
            const { signingKey, rotationKeys } = IdentityOperation.claims(latest.operation);

            return { signingKey, rotationKeys, digest: latest.digest };
        });
    }

    /** List a space's signed identity operations in order, nullified ones included. */
    async operations(space: string): Promise<readonly string[]> {
        const log = await this.database
            .select({ operation: identityOperationTable.operation })
            .from(identityOperationTable)
            .where(eq(identityOperationTable.space, space))
            .orderBy(identityOperationTable.sequence);

        return log.map((entry) => entry.operation);
    }

    // claims

    /** Reserve a request's claims in chunks until its write commits, returning the names other objects hold. */
    async claim(claims: readonly Claim[], requestId: string): Promise<readonly Claim[]> {
        // reserve each chunk for a minute, collecting the names other objects hold
        const expiresAt = this.#clock() + RESERVATION_MILLISECONDS;
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

    /** Replace an object's claims after a write that reserved none, unless other objects hold some of its names. */
    async replace(owned: ObjectClaims, requestId: string): Promise<readonly Claim[]> {
        // read the owners of the object's names, a chunk at a time
        const chunks: { claims: readonly Claim[]; owners: Map<string, string> }[] = [];
        for (let start = 0; start < owned.claims.length; start += CHAIN_TERMS) {
            const claims = owned.claims.slice(start, start + CHAIN_TERMS);
            chunks.push({ claims, owners: await this.#owned(claims) });
        }

        // refuse the names other objects hold before writing any
        const refused = chunks.flatMap(({ claims, owners }) => taken(claims, owners));
        if (refused.length > 0) {
            return refused;
        }

        // confirm the names the object owns already and insert the new ones
        const now = this.#clock();
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
        const now = this.#clock();
        const reserved = and(
            eq(claimTable.state, "reserved"),
            inArray(claimTable.index, [...indexes]),
        );
        const claims = await this.database
            .select({
                index: claimTable.index,
                packageId: claimTable.packageId,
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
        if (next === undefined) {
            throw new TypeError("min aggregate returned no row");
        }

        return next.expiresAt === null ? { claims } : { claims, next: next.expiresAt };
    }

    /** Refuse a zone its cell no longer serves at the epoch given. */
    async #requireServing(zone: Zone): Promise<void> {
        const placed = await this.locate(zone.id);
        if (placed?.cell !== zone.cell || placed.epoch !== zone.epoch) {
            throw new ServiceError("CONFLICT", {
                message: `${zone.id} is no longer placed in ${zone.cell} at epoch ${zone.epoch}`,
            });
        }
    }

    /** Match a zone as its cell serves it at its epoch. */
    #serving(zone: Zone) {
        return and(
            eq(zoneTable.id, zone.id),
            eq(zoneTable.cell, zone.cell),
            eq(zoneTable.epoch, zone.epoch),
        );
    }

    /** Find the priority of the rotation key of an identity that signed an operation, refusing one none signed. */
    async #priority(operation: string, identity: Pick<Identity, "rotationKeys">): Promise<number> {
        const priority = await IdentityOperation.priority(operation, identity.rotationKeys);
        if (priority === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "no rotation key of the identity signed the operation",
            });
        }

        return priority;
    }

    /** Append a verified operation to a space's log. */
    async #append(
        operation: string,
        space: string,
        sequence: number,
        priority: number,
        database: DatabaseConnection = this.database,
    ): Promise<void> {
        // refuse an operation another one appended at the same position first
        try {
            await database.insert(identityOperationTable).values({
                space,
                sequence,
                scope: ZONE_SCOPE,
                digest: await IdentityOperation.digest(operation),
                operation,
                priority,
                appliedAt: this.#clock(),
                isNullified: false,
            });
        } catch (error) {
            throw error instanceof DatabaseError && error.code === "DUPLICATE"
                ? new ServiceError("CONFLICT", {
                      message: `another operation of ${space} was applied first`,
                      cause: error,
                  })
                : error;
        }
    }

    /** Reserve a chunk of claims in one insert, returning the names other objects hold. */
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
        // forget each kind of read
        this.#owners.clear();
        this.#zones.clear();
        this.#cells.clear();
        this.#identities.clear();
    }

    /** Forget the reads one change affects, before and after it. */
    #forget(change: Change<(typeof CACHED_TABLES)[number]>): void {
        const images = [Change.before(change), Change.after(change)].filter(
            (image) => image !== null,
        );
        for (const image of images) {
            // forget a claim by its name
            if ("index" in image) {
                this.#owners.forget(nameKey({ index: image.index, key: image.key }));
            }
            // forget an identity by its space
            else if ("sequence" in image) {
                this.#identities.forget(image.space);
            }
            // forget a zone by its identifier
            else if (change.table === zoneTable) {
                this.#zones.forget(image.id);
            }
            // forget a cell by its identifier
            else {
                this.#cells.forget(image.id);
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
