import { identifier } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { expect, test } from "@destack/test";
import { v7 } from "uuid";
import { DirectoryClient } from "../src/client/index.ts";
import type { ProfilePage } from "../src/service/index.ts";
import { AccountFixture } from "./fixture.ts";

test("stream a space's cell the profiles of its account's users and of users who joined the space, and no others", async () => {
    // place a space of the owner's account in a host, and sign in a stranger and a person who joins the space
    await using fixture = await AccountFixture.open();
    const owner = await fixture.signIn("owner@example.com");
    const { accountId, handle } = await fixture.createSpace(owner);
    const stranger = await fixture.signIn("stranger@example.com");
    const joiner = await fixture.signIn("joiner@example.com");
    const host = fixture.host(accountId);
    const spaceId = identifier("space").parse(`space-${v7()}`);
    await new DirectoryClient(host.client).place({
        id: spaceId,
        scope: accountId,
        cell: host.id,
        epoch: 1,
    });
    await joiner.client.membership.create({
        userId: joiner.id,
        requestId: RequestId.create(),
        spaceId,
    });

    // stream the profiles the cell asks for, keeping the owner's and the joiner's alone
    const controller = new AbortController();
    const pages = await host.client.access.profiles(
        { spaceId, users: [owner.id, stranger.id, joiner.id] },
        { signal: controller.signal },
    );
    const value = (await pages[Symbol.asyncIterator]().next()).value as ProfilePage;
    controller.abort();
    expect([
        [...value.profiles].sort((left, right) => left.id.localeCompare(right.id)),
        value.handles,
    ]).toEqual([
        [
            { id: owner.id, name: "owner@example.com", image: null },
            { id: joiner.id, name: "joiner@example.com", image: null },
        ].sort((left, right) => left.id.localeCompare(right.id)),
        [{ accountId, userId: owner.id, handle }],
    ]);

    // leave the space, after which the joiner's profile is no longer streamed
    const { items } = await joiner.client.membership.list({ userId: joiner.id, limit: 10 });
    await joiner.client.membership.delete({
        userId: joiner.id,
        requestId: RequestId.create(),
        id: items[0]!.id,
        revision: items[0]!.revision,
    });
    const again = new AbortController();
    const left = await host.client.access.profiles(
        { spaceId, users: [joiner.id] },
        { signal: again.signal },
    );
    const after = (await left[Symbol.asyncIterator]().next()).value as ProfilePage;
    again.abort();
    expect(after.profiles).toEqual([]);
});
