import { Scope } from "@destack/sync";
import { Snapshot } from "@destack/db";
import { claimTable, Claim } from "@destack/object";
import { Identifier } from "@destack/schema";
import { expect, test } from "@destack/test";
import { account } from "../src/object/account.ts";
import { AccountHandle } from "../src/address/handle.ts";
import { AccountFixture } from "../src/test/index.ts";

/** Suggest the first free number of a stem past a whole batch of taken ones. */
test("suggest the first free handle past a hundred taken numbers of the stem", async () => {
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("crowded@example.com");

    // take the stem and its numbers 2 to 100 with accounts of the owner
    const now = Date.now();
    const { database } = fixture.storage;
    const rows = Array.from({ length: 100 }, (_, index) => ({
        id: Identifier.create("account"),
        createdAt: now,
        updatedAt: now,
        scope: owner.id,
        handle: index === 0 ? "crowded" : `crowded-${index + 1}`,
        name: "Crowded",
        residencyId: "eu" as const,
    }));
    await database.insert(account.table).values(rows);

    // claim their handles in one write
    const snapshot = Snapshot.live(database);
    const claims = await Promise.all(rows.map((row) => Claim.list(account, row, snapshot)));
    await database.insert(claimTable).values(
        claims.flat().map((claim) => ({
            ...claim,
            id: Identifier.create("claim"),
            scope: Scope.universe.id,
            state: "active" as const,
            requestId: null,
            expiresAt: null,
        })),
    );

    // suggest the number the second batch of candidates finds free
    expect(await owner.client.user.suggestHandle({ id: owner.id })).toEqual({
        handle: "crowded-101",
    });
});

test("refuse handles with doubled hyphens, which punycode labels such as xn-- start with", () => {
    const handles = ["xn--abc", "a--b", "a-b", "a"];
    expect(handles.map((handle) => AccountHandle.safeParse(handle).success)).toEqual([
        false,
        false,
        true,
        true,
    ]);
});
