import accountPackage from "../src/package.ts";
import {
    AccessContext,
    accessRelationship,
    Caller,
    CHAIN_PUBLICATION,
    InstallationId,
    principal,
    Relationship,
} from "@destack/access";
import { and, eq, Snapshot, TABLE } from "@destack/db";
import { Identifier, nth, present, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { expect, test, refusal, type Refusal } from "@destack/test";
import {
    Account,
    account,
    machine,
    organization,
    entitlement,
    Placement,
    Residency,
    residency,
    type ResidencyCode,
    type ResidencyInstallation,
    user,
} from "../src/object/index.ts";
import {
    TestAccountStore,
    ids,
    OPERATOR_EMAIL,
    STRANGER_EMAIL,
    OWNER_EMAIL,
    PEOPLE,
} from "../src/test/index.ts";
import type { TestCaller } from "@destack/object/test";
import { claimNames, firstPage, place } from "./fixture.ts";
import {
    Claim,
    Claimant,
    PACKAGE_DECLARATION,
    PackageHandle,
    QUERIES_PUBLICATION,
    RESERVATION_MILLISECONDS,
} from "@destack/object";
import { describePackage } from "@destack/object/inspect";
import objectPackage from "@destack/object/package";
import { Package } from "@destack/package";
import { MemoryBuild } from "@destack/package/test";

/** The account type, as replicas name it. */
const ACCOUNT_TYPE = {
    packageId: account.policy.definition.packageId,
    type: account.policy.definition.name,
};

/** Request the replica of an account's chain a residency home stores for its space. */
function chain(space: string, accountId: string) {
    return {
        name: CHAIN_PUBLICATION,
        scope: accountId,
        target: space,
        publication: CHAIN_PUBLICATION,
        parameters: { scopes: [], local: [], types: [], via: [accountId], between: [] },
    };
}

/** List the owners of the expired reservations a caller claims names for. */
async function expired(caller: TestCaller): Promise<unknown[]> {
    const listed = await caller
        .client(accountPackage)
        .claim.expired({ indexes: [Claim.index(place, "name")] });

    return listed.claims.map((entry) => entry.owner);
}

/** Replicate an account's chain as a residency home for its space, reporting the refusal. */
async function replicas(caller: TestCaller, space: string, accountId: string): Promise<Refusal> {
    const controller = new AbortController();
    const replicated = caller
        .client(accountPackage)
        .store.stream(chain(space, accountId), { signal: controller.signal })
        .then((pages) => firstPage(pages));

    return refusal(replicated).finally(() => controller.abort());
}

/** Read the accounts a residency home reads among the universe's rows, by identifier. */
async function accounts(caller: TestCaller, space: string): Promise<string[]> {
    const controller = new AbortController();
    const pages = await caller.client(accountPackage).store.stream(
        {
            name: space,
            scope: Scope.universe.id,
            target: space,
            publication: QUERIES_PUBLICATION,
            parameters: { queries: { account: { object: ACCOUNT_TYPE.type, scopes: "every" } } },
        },
        { signal: controller.signal },
    );
    const page = await firstPage(pages);
    controller.abort();

    return page.changes
        .filter((change) => change.table === account.table[TABLE].sqlName)
        .map((change) => schema.string().parse(change.row["id"]))
        .toSorted();
}

/** Act as a residency's home, which serves its accounts. */
async function homeOf(fixture: TestAccountStore, residencyId: ResidencyCode): Promise<TestCaller> {
    return fixture.as(await fixture.residencySpace(residencyId));
}

test("replicate the chains of a residency's accounts as its home, and refuse another residency's", async () => {
    // store an eu and a us account and each residency's home
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const { id: americanId } = await fixture.createAccount(owner, "us");
    const [europe, america] = [await homeOf(fixture, "eu"), await homeOf(fixture, "us")];
    const [europeHome, americaHome] = [europe.subject.id, america.subject.id];
    const snapshot = Snapshot.live(fixture.storage.database);
    const platformOf = async (home: string) =>
        nth(await Placement.find(snapshot, [schema.identifier("space").parse(home)]), 0).scope;
    const [europePlatform, americaPlatform] = [
        await platformOf(europeHome),
        await platformOf(americaHome),
    ];

    // replicate each account's chain and row as its residency's home alone, the platform's account in each residency included
    expect([
        await replicas(europe, europeHome, accountId),
        await replicas(america, americaHome, accountId),
        await replicas(america, europeHome, accountId),
        await replicas(america, americaHome, americanId),
        await replicas(europe, europeHome, americanId),
        await accounts(europe, europeHome),
        await accounts(america, americaHome),
    ]).toEqual([
        "done",
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"],
        "done",
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"],
        [
            ids.other,
            ...PEOPLE.map((person) => Account.personal(person.id)),
            europePlatform,
            accountId,
        ].toSorted(),
        [americaPlatform, americanId].toSorted(),
    ]);
});

test("claim names of objects in its residency's accounts as a residency home or a cloud machine, refuse another residency's, and list the expired reservation to both", async () => {
    // store an eu and a us account, the eu home and an eu cloud machine
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: europeanId } = await fixture.createAccount(owner);
    const { id: americanId } = await fixture.createAccount(owner, "us");
    const home = await homeOf(fixture, "eu");
    const cloud = fixture.as(await fixture.declareCloudMachine(fixture.platform, "eu"));

    // reserve a name in each account
    const reserve = async (caller: TestCaller, accountId: string, name: string) => {
        const row = { id: Identifier.create("space", fixture.environment), scope: accountId, name };
        const claimant = await Claimant.of(
            place,
            { objectId: row.id, owner: accountId },
            row,
            Snapshot.live(fixture.storage.database),
        );

        return refusal(
            caller.client(accountPackage).claim.reserve({
                ...claimant,
                request: RequestId.create(fixture.store.environment),
                requestId: RequestId.create(fixture.store.environment),
            }),
        );
    };
    expect([
        await reserve(home, europeanId, "notes"),
        await reserve(home, americanId, "notes"),
        await reserve(cloud, europeanId, "notes"),
        await reserve(cloud, americanId, "notes"),
    ]).toEqual([
        "done",
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: claim"],
        "done",
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: claim"],
    ]);

    // list a reservation of a write that never confirmed it, once expired, to the residency's claimants
    await reserve(home, europeanId, "lost");
    fixture.clock.advance(RESERVATION_MILLISECONDS + 1);
    expect([await expired(home), await expired(cloud)]).toEqual([
        [europeanId, europeanId],
        [europeanId, europeanId],
    ]);
});

