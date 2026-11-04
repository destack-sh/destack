import { type Consumer, type Reconciliation, type Table, Transaction } from "@destack/db";
import type { Event, EventReader } from "@destack/event";
import { Duration } from "@destack/schema";
import type { ObjectType } from "./object.ts";
import type { ObjectStore } from "../server/store.ts";

/** One page of a reader's events: its checkpoint's key, the events and the log range they span. */
export interface ReaderPage {
    /** The key the reader's checkpoint has, the scope the events append in. */
    readonly key: string;

    /** The events the page took, in commit order. */
    readonly events: readonly Event[];
    /** The log sequence the page starts after. */
    readonly after: number;
    /** The log sequence the page ends at, the claim a retry takes again. */
    readonly through: number;
    /** When the page was claimed, in Unix milliseconds, which every retry of it preserves. */
    readonly claimedAt: number;
}

/** What a reader's handler acts in: the store, the moving transaction, the time and the lease. */
export interface ReaderContext {
    /** The store serving the events. */
    readonly store: ObjectStore;
    /** The transaction that moves the reader past the page as it commits. */
    readonly transaction: Transaction;
    /** The time the page is taken at, in Unix milliseconds. */
    readonly now: number;
    /** The signal aborting the work once the reconciliation's lease is lost. */
    readonly signal: AbortSignal;
}

/** Act on one page of a reader's events, keying any outside effect by the page's range. */
export type ReaderHandler = (page: ReaderPage, context: ReaderContext) => Promise<void>;

/** Hands each scope's pages of a type's events to a handler, each claimed page exactly once. */
export class EventConsumer implements Consumer {
    /** The reader's checkpoint prefix and its name in reports: the type's and the reader's. */
    readonly name: string;
    /** The type's events table, whose appends the consumer takes. */
    readonly reads: readonly Table[];
    /** A checkpoint per scope, which the scope's first event places. */
    readonly checkpoint = "scope";
    /** How long a page's first append waits before the reader takes the page, in milliseconds. */
    readonly batch: number;
    /** The store serving the events. */
    readonly #store: ObjectStore;
    /** The events type the reader reads. */
    readonly #object: ObjectType;
    /** The reader's name on the type. */
    readonly #reader: string;
    /** The handler acting on each page. */
    readonly #handler: ReaderHandler;

    /** Consume a type's events under one of its declared readers by scope, page by page. */
    constructor(store: ObjectStore, object: ObjectType, reader: string, handler: ReaderHandler) {
        // refuse a reader by row, as its checkpoints key no scope of events
        const declared = object.definition.readers?.[reader];
        if (declared === undefined || declared.by !== "scope") {
            throw new TypeError(`object ${object.name} has no reader ${reader} by scope`);
        }

        // store the store, the type, the reader, its batch and the handler
        this.name = object.reader(reader).name;
        this.reads = [object.table];
        this.batch = declared.batch === undefined ? 0 : Duration.milliseconds(declared.batch);
        this.#store = store;
        this.#object = object;
        this.#reader = reader;
        this.#handler = handler;
    }

    /** Claim a scope's page through the log's end unless another host claimed it, and take it. */
    async consume(scope: string, reconciliation: Reconciliation): Promise<void> {
        // leave a scope without a checkpoint, which its first append places
        const { database, clock } = this.#store;
        const reader = this.#object.reader(this.#reader).at(scope);
        const after = await database.log.checkpoint(reader.name);
        if (after === undefined) {
            return;
        }

        // claim the page through the log's end, leaving one another host moved past
        const now = clock.now();
        const { sequence } = await database.log.position();
        if ((await reader.claim(after, sequence, now, database)) !== undefined) {
            await this.#take(scope, reader, now, reconciliation.signal);
        }
    }

    /** Hand the claimed page to the handler and move past it in the same commit. */
    async #take(key: string, reader: EventReader, now: number, signal: AbortSignal): Promise<void> {
        const { database, events } = this.#store;
        await Transaction.run(
            database,
            async (transaction) => {
                // read the place and the claim as this attempt sees them, done once another host moved on
                const after = await reader.after(transaction.database);
                const claim = await reader.claimed(transaction.database);
                if (claim === undefined) {
                    return;
                }

                // act on a claimed page with the scope's events, then move past it as the transaction commits
                const kind = this.#object.eventKind();
                const { through, claimedAt } = claim;
                const page = await events.range(kind, { scopes: [key] }, after, through);
                if (page.length > 0) {
                    await this.#handler(
                        { key, events: page, after, through, claimedAt },
                        { store: this.#store, transaction, now, signal },
                    );
                }
                transaction.record((committed) => reader.move(after, through, now, committed));
            },
            signal,
        );
    }
}
