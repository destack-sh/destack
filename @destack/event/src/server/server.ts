import type { JsonCondition } from "@destack/db";
import { ServiceError } from "@destack/service";
import {
    implement,
    type ServiceAccess,
    type ServiceContext,
    type ServiceImplementation,
} from "@destack/service/server";
import { type Event, type EventKind, EventTime } from "../kind/kind.ts";
import type { EventFilter } from "../query/query.ts";
import { eventService, type EventSelection, FilterText } from "../service/service.ts";
import type { Series } from "../store/series.ts";
import type { EventStore } from "../store/store.ts";
import { authorizeRead, type ShownEvent, type UnmaskedRead } from "./read.ts";

/** The events one query page returns unless the request asks for another number. */
const PAGE_EVENTS = 100;

/** What a host serves its store's events with: who may route copies to it, and the record of unmasked reads. */
export interface EventServiceOptions {
    /** The store whose events are read and received. */
    readonly store: EventStore;
    /** Admit a caller routing copies of a kind here, nobody when absent. */
    readonly admit?: (
        context: ServiceContext,
        kind: EventKind,
        events: readonly Event[],
    ) => Promise<void>;
    /** Record a read that showed personal values unmasked, absent where nobody unmasks them. */
    readonly unmasked?: (context: ServiceContext, read: UnmaskedRead) => Promise<void>;
}

/** Serve a store's events as each kind's access allows, and receive routed copies. */
export function serveEvents(options: EventServiceOptions) {
    // decide each read by its kind's access, recording unmasked ones
    const { store, unmasked } = options;
    const implementation = implement(eventService.router).$context<ServiceContext>();

    return {
        query: implementation.query.handler(async ({ input, context }) => {
            // read the page as the caller may read the kind
            const kind = store.kind(input.kind);
            const read = await authorizeRead(context, {
                store,
                kind,
                selection: input,
                operation: "query",
                unmasked,
            });
            const page = await store.query(kind, filterOf(input, read.where), {
                ...(input.cursor === undefined ? {} : { after: input.cursor }),
                limit: input.limit ?? PAGE_EVENTS,
                order: input.order ?? "descending",
            });

            return {
                events: page.events.map(read.redact),
                ...(page.cursor === undefined ? {} : { cursor: page.cursor }),
            };
        }),
        tail: implementation.tail.handler(async ({ input, context }) => {
            // follow from the log's position as the call starts, so nothing appended after it is missed
            const kind = store.kind(input.kind);
            const read = await authorizeRead(context, {
                store,
                kind,
                selection: input,
                operation: "tail",
                unmasked,
            });
            const { sequence } = await store.database.log.position();

            return revealed(
                store.tail(kind, filterOf(input, read.where), context.signal, sequence),
                read.redact,
            );
        }),
        series: implementation.series.handler(async ({ input, context }) => {
            // fold as the caller may read the kind
            const kind = store.kind(input.kind);
            const read = await authorizeRead(context, {
                store,
                kind,
                selection: input,
                operation: "series",
                unmasked,
            });
            const series = await store.series(kind, filterOf(input, read.where), {
                fold: input.fold,
                group: input.group,
                ...(input.measure === undefined ? {} : { measure: input.measure }),
                ...(input.step === undefined ? {} : { step: input.step }),
            });

            return { series: series.map(shown) };
        }),
        export: implementation.export.handler(async ({ input, context }) => {
            // stream the events that happened before the export started
            const kind = store.kind(input.kind);
            const read = await authorizeRead(context, {
                store,
                kind,
                selection: input,
                operation: "export",
                unmasked,
            });
            const before = Math.min(
                input.before ?? Number.POSITIVE_INFINITY,
                EventTime.of(Date.now()),
            );
            const filter = { ...filterOf(input, read.where), before };

            return exported(store.export(kind, filter), read.redact, context.signal);
        }),
        receive: implementation.receive.handler(async ({ input, context }) => {
            // admit the sender of the copies, then keep them once each
            context.requireAuthentication();
            const kind = store.kind(input.kind);
            if (options.admit === undefined) {
                throw new ServiceError("FORBIDDEN", {
                    message: `this host takes no routed ${input.kind} events`,
                });
            }
            await options.admit(context, kind, input.events);
            await store.receive(kind.key, input.events);

            return { received: input.events.length };
        }),
    };
}

/** Implement the event service on its own over a store, deciding reads by the policies a host's access holds. */
export function implementEvents(
    options: EventServiceOptions & { readonly access: ServiceAccess },
): ServiceImplementation {
    const implementation = implement(eventService.router).$context<ServiceContext>();

    return {
        service: eventService,
        access: options.access,
        router: implementation.router(serveEvents(options)),
    };
}

/** Read a selection's filter: its scope, its text, its time range, the read's condition and its own filter. */
function filterOf(
    selection: EventSelection & { readonly from?: number; readonly before?: number },
    condition: JsonCondition,
): EventFilter {
    // parse the filter's text, refusing one that does not parse
    let parsed: JsonCondition | undefined;
    try {
        parsed = FilterText.condition(selection.where);
    } catch (error) {
        throw new ServiceError("BAD_REQUEST", {
            message: error instanceof Error ? error.message : String(error),
        });
    }

    // combine it with the read's condition
    return {
        scope: selection.scope,
        where: { AND: [condition, ...(parsed === undefined ? [] : [parsed])] },
        ...(selection.text === undefined ? {} : { text: selection.text }),
        ...(selection.from === undefined ? {} : { from: selection.from }),
        ...(selection.before === undefined ? {} : { before: selection.before }),
    };
}

/** Answer a folded series as the service writes it. */
function shown(series: Series) {
    return { group: { ...series.group }, steps: [...series.steps] };
}

/** Show each event a tail yields as the caller may see it, ending when the reader stops. */
async function* revealed(
    stream: AsyncGenerator<{ readonly event: Event; readonly sequence: number }>,
    reveal: (event: Event) => ShownEvent,
): AsyncGenerator<ShownEvent> {
    for await (const { event } of stream) {
        yield reveal(event);
    }
}

/** Show each event an export yields as the caller may see it, stopping once the call is cancelled. */
async function* exported(
    stream: AsyncGenerator<Event>,
    reveal: (event: Event) => ShownEvent,
    signal: AbortSignal | undefined,
): AsyncGenerator<ShownEvent> {
    for await (const event of stream) {
        signal?.throwIfAborted();
        yield reveal(event);
    }
}
