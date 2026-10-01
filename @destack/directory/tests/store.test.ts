import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { expect, onTestFinished, test } from "@destack/test";
import { directoryTables, DirectoryStore } from "../src/index.ts";

test.each(TEST_DIALECTS)(
    "keep the directory's reads until their rows change on %s",
    async (dialect) => {
        // place a zone in host-1, publish its endpoint, and claim a name
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const operations = () => storage.database.driver.state.operations;
        const zone = { id: "space-1", scope: "account-1", cell: "host-1", epoch: 1 };
        await directory.place(zone);
        await directory.publish("host-1", "account-1", "https://host-1.test/");
        const index = "package-1/place/name";
        const owned = (key: string) => ({
            indexes: [index],
            objectId: "space-1",
            claims: [{ index, key, objectId: "space-1", scope: "account-1" }],
        });
        await directory.replace(owned("notes"), "request-1");

        // follow the log, then answer repeated lookups without reading the database again
        const following = new AbortController();
        const followed = directory.follow(following.signal);
        onTestFinished(async () => {
            following.abort();
            await followed;
        });
        await expect
            .poll(async () => {
                await directory.locate("space-1");
                const before = operations();
                const found = await directory.locate("space-1");

                return [found, operations() - before];
            })
            .toEqual([zone, 0]);

        // follow a move to another cell, its endpoint, and a renamed claim
        await directory.place({ ...zone, cell: "host-2", epoch: 2 });
        await directory.publish("host-2", "account-1", "https://host-2.test/");
        await directory.replace(owned("archive"), "request-2");
        await expect
            .poll(async () => [
                await directory.locate("space-1"),
                (await directory.cell("host-2"))?.endpoint,
                await directory.owner(index, "notes"),
                await directory.owner(index, "archive"),
            ])
            .toEqual([
                { ...zone, cell: "host-2", epoch: 2 },
                "https://host-2.test/",
                undefined,
                { objectId: "space-1", scope: "account-1" },
            ]);
    },
);
