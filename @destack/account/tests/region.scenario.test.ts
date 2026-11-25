import accountPackage from "../src/package.ts";
import { Identifier, nth } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Account } from "../src/object/index.ts";
import { TestAccount, ids, STRANGER_EMAIL, OWNER_EMAIL } from "../src/test/index.ts";
import { claimNames, place } from "./fixture.ts";
import { Claim, Claimant } from "@destack/object";

test("claim a name as the account's home", async () => {
    // claim a space's name as the owner's home, whose identity its machine started
    await using fixture = await TestAccount.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const home = await fixture.personalHome(owner);
    await fixture.startIdentity(home.spaceId, home.machine);
    const row = {
        id: Identifier.create("space", fixture.environment),
        scope: home.accountId,
        name: "notes",
    };
    const snapshot = fixture.database.snapshot();
    const object = { objectId: row.id, owner: home.accountId };
    const claimant = await Claimant.of(place, object, row, snapshot);
    await claimNames(fixture.as(home.spaceId), fixture.database, claimant, fixture.environment);

    // read its owner as another account's machine
    const stranger = await fixture.signIn(STRANGER_EMAIL);
    const machine = fixture.as(await fixture.enroll(Account.personal(ids.stranger), stranger));
    const { index, key } = nth(await Claim.list(place, row, snapshot), 0);
    expect(await machine.client(accountPackage).claim.owner({ index, key })).toEqual({
        objectId: row.id,
        owner: home.accountId,
        state: "active",
    });
});

test("read each account's relay", async () => {
    // store an eu and a us account
    await using fixture = await TestAccount.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: european } = await fixture.createAccount(owner);
    const { id: american } = await fixture.createAccount(owner, "us");

    // read both as the owner
    const regions = owner.client(accountPackage).region;
    expect([
        await regions.relay({ account: european }),
        await regions.relay({ account: american }),
    ]).toEqual([
        { url: "https://relay-eu.machine.test" },
        { url: "https://relay-us.machine.test" },
    ]);
});
