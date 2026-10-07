import { present } from "@destack/schema";
import type { Destination } from "./relay.ts";

/** How long an edge keeps a name's destination: one minute, as short DNS TTLs keep a change visible within a minute. */
const NAME_TTL_MILLISECONDS = 60_000;

/** The most names an edge keeps: 10,000 entries of about 200 bytes, 2 MB of a Worker isolate's 128 MB. */
const MAX_NAMES = 10_000;

/** A destination an edge keeps for a name until it expires. */
interface Entry {
    /** The name's destination. */
    readonly destination: Destination;
    /** When the entry expires, in UTC epoch milliseconds. */
    readonly expiresAt: number;
}

/** The destinations of names an edge Worker's isolate resolved recently, so a request for a kept name reads no database. */
export class DestinationCache {
    /** The kept destinations, oldest first. */
    readonly #entries = new Map<string, Entry>();

    /** Read a name's kept destination, absent once it expires. */
    get(name: string, now = Date.now()): Destination | undefined {
        const entry = this.#entries.get(name);
        if (entry === undefined || entry.expiresAt <= now) {
            this.#entries.delete(name);

            return undefined;
        }

        return entry.destination;
    }

    /** Keep a name's destination, dropping the oldest name beyond the limit. */
    set(name: string, destination: Destination, now = Date.now()): void {
        // keep the newest entry last
        this.#entries.delete(name);
        this.#entries.set(name, { destination, expiresAt: now + NAME_TTL_MILLISECONDS });

        // drop the oldest beyond the limit
        if (this.#entries.size > MAX_NAMES) {
            const [oldest] = this.#entries.keys();
            this.#entries.delete(present(oldest, "the oldest name"));
        }
    }

    /** Drop a name's destination, as its cell answers it misdirected. */
    delete(name: string): void {
        this.#entries.delete(name);
    }
}
