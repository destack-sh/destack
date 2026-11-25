import { TestAccess } from "@destack/access/test";
import objectPackage from "@destack/object/package";
import accountPackage from "../src/package.ts";
import {
    Contact,
    principal,
    accessRelationship,
    accessRole,
    INVITATION_LIFETIME_MILLISECONDS,
    Relationship,
} from "@destack/access";
import { and, eq, inArray, isNull } from "@destack/db";
import { RequestId, Scope } from "@destack/sync";
import { expect, test, refusal, type Refusal } from "@destack/test";
import { MachineKey } from "../src/client/machine-key.ts";
import {
    account,
    endpoint,
    hostname,
    key,
    machine,
    client,
    organization,
    space,
    Account,
} from "../src/object/index.ts";
import { Platform, PlatformDefinition, UniverseRole } from "../src/server/index.ts";
import { LocalKeyring } from "@destack/identity";
import { Identifier, nth, present } from "@destack/schema";
import {
    TestAccount,
    ids,
    OPERATOR_EMAIL,
    type TestPerson,
    PLATFORM_STACK,
    STACKS,
    STRANGER_EMAIL,
} from "../src/test/index.ts";
import { Directory } from "../src/client/index.ts";
import type { Environment } from "@destack/environment";

/** The OIDC issuer signing the platform's publishing workflow's tokens. */
const ISSUER = "https://issuer.test";

/** The platform's publishing workflow, as its issuer names it. */
const WORKFLOW = "repo:destack/destack:environment:production";

/** The platform organization of the fixture's universe, declaring no cloud machines. */
const PLATFORM: PlatformDefinition = {
    name: "Destack",
    regionId: "eu",
    operator: OPERATOR_EMAIL,
    reserved: ["admin", "security"],
    transfers: {},
    publishers: ["app"],
    clients: [{ name: "Release", issuer: ISSUER, subject: WORKFLOW }],
    handle: "destack",
    domains: [],
    home: PLATFORM_STACK,
    hostnames: [],
    stacks: STACKS,
    machines: [],
};

/** The email of an operator joining the platform's organization after the first. */
const SUCCESSOR_EMAIL = "successor@example.com";

/** The provider running the declared cloud machines. */
const PROVIDER = "cloudflare";

/** Create a shared account of a person under a handle, as the outcome of the call. */
function claim(fixture: TestAccount, person: TestPerson, handle: string): Promise<Refusal> {
    return refusal(
        person.client(accountPackage).account.create({
            scope: person.id,
            requestId: RequestId.create(fixture.environment),
            handle,
            name: handle,
            kind: "shared",
            regionId: "eu",
        }),
    );
}

/** Check whether a person may claim a handle. */
async function availability(person: TestPerson, handle: string): Promise<string> {
    return (await person.client(accountPackage).user.checkHandle({ id: person.id, handle }))
        .availability;
}

/** Derive a cloud machine's identity from a new keyring, as the universe declares it. */
async function declare(environment: Environment): Promise<MachineKey> {
    return MachineKey.derive(
        Identifier.create("machine", environment),
        await LocalKeyring.read(LocalKeyring.generate(environment), environment),
        environment,
    );
}

test("apply the platform organization once, with its publishers as accounts and its reserved handles as names without accounts", async () => {
    // apply the platform twice
    await using fixture = await TestAccount.open();
    const first = await new Platform(fixture.store, PLATFORM).apply(fixture.clock.now());
    const second = await new Platform(fixture.store, PLATFORM).apply(fixture.clock.now());

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
    const directory = new Directory(fixture.database.snapshot());
    expect([
        second,
        accounts.map((entry) => entry.handle),
        await directory.account("admin"),
        await directory.account("app"),
    ]).toEqual([first, ["app", "destack"], undefined, publisher.id]);
});

