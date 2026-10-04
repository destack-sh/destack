import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
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
            claims: [
                { index, key, packageId: "package-1", objectId: "space-1", scope: "account-1" },
            ],
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
        await directory.move(zone, "host-2");
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

test.each(TEST_DIALECTS)(
    "give cells work in a zone only from the cell serving it, and list each cell's zones on %s",
    async (dialect) => {
        // place two zones in host-1
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const first = { id: "space-1", scope: "account-1", cell: "host-1", epoch: 1 };
        const second = { id: "space-2", scope: "account-1", cell: "host-1", epoch: 1 };
        await directory.place(first);
        await directory.place(second);

        // give laptop work in both, twice in one, and withdraw it from the second
        await directory.assign(first, "laptop");
        await directory.assign(first, "laptop");
        await directory.assign(second, "laptop");
        await directory.unassign(second, "laptop");
        expect([
            await directory.assignments("laptop"),
            await directory.assigned("space-1"),
        ]).toEqual([["space-1"], ["laptop"]]);

        // refuse a cell or epoch that no longer serves the zone
        await directory.move(first, "host-2");
        await directory.place({ ...first, cell: "host-2", epoch: 2 });
        await expect(directory.assign(first, "laptop")).rejects.toMatchObject({
            code: "CONFLICT",
            message: "space-1 is no longer placed in host-1 at epoch 1",
        });
    },
);

test.each(TEST_DIALECTS)(
    "advance a zone's epoch in its own cell, ending its move and refusing every step of the earlier epoch, on %s",
    async (dialect) => {
        // place a zone in host-1 moving to host-2, then advance its epoch in host-1
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const zone = { id: "space-1", scope: "account-1", cell: "host-1", epoch: 1 };
        await directory.place(zone);
        await directory.move(zone, "host-2");
        await directory.place({ ...zone, epoch: 2 });

        // refuse the earlier epoch's announcement, takeover and placement
        expect([
            await refusal(directory.move(zone, "host-2")),
            await refusal(directory.place({ ...zone, cell: "host-2", epoch: 2 })),
            await refusal(directory.place(zone)),
            await directory.locate("space-1"),
        ]).toEqual([
            ["CONFLICT", "space-1 is no longer placed in host-1 at epoch 1"],
            ["CONFLICT", "space-1 is neither placed in host-2 at epoch 2 nor moving there"],
            ["CONFLICT", "space-1 is neither placed in host-1 at epoch 1 nor moving there"],
            { ...zone, epoch: 2 },
        ]);
    },
);
