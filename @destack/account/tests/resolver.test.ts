import { eq } from "@destack/db";
import { Snapshot } from "@destack/db/log";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { v7 } from "uuid";
import { ResolverCache } from "../src/directory/index.ts";
import { account } from "../src/object/account.ts";
import { user } from "../src/object/user.ts";
import { globalTables } from "./database.ts";
import { place } from "./fixture.ts";

test.each(TEST_DIALECTS)(
    "keep resolved addresses until their account, name, zone or cell changes on %s",
    async (dialect) => {
        // place notes of the account acme in host-1 and publish its endpoint
        const storage = await TestDatabase.create(dialect, globalTables, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        const now = Date.now();
        const record = { createdAt: now, updatedAt: now };
        const userId = identifier("user").parse(`user-${v7()}`);
        const accountId = identifier("account").parse(`account-${v7()}`);
        await database
            .insert(user.table)
            .values({ ...record, id: userId, name: "Owner", email: "owner@example.com" });
        await database.insert(account.table).values({
            ...record,
            id: accountId,
            scope: userId,
            handle: "acme",
            name: "Acme",
            defaultResidency: "eu",
        });
        const resolver = new ResolverCache(database);
        const { directory } = resolver;
        const row = { id: "space-1", scope: accountId, name: "notes" };
        const owned = async (name: string) =>
            place.owned("space-1", { ...row, name }, Snapshot.live(database));
        await directory.replace(await owned("notes"), "claim", now);
        const zone = { id: "space-1", scope: accountId, cell: "host-1", epoch: 1 };
        await directory.place(zone);
        await directory.publish("host-1", accountId, "https://host-1.test/");

        // follow the log and resolve the address
        const following = new AbortController();
        const followed = resolver.follow(following.signal);
        onTestFinished(async () => {
            following.abort();
            await followed;
        });
        expect(await resolver.resolve(place, "notes.acme")).toEqual({
            ...zone,
            endpoint: "https://host-1.test/",
        });

        // follow a move to another cell, and its endpoint
        await directory.place({ ...zone, cell: "host-2", epoch: 2 });
        await directory.publish("host-2", accountId, "https://host-2.test/");
        await expect
            .poll(() => resolver.resolve(place, "notes.acme"))
            .toEqual({ ...zone, cell: "host-2", epoch: 2, endpoint: "https://host-2.test/" });

        // follow a handle's rename and a renamed place
        await database
            .update(account.table)
            .set({ handle: "acme-renamed" })
            .where(eq(account.table.id, accountId));
        await expect.poll(() => resolver.find(place, "notes.acme")).toBeUndefined();
        await directory.replace(await owned("archive"), "rename", Date.now());
        await expect
            .poll(async () => [
                await resolver.find(place, "notes.acme-renamed"),
                await resolver.find(place, "archive.acme-renamed"),
            ])
            .toEqual([undefined, place.reference(accountId, "space-1")]);
    },
);
