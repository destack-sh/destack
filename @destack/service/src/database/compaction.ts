import type { DatabaseConnection } from "@destack/db";
import type { Controller } from "../control/index.ts";

/** How long a database keeps windowed changes: a week, longer than clients stay offline between syncs. */
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

    /** Compact a database's log. */
    constructor(database: DatabaseConnection) {
        this.#database = database;
    }

    /** List the log, compacted on start. */
    async list(): Promise<readonly string[]> {
        return [LOG];
    }

    /** Compact the log, then look again after the compaction interval. */
    async reconcile(): Promise<number> {
        await this.#database.log.compact(Date.now() - CHANGE_WINDOW_MILLISECONDS);

        return COMPACTION_INTERVAL_MILLISECONDS;
    }
}
