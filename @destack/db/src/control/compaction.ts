import type { DatabaseConnection } from "../database/index.ts";
import type { ReconcileController } from "./control.ts";
import { Clock } from "@destack/environment";

/** How long a database retains windowed changes: a week. */
const CHANGE_WINDOW_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** The least time between compactions, an hour, so a log a live slot holds never spins. */
const RETRY_MILLISECONDS = 60 * 60_000;

/** The one key the compaction controller reconciles: the database's log. */
const LOG = "log";

/** Compacts a database's log once its oldest windowed change leaves the window. */
export class CompactionController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "compaction";
    /** The database whose log it compacts. */
    readonly #database: DatabaseConnection;
    /** The clock, its store's. */
    readonly #clock: Clock;

    /** Compact a database's log on its store's clock. */
    constructor(database: DatabaseConnection, clock: Clock) {
        this.#database = database;
        this.#clock = clock;
    }

    /** List the log, compacted on start. */
    async list(): Promise<readonly string[]> {
        return [LOG];
    }

    /** Compact the log, and look again once its oldest windowed change leaves the window. */
    async reconcile(): Promise<number> {
        // drop the changes older than the window, then read the oldest left
        const now = this.#clock.now();
        const oldest =
            (await this.#database.log.compact(now - CHANGE_WINDOW_MILLISECONDS, now)) ?? now;

        return Math.max(oldest + CHANGE_WINDOW_MILLISECONDS - now, RETRY_MILLISECONDS);
    }
}