test("read an account's residency home, the residencies' homes and the enabled shared cloud machines serving them with their endpoints", async () => {
    // store an eu and a us account, and cloud machines of the platform's account serving each residency
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const { id: americanId } = await fixture.createAccount(owner, "us");
    const { platform } = fixture;
    const [eu, us, unpublished] = [
        fixture.as(await fixture.declareCloudMachine(platform, "eu")),
        fixture.as(await fixture.declareCloudMachine(platform, "us")),
        fixture.as(await fixture.declareCloudMachine(platform, "eu")),
    ];
    const homes = [await fixture.residencySpace("eu"), await fixture.residencySpace("us")];
    const euMachine = schema.identifier("machine").parse(eu.subject.id);
    const usMachine = schema.identifier("machine").parse(us.subject.id);

    // publish each machine's endpoint as the machine itself
    await eu.client(accountPackage).machine.publish({
        ...machine.target({ scope: platform, id: euMachine }),
        url: "https://eu.destack.test/",
        requestId: RequestId.create(fixture.store.environment),
    });
    await us.client(accountPackage).machine.publish({
        ...machine.target({ scope: platform, id: usMachine }),
        url: "https://us.destack.test/",
        requestId: RequestId.create(fixture.store.environment),
    });

    // drain the eu residency's shared cloud machine as the machine itself
    await fixture
        .as(fixture.cloud(ids.machine))
        .client(accountPackage)
        .machine.drain({
            ...machine.target({ scope: Scope.universe.id, id: ids.machine }),
            requestId: RequestId.create(fixture.store.environment),
        });

    // read each account's residency space, each residency's space, and the machines answering at a URL
    const residencies = unpublished.client(accountPackage).residency;
    const accountSpace = (asked: Identifier<"account">) => residencies.locate({ account: asked });
    const residencySpace = async (id: ResidencyCode) => (await residencies.get({ id })).space;

    // list the enabled shared machines alone, without the drained one and the account's machine
    expect([
        await accountSpace(accountId),
        await accountSpace(americanId),
        await residencySpace("eu"),
        await residencySpace("us"),
        await residencies.machines({ id: "eu" }),
        await residencies.machines({ id: "us" }),
    ]).toEqual([
        ...homes,
        ...homes,
        [],
        [{ machine: ids.usMachine, url: "https://us-east.machine.test/" }],
    ]);
});

