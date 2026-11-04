import type { DatabaseConnection } from "../database/index.ts";
import { DatabaseError } from "../error/index.ts";
import type { Change } from "../log/index.ts";
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
    /** The database with the inputs and the checkpoint. */
    readonly #database: DatabaseConnection;
    /** The projection's keys and work. */
    readonly #options: ProjectionOptions;

    /** Project rows over a database's input tables. */
    constructor(database: DatabaseConnection, options: ProjectionOptions) {
        // store the inputs, the database and the work
        this.name = options.name;
        this.tables = options.tables;
        this.#database = database;
        this.#options = options;
    }

    /** Project the keys the changes past the checkpoint touch, then move past them. */
    readonly take = async (reconciliation: Reconciliation): Promise<void> => {
        // read the touched keys and the position they reach, leaving a checkpoint without changes
        const touched = await this.#touched();
        if (touched === undefined) {
            return;
        }

        // project each, then move the checkpoint past them
        for (const key of touched.keys) {
            await this.project(key, reconciliation);
        }
        const now = this.#database.state.environment.clock.now();
        const reader = { name: this.name, tables: this.tables };
        await this.#database.log.advance(reader, touched.through, now + HOLD_MILLISECONDS);
    };

    /** Project one key's rows from its inputs as they are now. */
    async project(key: string, reconciliation: Reconciliation): Promise<void> {
        await this.#options.project(key, reconciliation);
    }

    /** Read the keys the changes past the checkpoint touch, every key without them, or none. */
    async #touched(): Promise<
        { readonly keys: readonly string[]; readonly through: number } | undefined
    > {
        // read the changes past the checkpoint
        const database = this.#database;
        const after = await database.log.checkpoint(this.name);
        if (after !== undefined) {
            try {
                const page = await database.log.read({ tables: this.tables, after });
                if (page.changes.length === 0) {
                    return undefined;
                }
                const touched = await Promise.all(
                    page.changes.map((change) => this.#options.keys(change, database)),
                );

                return { keys: [...new Set(touched.flat())], through: page.sequence };
            } catch (error) {
                // project everything once the log dropped the checkpoint's changes
                if (!DatabaseError.is(error, "CHANGES_COMPACTED")) {
                    throw error;
                }
            }
        }

        // project every key from the position now
        const { sequence } = await database.log.position();

        return { keys: await this.#options.all(database), through: sequence };
    }
}
