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
    type Select,
    type Table,
} from "@destack/db";
import { ServiceError } from "@destack/service/error";
import { Scope } from "@destack/sync";
import {
    claimTable,
    RESERVATION_MILLISECONDS,
    type Claim,
    type Expiry,
    type ObjectClaims,
} from "../claim/claim.ts";
import {
    assignmentTable,
    endpointTable,
    placementTable,
    type Endpoint,
    type Placement,
} from "../placement/placement.ts";
import { Directory } from "./directory.ts";
import { identityOperation } from "../identity/operation.ts";
import { type Identity, IdentityOperation } from "@destack/identity";

/** How long a higher-priority rotation key may nullify operations a lower one signed: 72 hours, as did:plc recovers. */
export const RECOVERY_MILLISECONDS = 72 * 60 * 60 * 1000;

/** The most reads of each kind the store keeps: about 3 MiB at 200 bytes an entry. */
const CAPACITY = 16_384;

/** The tables of the kept reads, with logged changes that invalidate them. */
const CACHED_TABLES = [claimTable, placementTable, endpointTable, identityOperation];

/** The columns of a placement. */
const PLACEMENT_COLUMNS = {
    id: placementTable.id,
    scope: placementTable.scope,
    machine: placementTable.machine,
    epoch: placementTable.epoch,
};

/** The directory itself, in the account service's database, keeping its reads while it follows the log. */
export class DirectoryStore extends Directory {
    /** The account service's database. */
    readonly database: DatabaseConnection;
    /** Claim owners by index and key. */
    readonly #owners = new ReadCache<Pick<Claim, "objectId" | "scope"> | undefined>(CAPACITY);
    /** Placements by space. */
    readonly #placements = new ReadCache<Placement | undefined>(CAPACITY);
    /** Endpoints by machine. */
    readonly #endpoints = new ReadCache<Endpoint | undefined>(CAPACITY);
    /** Current identities by subject. */
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

    /** Read the directory's clock. */
    protected override now(): number {
        return this.#clock();
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

    // placements and machines

    /** Place a space of its account on its machine: create the placement, keep it and end its move, advance its epoch on its own machine, or take it over on its move's target at the next epoch. */
    async place(placement: Placement): Promise<void> {
        // keep the placement, advance the epoch or take the space over
        const placed = await this.database
            .insert(placementTable)
            .values({
                id: placement.id,
                scope: placement.scope,
                machine: placement.machine,
                epoch: placement.epoch,
            })
            .onConflictDoUpdate({
                target: placementTable.id,
                set: { machine: placement.machine, epoch: placement.epoch, target: null },
                setWhere: sql`${placementTable.scope} = ${placement.scope} AND ((${placementTable.epoch} = ${placement.epoch} AND ${placementTable.machine} = ${placement.machine}) OR (${placementTable.epoch} + 1 = ${placement.epoch} AND (${placementTable.machine} = ${placement.machine} OR ${placementTable.target} = ${placement.machine})))`,
            })
            .returning({ id: placementTable.id });
        if (placed.length === 0) {
            throw new ServiceError("CONFLICT", {
                message: `${placement.id} is neither placed on ${placement.machine} at epoch ${placement.epoch} nor moving there`,
            });
        }
    }

    /** Withdraw a space its machine serves at an epoch and refuse a later epoch. */
    async withdraw(placement: Placement): Promise<void> {
        // delete the placement as its machine serves it
        const withdrawn = await this.database
            .delete(placementTable)
            .where(this.#serving(placement))
            .returning({ id: placementTable.id });

        // refuse a space another machine or a later epoch serves
        const moved = withdrawn.length === 0 ? await this.locate(placement.id) : undefined;
        if (moved !== undefined) {
            throw new ServiceError("CONFLICT", {
                message: `${placement.id} is no longer placed on ${placement.machine} at epoch ${placement.epoch}`,
            });
        }
    }

    /** Find where a space runs. */
    locate(space: string): Promise<Placement | undefined> {
        return this.#read(this.#placements, space, async () => {
            const [placement] = await this.database
                .select(PLACEMENT_COLUMNS)
                .from(placementTable)
                .where(eq(placementTable.id, space));

            return placement;
        });
    }

    /** List the placements of an account's spaces, in identity order. */
    async list(account: string): Promise<readonly Placement[]> {
        return this.database
            .select(PLACEMENT_COLUMNS)
            .from(placementTable)
            .where(eq(placementTable.scope, account))
            .orderBy(placementTable.id);
    }

    /** Give a machine work in a space, as the machine serving the space at its epoch. */
    async assign(placement: Placement, machine: string): Promise<void> {
        await this.#requireServing(placement);
        await this.database
            .insert(assignmentTable)
            .values({
                space: placement.id,
                machine,
                scope: Scope.universe.id,
                assignedAt: this.#clock(),
            })
            .onConflictDoNothing();
    }

