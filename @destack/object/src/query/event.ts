import type { DatabaseConnection, Row } from "@destack/db";
import {
    Event,
    EventCursor,
    type EventFilter,
    type EventKeyShape,
    type EventKind,
    type EventOrder,
    type EventPage,
    EventStore,
    type Series,
    type SeriesRequest,
} from "@destack/event";
import { schema, type JsonValue } from "@destack/schema";
import { Caller } from "@destack/access";
import { CallError } from "@destack/runtime/error";
import { Page } from "../method/page.ts";
import { Replica } from "@destack/sync";
import type { MethodContext } from "../method/context.ts";
import type { Session } from "../method/reader.ts";
import { type EventRequest, SeriesRecord } from "../hook/record.ts";
import type { ListInput } from "./listing.ts";
import { Execution, type StoreExecution } from "../method/execution.ts";

/** The list input fields only the SQL dataflow serves, which the event log refuses. */
const DATAFLOW_FIELDS = ["extras", "with", "aggregate", "deleted", "at", "branch"] as const;

/** The event log behind an events type: `create` appends to it, `list` and `get` read it. */
export const EventLog = {
    /** Append created rows as events of the scope. */
    async append(
        context: MethodContext,
        execution: StoreExecution,
        rows: readonly Row[],
    ): Promise<void> {
        // read each row as its event
        const kind = context.object.eventKind();
        const events = rows.map((row) => kind.event(row));

        // update the type's readers with the appended rows
        const { transaction } = execution;
        for (const row of rows) {
            context.object.updateReaders(transaction, { before: null, after: row }, context.now);
        }

        // append events recorded exactly once as the call commits
        const store = execution.store.events;
        if (kind.delivery === "exactly-once") {
            transaction.record((committing) => store.append(kind, events, committing));
        }
        // append events recorded at most once alone, outliving a failed call
        else {
            await store.append(kind, events);
        }
    },

    /** Read a page of the scope's events the caller may read, in time order. */
    async page(
        context: MethodContext,
        execution: StoreExecution,
        input: ListInput,
    ): Promise<{ items: Row[]; cursor: string | null }> {
        // refuse what only the SQL dataflow serves
        const kind = context.object.eventKind();
        const unserved = DATAFLOW_FIELDS.find((name) => input[name] !== undefined);
        if (unserved !== undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `${context.object.name} lists from the event log, which cannot read ${unserved}`,
            });
        }

        // read one event past the page from the log, the condition pruning segments
        const { where, orderBy, cursor, limit } = input;
        const listing = new Page(
            {
                ...(cursor === undefined ? {} : { cursor }),
                ...(limit === undefined ? {} : { limit }),
            },
            [context.object.name, context.scope],
            EventCursor,
        );
        const read = await execution.store.events.query(
            kind,
            {
                scopes: await EventLog.scopes(context, Execution.snapshot(execution).database),
                ...(where === undefined ? {} : { where }),
            },
            {
                ...(listing.after === undefined ? {} : { after: listing.after }),
                limit: listing.limit + 1,
                order: EventLog.readOrder(context, orderBy),
            },
        );

        // cut the page, then filter the rows the caller may read
        const page = listing.result(
            read.events.map((event) => kind.row(event)),
            (row) => EventCursor.parse(row),
        );
        const { rows: items } = await execution.authorization.filter(
            context.object,
            context.permission(),
            page.items,
        );

        return { items, cursor: page.cursor };
    },

    /** Read one event of the scope by its identifier, hot or flushed, absent if none. */
    async row(
        context: MethodContext,
        events: EventStore,
        database: DatabaseConnection,
        id: string,
    ): Promise<Row | undefined> {
        const kind = context.object.eventKind();
        const event = await events.get(kind, {
            scopes: await EventLog.scopes(context, database),
            id,
        });

        return event === undefined ? undefined : kind.row(event);
    },

    /** List the scopes a read of a scope's events covers: its own, and the caller's replicas. */
    async scopes(
        context: MethodContext,
        database: DatabaseConnection,
    ): Promise<readonly [string, ...string[]]> {
        const replicated = await Replica.scopes(
            database,
            context.object.table,
            context.scope,
            Caller.principal(context.caller),
        );

        return [context.scope, ...replicated.filter((scope) => scope !== context.scope)];
    },

    /** Read a list's order as the event log's: by time, newest first unless asked otherwise. */
    readOrder(context: MethodContext, orderBy: ListInput["orderBy"]): EventOrder {
        // read the time's direction, refusing any other field
        const [field, ...others] = Object.entries(orderBy ?? { time: "desc" });
        if (field === undefined || field[0] !== "time" || others.length > 0) {
            throw new CallError("BAD_REQUEST", {
                message: `${context.object.name} lists in time order only`,
            });
        }

        return field[1] === "asc" ? "ascending" : "descending";
    },
};

/** An event of an events object type, keyed and shaped as its kind stores it. */
export type EventOf<Type extends { readonly eventKind: () => EventKind }> =
    ReturnType<Type["eventKind"]> extends EventKind<infer Shape, infer Data>
        ? Event<Shape, Data>
        : never;

