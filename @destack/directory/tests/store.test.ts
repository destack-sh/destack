import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { directoryTables, DirectoryStore } from "../src/index.ts";

test.each(TEST_DIALECTS)(
    "keep the directory's reads until their rows change on %s",
    async (dialect) => {
        // place a space on machine-a, publish its endpoint and claim a name
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const operations = () => storage.database.driver.state.operations;
        const placement = { id: "space-1", scope: "account-1", machine: "machine-a", epoch: 1 };
        await directory.place(placement);
        await directory.publish("machine-a", "account-1", "https://machine-a.test/");
        const index = "package-1/place/name";
        const owned = (key: string) => ({
            indexes: [index],
            objectId: "space-1",
            claims: [
                { index, key, packageId: "package-1", objectId: "space-1", scope: "account-1" },
            ],
        });
        await directory.replace(owned("notes"), "request-1");

        // follow the log and answer repeated lookups without reading the database again
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
            .toEqual([placement, 0]);

        // follow a move to another machine, its endpoint and a renamed claim
        await directory.move(placement, "machine-b");
        await directory.place({ ...placement, machine: "machine-b", epoch: 2 });
        await directory.publish("machine-b", "account-1", "https://machine-b.test/");
        await directory.replace(owned("archive"), "request-2");
        await expect
            .poll(async () => [
                await directory.locate("space-1"),
                (await directory.endpoint("machine-b"))?.url,
                await directory.owner(index, "notes"),
                await directory.owner(index, "archive"),
            ])
            .toEqual([
                { ...placement, machine: "machine-b", epoch: 2 },
                "https://machine-b.test/",
                undefined,
                { objectId: "space-1", scope: "account-1" },
            ]);
    },
);

test.each(TEST_DIALECTS)(
    "give machines work in a space only from the machine serving it, and list each machine's spaces on %s",
    async (dialect) => {
        // place two spaces on machine-a
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const first = { id: "space-1", scope: "account-1", machine: "machine-a", epoch: 1 };
        const second = { id: "space-2", scope: "account-1", machine: "machine-a", epoch: 1 };
        await directory.place(first);
        await directory.place(second);

        // give machine-c work in both and withdraw it from the second
        await directory.assign(first, "machine-c");
        await directory.assign(first, "machine-c");
        await directory.assign(second, "machine-c");
        await directory.unassign(second, "machine-c");
        expect([
            await directory.assignments("machine-c"),
            await directory.assigned("space-1"),
        ]).toEqual([["space-1"], ["machine-c"]]);

        // refuse a machine or epoch that no longer serves the space
        await directory.move(first, "machine-b");
        await directory.place({ ...first, machine: "machine-b", epoch: 2 });
        await expect(directory.assign(first, "machine-c")).rejects.toMatchObject({
            code: "CONFLICT",
            message: "space-1 is no longer placed on machine-a at epoch 1",
        });
    },
);

test.each(TEST_DIALECTS)(
    "advance a space's epoch on its own machine, ending its move and refusing every step of the earlier epoch, on %s",
    async (dialect) => {
        // place a space on machine-a moving to machine-b and advance its epoch on machine-a
        const storage = await TestDatabase.create(dialect, directoryTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const placement = { id: "space-1", scope: "account-1", machine: "machine-a", epoch: 1 };
        await directory.place(placement);
        await directory.move(placement, "machine-b");
        await directory.place({ ...placement, epoch: 2 });

        // refuse the earlier epoch's announcement, takeover and placement
        expect([
            await refusal(directory.move(placement, "machine-b")),
            await refusal(directory.place({ ...placement, machine: "machine-b", epoch: 2 })),
            await refusal(directory.place(placement)),
            await directory.locate("space-1"),
        ]).toEqual([
            ["CONFLICT", "space-1 is no longer placed on machine-a at epoch 1"],
            ["CONFLICT", "space-1 is neither placed on machine-b at epoch 2 nor moving there"],
            ["CONFLICT", "space-1 is neither placed on machine-a at epoch 1 nor moving there"],
            { ...placement, epoch: 2 },
        ]);
    },
);
