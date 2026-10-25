import objectPackage from "@destack/object/package";
import accountPackage from "../src/package.ts";
import {
    accessRelationship,
    accessRole,
    Contact,
    InstallationId,
    INVITATION_LIFETIME_MILLISECONDS,
    principal,
    Relationship,
} from "@destack/access";
import { eq, inArray, Snapshot } from "@destack/db";
import { RequestId, Scope } from "@destack/sync";
import { expect, test, refusal, type Refusal } from "@destack/test";
import { MachineIdentity } from "../src/client/machine-identity.ts";
import { account, endpoint, key, machine, organization, space } from "../src/object/index.ts";
import { Platform, PlatformDefinition, UniverseRole } from "../src/server/index.ts";
import { LocalKeyring } from "@destack/identity";
import { Digest, Identifier, nth, present } from "@destack/schema";
import {
    TestAccountStore,
    ids,
    OPERATOR_EMAIL,
    type TestPerson,
    STACKS,
    STRANGER_EMAIL,
} from "../src/test/index.ts";
import { Directory } from "../src/address/index.ts";
import type { Environment } from "@destack/environment";

/** The platform organization of the fixture's universe, declaring no cloud machines. */
const PLATFORM: PlatformDefinition = {
    name: "Destack",
    residencyId: "eu",
    operator: OPERATOR_EMAIL,
    reserved: ["admin", "security"],
    publishers: ["app"],
    handle: "destack",
    machines: [],
};

/** The email of an operator joining the platform's organization after the first. */
const SUCCESSOR_EMAIL = "successor@example.com";

/** The provider running the declared cloud machines. */
const PROVIDER = "cloudflare";

/** Create a shared account of a person under a handle, as the outcome of the call. */
function claim(fixture: TestAccountStore, person: TestPerson, handle: string): Promise<Refusal> {
    return refusal(
        person.client(accountPackage).account.create({
            scope: person.id,
            requestId: RequestId.create(fixture.environment),
            handle,
            name: handle,
            kind: "shared",
            residencyId: "eu",
        }),
    );
}

/** Check whether a person may claim a handle. */
async function availability(person: TestPerson, handle: string): Promise<string> {
    return (await person.client(accountPackage).user.checkHandle({ id: person.id, handle }))
        .availability;
}

/** Derive a cloud machine's identity from a new keyring, as the universe declares it. */
async function declare(environment: Environment): Promise<MachineIdentity> {
    return MachineIdentity.derive(
        Identifier.create("machine", environment),
        await LocalKeyring.read(LocalKeyring.generate(environment), environment),
        environment,
    );
}

test("apply the platform organization once, with its publishers as accounts and its reserved handles as names without accounts", async () => {
    // apply the platform twice
    await using fixture = await TestAccountStore.open();
    const first = await new Platform(fixture.store, PLATFORM, STACKS.residency).apply(
        fixture.clock.now(),
    );
    const second = await new Platform(fixture.store, PLATFORM, STACKS.residency).apply(
        fixture.clock.now(),
    );

    // store the organization's accounts once, and the publisher's account alone in the directory
    const accounts = await fixture.database
        .select({ id: account.table.id, handle: account.table.handle })
        .from(account.table)
        .where(eq(account.table.scope, first))
        .orderBy(account.table.handle);
    const publisher = present(
        accounts.find((entry) => entry.handle === "app"),
        "the publisher's account",
    );
    const directory = new Directory(Snapshot.live(fixture.database));
    expect([
        second,
        accounts.map((entry) => entry.handle),
        await directory.account("admin"),
        await directory.account("app"),
    ]).toEqual([first, ["app", "destack", "destack-us"], undefined, publisher.id]);
});

