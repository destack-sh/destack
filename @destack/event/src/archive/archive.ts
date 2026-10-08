import type { Bucket } from "@destack/bucket";
import {
    and,
    asc,
    count,
    desc,
    eq,
    gte,
    inArray,
    lt,
    lte,
    min,
    type DatabaseConnection,
    type Select,
} from "@destack/db";
import { present } from "@destack/schema";
import type { AsyncBuffer } from "hyparquet";
import { eventSegment } from "./catalog.ts";
import { type Event, type EventPolicy, type EventKind, EventTime } from "../kind/kind.ts";
import { EventCursor, EventFilter, type EventOrder } from "../query/query.ts";
import { SegmentFile } from "./segment.ts";

/** The events one pruning statement deletes, far below every dialect's bound on parameters. */
const PRUNE_BATCH = 500;

/** The share of a segment's rows below which an unlocked segment is compacted with its neighbours. */
const SMALL_SHARE = 0.5;

/** The failure to read a segment file a compaction deleted after a read chose it. */
class DeletedSegment extends Error {
    /** Name the deleted file. */
    constructor(key: string) {
        super(`segment file ${key} was deleted`);
    }
}

/** Where a store keeps segments and how it reads each scope's policy. */
export interface EventArchiveOptions {
    /** The database of the hot events and the catalog. */
    readonly database: DatabaseConnection;
    /** The bucket keeping a scope's segments. */
    readonly files: (scope: string) => Bucket;
    /** The bucket key prefix of the segments. */
    readonly prefix: string;
    /** Read the policy of a kind's events in a scope. */
    readonly policy: (kind: EventKind, scope: string) => Promise<EventPolicy>;
}

/** The segments of a store's scopes, written, compacted, expired and read back. */
export class EventArchive {
    /** The database of the hot events and the catalog. */
    readonly #database: DatabaseConnection;
    /** The bucket of each scope's segments. */
    readonly #files: (scope: string) => Bucket;
    /** The bucket key prefix. */
    readonly #prefix: string;
    /** The policy of each kind and scope. */
    readonly #policy: (kind: EventKind, scope: string) => Promise<EventPolicy>;

    /** Keep segments as the options say. */
    constructor(options: EventArchiveOptions) {
        // hold the database, buckets, prefix and policy
        this.#database = options.database;
        this.#files = options.files;
        this.#prefix = options.prefix;
        this.#policy = options.policy;
    }

    /** List the scopes with hot events or segments of a kind. */
    async scopes(kind: EventKind): Promise<string[]> {
        const [hot, flushed] = await Promise.all([
            this.#database.selectDistinct({ scope: kind.table.scope }).from(kind.table),
            this.#database
                .selectDistinct({ scope: eventSegment.scope })
                .from(eventSegment)
                .where(eq(eventSegment.kind, kind.key)),
        ]);

        return [...new Set([...hot, ...flushed].map((row) => row.scope))];
    }

    /** Read whether and when a scope's hot events of a kind are due to flush. */
    async flushDue(
        kind: EventKind,
        scope: string,
        now: number,
    ): Promise<{ readonly isDue: boolean; readonly at: number | undefined }> {
        // read the hot events' count and oldest time
        const [held] = await this.#database
            .select({ events: count(), oldest: min(kind.table.time) })
            .from(kind.table)
            .where(eq(kind.table.scope, scope));
        const oldest = held?.oldest ?? null;
        if (held === undefined || oldest === null) {
            return { isDue: false, at: undefined };
        }

        // flush at the age or the count the scope's policy sets
        const { flush } = await this.#policy(kind, scope);
        const at = EventTime.milliseconds(oldest) + flush.maxAge;

        return { isDue: held.events >= flush.maxRows || at <= now, at };
    }

