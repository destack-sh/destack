import { Change, type Cursor, type Reconciliation, TABLE, Transaction } from "@destack/db";
import type { Event } from "@destack/event";
import { Duration, schema } from "@destack/schema";
import type { ObjectType } from "./object.ts";
import type { ObjectStore } from "../server/store.ts";

/** One scope's page of a cursor's events: the scope, its events and the log range they span. */
export interface CursorPage {
    /** The scope the events append in. */
    readonly scope: string;
    /** The scope's events the page took, in commit order. */
    readonly events: readonly Event[];
    /** The log sequence the page starts after. */
    readonly after: number;
    /** The log sequence the page ends at, the claim a retry takes again. */
    readonly through: number;
    /** When the page was claimed, in Unix milliseconds, which every retry of it preserves. */
    readonly claimedAt: number;
}

/** What a cursor's read acts in: the store, the moving transaction, the time and the lease. */
export interface CursorContext {
    /** The store serving the events. */
    readonly store: ObjectStore;
    /** The transaction that moves the cursor past the page as it commits. */
    readonly transaction: Transaction;
    /** The time the page is taken at, in Unix milliseconds. */
    readonly now: number;
    /** The signal aborting the work once the reconciliation's lease is lost. */
    readonly signal: AbortSignal;
}

/** Act on one scope's page of a cursor's events, keying any outside effect by the page's range. */
export type CursorRead = (page: CursorPage, context: CursorContext) => Promise<void>;

/** How a package's cursor over a type's events holds and gathers them. */
export interface CursorOptions {
    /** How long the log retains events past the cursor once it moves. */
    readonly hold: Duration;
    /** How long its first untaken event waits before it is taken, none when absent. */
    readonly batch?: Duration;
}

/** A package's position in a type's events, handing each page past it to a read exactly once. */
export class ObjectCursor implements Cursor {
    /** The cursor's name in reports and its checkpoint's: the type's events table and the name. */
    readonly name: string;
    /** The type's events table, whose appends the cursor takes. */
    readonly tables: readonly [ObjectType["table"]];
    /** How long its first untaken event waits before it takes the page, in milliseconds. */
    readonly batch: number;
    /** The store serving the events. */
    readonly #store: ObjectStore;
    /** The events type the cursor reads. */
    readonly #object: ObjectType;
    /** The read acting on each scope's page. */
    readonly #read: CursorRead;
    /** How long the log retains events past the cursor, in milliseconds. */
    readonly #hold: number;

    /** Read a type's events under a name of the reading package, page by page. */
    constructor(
        store: ObjectStore,
        object: ObjectType,
        name: string,
        read: CursorRead,
        options: CursorOptions,
    ) {
        // name its checkpoint by the type's events table, unique per type, and the cursor
        this.name = `${object.table[TABLE].sqlName}/${name}`;
        this.tables = [object.table];
        this.batch = options.batch === undefined ? 0 : Duration.milliseconds(options.batch);
        this.#store = store;
        this.#object = object;
        this.#read = read;
        this.#hold = Duration.milliseconds(options.hold);
    }

    /** Claim the page through the log's end unless another host claimed it, and take it. */
    readonly take = async (reconciliation: Reconciliation): Promise<void> => {
        // place the cursor at the log's oldest retained change unless placed
        const { database, clock } = this.#store;
        const now = clock.now();
        await database.log.place(this, now + this.#hold);

        // claim the page through the log's end, leaving one another host moved past
        const after = (await database.log.checkpoint(this.name)) ?? 0;
        const { sequence } = await database.log.position();
        const claim = await database.log.claim(this, after, sequence, now, now + this.#hold);
        if (claim !== undefined) {
            await this.#take(now, reconciliation.signal);
        }
    };

    /** Hand the claimed page to the read scope by scope, moving past it in the same commit. */
    async #take(now: number, signal: AbortSignal): Promise<void> {
        const { database, events } = this.#store;
        await Transaction.run(
            database,
            async (transaction) => {
                // read the place and the claim as this attempt sees them, done once another host moved on
                const after = (await transaction.database.log.checkpoint(this.name)) ?? 0;
                const claim = await transaction.database.log.claimed(this.name);
                if (claim === undefined) {
                    return;
                }

                // read the scopes the claimed page appended in, in commit order
                const { through, claimedAt } = claim;
                const changes = await transaction.database.log.range(
                    { tables: this.tables, after },
                    through,
                );
                const scopes = [
                    ...new Set(
                        changes
                            .filter((change) => change.operation === "insert")
                            .map((change) => schema.string().parse(Change.image(change)["scope"])),
                    ),
                ];

                // act on each scope's events, then move past the page as the transaction commits
                const kind = this.#object.eventKind();
                const context = { store: this.#store, transaction, now, signal };
                for (const scope of scopes) {
                    const page = await events.range(kind, { scopes: [scope] }, after, through);
                    if (page.length > 0) {
                        await this.#read(
                            { scope, events: page, after, through, claimedAt },
                            context,
                        );
                    }
                }
                transaction.record((committed) =>
                    committed.log.move(this, after, through, now + this.#hold),
                );
            },
            signal,
        );
    }
}
