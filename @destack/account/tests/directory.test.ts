import { testSimulation } from "@destack/object/test";
import type { ClientOf } from "@destack/object/client";
import accountPackage from "../src/package.ts";
import { defineDatabase, Snapshot } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Identifier, present, schema } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { expect, onTestFinished, test, refusal } from "@destack/test";
import { Directory, DOMAINS, MachineAddress } from "../src/address/index.ts";
import { Account, endpoint, machine, placement } from "../src/object/index.ts";
import { TestAccountStore, ids, STRANGER_EMAIL, OWNER_EMAIL } from "../src/test/index.ts";
import { claimNames, place } from "./fixture.ts";
import { Claimant } from "@destack/object";

/** The origin of the residency's relay serving the machine's tunnel. */
const RELAY_ORIGIN = "https://eu.relay.test";

/** How long the machine's tunnel stands before its token lapses: one minute. */
const TUNNEL_MILLISECONDS = 60_000;

/** The spaces the stress test places, enough to tell reads per space from reads per call. */
const STRESS_SPACES = 20;

/** The time the stress test writes and locates its spaces at, in UTC epoch milliseconds. */
const NOW = 1_767_225_600_000;

/** List the placements of an account a caller reads, as where each space runs. */
async function placements(
    client: ClientOf<typeof accountPackage>,
    accountId: string,
): Promise<unknown[]> {
    const { items } = await client.placement.list({ accountId });

    return items.map(({ id, scope, machineId, epoch }) => ({
        id,
        scope,
        machine: machineId,
        epoch,
    }));
}

/** Read a space's address as a caller. */
async function addressOf(
    client: ClientOf<typeof accountPackage>,
    space: { readonly scope: Identifier<"account">; readonly id: Identifier<"space"> },
): Promise<string> {
    return (await client.space.address({ accountId: space.scope, id: space.id })).address;
}

/** Serve the owner's account with a place called notes on a machine its relay publishes. */
async function openNotes() {
    // claim the place's name as its machine
    const fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { accountId, handle, spaceId, machine: computer } = await fixture.createSpace(owner);
    const row = { id: spaceId, scope: accountId, name: "notes" };
    const object = { objectId: spaceId, owner: accountId };
    const claimant = await Claimant.of(place, object, row, Snapshot.live(fixture.database));
    await claimNames(computer, fixture.database, claimant, fixture.environment);

    // publish the machine's tunnel as the residency's relay
    const relay = fixture.as(await fixture.relay("eu"));
    await relay.client(accountPackage).machine.relay({
        scope: accountId,
        id: computer.subject.id,
        url: RELAY_ORIGIN,
        lapsesAt: fixture.clock.now() + TUNNEL_MILLISECONDS,
        requestId: RequestId.create(fixture.environment),
    });

    // address the machine below its account's handle, as the relay publishes it
    const named = await Snapshot.live(fixture.database)
        .read(machine.table)
        .findFirst({ where: { id: schema.identifier("machine").parse(computer.subject.id) } });
    const address = MachineAddress.format(
        { machine: present(named, "the machine").name, handle },
        DOMAINS.machine,
    );

    return {
        fixture,
        owner,
        accountId,
        handle,
        spaceId,
        machine: computer,
        url: `https://${address}`,
    };
}

test("find and resolve a place's address over the directory's rows, to the URL its relay publishes", async () => {
    // store a place called notes on a machine its relay publishes
    const opened = await openNotes();
    await using fixture = opened.fixture;
    const { accountId, handle, spaceId, machine: computer, url } = opened;
    const resolver = new Directory(Snapshot.live(fixture.database));

    // find the place and resolve its machine, and nothing at another name
    expect([
        await resolver.find(place, `notes--${handle}`),
        await resolver.find(place, `other--${handle}`),
        await resolver.resolve(place, `notes--${handle}`, fixture.clock.now()),
        await refusal(resolver.resolve(place, `other--${handle}`, fixture.clock.now())),
    ]).toEqual([
        place.reference(accountId, spaceId),
        undefined,
        {
            id: spaceId,
            placement: { id: spaceId, scope: accountId, machine: computer.subject.id, epoch: 1 },
            url,
        },
        ["NOT_FOUND", "ADDRESS_NOT_FOUND", `nothing responds at other--${handle}`],
    ]);
});

