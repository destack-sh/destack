import type { DatabaseConnection } from "../database/index.ts";
import type { Controller } from "./control.ts";
import { Clock } from "@destack/environment";

/** How long a database retains windowed changes: a week. */
const CHANGE_WINDOW_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often a database compacts its log: hourly, a week of changes taking seconds to compact. */
const COMPACTION_INTERVAL_MILLISECONDS = 60 * 60_000;

/** The one key the compaction controller reconciles: the database's log. */
const LOG = "log";

/** Compacts a database's log once an hour, dropping windowed changes older than the window. */
export class CompactionController implements Controller {
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

    /** Compact the log and look again after the compaction interval. */
    async reconcile(): Promise<number> {
        const now = this.#clock.now();
        await this.#database.log.compact(now - CHANGE_WINDOW_MILLISECONDS, now);

        return COMPACTION_INTERVAL_MILLISECONDS;
    }
}