test("answer a person the platform's reserved and taken handles as taken", async () => {
    // apply the platform, and give a person an account of their own
    await using fixture = await TestAccount.open();
    await new Platform(fixture.store, PLATFORM).apply(fixture.clock.now());
    const person = await fixture.signIn("admin@example.com");
    const owned = await fixture.createAccount(person);

    // suggest a free handle, and refuse the reserved and taken handles and their lookalikes
    const taken = ["CONFLICT", "NAME_EXISTS", "'handle' already exists"];
    const lookalike = ["CONFLICT", "NAME_EXISTS", "'lookalike' already exists"];
    expect([
        await person.client(accountPackage).user.suggestHandle({ id: person.id }),
        await availability(person, "destack"),
        await availability(person, "de-stack"),
        await claim(fixture, person, "destack"),
        await claim(fixture, person, "de-stack"),
        await claim(fixture, person, "admin"),
        await claim(fixture, person, "ad-min"),
        await refusal(
            person.client(accountPackage).account.update({
                scope: person.id,
                id: owned.id,
                requestId: RequestId.create(fixture.environment),
                revision: owned.revision,
                handle: "security",
            }),
        ),
    ]).toEqual([
        { handle: "admin-2" },
        "taken",
        "taken",
        taken,
        lookalike,
        taken,
        lookalike,
        taken,
    ]);
});

test("answer a handle's holder and its payer, the reserving organization's for a reserved one", async () => {
    // apply the platform, whose account takes destack and whose organization reserves admin
    await using fixture = await TestAccount.open();
    await new Platform(fixture.store, PLATFORM).apply(fixture.clock.now());
    const person = await fixture.signIn("person@example.com");
    const holder = (handle: string) => person.client(accountPackage).claim.holder({ handle });

    // answer the platform's account for its handle, a lookalike and the handle its organization reserves
    const platform = Account.ofHome(await fixture.platformHome());
    const held = { account: platform, payer: platform };
    expect([
        await holder("destack"),
        await holder("de-stack"),
        await holder("admin"),
        await holder("unclaimed"),
    ]).toEqual([held, held, held, null]);
});

test("transfer a reserved handle to the account it renames, once, keeping its lookalikes taken", async () => {
    // give a person an account, then transfer the reserved security handle to it twice
    await using fixture = await TestAccount.open();
    await new Platform(fixture.store, PLATFORM).apply(fixture.clock.now());
    const person = await fixture.signIn("owner@example.com");
    const owned = await fixture.createAccount(person);
    const transferring = { ...PLATFORM, transfers: { security: owned.id } };
    await new Platform(fixture.store, transferring).apply(fixture.clock.now());
    await new Platform(fixture.store, transferring).apply(fixture.clock.now());

    // resolve the handle to the account, free its old handle, and keep the lookalike taken
    const directory = new Directory(fixture.database.snapshot());
    expect([
        await directory.account("security"),
        await directory.handle(owned.id),
        await availability(person, owned.handle),
        await availability(person, "secu-rity"),
    ]).toEqual([owned.id, "security", "available", "taken"]);
});

test("revoke the bindings of a universe role its definition no longer declares", async () => {
    // apply the platform, then leave a binding of the machines' role an older definition declared
    await using fixture = await TestAccount.open();
    const { database } = fixture.storage;
    await new Platform(fixture.store, PLATFORM).apply(fixture.store.clock.now());
    const [machines] = await database
        .select({ id: accessRole.id })
        .from(accessRole)
        .where(eq(accessRole.name, "machine"));
    const role = present(machines, "the machines' role").id;
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
    await new TestAccess(database, fixture.environment).relate({
        id: Identifier.create("relationship", fixture.environment),
        object: Scope.universe,
        role,
        subject: stale,
        createdAt: fixture.store.clock.now(),
        expiresAt: null,
    });

    // apply it again, keeping exactly the declared bindings
    await new Platform(fixture.store, PLATFORM).apply(fixture.store.clock.now());
    expect(await bindings()).toEqual(declared);
});