test("list an account's placements to those reading it and to its machines, and conceal it from others", async () => {
    // store a space of an account, and an outsider with a machine of their own account
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { accountId, spaceId, machine: computer } = await fixture.createSpace(owner);
    const outsider = await fixture.signIn(STRANGER_EMAIL);
    const foreign = fixture.as(await fixture.enroll(Account.personal(ids.stranger), outsider));
    const unknown = schema
        .identifier("account")
        .parse("account-01996ab0-0000-7000-8000-000000000009");

    // list the account's home, placed with the account on the residency's cloud machine, beside the space
    const home = { id: Account.home(accountId), scope: accountId, machine: ids.machine, epoch: 1 };
    const placed = { id: spaceId, scope: accountId, machine: computer.subject.id, epoch: 1 };
    expect([
        await placements(owner.client(accountPackage), accountId),
        await placements(computer.client(accountPackage), accountId),
        await refusal(placements(outsider.client(accountPackage), accountId)),
        await refusal(placements(foreign.client(accountPackage), accountId)),
        await refusal(placements(owner.client(accountPackage), unknown)),
    ]).toEqual([
        [home, placed],
        [home, placed],
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${accountId} not found`],
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${accountId} not found`],
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${unknown} not found`],
    ]);
});

test("find no account or place at an address once the account's deletion was requested", async () => {
    // delete the account of a named place as its owner stepped up
    const opened = await openNotes();
    await using fixture = opened.fixture;
    const { accountId, handle } = opened;
    const owner = await fixture.elevate(opened.owner);
    await owner.client(accountPackage).account.delete({
        id: accountId,
        scope: owner.id,
        requestId: RequestId.create(fixture.environment),
    });

    // find neither the account nor its place
    const resolver = new Directory(Snapshot.live(fixture.database));
    expect([
        await resolver.account(handle),
        await resolver.find(place, `notes--${handle}`),
    ]).toEqual([undefined, undefined]);
});

test("read a space's address in its account for those reading the account, an organization account's members among them, and for nobody else", async () => {
    // create a space in a person's and an organization's account
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const personal = await fixture.createAccount(owner);
    const notes = await owner.client(accountPackage).space.create({
        accountId: personal.id,
        requestId: RequestId.create(fixture.store.environment),
        name: "notes",
    });
    const founder = await fixture.signIn("founder@example.com");
    const organization = await founder.client(accountPackage).organization.create({
        requestId: RequestId.create(fixture.store.environment),
        name: "Acme",
        handle: "initech",
        residencyId: "eu",
    });
    const board = await founder.client(accountPackage).space.create({
        accountId: organization.payer,
        requestId: RequestId.create(fixture.store.environment),
        name: "board",
    });

    // add a member to the organization
    const member = await fixture.signIn("member@example.com");
    await fixture.join(founder, member, organization.id, "member");

    // read each address as the owner and the member
    const outsider = await fixture.signIn(STRANGER_EMAIL);
    expect([
        await addressOf(owner.client(accountPackage), notes),
        await addressOf(member.client(accountPackage), board),
        await refusal(addressOf(outsider.client(accountPackage), notes)),
        await refusal(addressOf(outsider.client(accountPackage), board)),
    ]).toEqual([
        `notes--${personal.handle}`,
        "board--initech",
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${personal.id} not found`],
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${organization.payer} not found`],
    ]);
});

test("locate each of many spaces placed on a machine in one statement for its placement, reading the machine's endpoint once", async () => {
    // run in a world of its own
    await using simulation = testSimulation();
    const world = simulation.environment("test");

    // open a database of placements and endpoints
    const storage = await TestDatabase.create(
        defineDatabase({ name: "directory", tables: [...placement.tables, ...endpoint.tables] }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;

    // publish a machine's endpoint at its URL
    await database.transaction(
        async (transaction) =>
            await transaction.insert(endpoint.table).values({
                id: ids.machine,
                scope: ids.account,
                parentId: ids.machine,
                url: "https://machine.test/",
                relay: null,
                createdAt: NOW,
                updatedAt: NOW,
            }),
    );

    // place many spaces on the machine
    const spaces = Array.from({ length: STRESS_SPACES }, () => Identifier.create("space", world));
    const placed = spaces.map((id) => ({ id, scope: ids.account, machine: ids.machine, epoch: 1 }));
    await database.transaction(
        async (transaction) =>
            await transaction.insert(placement.table).values(
                placed.map(({ machine: machineId, ...rest }) => ({
                    ...rest,
                    machineId,
                    schedule: {
                        kind: "stay",
                        condition: {
                            status: "true",
                            reason: "Unset",
                            message: "placed by the test",
                        },
                    } as const,
                    createdAt: NOW,
                    updatedAt: NOW,
                })),
            ),
    );

    // hear commits as a serving store does, then locate each through a connection counting its statements
    await database.log.listening();
    const reader = database.metered();
    const directory = new Directory(Snapshot.live(reader));
    const located = await Promise.all(spaces.map((id) => directory.locate(id, NOW)));

    // read each space's placement, and the machine's endpoint once from the database
    expect({ statements: reader.driver.operations, located }).toEqual({
        statements: STRESS_SPACES + 1,
        located: placed.map((row) => ({ placement: row, url: "https://machine.test/" })),
    });
});