test("read the residency home and the owners of names for another account's objects as a machine", async () => {
    // store two accounts and a name the residency's home claims in the first
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: namedId } = await fixture.createAccount(owner);
    const stranger = await fixture.signIn(STRANGER_EMAIL);
    const elsewhere = await fixture.enroll(Account.personal(ids.stranger), stranger);
    const row = {
        id: Identifier.create("space", fixture.environment),
        scope: namedId,
        name: "notes",
    };
    const snapshot = Snapshot.live(fixture.database);
    const object = { objectId: row.id, owner: namedId };
    const claimant = await Claimant.of(place, object, row, snapshot);
    await claimNames(await homeOf(fixture, "eu"), fixture.database, claimant, fixture.environment);
    const { index, key } = nth(await Claim.list(place, row, snapshot), 0);

    // read both as a machine of the other account
    const strangers = fixture.as(elsewhere).client(accountPackage);
    expect([
        await strangers.residency.locate({ account: namedId }),
        await strangers.claim.owner({ index, key }),
    ]).toEqual([await fixture.residencySpace("eu"), { objectId: row.id, owner: namedId }]);
});

test("relate a cloud machine as a server of its residency when declared or enrolled, as a resident of the residency's accounts, and release it when revoked", async () => {
    // store an eu and enroll a us cloud machine of the platform
    await using fixture = await TestAccountStore.open();
    const declared = await fixture.declareCloudMachine(fixture.platform, "eu");
    const operator = await fixture.signIn(OPERATOR_EMAIL);
    const enrolled = await fixture.enroll(fixture.platform, operator, { residencyId: "us" });
    const served = async () => [
        await Residency.of(fixture.database, fixture.as(declared).subject),
        await Residency.of(fixture.database, fixture.as(enrolled).subject),
    ];
    const serving = await served();

    // replicate an eu account's chain as the eu cloud machine, and refuse a us account's
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: americanId } = await fixture.createAccount(owner, "us");
    const cloud = fixture.as(declared);
    const replicated = [
        await replicas(cloud, cloud.subject.id, ids.account),
        await replicas(cloud, cloud.subject.id, americanId),
    ];

    // release both once the operator revokes them
    for (const identity of [declared, enrolled]) {
        await operator.client(accountPackage).machine.revoke({
            scope: fixture.platform,
            id: identity.machineId,
            requestId: RequestId.create(fixture.store.environment),
        });
    }
    expect([serving, replicated, await served()]).toEqual([
        ["eu", "us"],
        ["done", ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"]],
        [undefined, undefined],
    ]);
});

test("relate a shared account's contents to its residency as residents beside the residency's servers on the account, and unrelate both once the account is purged", async () => {
    // create a shared eu account as its owner stepped up
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.elevate(await fixture.signIn("founder@example.com"));
    const client = owner.client(accountPackage);
    const created = await client.account.create({
        scope: owner.id,
        requestId: RequestId.create(fixture.store.environment),
        handle: "initech",
        name: "Initech",
        kind: "shared",
        residencyId: "eu",
    });

    // read the residents on the account and the account's contents on the residency
    const residents = async () => {
        const rows = await fixture.database
            .select({
                objectType: accessRelationship.objectType,
                objectId: accessRelationship.objectId,
                subjectId: accessRelationship.subjectId,
                subjectRelation: accessRelationship.subjectRelation,
            })
            .from(accessRelationship)
            .where(eq(accessRelationship.relation, "resident"));

        return rows
            .filter((row) => row.subjectId === created.id || row.objectId === created.id)
            .map((row) => [row.objectType, row.subjectId, row.subjectRelation])
            .toSorted((left, right) => left.join().localeCompare(right.join()));
    };
    const related = await residents();

    // delete and purge the account
    const target = {
        id: created.id,
        scope: owner.id,
        requestId: RequestId.create(fixture.environment),
    };
    await client.account.delete(target);
    await client.account.purge({ ...target, requestId: RequestId.create(fixture.environment) });
    expect({ related, purged: await residents() }).toEqual({
        related: [
            [account.name, "eu", "server"],
            [residency.name, created.id, "contained"],
        ],
        purged: [],
    });
});

