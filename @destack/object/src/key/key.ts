import {
    and,
    eq,
    gt,
    inArray,
    lte,
    min,
    ne,
    or,
    type Column,
    type DatabaseConnection,
    type Table,
} from "@destack/db";
import { CHAIN_TERMS } from "@destack/db/query";
import { keyEntry } from "./table.ts";
import type { Change } from "@destack/db/log";
import { canonicalize } from "@destack/schema/json";
import { ServiceError } from "@destack/service/error";
import { GLOBAL_SCOPE, type ObjectReference } from "@destack/access";
import type { ObjectType } from "../object/object.ts";
import type { Controller } from "@destack/service/control";
import type { ObjectServer } from "../server/server.ts";

/** How long a reservation waits for its write to commit, in milliseconds: a minute, above any write. */
const RESERVATION_MILLISECONDS = 60_000;

/** One key an object holds in a declared unique index. */
export interface IndexKey {
    /** The index: the object type's package, type and index name. */
    readonly index: string;
    /** The indexed values, with the scope they are unique within, as canonical JSON. */
    readonly key: string;
    /** The object holding the key. */
    readonly objectId: string;
    /** The scope the object lives in. */
    readonly scope: string;
}

/** The keys one indexed object holds after a write. */
export interface KeyHolding {
    /** The indexes the object's type declares. */
    readonly indexes: readonly string[];
    /** The object's identity. */
    readonly objectId: string;
    /** The keys the object holds. */
    readonly keys: readonly IndexKey[];
}

/** The keys a request's write reserved. */
export interface Reservation {
    /** The request whose write reserved the keys. */
    readonly requestId: string;
    /** The keys each written object holds. */
    readonly holdings: readonly KeyHolding[];
}

/** The object holding a key. */
export interface KeyHolder {
    /** The object's identity. */
    readonly objectId: string;
    /** The scope the object lives in. */
    readonly scope: string;
}

/** The expired reservations and the next deadline. */
export interface Expiry {
    /** The expired reservations. */
    readonly keys: readonly IndexKey[];
    /** When the next reservation expires, in UTC epoch milliseconds. */
    readonly next?: number;
}

/** The storage of the key index's entries. */
export interface KeyIndexBacking {
    /** Reserve keys for a request, refusing a key another object holds. */
    reserve(keys: readonly IndexKey[], requestId: string, now: number): Promise<void>;
    /** Confirm a request's reserved keys and release dropped keys. */
    confirm(requestId: string, holdings: readonly KeyHolding[]): Promise<void>;
    /** Release the reservations of a request whose write failed. */
    release(requestId: string): Promise<void>;
    /** Hold an object's keys after an unreserved write and release the rest. */
    hold(holding: KeyHolding, requestId: string, now: number): Promise<void>;
    /** List the expired reservations of some indexes, and the next deadline. */
    expired(indexes: readonly string[], now: number): Promise<Expiry>;
    /** Find the object holding a confirmed key. */
    resolve(index: string, key: string): Promise<KeyHolder | undefined>;
}

/** The global key index of declared unique indexes across databases. */
export class KeyIndex {
    /** Where the entries live. */
    readonly backing: KeyIndexBacking;

    /** Keep the entries in the global database or a backing. */
    constructor(backing: DatabaseConnection | KeyIndexBacking) {
        this.backing = isBacking(backing) ? backing : new KeyIndexDatabase(backing);
    }

    /** The global database holding the entries, absent behind a service. */
    get database(): DatabaseConnection | undefined {
        return this.backing instanceof KeyIndexDatabase ? this.backing.database : undefined;
    }

    /** Key some values in one of an object type's indexes. */
    static key(
        object: ObjectType,
        name: string,
        values: readonly unknown[],
        scope?: string,
    ): Pick<IndexKey, "index" | "key"> {
        // require the index and a needed scope
        const declared = object.indexes[name];
        if (declared === undefined) {
            throw new TypeError(`object ${object.name} has no index ${name}`);
        }
        const within = declared.across === GLOBAL_SCOPE ? null : scope;
        if (within === undefined) {
            throw new TypeError(`index ${name} of ${object.name} keys values within a scope`);
        }

        return { index: indexOf(object, name), key: canonicalize([within, ...values]) };
    }

