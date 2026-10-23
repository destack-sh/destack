import { Change, type Table, Transaction } from "@destack/db";
import { type Event, type EventReader, EventTime } from "@destack/event";
import type { ReconcileController, Reconciliation } from "@destack/db";
import { Duration, schema } from "@destack/schema";
import type { ObjectType } from "./object.ts";
import type { ObjectStore } from "../server/store.ts";

/** One page of a reader's events: its slot's key, the events and the log range they span. */
export interface ReaderPage {
    /** The key the reader's slot has, the scope the events append in. */
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
export class Consumer implements ReconcileController {
    /** The controller's name in reports: the type's and the reader's. */
    readonly name: string;
    /** The type's events table, whose appends select their keys. */
    readonly watches: readonly Table[];
    /** The store serving the events. */
    readonly #store: ObjectStore;
    /** The events type the reader reads. */
    readonly #object: ObjectType;
    /** The reader's name on the type. */
    readonly #reader: string;
    /** How long a page's first append waits before the reader takes the page, in milliseconds. */
    readonly #batch: number;
    /** The handler acting on each page. */
    readonly #handler: ReaderHandler;

    /** Consume a type's events under one of its declared readers by scope, page by page. */
    constructor(store: ObjectStore, object: ObjectType, reader: string, handler: ReaderHandler) {
        // refuse a reader by row, as its slots key no scope of events
        const declared = object.definition.readers?.[reader];
        if (declared === undefined || declared.by !== "scope") {
            throw new TypeError(`object ${object.name} has no reader ${reader} by scope`);
        }

        // store the store, the type, the reader, its batch and the handler
        this.name = `${object.name}/${reader}`;
        this.watches = [object.table];
        this.#store = store;
        this.#object = object;
        this.#reader = reader;
        this.#batch = declared.batch === undefined ? 0 : Duration.milliseconds(declared.batch);
        this.#handler = handler;
    }

    /** Select the scope an appended event belongs to. */
    keys(change: Change): readonly string[] {
        return change.operation === "insert" && Change.of(change, this.#object.table)
            ? [schema.string().parse(Change.image(change)["scope"])]
            : [];
    }

    /** List the scopes with a slot, which their first event placed. */
    async list(): Promise<readonly string[]> {
        return this.#object.reader(this.#reader).keys(this.#store.database);
    }

    /** Claim a scope's page once its first append is a batch old, and take the claimed page. */
    async reconcile(key: string, reconciliation: Reconciliation): Promise<number | undefined> {
        // read the open page's first append, timed by the appender's clock, and its end
        const { database, clock } = this.#store;
        const reader = this.#object.reader(this.#reader).at(key);
        const now = clock.now();
        const after = await reader.after(database);
        const open = await database.log.read({
            tables: [this.#object.table],
            scopes: [key],
            after,
        });
        const first = open.changes.find((change) => change.operation === "insert");
        const age =
            first === undefined
                ? undefined
                : now - EventTime.milliseconds(schema.number().parse(Change.image(first)["time"]));

        // pass a page without appends and look again, wait out a young page, and claim an old one unless another host moved on
        if (age === undefined && open.sequence > after) {
            await reader.move(after, open.sequence, now, database);

            return 0;
        } else if (age === undefined) {
            return undefined;
        } else if (this.#batch > 0 && age < this.#batch) {
            return this.#batch - age;
        } else if ((await reader.claim(after, open.sequence, now, database)) === undefined) {
            return 0;
        } else {
            return this.#take(key, reader, now, reconciliation.signal);
        }
    }

    /** Hand the claimed page to the handler and move past it in the same commit. */
    async #take(
        key: string,
        reader: EventReader,
        now: number,
        signal: AbortSignal,
    ): Promise<number> {
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

                // act on the claimed page, then move past it as the transaction commits
                const kind = this.#object.eventKind();
                const { through, claimedAt } = claim;
                const page = await events.range(kind, { scopes: [key] }, after, through);
                await this.#handler(
                    { key, events: page, after, through, claimedAt },
                    { store: this.#store, transaction, now, signal },
                );
                transaction.record((committed) => reader.move(after, through, now, committed));
            },
            signal,
        );

        return 0;
    }
}
