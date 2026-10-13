import type { Row } from "@destack/db";
import { EventCursor, type EventOrder } from "@destack/event";
import { CallError } from "@destack/runtime/error";
import { Page } from "../method/page.ts";
import { Replica } from "@destack/sync";
import type { Call } from "../method/call.ts";
import type { ListInput } from "../method/record.ts";

/** The list input fields only the SQL dataflow serves, which the event log refuses. */
const DATAFLOW_FIELDS = ["extras", "with", "aggregate", "deleted", "at", "branch"] as const;

/** The event log behind an events type: `create` appends to it, `list` and `get` read it. */
export const EventLog = {
    /** Append created rows as events of the scope. */
    async append(call: Call, rows: readonly Row[]): Promise<void> {
        // read each row as its event
        const kind = call.object.eventKind();
        const events = rows.map((row) => kind.event(row));

        // append events kept exactly once as the call commits
        const store = call.requireStore().requireEvents();
        if (kind.delivery === "exactly-once") {
            call.requireTransaction().record((committing) =>
                store.append(kind, events, committing),
            );
        }
        // append events kept at most once alone, outliving a failed call
        else {
            await store.append(kind, events);
        }
    },

    /** Read a page of the scope's events the caller may read, in time order. */
    async page(call: Call, input: ListInput): Promise<{ items: Row[]; cursor: string | null }> {
        // refuse what only the SQL dataflow serves
        const kind = call.object.eventKind();
        const unserved = DATAFLOW_FIELDS.find((name) => input[name] !== undefined);
        if (unserved !== undefined) {
            throw new CallError("BAD_REQUEST", {
                message: `${call.object.name} lists from the event log, which reads no ${unserved}`,
            });
        }

        // read one event past the page from the log, the condition pruning segments
        const { where, orderBy, cursor, limit } = input;
        const listing = new Page(
            {
                ...(cursor === undefined ? {} : { cursor }),
                ...(limit === undefined ? {} : { limit }),
            },
            [call.object.name, call.scope],
            EventCursor,
        );
        const read = await call
            .requireStore()
            .requireEvents()
            .query(
                kind,
                {
                    scopes: await EventLog.scopes(call),
                    ...(where === undefined ? {} : { where }),
                },
                {
                    ...(listing.after === undefined ? {} : { after: listing.after }),
                    limit: listing.limit + 1,
                    order: orderOf(call, orderBy),
                },
            );

        // cut the page, then keep the rows the caller may read
        const page = listing.result(
            read.events.map((event) => kind.row(event)),
            (row) => EventCursor.parse(row),
        );
        const items = await call.requireAuthorization().keep(page.items, call.permission());

        return { items, cursor: page.cursor };
    },

    /** Read one event of the scope by its identifier, hot or flushed, absent if none. */
    async row(call: Call, id: string): Promise<Row | null> {
        const kind = call.object.eventKind();
        const event = await call
            .requireStore()
            .requireEvents()
            .get(kind, { scopes: await EventLog.scopes(call), id });

        return event === undefined ? null : kind.row(event);
    },

    /** List the scopes a read of a scope's events covers: its own, and the copies kept for it. */
    async scopes(call: Call): Promise<readonly [string, ...string[]]> {
        const copied = await Replica.scopes(call.view().database, call.object.table, call.scope);

        return [call.scope, ...copied.filter((scope) => scope !== call.scope)];
    },
};

/** Read a list's order as the event log's: by time alone, newest first unless asked otherwise. */
function orderOf(call: Call, orderBy: ListInput["orderBy"]): EventOrder {
    // read the time's direction, refusing any other field
    const [field, ...others] = Object.entries(orderBy ?? { time: "desc" });
    if (field === undefined || field[0] !== "time" || others.length > 0) {
        throw new CallError("BAD_REQUEST", {
            message: `${call.object.name} lists in time order alone`,
        });
    }

    return field[1] === "asc" ? "ascending" : "descending";
}
