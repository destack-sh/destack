import {
    and,
    asc,
    Change,
    Condition,
    count,
    eq,
    gt,
    gte,
    lt,
    max,
    min,
    or,
    sql,
    sum,
    TABLE,
    type DatabaseConnection,
    type SQL,
    type SQLWrapper,
} from "@destack/db";
import { type JsonValue } from "@destack/schema";
import type { Bucket } from "@destack/bucket";
import type { Controller } from "@destack/service/control";
import type { Outbox } from "@destack/service/outbox";
import { ArchiveController } from "../archive/controller.ts";
import { EventArchive } from "../archive/archive.ts";
import type { Event, EventKeyShape, EventKind, EventPolicy, Route } from "../kind/kind.ts";
import { type PersonalKeyring, PersonalSeal } from "../personal/personal.ts";
import { EventCursor, EventFilter } from "../query/query.ts";
import { routeAddress, routeDestination } from "../route/route.ts";
import { type Series, SeriesFold, type SeriesRequest, type StepFold } from "./series.ts";

/** The bucket key prefix segments are written under unless a host names another. */
const SEGMENT_PREFIX = "events";

/** The events one query page holds unless it asks for another number. */
const PAGE_EVENTS = 100;

/** The reads a query retries while flushes and compactions change the catalog under it. */
const SNAPSHOT_ATTEMPTS = 5;

/** An event as it is appended: everything but the source, which is the scope that appends it. */
export type EventInput<Shape extends EventKeyShape, Data extends JsonValue> = Omit<
    Event<Shape, Data>,
    "source"
>;

/** A page of events in time order and the cursor after its last one. */
export interface EventPage<Shape extends EventKeyShape, Data extends JsonValue> {
    /** The events. */
    readonly events: readonly Event<Shape, Data>[];
    /** The cursor after the last event, absent once no event follows. */
    readonly cursor: EventCursor | undefined;
}

/** What a host keeps events of some kinds with. */
export interface EventStoreOptions {
    /** The database keeping the kinds' hot events and the segment catalog. */
    readonly database: DatabaseConnection;
    /** The kinds kept. */
    readonly kinds: readonly EventKind[];
    /** The bucket keeping a scope's segments. */
    readonly files: (scope: string) => Bucket;
    /** The bucket key prefix of the segments. */
    readonly prefix?: string;
    /** The keyring sealing personal values, needed by kinds with a subject. */
    readonly personal?: PersonalKeyring;
    /** The outbox routing copies, on the database of the hot events, needed by kinds that route. */
    readonly outbox?: Outbox;
    /** Name the scopes a route copies a scope's events to, needed by kinds that route. */
    readonly targets?: (route: Route, scope: string) => Promise<readonly string[]>;
    /** Deliver routed copies to the hosts keeping their scopes, needed by kinds that route. */
    readonly deliver?: (
        kind: string,
        events: readonly Event[],
        signal: AbortSignal,
    ) => Promise<void>;
    /** Read a scope's policy for a kind, the kind's own when absent. */
    readonly policy?: (kind: EventKind, scope: string) => Promise<EventPolicy | undefined>;
    /** Read the current time in Unix milliseconds. */
    readonly now?: () => number;
    /** Report the failed write of events kept at most once, which were dropped. */
    readonly report?: (error: unknown) => void;
}

/** The append-only events of the scopes a host keeps: hot rows in its database, segments in their scopes' buckets. */
export class EventStore {
    /** The database of the hot events. */
    readonly database: DatabaseConnection;
    /** The controllers the host runs: flushing, compacting and expiring segments, and delivering routed copies once a kind routes. */
    readonly controllers: readonly Controller[];
    /** The kinds by their key. */
    readonly #kinds: ReadonlyMap<string, EventKind>;
    /** The segments. */
    readonly #archive: EventArchive;
    /** The keyring sealing personal values. */
    readonly #personal: PersonalKeyring | undefined;
    /** The outbox routing copies. */
    readonly #outbox: Outbox | undefined;
    /** The scopes a route copies a scope's events to. */
    readonly #targets: ((route: Route, scope: string) => Promise<readonly string[]>) | undefined;
    /** Report a dropped write of events kept at most once. */
    readonly #report: (error: unknown) => void;

