import { Snapshot } from "@destack/db";
import { Identifier, schema } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { expect, test } from "@destack/test";
import { AddressResolver } from "../src/address/index.ts";
import { Account } from "../src/object/index.ts";
import { AccountFixture, type Actor, ids, outcome } from "../src/test/index.ts";
import { claimNames, place } from "./fixture.ts";
import { Claimant } from "@destack/object";
import { isCallError } from "@destack/runtime/error";

test("find and resolve an address over the directory's rows, list an account's placements to those reading it and to machines, and find nothing once the account's deletion was requested", async () => {
    // keep a place called notes on a machine that publishes its URL
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId, handle, spaceId, machine } = await fixture.createSpace(owner);
    const resolver = new AddressResolver(Snapshot.live(fixture.storage.database));
    const row = { id: spaceId, scope: accountId, name: "notes" };
    const snapshot = Snapshot.live(fixture.storage.database);
    const object = { objectId: spaceId, owner: accountId };
    await claimNames(
        machine,
        fixture.storage.database,
        await Claimant.of(place, object, row, snapshot),
    );
    await machine.client.machine.publish({
        scope: accountId,
        id: machine.id,
        url: "https://machine.test/",
        relay: null,
        requestId: RequestId.create(Date.now()),
    });

    // find the place and resolve its machine
    const placement = { id: spaceId, scope: accountId, machine: machine.id, epoch: 1 };
    expect([
        await resolver.find(place, `notes--${handle}`),
        await resolver.find(place, `other--${handle}`),
        await resolver.resolve(place, `notes--${handle}`, Date.now()),
    ]).toEqual([
        place.reference(accountId, spaceId),
        undefined,
        { ...placement, id: spaceId, url: "https://machine.test/" },
    ]);

    // list the account's home, placed with the account on the residency's cloud machine, beside the space
    const home = { id: Account.home(accountId), scope: accountId, machine: ids.machine, epoch: 1 };
    expect([await placements(owner, accountId), await placements(machine, accountId)]).toEqual([
        [home, placement],
        [home, placement],
    ]);

    // conceal the account from an outsider, a machine of the outsider's account and an unknown account
    const outsider = await fixture.person("outsider@example.com");
    const foreign = (await fixture.createSpace(outsider)).machine;
    const unknown = schema
        .identifier("account")
        .parse("account-01996ab0-0000-7000-8000-000000000009");
    expect([
        await outcome(placements(outsider, accountId)),
        await outcome(placements(foreign, accountId)),
        await outcome(placements(owner, unknown)),
    ]).toEqual([
        `NOT_FOUND: no scope ${accountId}`,
        `NOT_FOUND: no scope ${accountId}`,
        `NOT_FOUND: no scope ${unknown}`,
    ]);
    expect(await outcome(resolver.resolve(place, `other--${handle}`, Date.now()))).toBe(
        `NOT_FOUND: nothing answers at other--${handle}`,
    );

    // hide the account's objects once it is deleted
    await fixture.elevate(owner.id);
    await owner.client.account.delete({
        id: accountId,
        scope: owner.id,
        requestId: RequestId.create(Date.now()),
    });
    expect([
        await resolver.account(handle),
        await resolver.find(place, `notes--${handle}`),
    ]).toEqual([undefined, undefined]);
});

test("read a space's address in its account for those reading the account, an organisation account's members among them, and for nobody else", async () => {
    // name a space in a person's and an organisation's account
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const personal = await fixture.createSpace(owner);
    const notes = await owner.client.space.create({
        accountId: personal.accountId,
        requestId: RequestId.create(Date.now()),
        name: "notes",
    });
    const founder = await fixture.person("founder@example.com");
    const organisation = await founder.client.organisation.create({
        requestId: RequestId.create(Date.now()),
        name: "Acme",
        handle: "initech",
        residencyId: "eu",
    });
    const board = await founder.client.space.create({
        accountId: organisation.payer,
        requestId: RequestId.create(Date.now()),
        name: "board",
    });

    // seat a member in the organisation
    const member = await fixture.person("member@example.com");
    const invitation = await founder.client.organisation.invite({
        id: organisation.id,
        requestId: RequestId.create(Date.now()),
        relation: "member",
        subject: member.subject,
    });
    await member.client.organisation.accept({
        id: organisation.id,
        invitationId: invitation.id,
        requestId: RequestId.create(Date.now()),
    });

    // read each address as the owner and the member
    const outsider = await fixture.person("outsider@example.com");
    expect([
        await addressOf(owner, notes),
        await addressOf(member, board),
        await addressOf(outsider, notes),
        await addressOf(outsider, board),
    ]).toEqual([`notes--${personal.handle}`, "board--initech", undefined, undefined]);
});

/** List the placements of an account a caller reads, as where each space runs. */
async function placements(caller: Pick<Actor, "client">, accountId: string): Promise<unknown[]> {
    const { items } = await caller.client.placement.list({ accountId });

    return items.map(({ id, scope, machineId, epoch }) => ({
        id,
        scope,
        machine: machineId,
        epoch,
    }));
}

/** Read a space's address as a caller, none for a space the caller does not read. */
async function addressOf(
    caller: Pick<Actor, "client">,
    space: { readonly scope: Identifier<"account">; readonly id: Identifier<"space"> },
): Promise<string | undefined> {
    return caller.client.space
        .address({ accountId: space.scope, id: space.id })
        .then(({ address }) => address)
        .catch((error: unknown) => {
            if (isCallError(error) && error.code === "NOT_FOUND") {
                return undefined;
            }
            throw error;
        });
}