test("relate an organization's residents again when one of its accounts lives in another residency", async () => {
    // create an eu organization
    await using fixture = await TestAccountStore.open();
    const founder = await fixture.signIn("founder@example.com");
    const created = await founder.client(accountPackage).organization.create({
        requestId: RequestId.create(fixture.store.environment),
        name: "Initech",
        handle: "initech",
        residencyId: "eu",
    });
    const object = organization.reference(Scope.universe.id, created.id);
    const residents = async () =>
        (
            await fixture.database
                .select({ residencyId: accessRelationship.subjectId })
                .from(accessRelationship)
                .where(and(Relationship.on(object), eq(accessRelationship.relation, "resident")))
        )
            .map((row) => row.residencyId)
            .toSorted();
    const before = await residents();

    // create a us account in it
    await founder.client(accountPackage).account.create({
        scope: created.id,
        requestId: RequestId.create(fixture.store.environment),
        handle: "initech-us",
        name: "Initech US",
        kind: "shared",
        residencyId: "us",
    });
    expect([before, await residents()]).toEqual([["eu"], ["eu", "us"]]);
});

test("find the account selling a handle's packages to a residency home's buyers: the handle's account, the home's platform account for the platform's handles, none otherwise", async () => {
    // store the residencies' homes and a cloud machine asking
    await using fixture = await TestAccountStore.open();
    const [europe, america] = [await fixture.residencySpace("eu"), await fixture.residencySpace("us")];
    const asking = fixture.as(
        (await fixture.declareCloudMachine(fixture.platform, "eu")).machineId,
    );
    const publisher = (handle: string, spaceId: string) =>
        asking
            .client(accountPackage)
            .residency.publisher({ handle, space: schema.identifier("space").parse(spaceId) });

    // sell a person's packages through their account, and the platform's through each residency space's account
    expect([
        await publisher("acme", europe),
        await publisher("destack", europe),
        await publisher("destack", america),
        await publisher("destack-us", europe),
        await publisher("unclaimed", europe),
        await publisher("destack", Account.home(ids.account)),
    ]).toEqual([
        ids.account,
        Account.ofHome(europe),
        Account.ofHome(america),
        Account.ofHome(europe),
        null,
        null,
    ]);
});

test("relate an installation its residency's space declares once its release asks for exactly what the relation gives, refusing another residency's space, another installation, and a release asking for more or fewer, and unrelate it", async () => {
    // publish one release asking for what a residency's installation holds, and one asking for more
    const billing = Package.parse({
        id: "package-019f5530-8000-7000-8000-0000000000b1",
        name: "@example/billing",
        version: "2026.9.0",
    });
    const greedy = Package.parse({
        id: "package-019f5530-8000-7000-8000-0000000000b2",
        name: "@example/greedy",
        version: "2026.9.0",
    });
    const modest = Package.parse({
        id: "package-019f5530-8000-7000-8000-0000000000b3",
        name: "@example/modest",
        version: "2026.9.0",
    });
    const universal = [account.permission("serve"), entitlement.permission("provide")];
    const asking = new Map([
        [billing.id, universal],
        [greedy.id, [...universal, user.permission("suspend")]],
        [modest.id, [account.permission("serve")]],
    ]);
    const opened = new Map([billing, greedy, modest].map((owner) => [owner.id, owner]));
    await using fixture = await TestAccountStore.open({
        openRelease: (packageId) => {
            const owner = present(opened.get(packageId), `the release of ${packageId}`);
            const handle = PackageHandle.of(owner, {
                permissions: { installation: { universe: asking.get(packageId) ?? [] } },
            });

            return MemoryBuild.of(
                owner,
                [
                    {
                        kind: PACKAGE_DECLARATION,
                        kindPackageId: objectPackage.id,
                        name: "default",
                        description: describePackage(handle),
                    },
                ],
                fixture.environment.crypto,
            );
        },
    });

    // derive the installation the eu home declares as billing
    const home = await homeOf(fixture, "eu");
    const homeId = schema.identifier("space").parse(home.subject.id);
    const snapshot = Snapshot.live(fixture.storage.database);
    const homeSpace = await Scope.object(snapshot, homeId);
    const crypto = fixture.environment.crypto;
    const installation = await InstallationId.declared(homeSpace, "billing", crypto);
    const target = () => ({ id: "eu", requestId: RequestId.create(fixture.store.environment) });
    const touch = (caller: TestCaller, input: Partial<ResidencyInstallation> = {}) =>
        refusal(
            caller.client(accountPackage).residency.touch({
                ...target(),
                installation,
                name: "billing",
                packageId: billing.id,
                release: billing.version,
                ...input,
            }),
        );
    const related = async () =>
        (
            await Relationship.readOn(snapshot, residency.reference(Scope.universe.id, "eu"), {
                relation: "installation",
            })
        ).map((row) => row.subjectId);

    // refuse another residency's home, another installation, and releases asking for more or fewer
    const foreignHome = await homeOf(fixture, "us");
    const other = Identifier.create("installation", fixture.environment);
    const refused = [
        await touch(foreignHome),
        await touch(home, { installation: other }),
        await touch(home, { packageId: greedy.id, release: greedy.version }),
        await touch(home, { packageId: modest.id, release: modest.version }),
    ];

    // relate the declared installation once, then unrelate it twice, the second leaving it unrelated
    const touched = [await touch(home), await touch(home), await related()];
    const revoke = () =>
        home
            .client(accountPackage)
            .residency.revoke({ ...target(), installation, name: "billing" });
    await revoke();
    await revoke();

    expect([refused, touched, await related()]).toEqual([
        [
            ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: install"],
            [
                "FORBIDDEN",
                "INSTALLATION_NOT_DECLARED",
                `installation ${other} is not billing of the space ${homeId}`,
            ],
            [
                "FORBIDDEN",
                "UNIVERSE_PERMISSIONS_MISMATCH",
                `${greedy.id}@2026.9.0 asks for account.serve, entitlement.provide, user.suspend in the universe, not what a residency's installation holds`,
            ],
            [
                "FORBIDDEN",
                "UNIVERSE_PERMISSIONS_MISMATCH",
                `${modest.id}@2026.9.0 asks for account.serve in the universe, not what a residency's installation holds`,
            ],
        ],
        ["done", "done", [installation]],
        [],
    ]);
});