    /** List the keys an object's row holds. */
    static keys(object: ObjectType, row: Readonly<Record<string, unknown>>): IndexKey[] {
        return Object.entries(object.indexes).flatMap(([name, declared]) => {
            // skip rows missing a value
            const values = declared.on.map((field) => row[field]);
            if (values.some((value) => value === null || value === undefined)) {
                return [];
            }

            // key the values within their scope
            const within = declared.across === GLOBAL_SCOPE ? null : String(row.scope);

            return [
                {
                    index: indexOf(object, name),
                    key: canonicalize([within, ...values]),
                    objectId: String(row.id),
                    scope: String(row.scope),
                },
            ];
        });
    }

    /** Describe the keys an object holds after a write. */
    static holding(
        object: ObjectType,
        objectId: string,
        row: Readonly<Record<string, unknown>> | undefined,
    ): KeyHolding {
        return {
            indexes: Object.keys(object.indexes).map((name) => indexOf(object, name)),
            objectId,
            keys: row === undefined ? [] : KeyIndex.keys(object, row),
        };
    }

    /** Reserve the keys of indexed objects an open transaction wrote. */
    async reserve(
        transaction: DatabaseConnection,
        objects: readonly ObjectType[],
        requestId: string,
        now: number,
    ): Promise<Reservation> {
        // read each written row's last image
        const indexed = objects.filter((object) => Object.keys(object.indexes).length > 0);
        const byTable = new Map(indexed.map((object) => [object.table as Table, object]));
        const images = new Map<string, KeyHolding>();
        if (indexed.length > 0) {
            for (const change of await transaction.log.written([...byTable.keys()])) {
                const object = byTable.get(change.table)!;
                const objectId = String(change.key.id);
                const row = change.after as Readonly<Record<string, unknown>> | undefined;
                images.set(`${object.name}/${objectId}`, KeyIndex.holding(object, objectId, row));
            }
        }

        // reserve their keys
        const holdings = [...images.values()];
        const keys = holdings.flatMap((holding) => holding.keys);
        if (keys.length > 0) {
            await this.backing.reserve(keys, requestId, now);
        }

        return { requestId, holdings };
    }

    /** Confirm a committed reservation. */
    async confirm(reservation: Reservation): Promise<void> {
        await this.backing.confirm(reservation.requestId, reservation.holdings);
    }

    /** Release the reservations of a request whose write failed. */
    async release(requestId: string): Promise<void> {
        await this.backing.release(requestId);
    }

    /** Keep an object type's keys in step with one change of its rows. */
    async apply(object: ObjectType, change: Change): Promise<void> {
        const row = change.after as Readonly<Record<string, unknown>> | undefined;
        const holding = KeyIndex.holding(object, String(change.key.id), row);
        await this.backing.hold(holding, `change-${change.sequence}`, change.changedAt);
    }

    /** Find the object holding values in a type's index. */
    async resolve(
        object: ObjectType,
        name: string,
        values: readonly unknown[],
        scope?: string,
    ): Promise<ObjectReference | undefined> {
        // find the confirmed holder
        const { index, key } = KeyIndex.key(object, name, values, scope);
        const found = await this.backing.resolve(index, key);

        return found === undefined ? undefined : object.reference(found.scope, found.objectId);
    }