    /** Withdraw a machine's work in a space, as the machine serving the space at its epoch. */
    async unassign(placement: Placement, machine: string): Promise<void> {
        await this.#requireServing(placement);
        await this.database
            .delete(assignmentTable)
            .where(
                and(eq(assignmentTable.space, placement.id), eq(assignmentTable.machine, machine)),
            );
    }

    /** List the spaces that gave a machine work, in identity order. */
    async assignments(machine: string): Promise<readonly string[]> {
        const rows = await this.database
            .select({ space: assignmentTable.space })
            .from(assignmentTable)
            .where(eq(assignmentTable.machine, machine))
            .orderBy(assignmentTable.space);

        return rows.map((row) => row.space);
    }

    /** List the machines a space gave work, in identity order. */
    async assigned(space: string): Promise<readonly string[]> {
        const rows = await this.database
            .select({ machine: assignmentTable.machine })
            .from(assignmentTable)
            .where(eq(assignmentTable.space, space))
            .orderBy(assignmentTable.machine);

        return rows.map((row) => row.machine);
    }

    /** Mark a space as moving to a target machine. */
    async move(placement: Placement, target: string): Promise<void> {
        const moving = await this.database
            .update(placementTable)
            .set({ target })
            .where(this.#serving(placement))
            .returning({ id: placementTable.id });
        if (moving.length === 0) {
            throw new ServiceError("CONFLICT", {
                message: `${placement.id} is no longer placed on ${placement.machine} at epoch ${placement.epoch}`,
            });
        }
    }

    /** Record the URL a machine answers at, replacing the one it published before. */
    async publish(machine: string, scope: string, url: string): Promise<void> {
        const now = this.#clock();
        await this.database
            .insert(endpointTable)
            .values({ machine, scope, url, publishedAt: now })
            .onConflictDoUpdate({
                target: endpointTable.machine,
                set: { scope, url, publishedAt: now },
            });
    }

    /** Read the URL a machine answers at, absent before it published one. */
    endpoint(machine: string): Promise<Endpoint | undefined> {
        return this.#read(this.#endpoints, machine, async () => {
            const [found] = await this.database
                .select({
                    machine: endpointTable.machine,
                    scope: endpointTable.scope,
                    url: endpointTable.url,
                })
                .from(endpointTable)
                .where(eq(endpointTable.machine, machine));

            return found;
        });
    }

    // identities

    /** Apply a signed identity operation: a space's first only from the machine serving it, the universe's first from the process hosting the directory. */
    async apply(operation: string, placement?: Placement): Promise<void> {
        // read the operation and its identity's log
        const claims = IdentityOperation.read(operation);
        const log = await this.database
            .select()
            .from(identityOperation)
            .where(eq(identityOperation.subject, claims.subject))
            .orderBy(identityOperation.sequence);

        // start the identity or follow a current operation of it
        if (log.length === 0) {
            await this.#start(operation, claims, placement);
        } else {
            await this.#follow(operation, claims, log);
        }
    }

    /** Start an identity signed by one of the operation's rotation keys: a space's from the machine serving it, the universe's from the process hosting the directory. */
    async #start(
        operation: string,
        claims: IdentityOperation,
        placement?: Placement,
    ): Promise<void> {
        // require the universe's first operation or the machine serving the space
        const isUniverse =
            claims.subject === Scope.universe.id &&
            claims.previous === null &&
            placement === undefined;
        const isServed =
            isUniverse ||
            (placement !== undefined &&
                placement.id === claims.subject &&
                claims.previous === null &&
                (
                    await this.database
                        .select({ id: placementTable.id })
                        .from(placementTable)
                        .where(this.#serving(placement))
                ).length > 0);
        if (!isServed) {
            throw new ServiceError("FORBIDDEN", {
                message: `only the machine serving ${claims.subject} starts its identity`,
            });
        }

        // append it first in the log
        await this.#append(operation, claims.subject, 0, await this.#priority(operation, claims));
    }

    /** Follow a current operation of an identity, nullifying later ones its higher-priority key may recover. */
    async #follow(
        operation: string,
        claims: IdentityOperation,
        log: readonly Select<typeof identityOperation>[],
    ): Promise<void> {
        // find the current operation it follows
        const current = log.filter((entry) => !entry.isNullified);
        const previous = current.find((entry) => entry.digest === claims.previous);
        if (previous === undefined) {
            throw new ServiceError("CONFLICT", {
                message: `the operation follows no current operation of ${claims.subject}`,
            });
        }

        // require a rotation key of the identity it follows
        const priority = await this.#priority(
            operation,
            IdentityOperation.read(previous.operation),
        );
        const following = current.find((entry) => entry.sequence > previous.sequence);
        const isRecoverable =
            following === undefined ||
            (priority < following.priority &&
                following.appliedAt + RECOVERY_MILLISECONDS >= this.#clock());
        if (!isRecoverable) {
            throw new ServiceError("CONFLICT", {
                message: `the operation cannot nullify the later operations of ${claims.subject}`,
            });
        }

        // nullify the operations after the one it follows and append it
        await this.database.transaction(async (transaction) => {
            await transaction
                .update(identityOperation)
                .set({ isNullified: true })
                .where(
                    and(
                        eq(identityOperation.subject, claims.subject),
                        gt(identityOperation.sequence, previous.sequence),
                    ),
                );
            await this.#append(operation, claims.subject, log.length, priority, transaction);
        });
    }

    /** Read a space's or the universe's current identity, absent before its first operation. */
    async identity(subject: string): Promise<Identity | undefined> {
        return this.#read(this.#identities, subject, async () => {
            // read the latest operation in force
            const [latest] = await this.database
                .select({
                    operation: identityOperation.operation,
                    digest: identityOperation.digest,
                })
                .from(identityOperation)
                .where(
                    and(
                        eq(identityOperation.subject, subject),
                        eq(identityOperation.isNullified, false),
                    ),
                )
                .orderBy(desc(identityOperation.sequence))
                .limit(1);
            if (latest === undefined) {
                return undefined;
            }
            const { signingKeys, rotationKeys } = IdentityOperation.read(latest.operation);

            return { signingKeys, rotationKeys, digest: latest.digest };
        });
    }

    /** List an identity's signed operations in order, nullified ones included. */
    async operations(subject: string): Promise<readonly string[]> {
        const log = await this.database
            .select({ operation: identityOperation.operation })
            .from(identityOperation)
            .where(eq(identityOperation.subject, subject))
            .orderBy(identityOperation.sequence);

        return log.map((entry) => entry.operation);
    }

    // claims

    /** Reserve a request's claims in chunks until its write commits, returning the names other objects hold. */
    async claim(claims: readonly Claim[], requestId: string): Promise<readonly Claim[]> {
        // reserve each chunk for a minute
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
        // read the owners of the object's names a chunk at a time
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

    /** Refuse a space its machine no longer serves at the epoch given. */
    async #requireServing(placement: Placement): Promise<void> {
        const placed = await this.locate(placement.id);
        if (placed?.machine !== placement.machine || placed.epoch !== placement.epoch) {
            throw new ServiceError("CONFLICT", {
                message: `${placement.id} is no longer placed on ${placement.machine} at epoch ${placement.epoch}`,
            });
        }
    }

    /** Match a placement as its machine serves it at its epoch. */
    #serving(placement: Placement) {
        return and(
            eq(placementTable.id, placement.id),
            eq(placementTable.machine, placement.machine),
            eq(placementTable.epoch, placement.epoch),
        );
    }

    /** Find the priority of the rotation key of an identity that signed an operation, refusing one none signed. */
    async #priority(operation: string, identity: Pick<Identity, "rotationKeys">): Promise<number> {
        const priority = await IdentityOperation.verify(operation, identity.rotationKeys);
        if (priority === undefined) {
            throw new ServiceError("UNAUTHORIZED", {
                message: "no rotation key of the identity signed the operation",
            });
        }

        return priority;
    }

    /** Append a verified operation to an identity's log. */
    async #append(
        operation: string,
        subject: string,
        sequence: number,
        priority: number,
        database: DatabaseConnection = this.database,
    ): Promise<void> {
        // append the operation
        await database.insert(identityOperation).values({
            subject,
            sequence,
            scope: Scope.universe.id,
            digest: await IdentityOperation.digest(operation),
            operation,
            priority,
            appliedAt: this.#clock(),
            isNullified: false,
        });
    }

    /** Reserve a chunk of claims in one insert, returning the names other objects hold. */
    async #reserve(
        claims: readonly Claim[],
        requestId: string,
        expiresAt: number,
    ): Promise<readonly Claim[]> {
        // insert the free names
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
        this.#placements.clear();
        this.#endpoints.clear();
        this.#identities.clear();
    }

    /** Forget the reads one change affects, before and after it. */
    #forget(change: Change<(typeof CACHED_TABLES)[number]>): void {
        // forget a claim by its name
        if (Change.of(change, claimTable)) {
            for (const image of imagesOf(change)) {
                this.#owners.forget(nameKey({ index: image.index, key: image.key }));
            }
        }
        // forget an identity by its subject
        else if (Change.of(change, identityOperation)) {
            for (const image of imagesOf(change)) {
                this.#identities.forget(image.subject);
            }
        }
        // forget a placement by its space
        else if (Change.of(change, placementTable)) {
            for (const image of imagesOf(change)) {
                this.#placements.forget(image.id);
            }
        }
        // forget an endpoint by its machine
        else if (Change.of(change, endpointTable)) {
            for (const image of imagesOf(change)) {
                this.#endpoints.forget(image.machine);
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

/** List a change's row images before and after it, leaving out an absent one. */
function imagesOf<Definition extends Table>(change: Change<Definition>) {
    return [Change.before(change), Change.after(change)].filter((image) => image !== null);
}
