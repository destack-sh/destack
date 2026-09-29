import { Snapshot } from "@destack/db/log";
import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import { DirectoryClient } from "../src/client/index.ts";
import { Resolver } from "../src/directory/index.ts";
import { AccountFixture, place } from "./fixture.ts";

test("find and resolve an address, list an account's zones to those reading it, and find nothing once the account's deletion was requested", async () => {
    // hold a place called notes in a region that publishes its endpoint
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const { accountId, handle, spaceId, regionId } = await fixture.createSpace(owner);
    const regional = fixture.host(accountId, regionId);
    const directory = new DirectoryClient(regional.client);
    const resolver = new Resolver(directory, (named) => directory.account(named));
    const row = { id: spaceId, scope: accountId, name: "notes" };
    const requestId = RequestId.create();
    const snapshot = Snapshot.live(fixture.opened.database);
    await directory.claim(await place.claims(row, snapshot), requestId);
    await directory.confirm(requestId, [await place.owned(spaceId, row, snapshot)]);
    await directory.publish(regionId, "universe", "https://region.test/");

    // find the place and resolve its cell
    const zone = { id: spaceId, scope: accountId, cell: regionId, epoch: 1 };
    expect([
        await resolver.find(place, `notes.${handle}`),
        await resolver.find(place, `other.${handle}`),
        await resolver.resolve(place, `notes.${handle}`),
    ]).toEqual([
        place.reference(accountId, spaceId),
        undefined,
        { ...zone, endpoint: "https://region.test/" },
    ]);

    // list the account's zones to its owner and its hosts, and hide them from other callers like an unknown account
    const outsider = await fixture.signIn("outsider@example.com");
    const foreign = fixture.host((await fixture.createSpace(outsider)).accountId);
    expect([
        await new DirectoryClient(owner.client).list(accountId),
        await directory.list(accountId),
    ]).toEqual([[zone], [zone]]);
    const unknown = "account-01996ab0-0000-7000-8000-000000000009";
    for (const [client, scope] of [
        [outsider.client, accountId],
        [foreign.client, accountId],
        [owner.client, unknown],
    ] as const) {
        await expect(new DirectoryClient(client).list(scope)).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no account ${scope}`,
        });
    }
    await expect(resolver.resolve(place, `other.${handle}`)).rejects.toMatchObject({
        code: "NOT_FOUND",
        message: `nothing answers at other.${handle}`,
    });

    // hide the account's objects once its deletion was requested
    await fixture.elevate(owner);
    await owner.client.account.delete({
        id: accountId,
        scope: owner.id,
        requestId: RequestId.create(),
    });
    expect([await resolver.account(handle), await resolver.find(place, `notes.${handle}`)]).toEqual(
        [undefined, undefined],
    );
});
