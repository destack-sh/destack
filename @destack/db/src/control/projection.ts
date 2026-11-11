import type { DatabaseConnection } from "../database/index.ts";
import { DatabaseError } from "../error/index.ts";
import { Change } from "../log/index.ts";
import type { Table } from "../table/index.ts";
import type { Cursor, Reconciliation } from "./control.ts";

/** How long the log keeps changes past a stopped projection's checkpoint: a week. */
const HOLD_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** What a projection declares: its inputs, the keys their changes touch, and how a key projects. */
export interface ProjectionOptions {
    /** The projection's name in reports and locks, and its checkpoint's. */
    readonly name: string;
    /** The input tables whose changes touch keys. */
    readonly tables: readonly Table[];
    /** The input tables whose observations touch keys too, none by default. */
    readonly observed?: readonly Table[];
    /** List the keys a changed input row touches. */
    readonly keys: (change: Change, database: DatabaseConnection) => Promise<readonly string[]>;
    /** List every key, projected first or once the log dropped the checkpoint's changes. */
    readonly all: (database: DatabaseConnection) => Promise<readonly string[]>;
    /** Project one key's rows from its inputs as they are now. */
    readonly project: (key: string, reconciliation: Reconciliation) => Promise<void>;
}

/** Rows projected from input tables, kept current by the input changes past one checkpoint. */
export class Projection implements Cursor {
    /** The projection's name and its checkpoint's. */
    readonly name: string;
    /** The input tables. */
    readonly tables: readonly Table[];
    /** The input tables whose observations touch keys too. */
    readonly observed: readonly Table[];
    /** The database with the inputs and the checkpoint. */
    readonly #database: DatabaseConnection;
    /** The projection's keys and work. */
    readonly #options: ProjectionOptions;
    /** The position the projection took changes through, read from its checkpoint once. */
    #after: number | undefined;
    /** The checkpoint as the projection last read or wrote it, none before the first. */
    #checkpointed: number | undefined;

    /** Project rows over a database's input tables. */
    constructor(database: DatabaseConnection, options: ProjectionOptions) {
        // store the inputs, the database and the work
        this.name = options.name;
        this.tables = options.tables;
        this.observed = options.observed ?? [];
        this.#database = database;
        this.#options = options;
    }

    /** Project the keys the changes past the checkpoint touch, page by page until caught up. */
    readonly take = async (reconciliation: Reconciliation): Promise<void> => {
        // take the changes past the position, page by page
        for (
            let touched = await this.#touched();
            touched !== undefined;
            touched = touched.isLast ? undefined : await this.#touched()
        ) {
            // project each key, then move the position past them
            for (const key of touched.keys) {
                await this.project(key, reconciliation);
            }
            this.#after = touched.through;
        }
    };

    /** Write the position as the checkpoint once it moved, as the loop goes idle or stops. */
    readonly settle = async (): Promise<void> => {
        // leave a position never taken or written already
        const after = this.#after;
        if (after === undefined || after === this.#checkpointed) {
            return;
        }

        // move the checkpoint from where it was read, holding the log's changes past it
        const now = this.#database.state.environment.clock.now();
        try {
            await this.#database.log.move(
                this,
                this.#checkpointed ?? 0,
                after,
                now + HOLD_MILLISECONDS,
            );
            this.#checkpointed = after;
        }
        // forget a position another instance moved past, reading the checkpoint again on the next take
        catch (error) {
            if (!DatabaseError.is(error, "UPDATE_CONFLICT")) {
                throw error;
            }
            this.#after = undefined;
            this.#checkpointed = undefined;
        }
    };

    /** Project one key's rows from its inputs as they are now. */
    async project(key: string, reconciliation: Reconciliation): Promise<void> {
        await this.#options.project(key, reconciliation);
    }

    /** Read the keys the changes past the checkpoint touch, every key without them, or none. */
    async #touched(): Promise<
        | { readonly keys: readonly string[]; readonly through: number; readonly isLast: boolean }
        | undefined
    > {
        // read the changes past the checkpoint, read from the log once and kept since
        const database = this.#database;
        if (this.#after === undefined) {
            this.#after = await database.log.checkpoint(this.name);
            this.#checkpointed = this.#after;
        }
        const after = this.#after;
        if (after !== undefined) {
            try {
                const page = await database.log.read({ tables: this.tables, after });
                if (page.changes.length === 0) {
                    return undefined;
                }
                const read = page.changes.filter(
                    (change) =>
                        !Change.isObservation(change) || this.observed.includes(change.table),
                );
                const touched = await Promise.all(
                    read.map((change) => this.#options.keys(change, database)),
                );

                return {
                    keys: [...new Set(touched.flat())],
                    through: page.sequence,
                    isLast: page.isLast,
                };
            } catch (error) {
                // project everything once the log dropped the checkpoint's changes
                if (!DatabaseError.is(error, "CHANGES_NOT_FOUND")) {
                    throw error;
                }
            }
        }

        // project every key from the position now
        const { sequence } = await database.log.position();

        return { keys: await this.#options.all(database), through: sequence, isLast: true };
    }
}