    /** Finish expired reservations, returning the delay until the next. */
    async finish(
        objects: readonly ObjectType[],
        database: DatabaseConnection,
        now: number,
    ): Promise<number | undefined> {
        // list expired reservations
        const byIndex = new Map(
            objects.flatMap((object) =>
                Object.keys(object.indexes).map((name) => [indexOf(object, name), object] as const),
            ),
        );
        const expiry = await this.backing.expired([...byIndex.keys()], now);

        // finish one chain's worth per run
        const due = expiry.keys.slice(0, CHAIN_TERMS);
        const byObject = Map.groupBy(due, (entry) => byIndex.get(entry.index)!);
        for (const [object, entries] of byObject) {
            // read present rows
            const table = object.table as Table & Record<string, Column>;
            const ids = [...new Set(entries.map((entry) => entry.objectId))];
            const rows = (await database
                .select()
                .from(table)
                .where(inArray(table.id!, ids))) as Record<string, unknown>[];
            const present = new Map(rows.map((row) => [row.id as string, row]));

            // hold each object's current keys
            for (const id of ids) {
                const holding = KeyIndex.holding(object, id, present.get(id));
                await this.backing.hold(holding, `finish-${id}`, now);
            }
        }

        // schedule the next run
        if (expiry.keys.length > due.length) {
            return 0;
        }

        return expiry.next === undefined ? undefined : Math.max(0, expiry.next - now);
    }

    /** Finish the expired reservations of a server's indexed objects. */
    controller(server: Pick<ObjectServer, "database" | "objects">): Controller {
        // watch indexed objects
        const indexed = server.objects.filter((object) => Object.keys(object.indexes).length > 0);

        return {
            name: "keys",
            watches: indexed.map((object) => object.table as Table),
            keys: () => ["reservations"],
            list: async () => ["reservations"],
            reconcile: () => this.finish(indexed, server.database, Date.now()),
        };
    }
}

/** The key index's entries in the global database. */
export class KeyIndexDatabase implements KeyIndexBacking {
    /** The global database holding the entries. */
    readonly database: DatabaseConnection;

    /** Keep the entries in the global database. */
    constructor(database: DatabaseConnection) {
        this.database = database;
    }

    /** Reserve a request's keys in chunks. */
    async reserve(keys: readonly IndexKey[], requestId: string, now: number): Promise<void> {
        for (let start = 0; start < keys.length; start += CHAIN_TERMS) {
            await this.#reserve(keys.slice(start, start + CHAIN_TERMS), requestId, now);
        }
    }

    /** Confirm a request's reserved keys and release dropped keys. */
    async confirm(requestId: string, holdings: readonly KeyHolding[]): Promise<void> {
        await this.database
            .update(keyEntry)
            .set({ state: "confirmed" })
            .where(eq(keyEntry.requestId, requestId));
        await this.#release(holdings);
    }

    /** Release the reservations of a request whose write failed. */
    async release(requestId: string): Promise<void> {
        await this.database
            .delete(keyEntry)
            .where(and(eq(keyEntry.requestId, requestId), eq(keyEntry.state, "reserved")));
    }

    /** Hold an object's keys after an unreserved write and release the rest. */
    async hold(holding: KeyHolding, requestId: string, now: number): Promise<void> {
        // release every key of an object holding none
        const { keys } = holding;
        if (keys.length === 0) {
            await this.#release([holding]);

            return;
        }

        // read the holders of the object's keys
        const held = await this.database
            .select({
                index: keyEntry.index,
                key: keyEntry.key,
                objectId: keyEntry.objectId,
            })
            .from(keyEntry)
            .where(or(...keys.map((entry) => matches(entry))));

        // refuse a key another object holds
        for (const entry of keys) {
            const holder = held.find((row) => row.index === entry.index && row.key === entry.key);
            if (holder !== undefined && holder.objectId !== entry.objectId) {
                throw new ServiceError("CONFLICT", {
                    message: `${entry.index.split("/").at(-1)} of ${entry.index.split("/").at(-2)} ${entry.objectId} is taken by ${holder.objectId}`,
                });
            }
        }

        // confirm, insert and release
        const fresh = keys.filter(
            (entry) => !held.some((row) => row.index === entry.index && row.key === entry.key),
        );
        if (held.length > 0) {
            await this.database
                .update(keyEntry)
                .set({ state: "confirmed" })
                .where(or(...held.map((entry) => matches(entry))));
        }
        if (fresh.length > 0) {
            await this.database.insert(keyEntry).values(
                fresh.map((entry) => ({
                    ...entry,
                    state: "confirmed" as const,
                    requestId,
                    expiresAt: now,
                })),
            );
        }
        await this.#release([holding]);
    }

