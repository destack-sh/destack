import { accessRelationship, accessRole, Contact, Relationship } from "@destack/access";
import { eq, inArray, Snapshot } from "@destack/db";
import { RequestId } from "@destack/runtime/request";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import { MachineIdentity } from "../src/client/machine-identity.ts";
import { account, endpoint, key, machine, organisation, space } from "../src/object/index.ts";
import { Platform, PlatformDefinition, UniverseRole } from "../src/server/index.ts";
import { LocalKeyring } from "@destack/identity";
import { Digest, Identifier, present } from "@destack/schema";
import {
    AccountFixture,
    ids,
    OPERATOR_EMAIL,
    type Person,
    STACKS,
    outcome,
} from "../src/test/index.ts";

import { Directory } from "../src/address/index.ts";
import { Clock } from "@destack/environment";

/** The platform organisation of the fixture's universe, declaring no cloud machines. */
const PLATFORM: PlatformDefinition = {
    name: "Destack",
    residencyId: "eu",
    operator: OPERATOR_EMAIL,
    reserved: ["admin", "security"],
    publishers: ["app"],
    handle: "destack",
    machines: [],
};

/** The email of an operator joining the platform's organisation after the first. */
const SUCCESSOR_EMAIL = "successor@example.com";

/** The provider running the declared cloud machines. */
const PROVIDER = "cloudflare";

test("apply the platform organisation once with its publishers as accounts and its reserved handles as names without accounts, answering a stranger as for any taken handle", async () => {
    // apply the platform twice
    await using fixture = await AccountFixture.open();
    const first = await new Platform(PLATFORM, STACKS.residency).apply(fixture.objects, Date.now());
    const second = await new Platform(PLATFORM, STACKS.residency).apply(
        fixture.objects,
        Date.now(),
    );
    const accounts = await fixture.storage.database
        .select({ id: account.table.id, handle: account.table.handle, scope: account.table.scope })
        .from(account.table);

    // refuse the stranger the taken handles
    const stranger = await fixture.person("admin@example.com");
    const { accountId } = await fixture.createSpace(stranger);
    const current = await stranger.client.account.get({ scope: stranger.id, id: accountId });
    const renamed = await outcome(
        stranger.client.account.update({
            scope: stranger.id,
            id: accountId,
            requestId: RequestId.create(Date.now()),
            revision: current.revision,
            handle: "security",
        }),
    );
    const publisher = accounts.find((entry) => entry.handle === "app")?.id;
    expect([
        second,
        accounts
            .filter((entry) => entry.scope === first)
            .map((entry) => entry.handle)
            .toSorted(),
        await new Directory(Snapshot.live(fixture.database)).account("admin"),
        await new Directory(Snapshot.live(fixture.database)).account("app"),
        await stranger.client.user.suggestHandle({ id: stranger.id }),
        await availability(stranger, "destack"),
        await claim(stranger, "destack"),
        await claim(stranger, "admin"),
        renamed,
    ]).toEqual([
        first,
        ["app", "destack", "destack-us"],
        undefined,
        present(publisher, "the app account"),
        { handle: "admin-2" },
        "taken",
        "CONFLICT: handle is taken",
        "CONFLICT: handle is taken",
        "CONFLICT: handle is taken",
    ]);
});

test("make the platform organisation's owners operators when an apply created it without the operator role", async () => {
    // apply the platform, then drop the operator role with its bindings, as an apply failing after the creation leaves it
    await using fixture = await AccountFixture.open();
    const first = await new Platform(PLATFORM, STACKS.residency).apply(fixture.objects, Date.now());
    const { database } = fixture.storage;
    const [operator] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(eq(accessRole.name, "operator"));
    const roleId = present(operator, "the operator role").id;
    await database.delete(accessRelationship).where(eq(accessRelationship.roleId, roleId));
    await database.delete(accessRole).where(eq(accessRole.id, roleId));

    // apply it again, binding the organisation its handle names
    const second = await new Platform(PLATFORM, STACKS.residency).apply(
        fixture.objects,
        Date.now(),
    );
    expect({
        second,
        operating: await UniverseRole.operating(Snapshot.live(fixture.database)),
    }).toEqual({ second: first, operating: first });
});

