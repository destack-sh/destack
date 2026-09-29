import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { expect, onTestFinished, test } from "@destack/test";
import { directoryTables, DirectoryCache, DirectoryDatabase, type Zone } from "../src/index.ts";

/** A directory in the global database counting the zone reads that reach it. */
class CountingDirectory extends DirectoryDatabase {
    /** The zone reads so far. */
    locates = 0;

    /** Count and read a zone. */
    override locate(scope: string): Promise<Zone | undefined> {
        this.locates += 1;

        return super.locate(scope);
    }
}

test.each(TEST_DIALECTS)(
    "keep the directory's reads until their rows change on %s",
    async (dialect) => {
        // place a zone in host-1, publish its endpoint, and claim a name
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new CountingDirectory(storage.database);
        const zone = { id: "space-1", scope: "account-1", cell: "host-1", epoch: 1 };
        await directory.place(zone);
        await directory.publish("host-1", "account-1", "https://host-1.test/");
        const index = "package-1/place/name";
        const owned = (key: string) => ({
            indexes: [index],
            objectId: "space-1",
            claims: [{ index, key, objectId: "space-1", scope: "account-1" }],
        });
        await directory.replace(owned("notes"), "request-1", Date.now());

        // follow the log, then answer repeated lookups without reading the zone again
        const cache = new DirectoryCache(directory);
        const following = new AbortController();
        const followed = cache.follow(following.signal);
        onTestFinished(async () => {
            following.abort();
            await followed;
        });
        await expect
            .poll(async () => {
                const reads = directory.locates;
                const found = await cache.locate("space-1");

                return [found, directory.locates - reads];
            })
            .toEqual([zone, 0]);

        // follow a move to another cell, its endpoint, and a renamed claim
        await directory.place({ ...zone, cell: "host-2", epoch: 2 });
        await directory.publish("host-2", "account-1", "https://host-2.test/");
        await directory.replace(owned("archive"), "request-2", Date.now());
        await expect
            .poll(async () => [
                await cache.locate("space-1"),
                (await cache.cell("host-2"))?.endpoint,
                await cache.owner(index, "notes"),
                await cache.owner(index, "archive"),
            ])
            .toEqual([
                { ...zone, cell: "host-2", epoch: 2 },
                "https://host-2.test/",
                undefined,
                { objectId: "space-1", scope: "account-1" },
            ]);
    },
);