/** A package's reads of its own event kind: pages in time order, single events and series. */
export class EventView<Shape extends EventKeyShape, Data extends JsonValue> {
    /** The event kind read. */
    readonly kind: EventKind<Shape, Data>;
    /** The store's event log, or a hook's session with its store. */
    readonly #source: EventStore | Session;

    /** Read a kind's events through the store's event log or a hook's session. */
    constructor(kind: EventKind<Shape, Data>, source: EventStore | Session) {
        this.kind = kind;
        this.#source = source;
    }

    /** Read a page of some scopes' events in time order. */
    async query(filter: EventFilter, page: EventPageRequest = {}): Promise<EventPage<Shape, Data>> {
        // read through the event log, else through the session
        const source = this.#source;
        if (source instanceof EventStore) {
            return source.query(this.kind, filter, page);
        }
        const answered = await source.events(this.kind.table, {
            verb: "query",
            filter: EventView.#filter(filter),
            page: {
                ...(page.after === undefined ? {} : { after: page.after }),
                ...(page.limit === undefined ? {} : { limit: page.limit }),
                ...(page.order === undefined ? {} : { order: page.order }),
            },
        });

        // open the answered events with the kind's keys and data
        const { events, cursor } = EVENT_PAGE.parse(answered);

        return { events: events.map((event) => this.#open(event)), cursor };
    }

    /** Read one event of some scopes by its identity, absent if none. */
    async get(
        key: Pick<EventFilter, "scopes"> & { readonly id: string },
    ): Promise<Event<Shape, Data> | undefined> {
        // read through the event log, else through the session
        const source = this.#source;
        if (source instanceof EventStore) {
            return source.get(this.kind, key);
        }
        const answered = await source.events(this.kind.table, {
            verb: "get",
            scopes: [...key.scopes],
            id: key.id,
        });

        return answered === null ? undefined : this.#open(Event.parse(answered));
    }

    /** Read the events some scopes appended after a log sequence, and the sequence reached. */
    async changes(
        filter: EventFilter,
        after: number,
        limit?: number,
    ): Promise<{ readonly events: readonly Event<Shape, Data>[]; readonly sequence: number }> {
        // read through the event log, else through the session
        const source = this.#source;
        if (source instanceof EventStore) {
            return source.changes(this.kind, filter, after, limit);
        }
        const answered = await source.events(this.kind.table, {
            verb: "changes",
            filter: EventView.#filter(filter),
            after,
            ...(limit === undefined ? {} : { limit }),
        });

        // open the answered events with the kind's keys and data
        const { events, sequence } = EVENT_CHANGES.parse(answered);

        return { events: events.map((event) => this.#open(event)), sequence };
    }

    /** Read the events some scopes appended after a log sequence through another, as one page. */
    async range(
        filter: EventFilter,
        after: number,
        through: number,
    ): Promise<readonly Event<Shape, Data>[]> {
        // read through the event log, else through the session
        const source = this.#source;
        if (source instanceof EventStore) {
            return source.range(this.kind, filter, after, through);
        }
        const answered = await source.events(this.kind.table, {
            verb: "range",
            filter: EventView.#filter(filter),
            after,
            through,
        });

        // open the answered events with the kind's keys and data
        return schema
            .array(Event)
            .parse(answered)
            .map((event) => this.#open(event));
    }

    /** Fold some scopes' events into a series per group of key values. */
    async series(filter: EventFilter, request: SeriesRequest<Shape>): Promise<Series[]> {
        // fold through the event log, else through the session
        const source = this.#source;
        if (source instanceof EventStore) {
            return source.series(this.kind, filter, request);
        }
        const { measure, fold, group, step } = request;
        const answered = await source.events(this.kind.table, {
            verb: "series",
            filter: EventView.#filter(filter),
            series: {
                fold,
                ...(measure === undefined ? {} : { measure }),
                ...(group === undefined ? {} : { group: [...group] }),
                ...(step === undefined ? {} : { step }),
            },
        });

        return schema.array(SeriesRecord).parse(answered);
    }

    /** Write a filter in a session request's form. */
    static #filter(filter: EventFilter): Extract<EventRequest, { verb: "query" }>["filter"] {
        return {
            scopes: [...filter.scopes],
            ...(filter.where === undefined ? {} : { where: filter.where }),
        };
    }

    /** Open an answered event with the kind's keys and data. */
    #open(event: Event): Event<Shape, Data> {
        return {
            ...event,
            keys: this.kind.parseKeys(event.keys),
            data: this.kind.parseData(event.data),
        };
    }
}

/** Where a page of events starts and ends, and its order in time. */
export interface EventPageRequest {
    /** The previous page's last event. */
    readonly after?: EventCursor;
    /** The most events read. */
    readonly limit?: number;
    /** The order in time. */
    readonly order?: EventOrder;
}

/** The events appended after a sequence, as a session answers them. */
const EVENT_CHANGES = schema.object({
    /** The events in commit order. */
    events: schema.array(Event),
    /** The log sequence the read reached. */
    sequence: schema.number().int().min(0),
});

/** A page of events as a session answers it. */
const EVENT_PAGE = schema.object({
    /** The events. */
    events: schema.array(Event),
    /** Where the next page starts, absent at the end. */
    cursor: schema.object({ time: schema.number(), id: schema.string().min(1) }).exactOptional(),
});