    /** Find the object holding a confirmed key. */
    async resolve(index: string, key: string): Promise<KeyHolder | undefined> {
        const [found] = await this.database
            .select({ objectId: keyEntry.objectId, scope: keyEntry.scope })
            .from(keyEntry)
            .where(
                and(
                    eq(keyEntry.index, index),
                    eq(keyEntry.key, key),
                    eq(keyEntry.state, "confirmed"),
                ),
            );

        return found;
    }

    /** List the expired reservations of some indexes, and the next deadline. */
    async expired(indexes: readonly string[], now: number): Promise<Expiry> {
        // read the expired reservations of the indexes
        const reserved = and(eq(keyEntry.state, "reserved"), inArray(keyEntry.index, [...indexes]));
        const keys = await this.database
            .select({
                index: keyEntry.index,
                key: keyEntry.key,
                objectId: keyEntry.objectId,
                scope: keyEntry.scope,
            })
            .from(keyEntry)
            .where(and(reserved, lte(keyEntry.expiresAt, now)));

        // find the next deadline
        const [next] = await this.database
            .select({ expiresAt: min(keyEntry.expiresAt) })
            .from(keyEntry)
            .where(and(reserved, gt(keyEntry.expiresAt, now)));

        return next?.expiresAt == null ? { keys } : { keys, next: next.expiresAt };
    }

    /** Reserve a chunk of keys in one insert. */
    async #reserve(keys: readonly IndexKey[], requestId: string, now: number): Promise<void> {
        // insert, skipping taken keys
        const expiresAt = now + RESERVATION_MILLISECONDS;
        const inserted = await this.database
            .insert(keyEntry)
            .values(
                keys.map((entry) => ({
                    ...entry,
                    state: "reserved" as const,
                    requestId,
                    expiresAt,
                })),
            )
            .onConflictDoNothing()
            .returning({ index: keyEntry.index, key: keyEntry.key });
        const reserved = new Set(inserted.map((entry) => JSON.stringify([entry.index, entry.key])));
        const taken = keys.filter(
            (entry) => !reserved.has(JSON.stringify([entry.index, entry.key])),
        );
        if (taken.length === 0) {
            return;
        }

        // refuse a key another object holds
        const held = await this.database
            .select({
                index: keyEntry.index,
                key: keyEntry.key,
                objectId: keyEntry.objectId,
            })
            .from(keyEntry)
            .where(or(...taken.map((entry) => matches(entry))));
        for (const entry of taken) {
            const holder = held.find((row) => row.index === entry.index && row.key === entry.key);
            if (holder?.objectId !== entry.objectId) {
                throw new ServiceError("CONFLICT", {
                    message: `${entry.index.split("/").at(-1)} is taken`,
                });
            }
        }
    }

    /** Release the keys objects no longer hold. */
    async #release(holdings: readonly KeyHolding[]): Promise<void> {
        for (let start = 0; start < holdings.length; start += CHAIN_TERMS) {
            const chunk = holdings.slice(start, start + CHAIN_TERMS);
            await this.database
                .delete(keyEntry)
                .where(
                    or(
                        ...chunk.map(({ indexes, objectId, keys }) =>
                            and(
                                inArray(keyEntry.index, [...indexes]),
                                eq(keyEntry.objectId, objectId),
                                ...keys.map((entry) =>
                                    or(
                                        ne(keyEntry.index, entry.index),
                                        ne(keyEntry.key, entry.key),
                                    ),
                                ),
                            ),
                        ),
                    ),
                );
        }
    }
}

/** Name an object type's index by package, type and index name. */
function indexOf(object: ObjectType, name: string): string {
    return `${object.policy.definition.packageId}/${object.name}/${name}`;
}

/** Match the entry of one key. */
function matches(entry: { readonly index: string; readonly key: string }) {
    return and(eq(keyEntry.index, entry.index), eq(keyEntry.key, entry.key));
}

/** Decide whether a value is a backing. */
function isBacking(value: DatabaseConnection | KeyIndexBacking): value is KeyIndexBacking {
    return "reserve" in value;
}