    /** Keep events of some kinds over a host's database and buckets, refusing a kind whose subject or route lacks what they need. */
    constructor(options: EventStoreOptions) {
        // require a keyring for subjects, and an outbox, targets and a delivery for routes
        const { kinds, outbox, deliver, policy } = options;
        if (options.personal === undefined && kinds.some((kind) => kind.subject !== undefined)) {
            throw new TypeError("event kinds with a subject need a personal keyring");
        }
        const routes = kinds.some((kind) => kind.route !== undefined);
        if (
            routes &&
            (outbox === undefined || options.targets === undefined || deliver === undefined)
        ) {
            throw new TypeError(
                "event kinds that route copies need an outbox, targets and a delivery",
            );
        }

        // keep segments under each scope's policy, flushed and delivered by the controllers
        this.database = options.database;
        this.#kinds = new Map(kinds.map((kind) => [kind.key, kind]));
        this.#archive = new EventArchive({
            database: options.database,
            files: options.files,
            prefix: options.prefix ?? SEGMENT_PREFIX,
            policy: async (kind, scope) => (await policy?.(kind, scope)) ?? kind.policy,
        });
        this.#personal = options.personal;
        this.#outbox = outbox;
        this.#targets = options.targets;
        this.#report = options.report ?? (() => undefined);
        this.controllers = [
            new ArchiveController(this.#archive, kinds, options.now ?? Date.now),
            ...(outbox === undefined || deliver === undefined
                ? []
                : kinds
                      .filter((kind) => kind.route !== undefined)
                      .map((kind) => outbox.controller(routeDestination(kind, deliver)))),
        ];
    }

    /** Append events of a kind, an exactly-once kind's in a caller's transaction, sealing personal values, dropping secrets and routing copies, an event appended again changing nothing. */
    async append<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        events: readonly EventInput<Shape, Data>[],
        transaction?: DatabaseConnection,
    ): Promise<void> {
        // refuse a transaction for a kind whose events outlive the work recording them
        this.#kind(kind.key);
        if (transaction !== undefined && kind.delivery === "at-most-once") {
            throw new TypeError(
                `event kind ${kind.key} is kept at most once, never in a caller's transaction`,
            );
        }

        // check and seal the events and name their copies first, reading keys in the caller's transaction or before the store's
        const stored = await this.#prepare(kind, events, transaction);
        if (stored.length === 0) {
            return;
        }
        const copies = await this.#copies(kind, stored);

        // append in the caller's transaction or one of the store's, dropping an event kept at most once when its write fails
        const write = (connection: DatabaseConnection) =>
            this.#write(kind, stored, copies, connection);
        if (transaction !== undefined) {
            await write(transaction);
        } else if (kind.delivery === "at-most-once") {
            await this.database.transaction(write).catch((error: unknown) => {
                this.#report(error);
            });
        } else {
            await this.database.transaction(write);
        }
    }

    /** Append the events of a kind another scope's store routed here, as they were, an event received again changing nothing. */
    async receive(key: string, events: readonly Event[]): Promise<void> {
        const kind = this.#kind(key);
        if (events.length > 0) {
            await this.database
                .insert(kind.table)
                .values(events.map((event) => kind.row(event)))
                .onConflictDoNothing();
        }
    }

    /** Read a page of a scope's events a filter selects in time order, hot and flushed alike, after a cursor. */
    async query<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        filter: EventFilter,
        page: { readonly after?: EventCursor; readonly limit?: number } = {},
    ): Promise<EventPage<Shape, Data>> {
        // read candidates a page at a time until the page fills with events holding the filter's text
        EventFilter.check(kind, filter);
        const limit = page.limit ?? PAGE_EVENTS;
        const kept: Event<Shape, Data>[] = [];
        let after = page.after;
        for (;;) {
            const candidates = await this.#candidates(kind, filter, after, limit + 1);
            for (const candidate of candidates) {
                const event = await this.#open(kind, candidate);
                if (EventFilter.contains(filter, event.data)) {
                    kept.push(event);
                }
            }
            const last = candidates.at(-1);
            if (kept.length > limit || candidates.length <= limit || last === undefined) {
                break;
            }
            after = { time: last.time, id: last.id };
        }

        // end the page after its last event while another follows
        const events = kept.slice(0, limit);
        const final = events.at(-1);

        return {
            events,
            cursor:
                kept.length > limit && final !== undefined
                    ? { time: final.time, id: final.id }
                    : undefined,
        };
    }

    /** Read every event of a scope a filter selects in time order, page by page. */
    async *export<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        filter: EventFilter,
    ): AsyncGenerator<Event<Shape, Data>> {
        let after: EventCursor | undefined;
        do {
            const page = await this.query(kind, filter, after === undefined ? {} : { after });
            yield* page.events;
            after = page.cursor;
        } while (after !== undefined);
    }

    /** Fold a scope's events a filter selects into a series per group of key values, a step at a time, hot events in SQL. */
    async series<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        filter: EventFilter,
        request: SeriesRequest<Shape>,
    ): Promise<Series[]> {
        // fold the hot events in SQL and the flushed ones in turn, unless the fold or the text needs each opened event
        EventFilter.check(kind, filter);
        const series = new SeriesFold(request, filter.from);
        if (filter.text === undefined && request.fold !== "last") {
            const { hot, flushed } = await this.#snapshot(kind, filter, undefined, () =>
                this.#hotSeries(kind, filter, series),
            );
            for (const row of hot) {
                series.add(row.keys, row.start ?? series.startOf(0), row.fold);
            }
            flushed.forEach((event) => {
                series.addEvent(event);
            });
        }
        // fold each opened event the filter selects
        else {
            for await (const event of this.export(kind, filter)) {
                series.addEvent(event);
            }
        }

        return series.close();
    }

    /** Follow a scope's events of a kind a filter selects as they commit, in commit order, from a log sequence or from now. */
    async *tail<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        filter: EventFilter,
        signal: AbortSignal,
        after?: number,
    ): AsyncGenerator<{ readonly event: Event<Shape, Data>; readonly sequence: number }> {
        // follow the log of the kind's table from the given sequence, or the current position
        EventFilter.check(kind, filter);
        const match = EventFilter.match(kind, filter);
        const start = after ?? (await this.database.log.position()).sequence;
        const selection = { tables: [kind.table], scopes: [filter.scope], after: start };
        for await (const page of this.database.log.follow(selection, signal)) {
            for (const change of page.changes) {
                // yield each appended event the filter selects
                if (change.operation !== "insert" || !Change.of(change, kind.table)) {
                    continue;
                }
                const stored = kind.event(Change.image(change));
                if (!match(stored)) {
                    continue;
                }
                const event = await this.#open(kind, stored);
                if (EventFilter.contains(filter, event.data)) {
                    yield { event, sequence: change.sequence };
                }
            }
        }
    }

    /** Forget a person, erasing their personal values in every event sealed under their keys. */
    async forget(subject: string): Promise<void> {
        if (this.#personal === undefined) {
            throw new TypeError("a store without a personal keyring seals no personal values");
        }
        await this.#personal.forget(subject);
    }

    /** Check events against their kind, seal their personal values and drop their secrets, reading keys in a transaction when given. */
    async #prepare<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        events: readonly EventInput<Shape, Data>[],
        transaction: DatabaseConnection | undefined,
    ): Promise<Event[]> {
        return Promise.all(
            events.map(async (event) => {
                // check the keys and data, then seal its personal values and drop its secrets
                const keys = kind.parseKeys(event.keys);
                const data = kind.parseData(event.data);
                const sealed = await PersonalSeal.seal(
                    kind,
                    keys,
                    data,
                    this.#personal,
                    transaction,
                );

                return {
                    scope: event.scope,
                    id: event.id,
                    source: event.scope,
                    time: event.time,
                    keys: kind.keyValues(keys),
                    data: sealed,
                };
            }),
        );
    }

    /** Write checked events with their routed copies, in one transaction. */
    async #write(
        kind: EventKind,
        stored: readonly Event[],
        copies: readonly Event[],
        transaction: DatabaseConnection,
    ): Promise<void> {
        // write the events
        await transaction
            .insert(kind.table)
            .values(stored.map((event) => kind.row(event)))
            .onConflictDoNothing();

        // append each copy to the outbox, keyed so a repeated append routes nothing again
        for (const copy of copies) {
            const key = new TextEncoder().encode(`${copy.scope} ${copy.source} ${copy.id}`);
            const digest = new Uint8Array(await crypto.subtle.digest("SHA-256", key)).toHex();
            await this.#outbox?.append(routeAddress(kind), digest, copy, transaction);
        }
    }

    /** Copy events to the scopes their kind's route names for each of theirs, leaving out their own. */
    async #copies(kind: EventKind, stored: readonly Event[]): Promise<Event[]> {
        // read each scope's targets once
        const route = kind.route;
        const targets = this.#targets;
        if (route === undefined || targets === undefined) {
            return [];
        }
        const scopes = [...new Set(stored.map((event) => event.scope))];
        const named = new Map(
            await Promise.all(
                scopes.map(async (scope) => [scope, await targets(route, scope)] as const),
            ),
        );

        // copy each event to its scope's targets
        return stored.flatMap((event) =>
            (named.get(event.scope) ?? [])
                .filter((target) => target !== event.scope)
                .map((target) => ({ ...event, scope: target })),
        );
    }

    /** Read the hot and flushed events a filter selects after a cursor in time order, once each, at most a number. */
    async #candidates(
        kind: EventKind,
        filter: EventFilter,
        after: EventCursor | undefined,
        limit: number,
    ): Promise<Event[]> {
        // read both sides of one catalog state
        const { hot, flushed } = await this.#snapshot(kind, filter, after, () =>
            this.#hot(kind, filter, after, limit),
        );

        // merge them in time order, once each
        const seen = new Set<string>();

        return [...hot, ...flushed]
            .toSorted((left, right) => EventCursor.compare(left, right))
            .filter((event) => {
                // keep the first of each source and identity
                const key = `${event.source} ${event.id}`;
                const isNew = !seen.has(key);
                seen.add(key);

                return isNew;
            })
            .slice(0, limit);
    }

    /** Read hot events and the flushed ones of one catalog state, reading again while a flush or compaction changes the catalog or deletes a file between. */
    async #snapshot<Hot>(
        kind: EventKind,
        filter: EventFilter,
        after: EventCursor | undefined,
        read: () => Promise<Hot>,
    ): Promise<{ readonly hot: Hot; readonly flushed: Event[] }> {
        // read the catalog, the hot events, then the catalog again, until both catalog reads agree
        for (let attempt = 1; attempt <= SNAPSHOT_ATTEMPTS; attempt += 1) {
            const before = await this.#archive.choose(kind, filter, after);
            const hot = await read();
            const current = await this.#archive.choose(kind, filter, after);
            const flushed = sameSegments(before, current)
                ? await this.#archive.decode(kind, current, filter, after)
                : undefined;
            if (flushed !== undefined) {
                return { hot, flushed };
            }
        }

        throw new TypeError(
            `the segments of ${kind.key} in ${filter.scope} kept changing during a read`,
        );
    }

    /** Read the hot events a filter selects after a cursor, at most a number. */
    async #hot(
        kind: EventKind,
        filter: EventFilter,
        after: EventCursor | undefined,
        limit: number,
    ): Promise<Event[]> {
        const table = kind.table;
        const rows = await this.database
            .select()
            .from(table)
            .where(
                and(
                    this.#selection(kind, filter),
                    after === undefined
                        ? undefined
                        : or(
                              gt(table.time, after.time),
                              and(eq(table.time, after.time), gt(table.id, after.id)),
                          ),
                ),
            )
            .orderBy(asc(table.time), asc(table.id))
            .limit(limit);

        return rows.map((row) => kind.event(row));
    }

    /** Fold the hot events a filter selects in SQL per step and group: their count, sum, smallest and largest value. */
    async #hotSeries(
        kind: EventKind,
        filter: EventFilter,
        series: SeriesFold<EventKeyShape>,
    ): Promise<
        {
            readonly start: number | undefined;
            readonly keys: Readonly<Record<string, unknown>>;
            readonly fold: StepFold;
        }[]
    > {
        // group by the step's start and each grouping key's column
        const table = kind.table;
        const { measure, group, width } = series;
        const columns: SQLWrapper[] = group.map((name) => table[TABLE].column(name));
        const step = sql.raw(String(width ?? 1));
        const start: SQL = sql`(${table.time} / ${step}) * ${step}`;
        const value: SQLWrapper = measure === undefined ? sql`0` : table[TABLE].column(measure);
        const rows = await this.database
            .select({
                ...(width === undefined ? {} : { start }),
                events: count(),
                sum: sum(value),
                min: min(value),
                max: max(value),
                ...Object.fromEntries(
                    columns.map((column, index) => [`key${String(index)}`, column]),
                ),
            })
            .from(table)
            .where(
                and(
                    this.#selection(kind, filter),
                    measure === undefined ? undefined : sql`${value} IS NOT NULL`,
                ),
            )
            .groupBy(...(width === undefined ? [] : [start]), ...columns);

        // read each row's group and partial fold
        return rows.map((row) => {
            const held: Readonly<Record<string, unknown>> = row;

            return {
                start: width === undefined ? undefined : Number(held["start"]),
                keys: Object.fromEntries(
                    group.map((name, index) => [name, held[`key${String(index)}`]]),
                ),
                fold: {
                    events: Number(held["events"]),
                    sum: Number(held["sum"] ?? 0),
                    min: Number(held["min"] ?? 0),
                    max: Number(held["max"] ?? 0),
                    last: undefined,
                },
            };
        });
    }

    /** Select a filter's scope, time range and condition over a kind's hot events. */
    #selection(kind: EventKind, filter: EventFilter): SQL | undefined {
        const table = kind.table;

        return and(
            eq(table.scope, filter.scope),
            filter.from === undefined ? undefined : gte(table.time, filter.from),
            filter.before === undefined ? undefined : lt(table.time, filter.before),
            filter.where === undefined ? undefined : Condition.render(filter.where, table),
        );
    }

    /** Open a stored event as its kind types it: its keys checked, its personal values opened or left out once forgotten. */
    async #open<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        stored: Event,
    ): Promise<Event<Shape, Data>> {
        const keys = kind.parseKeys(stored.keys);
        const data = await PersonalSeal.open(kind, keys, stored.data, this.#personal);

        return {
            scope: stored.scope,
            id: stored.id,
            source: stored.source,
            time: stored.time,
            keys,
            data: kind.parseData(data),
        };
    }

    /** Find a kind the store keeps by its key. */
    #kind(key: string): EventKind {
        const kind = this.#kinds.get(key);
        if (kind === undefined) {
            throw new TypeError(`this store keeps no event kind ${key}`);
        }

        return kind;
    }
}

/** Report whether two catalog reads chose the same segments. */
function sameSegments(
    left: readonly { readonly id: string }[],
    right: readonly { readonly id: string }[],
): boolean {
    return (
        left.length === right.length &&
        left.every((segment, index) => segment.id === right[index]?.id)
    );
}
