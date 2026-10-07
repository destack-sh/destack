import { Change } from "@destack/db";
import type { Controller } from "@destack/service/control";
import type { EventKind } from "../kind/kind.ts";
import type { EventArchive } from "./archive.ts";

/** How often a scope's segments are compacted and checked against their retention without new events: daily. */
const UPKEEP_MILLISECONDS = 24 * 60 * 60 * 1000;

/** Flushes each scope's hot events of a store's kinds as their policy comes due, compacts small segments and expires old ones. */
export class ArchiveController implements Controller {
    /** The controller's name in reports. */
    readonly name = "event-segments";
    /** The kinds' hot tables, whose appends name the scopes to flush. */
    readonly watches;
    /** The segments of the store's scopes. */
    readonly #archive: EventArchive;
    /** The kinds by their key. */
    readonly #kinds: ReadonlyMap<string, EventKind>;
    /** Read the current time in Unix milliseconds. */
    readonly #now: () => number;

    /** Keep the segments of kinds. */
    constructor(archive: EventArchive, kinds: readonly EventKind[], now: () => number) {
        // index the kinds and watch their hot tables
        this.#archive = archive;
        this.#kinds = new Map(kinds.map((kind) => [kind.key, kind]));
        this.watches = kinds.map((kind) => kind.table);
        this.#now = now;
    }

    /** Name the kind and scope an appended event belongs to. */
    keys(change: Change): readonly string[] {
        const kind = [...this.#kinds.values()].find((each) => Change.of(change, each.table));
        const scope: unknown = Change.image(change)["scope"];

        return kind === undefined || typeof scope !== "string" ? [] : [`${kind.key} ${scope}`];
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

    /** Flush a scope's due events, compact its small segments and expire its old ones, returning the wait until its next flush or upkeep. */
    async reconcile(key: string): Promise<number | undefined> {
        // find the kind and scope the key names
        const [name = "", scope = ""] = key.split(" ");
        const kind = this.#kinds.get(name);
        if (kind === undefined) {
            return undefined;
        }

        // flush while the policy says so, then compact and expire
        const now = this.#now();
        let due = await this.#archive.flushDue(kind, scope, now);
        while (due.isDue && (await this.#archive.flush(kind, scope, now))) {
            due = await this.#archive.flushDue(kind, scope, now);
        }
        await this.#archive.compact(kind, scope, now);
        await this.#archive.expire(kind, scope, now);

        // look again once the oldest hot event comes due, or at the next upkeep
        return Math.max(0, Math.min(due.at ?? Infinity, now + UPKEEP_MILLISECONDS) - now);
    }
}
