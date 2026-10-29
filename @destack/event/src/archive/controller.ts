import { Change } from "@destack/db";
import type { ReconcileController } from "@destack/db";
import type { EventKind } from "../kind/kind.ts";
import type { EventArchive } from "./archive.ts";
import { present } from "@destack/schema";
import { Clock } from "@destack/environment";

/** How often a quiet scope's segments are compacted and checked for retention: daily. */
const UPKEEP_MILLISECONDS = 24 * 60 * 60 * 1000;

/** Flushes, compacts and expires each scope's segments as their policies come due. */
export class ArchiveController implements ReconcileController {
    /** The controller's name in reports. */
    readonly name = "event-segments";
    /** The kinds' hot tables, whose appends list the scopes to flush. */
    readonly watches;
    /** The segments of the store's scopes. */
    readonly #archive: EventArchive;
    /** The kinds by their key. */
    readonly #kinds: ReadonlyMap<string, EventKind>;
    /** The clock segments flush, compact and expire by. */
    readonly #clock: Clock;
    /** When this controller last swept each kind and scope for orphaned files, by key. */
    readonly #swept = new Map<string, number>();
    /** The hot events each kind and scope held at its last reconcile and appended since, by key. */
    readonly #held = new Map<string, number>();

    /** Maintain the segments of kinds. */
    constructor(archive: EventArchive, kinds: readonly EventKind[], clock: Clock) {
        // index the kinds and watch their hot tables
        this.#archive = archive;
        this.#kinds = new Map(kinds.map((kind) => [kind.key, kind]));
        this.watches = kinds.map((kind) => kind.table);
        this.#clock = clock;
    }

    /** Count an append, taking its scope at its first hot event or once it fills a flush. */
    keys(change: Change): readonly string[] {
        // take nothing of other tables or rows without a scope
        const kind = [...this.#kinds.values()].find((each) => Change.of(change, each.table));
        const scope: unknown = Change.image(change)["scope"];
        if (kind === undefined || typeof scope !== "string") {
            return [];
        }

        // count the append, taking the scope unreconciled, at its first hot event, or at a flush's worth
        const key = `${kind.key} ${scope}`;
        const held = this.#held.get(key);
        if (held !== undefined) {
            this.#held.set(key, held + 1);
        }

        return held === undefined || held === 0 || held + 1 >= kind.policy.flush.maxRows
            ? [key]
            : [];
    }

    /** List every kind and scope with hot events or segments. */
    async list(): Promise<readonly string[]> {
        const listed = await Promise.all(
            [...this.#kinds.values()].map(async (kind) =>
                (await this.#archive.scopes(kind)).map((scope) => `${kind.key} ${scope}`),
            ),
        );

        return listed.flat();
    }

    /** Maintain a scope's segments, returning the wait until the next due work. */
    async reconcile(key: string): Promise<number> {
        // find the kind and scope of the key
        const separator = key.indexOf(" ");
        const kind = present(this.#kinds.get(key.slice(0, separator)), `the event kind of ${key}`);
        const scope = key.slice(separator + 1);

        // flush while the policy says so, then compact and expire
        const now = this.#clock.now();
        let due = await this.#archive.flushDue(kind, scope, now);
        while (due.isDue && (await this.#archive.flush(kind, scope, now))) {
            due = await this.#archive.flushDue(kind, scope, now);
        }
        this.#held.set(key, due.events);
        await this.#archive.compact(kind, scope, now);
        await this.#archive.expire(kind, scope, now);

        // sweep orphaned files once per upkeep, as listing a bucket costs a request per page
        const sweptAt = this.#swept.get(key);
        if (sweptAt === undefined || sweptAt + UPKEEP_MILLISECONDS <= now) {
            await this.#archive.sweep(kind, scope, now);
            this.#swept.set(key, now);
        }

        // look again once the oldest hot event comes due, or at the next upkeep
        return Math.max(0, Math.min(due.at ?? Infinity, now + UPKEEP_MILLISECONDS) - now);
    }
}