    /** Flush a scope's oldest hot events into a verified, catalogued segment. */
    async flush(kind: EventKind, scope: string, now: number): Promise<boolean> {
        // read the oldest hot events, at most a segment's worth
        const policy = await this.#policy(kind, scope);
        const rows = await this.#database
            .select()
            .from(kind.table)
            .where(eq(kind.table.scope, scope))
            .orderBy(asc(kind.table.time), asc(kind.table.id))
            .limit(policy.flush.maxRows);
        const events = rows.map((row) => kind.event(row));
        if (events.length === 0) {
            return false;
        }

        // write the segment, chained to the scope's previous one when the kind is locked
        const previous = kind.isLocked ? await this.#latestDigest(kind, scope) : null;
        await this.#write(kind, scope, events, now, policy, previous, async (transaction) => {
            // prune exactly the flushed events by source, never a later one with an earlier time
            for (const [source, held] of Map.groupBy(events, (event) => event.source)) {
                for (let start = 0; start < held.length; start += PRUNE_BATCH) {
                    const batch = held.slice(start, start + PRUNE_BATCH).map((event) => event.id);
                    await transaction
                        .delete(kind.table)
                        .where(
                            and(
                                eq(kind.table.scope, scope),
                                eq(kind.table.source, source),
                                inArray(kind.table.id, batch),
                            ),
                        );
                }
            }
        });

        return true;
    }

    /** Compact a scope's small segments of an unlocked kind into fuller ones. */
    async compact(kind: EventKind, scope: string, now: number): Promise<number> {
        // leave locked kinds, whose chained segments stay as written
        if (kind.isLocked) {
            return 0;
        }

        // gather consecutive small segments up to a segment's worth of rows
        const policy = await this.#policy(kind, scope);
        const segments = await this.#catalog(kind, scope);
        const small = segments.filter(
            (segment) => segment.rows < policy.flush.maxRows * SMALL_SHARE,
        );
        const group: typeof small = [];
        let rows = 0;
        for (const segment of small) {
            if (rows + segment.rows > policy.flush.maxRows) {
                break;
            }
            group.push(segment);
            rows += segment.rows;
        }
        if (group.length < 2) {
            return 0;
        }

        // write their events as one segment, replacing their catalog rows together
        const read = await Promise.all(
            group.map((segment) =>
                SegmentFile.decode(kind, this.#file(scope, segment.file, segment.bytes), { scope }),
            ),
        );
        const events = read
            .flat()
            .toSorted((left, right) => left.time - right.time || (left.id < right.id ? -1 : 1));
        await this.#write(kind, scope, events, now, policy, null, async (transaction) => {
            await transaction.delete(eventSegment).where(
                inArray(
                    eventSegment.id,
                    group.map((segment) => segment.id),
                ),
            );
        });
        await this.#files(scope).delete(group.map((segment) => segment.file));

        return group.length;
    }

    /** Delete a scope's segments of a kind past its retention, their files first. */
    async expire(kind: EventKind, scope: string, now: number): Promise<number> {
        // read the segments whose latest event is older than the scope's retention
        const { retention } = await this.#policy(kind, scope);
        const before = EventTime.of(now - retention);
        const expired = await this.#database
            .select({ id: eventSegment.id, file: eventSegment.file })
            .from(eventSegment)
            .where(
                and(
                    eq(eventSegment.kind, kind.key),
                    eq(eventSegment.scope, scope),
                    lt(eventSegment.to, before),
                ),
            );
        if (expired.length === 0) {
            return 0;
        }

        // delete the files, then their catalog rows
        await this.#files(scope).delete(expired.map((segment) => segment.file));
        await this.#database.delete(eventSegment).where(
            inArray(
                eventSegment.id,
                expired.map((segment) => segment.id),
            ),
        );

        return expired.length;
    }

    /** Choose the catalogued segments that may hold the events a read selects. */
    async choose(
        kind: EventKind,
        filter: EventFilter,
        after: EventCursor | undefined,
        order: EventOrder = "ascending",
        connection: DatabaseConnection = this.#database,
    ): Promise<Select<typeof eventSegment>[]> {
        const segments = await connection
            .select()
            .from(eventSegment)
            .where(
                and(
                    eq(eventSegment.kind, kind.key),
                    eq(eventSegment.scope, filter.scope),
                    filter.before === undefined ? undefined : lt(eventSegment.from, filter.before),
                    gte(
                        eventSegment.to,
                        Math.max(filter.from ?? 0, order === "ascending" ? (after?.time ?? 0) : 0),
                    ),
                    order === "descending" && after !== undefined
                        ? lte(eventSegment.from, after.time)
                        : undefined,
                ),
            )
            .orderBy(asc(eventSegment.from), asc(eventSegment.id));

        return segments.filter((segment) => SegmentFile.mayHold(kind, segment.keys, filter));
    }

    /** Decode the events a read selects from chosen segments, absent once one was compacted away. */
    async decode(
        kind: EventKind,
        segments: readonly Select<typeof eventSegment>[],
        filter: EventFilter,
        after: EventCursor | undefined,
        order: EventOrder = "ascending",
    ): Promise<Event[] | undefined> {
        // decode each segment, noticing a file deleted since it was chosen
        const decoded = await Promise.all(
            segments.map((segment) =>
                SegmentFile.decode(
                    kind,
                    this.#file(filter.scope, segment.file, segment.bytes),
                    filter,
                ).catch((error: unknown) => {
                    if (error instanceof DeletedSegment) {
                        return undefined;
                    }
                    throw error;
                }),
            ),
        );
        if (decoded.some((events) => events === undefined)) {
            return undefined;
        }

        // keep the events past the cursor the filter selects
        const match = EventFilter.match(kind, filter);

        return decoded
            .flatMap((events) => events ?? [])
            .filter((event) => EventCursor.isPast(event, after, order) && match(event));
    }

    /** Write events as a content-named segment and catalog it with the caller's change. */
    async #write(
        kind: EventKind,
        scope: string,
        events: readonly Event[],
        now: number,
        policy: EventPolicy,
        previous: string | null,
        change: (transaction: DatabaseConnection) => Promise<void>,
    ): Promise<void> {
        // write and verify the file, locked for the retention when the kind is locked
        const first = present(events.at(0), "a segment's first event");
        const last = present(events.at(-1), "a segment's last event");
        const data = await SegmentFile.encode(kind, events);
        const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", data)).toHex();
        const id = `${String(first.time)}-${String(last.time)}-${digest.slice(0, 16)}`;
        const file = `${this.#prefix}/${kind.package.id}/${kind.name}/${scope}/${id}.parquet`;
        await this.#files(scope).put(file, data, {
            sha256: digest,
            ...(kind.isLocked ? { retainUntil: new Date(now + policy.retention) } : {}),
        });

        // catalog it with the caller's change
        await this.#database.transaction(async (transaction) => {
            await transaction
                .insert(eventSegment)
                .values({
                    id,
                    kind: kind.key,
                    scope,
                    from: first.time,
                    to: last.time,
                    rows: events.length,
                    bytes: data.byteLength,
                    keys: SegmentFile.keyIndex(kind, events),
                    file,
                    digest,
                    previous,
                    flushedAt: now,
                })
                .onConflictDoNothing();
            await change(transaction);
        });
    }

    /** Read the digest of a scope's latest segment of a kind, absent before its first. */
    async #latestDigest(kind: EventKind, scope: string): Promise<string | null> {
        const [latest] = await this.#database
            .select({ digest: eventSegment.digest })
            .from(eventSegment)
            .where(and(eq(eventSegment.kind, kind.key), eq(eventSegment.scope, scope)))
            .orderBy(desc(eventSegment.flushedAt), desc(eventSegment.to))
            .limit(1);

        return latest?.digest ?? null;
    }

    /** Read a scope's segments of a kind in time order. */
    async #catalog(kind: EventKind, scope: string) {
        return this.#database
            .select()
            .from(eventSegment)
            .where(and(eq(eventSegment.kind, kind.key), eq(eventSegment.scope, scope)))
            .orderBy(asc(eventSegment.from));
    }

    /** Read a segment file through ranged reads of its scope's bucket. */
    #file(scope: string, key: string, byteLength: number): AsyncBuffer {
        const bucket = this.#files(scope);

        return {
            byteLength,
            slice: async (start, end = byteLength) => {
                const data = await bucket.get(key, {
                    range: { offset: start, length: end - start },
                });
                if (data === null) {
                    throw new DeletedSegment(key);
                }

                return data.arrayBuffer();
            },
        };
    }
}
