import {
    Order,
    and,
    asc,
    desc,
    Change,
    Condition,
    count,
    eq,
    inArray,
    gt,
    lt,
    max,
    min,
    or,
    sql,
    sum,
    type DatabaseConnection,
    type SQL,
    type SQLWrapper,
    TABLE,
    listed,
} from "@destack/db";
import { canonicalize, type JsonValue, schema } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { telemetry } from "@destack/telemetry";
import type { Bucket } from "@destack/bucket";
import type { Controller } from "@destack/runtime/control";
import { ArchiveController } from "../archive/controller.ts";
import { EventArchive } from "../archive/archive.ts";
import { type Event, type EventKeyShape, EventKind } from "../kind/kind.ts";
import { type PersonalKeyring, PersonalSeal } from "./personal-keyring.ts";
import { EventCursor, EventFilter, type EventOrder } from "./filter.ts";
import { Fold, type Series, SeriesFold, type SeriesRequest, type StepFold } from "./series.ts";
import { Clock } from "@destack/environment";

/** The instruments capturing the dropped writes of events kept at most once. */
const instruments = telemetry.scope(import.meta.destack.package);

/** The bucket key prefix segments are written under unless a host sets another. */
const SEGMENT_PREFIX = "events";

/** The events one query page has unless it asks for another number. */
const PAGE_EVENTS = 100;

/** The reads a query retries while flushes and compactions change the catalog under it. */
const SNAPSHOT_ATTEMPTS = 5;

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
    /** The clock segments flush, compact and expire by. */
    readonly clock: Clock;
}

/** The append-only events of the scopes a host keeps. */
export class EventStore {
    /** The database of the hot events. */
    readonly database: DatabaseConnection;
    /** The controllers flushing, compacting and expiring segments. */
    readonly controllers: readonly Controller[];
    /** The kinds by their key. */
    readonly #kinds: ReadonlyMap<string, EventKind>;
    /** The segments. */
    readonly #archive: EventArchive;
    /** The keyring sealing personal values. */
    readonly #personal: PersonalKeyring | undefined;