test("answer a person the platform's reserved and taken handles as taken", async () => {
    // apply the platform, and give a person an account of their own
    await using fixture = await TestAccountStore.open();
    await new Platform(fixture.store, PLATFORM, STACKS.residency).apply(fixture.clock.now());
    const person = await fixture.signIn("admin@example.com");
    const owned = await fixture.createAccount(person);

    // suggest a free handle, and refuse the reserved and the taken handles to new and renamed accounts
    const taken = ["CONFLICT", "NAME_TAKEN", "handle is taken"];
    expect([
        await person.client(accountPackage).user.suggestHandle({ id: person.id }),
        await availability(person, "destack"),
        await claim(fixture, person, "destack"),
        await claim(fixture, person, "admin"),
        await refusal(
            person.client(accountPackage).account.update({
                scope: person.id,
                id: owned.id,
                requestId: RequestId.create(fixture.environment),
                revision: owned.revision,
                handle: "security",
            }),
        ),
    ]).toEqual([{ handle: "admin-2" }, "taken", taken, taken, taken]);
});

test("revoke the bindings of a universe role its definition no longer declares", async () => {
    // apply the platform, then leave a binding of the servers' role an older definition declared
    await using fixture = await TestAccountStore.open();
    const { database } = fixture.storage;
    await new Platform(fixture.store, PLATFORM, STACKS.residency).apply(fixture.store.clock.now());
    const [server] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(eq(accessRole.name, "server"));
    const role = present(server, "the servers' role").id;
    const bindings = async () =>
        (
            await database
                .select({
                    subjectId: accessRelationship.subjectId,
                    relation: accessRelationship.subjectRelation,
                })
                .from(accessRelationship)
                .where(eq(accessRelationship.roleId, role))
        )
            .map((row) => `${row.subjectId}#${row.relation}`)
            .toSorted();
    const declared = await bindings();
    const stale = principal.machine.reference(
        Scope.universe.id,
        Identifier.create("machine", fixture.environment),
    );
    await Relationship.insert(database, {
        id: Identifier.create("relationship", fixture.environment),
        object: Scope.universe,
        role,
        subject: stale,
        createdAt: fixture.store.clock.now(),
        expiresAt: null,
    });

    // apply it again, keeping exactly the declared bindings
    await new Platform(fixture.store, PLATFORM, STACKS.residency).apply(fixture.store.clock.now());
    expect(await bindings()).toEqual(declared);
});

test("make the platform organization's owners operators when an apply created it without the operator role", async () => {
    // apply the platform, then drop the operator role with its bindings, as an apply failing after the creation leaves it
    await using fixture = await TestAccountStore.open();
    const first = await new Platform(fixture.store, PLATFORM, STACKS.residency).apply(
        fixture.store.clock.now(),
    );
    const { database } = fixture.storage;
    const [operator] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(eq(accessRole.name, "operator"));
    const roleId = present(operator, "the operator role").id;
    await database.transaction(
        async (transaction) =>
            await transaction
                .delete(accessRelationship)
                .where(eq(accessRelationship.roleId, roleId)),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.delete(accessRole).where(eq(accessRole.id, roleId)),
    );

    // apply it again, binding the organization with its handle
    const second = await new Platform(fixture.store, PLATFORM, STACKS.residency).apply(
        fixture.store.clock.now(),
    );
    expect({
        second,
        operating: await UniverseRole.operating(Snapshot.live(fixture.database)),
    }).toEqual({ second: first, operating: first });
});

test("move the residency homes to a newly declared stack once", async () => {
    // apply the platform, then declare the residency stack at a new release and apply it twice
    await using fixture = await TestAccountStore.open();
    await new Platform(fixture.store, PLATFORM, STACKS.residency).apply(fixture.store.clock.now());
    const released = { ...STACKS.residency, manifest: Digest.parse("c".repeat(64)) };
    await new Platform(fixture.store, PLATFORM, released).apply(fixture.store.clock.now());
    await new Platform(fixture.store, PLATFORM, released).apply(fixture.store.clock.now());

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
    await using fixture = await TestAccountStore.open();
    const [europe, america] = [
        await declare(fixture.store.environment),
        await declare(fixture.store.environment),
    ];
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
    const organizationId = await new Platform(fixture.store, definition, STACKS.residency).apply(
        fixture.store.clock.now(),
    );
    await new Platform(fixture.store, definition, STACKS.residency).apply(
        fixture.store.clock.now(),
    );

    // create each machine once with one key in the universe, its origin published as its endpoint, and relate the platform residency's machines as administrators once
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
    const urls = await database
        .select({ url: endpoint.table.url })
        .from(endpoint.table)
        .where(inArray(endpoint.table.id, declared))
        .orderBy(endpoint.table.url);
    const administrators = await Relationship.readOn(
        Snapshot.live(database),
        organization.reference(Scope.universe.id, organizationId),
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
        [{ url: "https://eu-west.machine.test/" }, { url: "https://us-west.machine.test/" }],
        [["eu", "machine"]],
    ]);
});

