import { asc, eq, inArray, lte, Now } from "@destack/db";
import type { Controller } from "@destack/db/database";
import type { Digest } from "@destack/schema";
import { BuildStore } from "../store/store.ts";
import { buildCache, buildRoot } from "../store/table.ts";
import { BuildCacheEntry } from "./entry.ts";

/** How long a cache entry or a used build lasts past its last use: a week. */
export const CACHE_MILLISECONDS = 7 * 24 * 60 * 60_000;

/** How often a store records a use: daily, a seventh of a cache entry's or build's lifetime. */
export const USE_MILLISECONDS = 24 * 60 * 60_000;

/** The prefix of the root a cache entry retains its blobs by, followed by its key. */
export const CACHE_ROOT_PREFIX = "cache:";

/** A cache key's build, and how the cache answered: stored, built now, or joined. */
export interface BuildCacheOutcome {
    /** The build's manifest digest. */
    readonly manifest: Digest;
    /** How the cache answered the key. */
    readonly cache: "hit" | "miss" | "joined";
}

/** A build store's cache: entries by key, each retaining what it references until unused a week. */
export class BuildCache {
    /** The store whose blobs the entries reference. */
    readonly #store: BuildStore;
    /** The builds of cache keys in flight, so callers asking at once build each key once. */
    readonly #building = new Map<Digest, Promise<BuildCacheOutcome>>();

    /** Cache over a build store. */
    constructor(store: BuildStore) {
        this.#store = store;
    }

    /** Read a cache key's entry, recording its use once a day, absent for a miss. */
    async get(key: Digest): Promise<BuildCacheEntry | undefined> {
        // read the entry, a root of what it references while it lives
        const [row] = await this.#store.database
            .select({ entry: buildCache.entry, usedAt: buildCache.usedAt })
            .from(buildCache)
            .where(eq(buildCache.key, key));

        // record a hit's use once a day
        const now = this.#store.environment.clock.now();
        if (row !== undefined && now - row.usedAt >= USE_MILLISECONDS) {
            await this.#store.database
                .update(buildCache)
                .set({ usedAt: now })
                .where(eq(buildCache.key, key));
        }

        return row?.entry;
    }

    /** Store an entry under a cache key, once the manifest or files it references are stored. */
    async set(key: Digest, entry: BuildCacheEntry): Promise<void> {
        // record the entry at the use time
        const usedAt = this.#store.environment.clock.now();
        await this.#store.database.transaction(async (transaction) => {
            await transaction
                .insert(buildCache)
                .values({ key, entry, usedAt })
                .onConflictDoUpdate({ target: buildCache.key, set: { entry, usedAt } });

            // retain what it references under its root
            await BuildStore.replaceRoot(
                transaction,
                `${CACHE_ROOT_PREFIX}${key}`,
                new Set(BuildCacheEntry.digests(entry)),
            );
        });
    }

    /** Take the build a key caches, else build and cache it once for all callers at once. */
    build(key: Digest, build: () => Promise<Digest>): Promise<BuildCacheOutcome> {
        // join a build of the key in flight
        const flying = this.#building.get(key);
        if (flying !== undefined) {
            return flying.then(({ manifest }) => ({ manifest, cache: "joined" }));
        }

        // build the key once, forgetting the build once it settles
        const building = (async (): Promise<BuildCacheOutcome> => {
            // take the cached build, else build and cache its manifest
            const entry = await this.get(key);
            if (entry?.kind === "build") {
                return { manifest: entry.manifest, cache: "hit" };
            }
            const built = await build();
            await this.set(key, { kind: "build", manifest: built });

            return { manifest: built, cache: "miss" };
        })();
        this.#building.set(key, building);
        void building.finally(() => this.#building.delete(key)).catch(() => undefined);

        return building;
    }

    /** Drop cache entries unused for a week with their roots, answering the next one's end. */
    async expire(): Promise<number | undefined> {
        // drop the unused entries and their roots, which retires what only they kept
        const now = this.#store.environment.clock.now();
        await this.#store.database.transaction(async (transaction) => {
            const unused = await transaction
                .delete(buildCache)
                .where(lte(buildCache.usedAt, now - CACHE_MILLISECONDS))
                .returning({ key: buildCache.key });
            await transaction.delete(buildRoot).where(
                inArray(
                    buildRoot.name,
                    unused.map(({ key }) => `${CACHE_ROOT_PREFIX}${key}`),
                ),
            );
        });

        // look again once the oldest entry left ends
        const [oldest] = await this.#store.database
            .select({ usedAt: buildCache.usedAt })
            .from(buildCache)
            .orderBy(asc(buildCache.usedAt))
            .limit(1);

        return oldest === undefined ? undefined : oldest.usedAt + CACHE_MILLISECONDS;
    }

    /** Declare the controller dropping the cache entries unused for a week. */
    controller(): Controller<typeof buildCache> {
        return {
            name: "destack.build.expire",
            table: buildCache,
            when: { usedAt: { lte: new Now().minus({ milliseconds: CACHE_MILLISECONDS }) } },
            by: [],
            reconcile: async () => {
                await this.expire();
            },
        };
    }

    /** Evict the least recently used entries and leases until the blobs fit a size, while alone. */
    evict(maxBytes: number): Promise<void> {
        return this.#store.scope.span("destack.build.evict", {}, () => this.#evict(maxBytes));
    }

    /** Drop the oldest entry or leases and collect, until the blobs fit a size. */
    async #evict(maxBytes: number): Promise<void> {
        while ((await this.#store.blobs.size()) > maxBytes) {
            // take the least recently used: the oldest cache entry, else the blobs a use leased longest ago
            const [entry] = await this.#store.database
                .select({ key: buildCache.key })
                .from(buildCache)
                .orderBy(asc(buildCache.usedAt))
                .limit(1);
            const leased = entry === undefined ? await this.#store.blobs.oldestLeased() : [];
            if (entry === undefined && leased.length === 0) {
                return;
            }

            // drop the entry and its root
            if (entry !== undefined) {
                await this.#store.database.transaction(async (transaction) => {
                    await transaction.delete(buildCache).where(eq(buildCache.key, entry.key));
                    await transaction
                        .delete(buildRoot)
                        .where(eq(buildRoot.name, `${CACHE_ROOT_PREFIX}${entry.key}`));
                });
            }
            // end the leases
            else {
                await this.#store.blobs.release(leased);
            }

            // delete what nothing retains at once, as no other process reads the store
            await this.#store.blobs.collect(0);
        }
    }
}