test("make the platform organization's owners operators when an apply created it without the operator role", async () => {
    // apply the platform, then drop the operator role with its bindings, as an apply failing after the creation leaves it
    await using fixture = await TestAccount.open();
    const first = await new Platform(fixture.store, PLATFORM).apply(fixture.store.clock.now());
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
    const second = await new Platform(fixture.store, PLATFORM).apply(fixture.store.clock.now());
    expect({
        second,
        platform: await UniverseRole.platform(fixture.database.snapshot()),
    }).toEqual({ second: first, platform: first });
});

test("move the platform's home to a newly declared stack once, with its hostnames", async () => {
    // apply the platform, then declare its home's stack at a new release with hostnames, and apply it twice
    await using fixture = await TestAccount.open();
    await new Platform(fixture.store, PLATFORM).apply(fixture.store.clock.now());
    const released = {
        ...PLATFORM_STACK,
        version: { ...PLATFORM_STACK.version, version: "2026.9.1" },
    };
    const declared: PlatformDefinition = {
        ...PLATFORM,
        domains: ["destack.space", "destack.computer"],
        home: released,
        hostnames: ["*.destack.space", "relay-eu.destack.computer", "relay-us.destack.computer"],
    };
    await new Platform(fixture.store, declared).apply(fixture.store.clock.now());
    await new Platform(fixture.store, declared).apply(fixture.store.clock.now());

    // read the platform accounts' homes and hostnames
    const homes = await fixture.storage.database
        .select({
            handle: account.table.handle,
            stack: space.table.stack,
            generation: space.table.generation,
        })
        .from(space.table)
        .innerJoin(account.table, eq(account.table.id, space.table.scope))
        .where(eq(space.table.id, await fixture.platformHome()));
    const hostnames = await fixture.storage.database
        .select({ handle: account.table.handle, name: hostname.table.name })
        .from(hostname.table)
        .innerJoin(account.table, eq(account.table.id, hostname.table.scope))
        .orderBy(account.table.handle, hostname.table.name);

    // move the one home to its release in one update, and store each hostname once
    expect({ homes, hostnames }).toEqual({
        homes: [{ handle: "destack", stack: released, generation: 2 }],
        hostnames: [
            { handle: "destack", name: "*.destack.space" },
            { handle: "destack", name: "relay-eu.destack.computer" },
            { handle: "destack", name: "relay-us.destack.computer" },
        ],
    });
});