test("invite a new operator as the platform's owner beside the first", async () => {
    // apply the platform for a new operator at a clock standing still, and sign in a stranger and the new operator
    await using fixture = await TestAccountStore.open();
    fixture.clock.pause();
    const successor = { ...PLATFORM, operator: SUCCESSOR_EMAIL };
    const platformId = await new Platform(fixture.store, successor, STACKS.residency).apply(
        fixture.clock.now(),
    );
    const stranger = await fixture.signIn(STRANGER_EMAIL);
    const operator = await fixture.signIn(SUCCESSOR_EMAIL);
    const inviter = principal.installation.reference(
        Scope.universe.id,
        await InstallationId.base(Scope.universe, accountPackage.id, fixture.environment.crypto),
    );

    // list the invitation to the new operator alone, from account's base installation, and accept it
    const invited = (await operator.client(accountPackage).user.invited({ id: operator.id })).items;
    const { id } = nth(invited, 0);
    await operator.client(objectPackage).invitation.accept({
        scope: Scope.universe.id,
        requestId: RequestId.create(fixture.environment),
        object: organization.reference(Scope.universe.id, platformId),
        invitationId: id,
    });

    // add the new operator as an owner once, beside the first
    await new Platform(fixture.store, successor, STACKS.residency).apply(fixture.store.clock.now());
    const { items } = await operator.client(objectPackage).relationship.list({
        scope: Scope.universe.id,
        where: { objectId: platformId, relation: "owner" },
    });
    expect([
        invited,
        (await stranger.client(accountPackage).user.invited({ id: stranger.id })).items,
        items
            .map((membership) => [membership.subjectId, membership.relation])
            .toSorted((left, right) => String(left[0]).localeCompare(String(right[0]))),
        (await operator.client(accountPackage).user.invited({ id: operator.id })).items,
    ]).toEqual([
        [
            {
                id,
                relationship: {
                    object: { ...organization.reference(Scope.universe.id, platformId) },
                    relation: "owner",
                    subject: Contact.subject(Contact.email(SUCCESSOR_EMAIL)),
                    expiresAt: null,
                },
                inviter,
                status: "pending",
                createdAt: fixture.clock.now(),
                expiresAt: fixture.clock.now() + INVITATION_LIFETIME_MILLISECONDS,
            },
        ],
        [],
        [
            [ids.operator, "owner"],
            [operator.id, "owner"],
        ].toSorted((left, right) => String(left[0]).localeCompare(String(right[0]))),
        [],
    ]);
});

test("enroll a cloud machine serving a residency as an owner of the platform organization, and refuse anyone else", async () => {
    // give a stranger an account beside the platform of the operator
    await using fixture = await TestAccountStore.open();
    const stranger = await fixture.signIn(STRANGER_EMAIL);
    const { id: accountId } = await fixture.createAccount(stranger);

    // enroll a cloud machine of the eu residency as each person
    expect([
        await refusal(
            fixture.enroll(fixture.platform, await fixture.signIn(OPERATOR_EMAIL), {
                residencyId: "eu",
            }),
        ),
        await refusal(fixture.enroll(accountId, stranger, { residencyId: "eu" })),
        await refusal(fixture.enroll(accountId, stranger)),
    ]).toEqual(["done", ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: operate"], "done"]);
});