test("move the residency homes to a newly declared stack once", async () => {
    // apply the platform, then declare the residency stack at a new release and apply it twice
    await using fixture = await AccountFixture.open();
    await new Platform(PLATFORM, STACKS.residency).apply(fixture.objects, Date.now());
    const released = { ...STACKS.residency, manifest: Digest.parse("c".repeat(64)) };
    await new Platform(PLATFORM, released).apply(fixture.objects, Date.now());
    await new Platform(PLATFORM, released).apply(fixture.objects, Date.now());

    // read the homes' stacks and revisions
    const homes = await fixture.storage.database
        .select({
            handle: account.table.handle,
            stack: space.table.stack,
            revision: space.table.revision,
        })
        .from(space.table)
        .innerJoin(account.table, eq(account.table.id, space.table.scope))
        .where(inArray(account.table.handle, ["destack", "destack-us"]));

    // move both residency homes to the declared release in one update each
    expect(homes.toSorted((left, right) => left.handle.localeCompare(right.handle))).toEqual([
        { handle: "destack", stack: released, revision: 2 },
        { handle: "destack-us", stack: released, revision: 2 },
    ]);
});

test("apply the declared cloud machines with their keys in the universe once, and the platform residency's machines as administrators", async () => {
    // declare a cloud machine for each residency
    await using fixture = await AccountFixture.open();
    const [europe, america] = [await declare(), await declare()];
    const definition: PlatformDefinition = {
        ...PLATFORM,
        machines: [
            {
                id: europe.machineId,
                name: "eu-west",
                residencyId: "eu",
                providerCode: PROVIDER,
                location: "eu",
                key: await europe.key(),
                origin: "https://eu-west.machine.test/",
            },
            {
                id: america.machineId,
                name: "us-west",
                residencyId: "us",
                providerCode: PROVIDER,
                location: "us",
                key: await america.key(),
                origin: "https://us-west.machine.test/",
            },
        ],
    };

    // apply the platform twice
    const organisationId = await new Platform(definition, STACKS.residency).apply(
        fixture.objects,
        Date.now(),
    );
    await new Platform(definition, STACKS.residency).apply(fixture.objects, Date.now());

    // create each machine once with one key in the universe, its origin published as its endpoint
    const { database } = fixture.storage;
    const declared = [europe.machineId, america.machineId];
    const machines = await database
        .select({
            id: machine.table.id,
            scope: machine.table.scope,
            name: machine.table.name,
            kind: machine.table.kind,
            residencyId: machine.table.residencyId,
            providerCode: machine.table.providerCode,
            location: machine.table.location,
        })
        .from(machine.table)
        .where(inArray(machine.table.id, declared))
        .orderBy(machine.table.name);
    const keys = await database
        .select({ parentId: key.table.parentId })
        .from(key.table)
        .where(inArray(key.table.parentId, declared))
        .orderBy(key.table.parentId);
    const urls = await Promise.all(
        [europe.machineId, america.machineId].map(
            async (id) =>
                (await Snapshot.live(database).read(endpoint.table).findFirst({ where: { id } }))
                    ?.url,
        ),
    );
    const administrators = await Relationship.readOn(
        Snapshot.live(database),
        organisation.reference(Scope.universe.id, organisationId),
        { relation: "admin" },
    );
    expect([
        machines,
        keys.map((row) => row.parentId),
        urls,
        administrators
            .filter((row) => row.subjectType === "residency")
            .map((row) => [row.subjectId, row.subjectRelation]),
    ]).toEqual([
        [
            {
                id: europe.machineId,
                scope: Scope.universe.id,
                name: "eu-west",
                kind: "cloud",
                residencyId: "eu",
                providerCode: PROVIDER,
                location: "eu",
            },
            {
                id: america.machineId,
                scope: Scope.universe.id,
                name: "us-west",
                kind: "cloud",
                residencyId: "us",
                providerCode: PROVIDER,
                location: "us",
            },
        ],
        declared.toSorted(),
        ["https://eu-west.machine.test/", "https://us-west.machine.test/"],
        // relate the machines of the platform's residency as administrators once
        [["eu", "machine"]],
    ]);
});