test("apply the declared managed machines with their keys once, and the declared client alone as the platform's publisher", async () => {
    // declare a managed machine for each region
    await using fixture = await TestAccount.open();
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
                regionId: "eu",
                providerCode: PROVIDER,
                location: "eu",
                key: await europe.key(),
                origin: "https://eu-west.machine.test/",
                rates: {},
            },
            {
                id: america.machineId,
                name: "us-west",
                regionId: "us",
                providerCode: PROVIDER,
                location: "us",
                key: await america.key(),
                origin: "https://us-west.machine.test/",
                rates: {},
            },
        ],
    };

    // apply the platform with a workflow it then replaces, twice
    const replaced = {
        name: "Release",
        issuer: ISSUER,
        subject: "repo:destack/destack:environment:old",
    };
    const organizationId = await new Platform(fixture.store, {
        ...definition,
        clients: [replaced],
    }).apply(fixture.store.clock.now());
    await new Platform(fixture.store, definition).apply(fixture.store.clock.now());
    await new Platform(fixture.store, definition).apply(fixture.store.clock.now());

    // create each machine once with one key in the universe, its origin published as its endpoint, and the client publisher once, publishing for each account
    const { database } = fixture.storage;
    const declared = [europe.machineId, america.machineId];
    const machines = await database
        .select({
            id: machine.table.id,
            scope: machine.table.scope,
            name: machine.table.name,
            kind: machine.table.kind,
            regionId: machine.table.regionId,
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
        .where(inArray(endpoint.table.parentId, declared))
        .orderBy(endpoint.table.url);
    const accounts = await database
        .select({ id: account.table.id, handle: account.table.handle })
        .from(account.table)
        .where(inArray(account.table.handle, [PLATFORM.handle, ...PLATFORM.publishers]))
        .orderBy(account.table.handle);
    const publishers = await Promise.all(
        accounts.map(async (row) => [
            row.handle,
            (
                await Relationship.read(
                    database.snapshot(),
                    account.reference(organizationId, row.id),
                    { relation: "publisher" },
                )
            ).map((relation) => [
                relation.subjectType,
                relation.subjectId,
                relation.subjectRelation,
            ]),
        ]),
    );
    const [eu, us] = [await fixture.region("eu"), await fixture.region("us")];
    const trusted = await database
        .select({
            id: client.table.id,
            issuer: client.table.issuer,
            subject: client.table.subject,
        })
        .from(client.table)
        .where(and(eq(client.table.kind, "federated"), isNull(client.table.revokedAt)));
    const workflow = nth(trusted, 0).id;
    expect([machines, keys.map((row) => row.parentId), urls, trusted, publishers]).toEqual([
        [
            {
                id: europe.machineId,
                scope: Account.ofHome(await fixture.platformHome()),
                name: "eu-west",
                kind: "managed",
                regionId: eu.id,
                providerCode: PROVIDER,
                location: "eu",
            },
            {
                id: america.machineId,
                scope: Account.ofHome(await fixture.platformHome()),
                name: "us-west",
                kind: "managed",
                regionId: us.id,
                providerCode: PROVIDER,
                location: "us",
            },
        ],
        declared.toSorted(),
        [{ url: "https://eu-west.machine.test/" }, { url: "https://us-west.machine.test/" }],
        [{ id: workflow, issuer: ISSUER, subject: WORKFLOW }],
        [PLATFORM.handle, ...PLATFORM.publishers]
            .toSorted()
            .map((handle) => [handle, [["client", workflow, null]]]),
    ]);
});

test("invite a new operator as the platform's owner beside the first", async () => {
    // apply the platform for a new operator at a clock standing still, and sign in a stranger and the new operator
    await using fixture = await TestAccount.open();
    const successor = { ...PLATFORM, operator: SUCCESSOR_EMAIL };
    const platformId = await new Platform(fixture.store, successor).apply(fixture.clock.now());
    const stranger = await fixture.signIn(STRANGER_EMAIL);
    const operator = await fixture.signIn(SUCCESSOR_EMAIL);
    const inviter = Scope.universe;

    // list the invitation to the new operator alone, from the universe itself, and accept it
    const invited = (await operator.client(accountPackage).user.invited({ id: operator.id })).items;
    const { id } = nth(invited, 0);
    await operator.client(objectPackage).invitation.accept({
        scope: Scope.universe.id,
        requestId: RequestId.create(fixture.environment),
        id: id,
    });

    // add the new operator as an owner once, beside the first
    await new Platform(fixture.store, successor).apply(fixture.store.clock.now());
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

test("enroll a cloud machine serving a region as an owner of the platform organization, and refuse anyone else", async () => {
    // give a stranger an account beside the platform of the operator
    await using fixture = await TestAccount.open();
    const stranger = await fixture.signIn(STRANGER_EMAIL);
    const { id: accountId } = await fixture.createAccount(stranger);

    // enroll a cloud machine of the eu region as each person
    expect([
        await refusal(
            fixture.enroll(fixture.platform, await fixture.signIn(OPERATOR_EMAIL), {
                regionId: "eu",
            }),
        ),
        await refusal(fixture.enroll(accountId, stranger, { regionId: "eu" })),
        await refusal(fixture.enroll(accountId, stranger)),
    ]).toEqual([
        "done",
        ["FORBIDDEN", "PERMISSION_DENIED", "permission 'operate' is forbidden: it is not granted"],
        "done",
    ]);
});