test("let an installation its residency's home declares provide entitlements in the residency's accounts once related, and not before", async () => {
    // store an eu account, and derive the installation the eu home declares as billing
    const billing = Package.parse({
        id: "package-019f5530-8000-7000-8000-0000000000b1",
        name: "@example/billing",
        version: "2026.9.0",
    });
    await using fixture = await TestAccountStore.open({
        openRelease: () =>
            MemoryBuild.of(
                billing,
                [
                    {
                        kind: PACKAGE_DECLARATION,
                        kindPackageId: objectPackage.id,
                        name: "default",
                        description: describePackage(
                            PackageHandle.of(billing, {
                                permissions: {
                                    installation: {
                                        universe: [
                                            account.permission("serve"),
                                            entitlement.permission("provide"),
                                        ],
                                    },
                                },
                            }),
                        ),
                    },
                ],
                fixture.environment.crypto,
            ),
    });
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const home = await homeOf(fixture, "eu");
    const homeId = schema.identifier("space").parse(home.subject.id);
    const homeSpace = await Scope.object(Snapshot.live(fixture.storage.database), homeId);
    const installation = await InstallationId.declared(
        homeSpace,
        "billing",
        fixture.environment.crypto,
    );
    const installed = principal.installation.reference(homeId, installation);

    // provide an entitlement in the account as the installation, before and after its home relates it
    const provide = () =>
        refusal(
            fixture.store.mutate(
                AccessContext.of(Caller.of(installed), fixture.store.clock.now()),
                {
                    id: RequestId.create(fixture.store.environment),
                    calls: [
                        entitlement.call("create", {
                            ...entitlement.in(accountId),
                            packageId: billing.id,
                            feature: "seats",
                            resource: null,
                            kind: "boolean",
                            value: null,
                            limit: null,
                            state: null,
                        }),
                    ],
                },
            ),
        );
    const before = await provide();
    await home.client(accountPackage).residency.touch({
        id: "eu",
        requestId: RequestId.create(fixture.store.environment),
        installation,
        name: "billing",
        packageId: billing.id,
        release: billing.version,
    });
    const after = await provide();

    // conceal the account's entitlements from the installation until it serves the account
    expect([before, after]).toEqual([["NOT_FOUND", "NOT_FOUND", "entitlement not found"], "done"]);
});