test("invite a new operator as the platform's owner beside the first, and refuse the invitation to anyone else", async () => {
    // apply the platform for a new operator and sign in a stranger and the new operator
    await using fixture = await AccountFixture.open();
    const successor = { ...PLATFORM, operator: SUCCESSOR_EMAIL };
    const platformId = await new Platform(successor, STACKS.residency).apply(
        fixture.objects,
        Date.now(),
    );
    const stranger = await fixture.person("stranger@example.com");
    const operator = await fixture.person(SUCCESSOR_EMAIL);

    // list the invitation to the new operator alone
    const [invited] = (await operator.client.user.invited({ id: operator.id })).items;
    const { id } = present(invited, "the new operator's invitation");
    const accept = (person: Person) =>
        person.client.organisation.accept({
            id: platformId,
            invitationId: id,
            requestId: RequestId.create(Date.now()),
        });
    const refused = await outcome(accept(stranger));
    await accept(operator);

    // seat the new operator as an active owner once, beside the first
    await new Platform(successor, STACKS.residency).apply(fixture.objects, Date.now());
    const { items } = await operator.client.member.list({ organisationId: platformId });
    expect([
        invited,
        (await stranger.client.user.invited({ id: stranger.id })).items,
        refused,
        items.map((seat) => [seat.userId, seat.role, seat.status]),
        (await operator.client.user.invited({ id: operator.id })).items,
    ]).toEqual([
        {
            id,
            relationship: {
                object: { ...organisation.reference(Scope.universe.id, platformId) },
                relation: "owner",
                subject: Contact.subject(Contact.email(SUCCESSOR_EMAIL)),
                expiresAt: null,
            },
            inviter: { ...Scope.universe },
            status: "pending",
            createdAt: invited?.createdAt,
            expiresAt: invited?.expiresAt,
        },
        [],
        "FORBIDDEN: invitation is addressed to someone else",
        [
            [ids.operator, "owner", "active"],
            [operator.id, "owner", "active"],
        ],
        [],
    ]);
});

test("enroll a cloud machine serving a residency as an owner of the platform organisation, and refuse anyone else", async () => {
    // give a stranger an account beside the platform the operator owns
    await using fixture = await AccountFixture.open();
    const stranger = await fixture.person("stranger@example.com");
    const { accountId } = await fixture.createSpace(stranger);

    // enroll a cloud machine of the eu residency as each person
    expect([
        await outcome(fixture.enroll(fixture.platform, { residencyId: "eu" })),
        await outcome(fixture.enroll(accountId, { client: stranger.client, residencyId: "eu" })),
        await outcome(fixture.enroll(accountId, { client: stranger.client })),
    ]).toEqual(["executed", "FORBIDDEN: permission denied: operate", "executed"]);
});

/** Create a shared account of a person under a handle, as the outcome of the call. */
function claim(person: Person, handle: string): Promise<string> {
    return outcome(
        person.client.account.create({
            scope: person.id,
            requestId: RequestId.create(Date.now()),
            handle,
            name: handle,
            kind: "shared",
        }),
    );
}

/** Check whether a person may claim a handle. */
async function availability(person: Person, handle: string): Promise<string> {
    return (await person.client.user.checkHandle({ id: person.id, handle })).availability;
}

/** Derive a cloud machine's identity from a new keyring, as the universe declares it. */
async function declare(): Promise<MachineIdentity> {
    return MachineIdentity.derive(
        Identifier.create("machine"),
        await LocalKeyring.read(LocalKeyring.generate()),
        Clock.system,
    );
}
