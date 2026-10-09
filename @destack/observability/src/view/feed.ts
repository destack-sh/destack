import { type Event, EventCursor } from "@destack/event";
import {
    eventService,
    type EventQuery,
    type EventResult,
    type EventSelection,
} from "@destack/event/service";
import type { Client } from "@destack/service";
import { type Accessor, createMemo, onCleanup, useService } from "@destack/view";
import { PAGE_ROWS } from "./page.tsx";

/**
 * The pages of events a live feed keeps, dropping older ones as new ones arrive.
 *
 * At 1,000 logs/s of about 1 KB each, an uncapped feed holds about 3.6 GB after an hour.
 * Ten pages of 100 rows hold about 1 MB, the last second of such a stream.
 */
const LIVE_PAGES = 10;

/** A client of the event service. */
type EventClient = Client<(typeof eventService)["router"]>;

/** The events a feed follows, and the earliest time its pages read. */
export type EventRange = EventSelection & Pick<EventQuery, "from">;

/** Events newest first, with the read of the next older page. */
export interface EventList {
    /** The events, newest first. */
    readonly events: readonly Event[];
    /** Read the next older page, absent once no older event remains. */
    readonly more: (() => void) | undefined;
}

/** A selection's events newest first: a first read, the events its tail follows, and older pages on demand. */
export class EventFeed {
    /** The event service. */
    readonly #client: EventClient;
    /** The events followed. */
    readonly #selection: EventSelection;
    /** The earliest time the pages read, in Unix microseconds, the oldest kept when absent. */
    readonly #from: number | undefined;
    /** The events, newest first. */
    #events: readonly Event[] = [];
    /** Whether older events remain past the oldest held. */
    #isPartial = false;
    /** Whether a read of an older page paused the tail, which stops adding events. */
    #isPaused = false;
    /** The failure of the tail or of an older page, which the follower throws. */
    #failure: { readonly error: unknown } | undefined;
    /** Wake the follower after a change. */
    #changed = Promise.withResolvers<void>();

    /** Follow a selection's events through the event service. */
    constructor(client: EventClient, range: EventRange) {
        // tail the selection, paging it from the range's start
        const { from, ...selection } = range;
        this.#client = client;
        this.#selection = selection;
        this.#from = from;
    }

    /** Read the page of events newest first, past a cursor when given. */
    page(cursor?: EventCursor): Promise<EventResult> {
        return this.#client.query({
            ...this.#selection,
            ...(this.#from === undefined ? {} : { from: this.#from }),
            ...(cursor === undefined ? {} : { cursor }),
            limit: PAGE_ROWS,
            order: "descending",
        });
    }

    /** Take a first read, follow the tail while live, and yield the events after each change until the signal aborts. */
    async *follow(
        first: () => Promise<EventResult>,
        isLive: boolean,
        signal: AbortSignal,
    ): AsyncGenerator<EventList> {
        // open the tail before the first read, so an event committing between them shows once
        const tail = isLive ? await this.#client.tail(this.#selection, { signal }) : undefined;
        const read = await first();
        this.#isPartial = read.cursor !== undefined;
        this.#add(read.events);

        // follow the tail, waking the follower to stop once aborted
        if (tail !== undefined) {
            void this.#tail(tail, signal);
        }
        signal.addEventListener("abort", () => this.#changed.resolve(), { once: true });

        // yield the events, then wait for the next change
        while (!signal.aborted) {
            yield this.#list();
            await this.#changed.promise;
            this.#changed = Promise.withResolvers();

            // throw a failure of the tail or of an older page
            if (this.#failure !== undefined) {
                throw this.#failure.error;
            }
        }
    }

    /** List the events with the read of the next older page while older events remain. */
    #list(): EventList {
        const oldest = this.#events.at(-1);
        const more =
            this.#isPartial && oldest !== undefined ? () => void this.#more(oldest) : undefined;

        return { events: this.#events, more };
    }

    /** Add each event a tail yields, failing the follower once the tail fails or ends before the signal aborts. */
    async #tail(tail: AsyncIterable<Event>, signal: AbortSignal): Promise<void> {
        // add each tailed event, closing the tail once paused
        try {
            for await (const event of tail) {
                if (this.#isPaused) {
                    return;
                }
                this.#add([event]);

                // keep the newest pages, dropping the oldest beyond them
                const rows = LIVE_PAGES * PAGE_ROWS;
                if (this.#events.length > rows) {
                    this.#events = this.#events.slice(0, rows);
                    this.#isPartial = true;
                }
                this.#changed.resolve();
            }
        } catch (error) {
            // fail the follower unless aborted
            if (!signal.aborted) {
                this.#fail(error);
            }

            return;
        }

        // fail a tail the service ended
        if (!signal.aborted) {
            this.#fail(new Error("the event service ended the tail"));
        }
    }

    /** Pause the tail and read the page of events older than the oldest held, failing the follower on a failed read. */
    async #more(oldest: Event): Promise<void> {
        this.#isPaused = true;
        try {
            const read = await this.page({ time: oldest.time, id: oldest.id });
            this.#isPartial = read.cursor !== undefined;
            this.#add(read.events);
            this.#changed.resolve();
        } catch (error) {
            this.#fail(error);
        }
    }

    /** Keep a failure and wake the follower to throw it. */
    #fail(error: unknown): void {
        this.#failure = { error };
        this.#changed.resolve();
    }

    /** Merge events into the held ones newest first, each once. */
    #add(events: readonly Event[]): void {
        const held = new Set(this.#events.map((event) => event.id));
        const added = events.filter((event) => !held.has(event.id));
        this.#events = [...this.#events, ...added].toSorted((left, right) =>
            EventCursor.compare(right, left),
        );
    }
}

/** Follow a selection's events newest first: its latest page, each new one as it commits while live, and older pages on demand. */
export function useEvents(
    range: Accessor<EventRange | undefined>,
    isLive: Accessor<boolean>,
): Accessor<EventList> {
    const client = useService(eventService);

    return createMemo((): EventList | AsyncIterable<EventList> => {
        // follow the current selection until it changes or the component goes
        const current = range();
        const live = isLive();
        const stopping = new AbortController();
        onCleanup(() => stopping.abort());
        if (current === undefined) {
            return { events: [], more: undefined };
        }
        const feed = new EventFeed(client, current);

        return feed.follow(() => feed.page(), live, stopping.signal);
    });
}
