/** The most entries a tail keeps, four full pages: a few MB at a few hundred bytes an entry. */
const WINDOW_ENTRIES = 4000;

/** What a tail reads of a log entry: its sequence, transaction, table and scope. */
export interface TailEntry {
    /** The entry's sequence. */
    readonly sequence: number;
    /** The transaction that wrote it, null outside one. */
    readonly transaction: string | null;
    /** The SQL name of the table it changed. */
    readonly table: string;
    /** The changed row's scope. */
    readonly scope: string;
}

/** What one read of every table past a sequence returned. */
export interface TailRead<Entry extends TailEntry> {
    /** The entries read, in sequence order. */
    readonly entries: readonly Entry[];
    /** The log's latest sequence as the read saw it: its last entry's, or the head past it. */
    readonly sequence: number;
    /** The sequence compaction removed the changes up to. */
    readonly horizon: number;
    /** Whether the read reached the head, a full page leaving more to read. */
    readonly isLast: boolean;
}

/** A follower's entries from the tail: those of its tables and scopes after its sequence. */
export interface TailPage<Entry extends TailEntry> {
    /** The entries, in sequence order, at most the page's limit. */
    readonly entries: readonly Entry[];
    /** The sequence the page ends at: its last entry's when full, else the tail's. */
    readonly sequence: number;
    /** Whether the page reached the tail's end, so the follower waits for the next commit. */
    readonly isLast: boolean;
}

/** A process's latest read of a database's log, which every follower in the process shares. */
export class LogTail<Entry extends TailEntry> {
    /** The entries read, in sequence order, all after the window's start. */
    #entries: Entry[] = [];
    /** The sequence the window starts after, absent before the first read. */
    #start: number | undefined;
    /** The latest sequence read: the last entry's, or the head past it. */
    #sequence = 0;
    /** The sequence compaction removed the changes up to, as the last read saw it. */
    #horizon = 0;
    /** The commit generation the latest read began at, which a follower woken earlier may reuse. */
    #generation = -1;
    /** Whether the latest read reached the log's head, a full page leaving more to read. */
    #isCaughtUp = false;
    /** The read in flight, which every follower woken by the same commits joins. */
    #reading: Promise<void> | undefined;

    /** The latest sequence read. */
    get sequence(): number {
        return this.#sequence;
    }

    /** Read every table past the window once for followers woken at a generation, sharing reads. */
    async advance(
        generation: number,
        after: number,
        read: (after: number) => Promise<TailRead<Entry>>,
    ): Promise<void> {
        // wait out every read in flight, then reuse one that began at the generation or after
        while (this.#reading !== undefined) {
            await this.#reading;
        }
        if (this.#generation >= generation && this.#isCaughtUp) {
            return;
        }

        // read past the window, the first read starting it at the first follower's sequence
        this.#generation = generation;
        const start = this.#start ?? after;
        this.#reading = this.#read(Math.max(start, this.#sequence), start, read);
        await this.#reading;
    }

    /** Append one read's entries and bounds, ending the read in flight before anyone resumes. */
    async #read(
        after: number,
        start: number,
        read: (after: number) => Promise<TailRead<Entry>>,
    ): Promise<void> {
        // append the read's entries, keeping its head and horizon
        try {
            const { entries, sequence, horizon, isLast } = await read(after);
            this.#start ??= start;
            this.#isCaughtUp = isLast;
            this.#entries.push(...entries);
            this.#sequence = Math.max(this.#sequence, sequence);
            this.#horizon = horizon;
            this.#trim();
        }
        // end the read in flight, failed or not
        finally {
            this.#reading = undefined;
        }
    }

    /** Select a follower's page after a sequence, absent when the window starts after it. */
    page(
        after: number,
        tables: ReadonlySet<string>,
        scopes: ReadonlySet<string> | undefined,
        limit: number,
    ): TailPage<Entry> | undefined {
        // leave a follower behind the window or the horizon to read on its own
        if (this.#start === undefined || after < this.#start || after < this.#horizon) {
            return undefined;
        }

        // take the follower's entries, completing the last transaction of a full page
        const selected = this.#entries.filter(
            (entry) =>
                entry.sequence > after &&
                tables.has(entry.table) &&
                (scopes === undefined || scopes.has(entry.scope)),
        );
        const full = selected.slice(0, limit);
        const last = full.at(-1);
        const entries =
            full.length === limit && last !== undefined && last.transaction !== null
                ? [
                      ...full,
                      ...selected
                          .slice(limit)
                          .filter((entry) => entry.transaction === last.transaction),
                  ]
                : full;

        // end a full page at its last entry, else at the tail
        const isLast = full.length < limit;
        const end = entries.at(-1)?.sequence ?? after;

        return { entries, sequence: isLast ? Math.max(end, this.#sequence) : end, isLast };
    }

    /** Forget the window, as a new epoch or a restore leaves it stale. */
    reset(): void {
        // empty the window and its bounds
        this.#entries = [];
        this.#start = undefined;
        this.#sequence = 0;
        this.#horizon = 0;
        this.#generation = -1;
        this.#isCaughtUp = false;
    }

    /** Drop the oldest entries past the window's bound, moving its start up. */
    #trim(): void {
        // drop the excess, starting the window after the last dropped entry
        const excess = this.#entries.length - WINDOW_ENTRIES;
        const dropped = excess > 0 ? this.#entries.splice(0, excess) : [];
        const last = dropped.at(-1);
        if (last !== undefined) {
            this.#start = last.sequence;
        }
    }
}