    /** Keep events of some kinds over a host's database and buckets. */
    constructor(options: EventStoreOptions) {
        // require a keyring for subjects
        const { kinds } = options;
        if (options.personal === undefined && kinds.some((kind) => kind.subject !== undefined)) {
            throw new TypeError("event kinds with a subject need a personal keyring");
        }

        // refuse a kind kept twice
        const keys = kinds.map((kind) => kind.key);
        const repeated = keys.find((key, index) => keys.indexOf(key) !== index);
        if (repeated !== undefined) {
            throw new TypeError(`event kind ${repeated} is kept twice`);
        }

        // keep segments under each kind's policy, flushed and delivered by the controllers
        this.database = options.database;
        this.#kinds = new Map(kinds.map((kind) => [kind.key, kind]));
        this.#archive = new EventArchive({
            database: options.database,
            files: options.files,
            prefix: options.prefix ?? SEGMENT_PREFIX,
        });
        this.#personal = options.personal;
        this.controllers = [new ArchiveController(this.#archive, kinds, options.clock)];
    }

    /** Append events of a kind once each, sealing personal values. */
    async append<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        events: readonly Event<Shape, Data>[],
        transaction?: DatabaseConnection,
    ): Promise<void> {
        // refuse a transaction for a kind whose events outlive the work recording them
        this.kind(kind.key);
        if (transaction !== undefined && kind.delivery === "at-most-once") {
            throw new TypeError(
                `event kind ${kind.key} is kept at most once and cannot join a caller's transaction`,
            );
        }

        // check and seal the events
        const stored = await this.#prepare(kind, events, transaction);
        if (stored.length === 0) {
            return;
        }

        // append them in the caller's transaction or the store's
        const write = (connection: DatabaseConnection) => this.#write(kind, stored, connection);
        if (transaction !== undefined) {
            await write(transaction);
        } else if (kind.delivery === "at-most-once") {
            await this.database.transaction(write).catch((error: unknown) => {
                instruments.captureException(error);
            });
        } else {
            await this.database.transaction(write);
        }
    }

    /** Read a page of some scopes' events in time order, hot and flushed alike. */
    async query<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        filter: EventFilter,
        page: {
            readonly after?: EventCursor;
            readonly limit?: number;
            readonly order?: EventOrder;
        } = {},
    ): Promise<EventPage<Shape, Data>> {
        // read one event past the page, hot and flushed alike, and open the page's
        EventFilter.check(kind, filter);
        const limit = page.limit ?? PAGE_EVENTS;
        const candidates = await this.#candidates(
            kind,
            filter,
            page.after,
            limit + 1,
            page.order ?? "ascending",
        );
        const events = await Promise.all(
            candidates.slice(0, limit).map((candidate) => this.#open(kind, candidate)),
        );

        // end the page after its last event while another follows
        const final = events.at(-1);

        return {
            events,
            cursor:
                candidates.length > limit && final !== undefined
                    ? { time: final.time, id: final.id }
                    : undefined,
        };
    }

    /** Read one event of some scopes by its identity, hot or flushed. */
    async get<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        key: Pick<EventFilter, "scopes"> & Pick<Event, "id">,
    ): Promise<Event<Shape, Data> | undefined> {
        const page = await this.query(
            kind,
            { scopes: key.scopes, where: { id: key.id } },
            { limit: 1 },
        );

        return page.events.at(0);
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

    /** Fold a scope's events into a series per group of key values. */
    async series<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        filter: EventFilter,
        request: SeriesRequest<Shape>,
    ): Promise<Series[]> {
        // refuse a quantile of a kind recording no distributions
        EventFilter.check(kind, filter);
        const series = new SeriesFold(request, EventFilter.range(filter).from);
        const { fold } = request;
        const isQuantile = Fold.isQuantile(fold);
        if (isQuantile && kind.distribution === undefined) {
            throw new TypeError(
                `a ${fold} series reads distributions, but ${kind.key} records none`,
            );
        }

        // fold the hot events in SQL and the flushed ones in turn
        if (fold !== "last" && fold !== "unique" && !isQuantile) {
            const { hot, flushed } = await this.#snapshot(
                kind,
                filter,
                undefined,
                "ascending",
                () => this.#hotSeries(kind, filter, series),
            );
            for (const row of hot) {
                series.add(row.keys, row.start ?? series.stepStart(0), row.fold);
            }
            flushed.forEach((event) => {
                series.addEvent({
                    time: event.time,
                    keys: EventFilter.flatten(kind, event),
                    distribution: undefined,
                });
            });
        }
        // fold each opened event the filter selects, with its distribution
        else {
            for await (const event of this.export(kind, filter)) {
                series.addEvent({
                    time: event.time,
                    keys: EventFilter.flatten(kind, event),
                    distribution: kind.readDistribution(event.data),
                });
            }
        }

        return series.close();
    }

    /** Read a page of some scopes' events in commit order after a log sequence. */
    async changes<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        filter: EventFilter,
        after: number,
        limit?: number,
    ): Promise<{ readonly events: readonly Event<Shape, Data>[]; readonly sequence: number }> {
        // read the page of the kind's committed appends after the sequence
        EventFilter.check(kind, filter);
        const page = await this.database.log.read({
            tables: [kind.table],
            scopes: filter.scopes,
            after,
            ...(limit === undefined ? {} : { limit }),
        });

        return {
            events: await this.#appended(kind, filter, page.changes),
            sequence: page.sequence,
        };
    }

    /** Read the events some scopes appended after a log sequence through another, as one page. */
    async range<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        filter: EventFilter,
        after: number,
        through: number,
    ): Promise<readonly Event<Shape, Data>[]> {
        // read the kind's committed appends in the range
        EventFilter.check(kind, filter);
        const changes = await this.database.log.range(
            { tables: [kind.table], scopes: filter.scopes, after },
            through,
        );

        return this.#appended(kind, filter, changes);
    }

    /** Open each event some changes append that a filter selects, in commit order. */
    async #appended<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        filter: EventFilter,
        changes: readonly Change[],
    ): Promise<Event<Shape, Data>[]> {
        // keep the kind's inserts the filter selects, then open each
        const match = EventFilter.match(kind, filter);
        const appended = changes
            .filter((change) => change.operation === "insert" && Change.of(change, kind.table))
            .map((change) => kind.event(Change.image(change)))
            .filter((stored) => match(stored));
        const events: Event<Shape, Data>[] = [];
        for (const stored of appended) {
            events.push(await this.#open(kind, stored));
        }

        return events;
    }

    /** Forget a person by their subject keys' values, erasing their sealed personal values. */
    async forget(subject: readonly string[]): Promise<void> {
        if (this.#personal === undefined) {
            throw new TypeError("a store without a personal keyring cannot seal personal values");
        }
        await this.#personal.forget(EventKind.person(subject));
    }

    /** Check events against their kind, seal their personal values and drop their secrets. */
    async #prepare<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        events: readonly Event<Shape, Data>[],
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
                    time: event.time,
                    keys: kind.keyValues(keys),
                    data: sealed,
                };
            }),
        );
    }

    /** Write checked events in one transaction. */
    async #write(
        kind: EventKind,
        stored: readonly Event[],
        transaction: DatabaseConnection,
    ): Promise<void> {
        // write the events, requiring each one stored before to repeat its contents
        const table = kind.table[TABLE];
        const written = await transaction
            .insert(kind.table)
            .values(stored.map((event) => kind.row(event)))
            .onConflictDoNothing()
            .returning({ id: table.column("id") });
        const isWritten = new Set(written.map((row) => schema.string().parse(row.id)));
        const repeated = stored.filter((event) => !isWritten.has(event.id));
        if (repeated.length > 0) {
            await this.#requireRepeats(kind, repeated, transaction);
        }
    }

    /** Require stored events to repeat their time, keys and data. */
    async #requireRepeats(
        kind: EventKind,
        repeated: readonly Event[],
        transaction: DatabaseConnection,
    ): Promise<void> {
        // read the stored events of the repeated identities
        const table = kind.table[TABLE];
        const rows = await transaction
            .select()
            .from(kind.table)
            .where(
                inArray(
                    table.column("id"),
                    repeated.map((event) => event.id),
                ),
            );
        const kept = new Map(
            rows.map((row) => {
                const event = kind.event(row);

                return [event.id, searchedText(kind, event)];
            }),
        );

        // refuse an identity stored with other contents
        const changed = repeated.find((event) => kept.get(event.id) !== searchedText(kind, event));
        if (changed !== undefined) {
            throw new CallError("CONFLICT", {
                message: `event ${changed.id} of ${kind.key} was stored with other contents`,
            });
        }
    }

    /** Read the hot and flushed events after a cursor in time order, once each. */
    async #candidates(
        kind: EventKind,
        filter: EventFilter,
        after: EventCursor | undefined,
        limit: number,
        order: EventOrder,
    ): Promise<Event[]> {
        // read both sides of one catalog state
        const { hot, flushed } = await this.#snapshot(kind, filter, after, order, () =>
            this.#hot(kind, filter, after, limit, order),
        );

        // merge them in the page's order, once each
        const seen = new Set<string>();
        const direction = order === "ascending" ? 1 : -1;

        return [...hot, ...flushed]
            .toSorted((left, right) => direction * EventCursor.compare(left, right))
            .filter((event) => {
                // keep the first of each identity
                const isNew = !seen.has(event.id);
                seen.add(event.id);

                return isNew;
            })
            .slice(0, limit);
    }

    /** Read hot and flushed events of one consistent catalog state. */
    async #snapshot<Hot>(
        kind: EventKind,
        filter: EventFilter,
        after: EventCursor | undefined,
        order: EventOrder,
        read: () => Promise<Hot>,
    ): Promise<{ readonly hot: Hot; readonly flushed: Event[] }> {
        // read the catalog, the hot events, then the catalog again, until both catalog reads agree
        for (let attempt = 1; attempt <= SNAPSHOT_ATTEMPTS; attempt += 1) {
            const before = await this.#archive.choose(kind, filter, after, order);
            const hot = await read();
            const current = await this.#archive.choose(kind, filter, after, order);
            const flushed = sameSegments(before, current)
                ? await this.#archive.decode(kind, current, filter, after, order)
                : undefined;
            if (flushed !== undefined) {
                return { hot, flushed };
            }
        }

        throw new CallError("SERVICE_UNAVAILABLE", {
            message: `the segments of ${kind.key} in ${filter.scopes.join(", ")} kept changing during a read`,
        });
    }

    /** Read the hot events a filter selects after a cursor, at most a number. */
    async #hot(
        kind: EventKind,
        filter: EventFilter,
        after: EventCursor | undefined,
        limit: number,
        order: EventOrder,
    ): Promise<Event[]> {
        // select the events past the cursor in the page's order
        const table = kind.table;
        const past = order === "ascending" ? gt : lt;
        const direction = order === "ascending" ? asc : desc;
        const rows = await this.database
            .select()
            .from(table)
            .where(
                and(
                    this.#selection(kind, filter),
                    after === undefined
                        ? undefined
                        : or(
                              past(table.time, after.time),
                              and(eq(table.time, after.time), past(table.id, after.id)),
                          ),
                ),
            )
            .orderBy(direction(table.time), direction(table.id))
            .limit(limit);

        return rows.map((row) => kind.event(row));
    }

    /** Fold the hot events in SQL per step and group. */
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
        // group by the step's start and each grouping key's column or map entry, named in the selection
        const table = kind.table;
        const { measure, group, width } = series;
        const namespace = EventFilter.namespace(kind, filter, {
            text: group,
            numeric: measure === undefined ? [] : [measure],
        });
        const columns: SQLWrapper[] = group.map((name) => Order.expression(table, name, namespace));
        const step = sql.raw(String(width ?? 1));
        const start: SQL = sql`(${table.time} / ${step}) * ${step}`;
        const value: SQLWrapper =
            measure === undefined ? sql`0` : Order.expression(table, measure, namespace);
        const rows = await this.database
            .select({
                ...(width === undefined ? {} : { start }),
                events: count(),
                sum: sum(value),
                min: min(value.getSQL()),
                max: max(value.getSQL()),
                ...Object.fromEntries(
                    columns.map((column, index) => [
                        `key${String(index)}`,
                        sql`${column}`.as(`key${String(index)}`),
                    ]),
                ),
            })
            .from(table)
            .where(
                and(
                    this.#selection(kind, filter),
                    measure === undefined ? undefined : sql`${value} IS NOT NULL`,
                ),
            )
            .groupBy(
                ...(width === undefined ? [] : [start]),
                ...columns.map((_, index) => sql.identifier(`key${String(index)}`)),
            );

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
                    sum: Number(held["sum"]),
                    min: Number(held["min"]),
                    max: Number(held["max"]),
                    last: undefined,
                    distinct: undefined,
                    distribution: undefined,
                },
            };
        });
    }

    /** Select a filter's scope, time range and condition over a kind's hot events. */
    #selection(kind: EventKind, filter: EventFilter): SQL {
        const table = kind.table;

        return and(
            listed(table.scope, sql.param(JSON.stringify(filter.scopes))),
            filter.where === undefined
                ? undefined
                : Condition.render(filter.where, table, EventFilter.namespace(kind, filter)),
        );
    }

    /** Open a stored event as its kind types it. */
    async #open<Shape extends EventKeyShape, Data extends JsonValue>(
        kind: EventKind<Shape, Data>,
        stored: Event,
    ): Promise<Event<Shape, Data>> {
        const keys = kind.parseKeys(stored.keys);
        const data = await PersonalSeal.open(kind, keys, stored.data, this.#personal);

        return {
            scope: stored.scope,
            id: stored.id,
            time: stored.time,
            keys,
            data: kind.parseData(data),
        };
    }

    /** Report whether the store has a kind. */
    has(kind: EventKind): boolean {
        return this.#kinds.get(kind.key) === kind;
    }

    /** Find a kept kind by its key, refusing a key no kept kind has. */
    kind(key: string): EventKind {
        const kind = this.#kinds.get(key);
        if (kind === undefined) {
            throw new TypeError(`this store has no event kind ${key}`);
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

/** Write an event's time, keys and data without personal values, which each seal writes anew. */
function searchedText(kind: EventKind, event: Event): string {
    const data = kind.mapSensitive(event.data, (value, sensitivity) =>
        sensitivity === "personal" ? undefined : value,
    );

    return canonicalize({ time: event.time, keys: kind.keyValues(event.keys), data });
}
