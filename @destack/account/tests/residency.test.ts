import accountPackage from "../src/package.ts";
import { accessRelationship, CHAIN_PUBLICATION, Relationship } from "@destack/access";
import { and, eq, Snapshot, TABLE } from "@destack/db";
import { Identifier, nth, schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { expect, test, refusal, type Refusal } from "@destack/test";
import {
    Account,
    account,
    machine,
    organisation,
    Placement,
    Residency,
    type ResidencyCode,
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
import { Claim, Claimant, QUERIES_PUBLICATION, RESERVATION_MILLISECONDS } from "@destack/object";

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

/** Read the accounts a residency home reads among the universe's rows. */
async function accounts(caller: TestCaller, space: string): Promise<unknown[]> {
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
        .map((change) => change.row["id"]);
}

/** Act as a residency's home, which serves its accounts. */
async function homeOf(fixture: TestAccountStore, residencyId: ResidencyCode): Promise<TestCaller> {
    return fixture.as(await fixture.home(residencyId));
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
        ],
        [americaPlatform, americanId],
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
        const row = { id: Identifier.create("space"), scope: accountId, name };
        const claimant = await Claimant.of(
            place,
            { objectId: row.id, owner: accountId },
            row,
            Snapshot.live(fixture.storage.database),
        );

        return refusal(
            caller.client(accountPackage).claim.reserve({
                ...claimant,
                request: RequestId.create(fixture.store.clock.now()),
                requestId: RequestId.create(fixture.store.clock.now()),
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
    const homes = [await fixture.home("eu"), await fixture.home("us")];
    const euMachine = schema.identifier("machine").parse(eu.subject.id);
    const usMachine = schema.identifier("machine").parse(us.subject.id);

    // publish each machine's endpoint as the machine itself
    await eu.client(accountPackage).machine.publish({
        ...machine.target({ scope: platform, id: euMachine }),
        url: "https://eu.destack.test/",
        relay: null,
        requestId: RequestId.create(fixture.store.clock.now()),
    });
    await us.client(accountPackage).machine.publish({
        ...machine.target({ scope: platform, id: usMachine }),
        url: "https://us.destack.test/",
        relay: null,
        requestId: RequestId.create(fixture.store.clock.now()),
    });

    // drain the eu residency's shared cloud machine as the machine itself
    await fixture
        .as(fixture.cloud(ids.machine))
        .client(accountPackage)
        .machine.drain({
            ...machine.target({ scope: Scope.universe.id, id: ids.machine }),
            requestId: RequestId.create(fixture.store.clock.now()),
        });

    // read each account's residency home, each residency's home, and the machines answering at a URL
    const residencies = unpublished.client(accountPackage).residency;
    const accountHome = (asked: Identifier<"account">) => residencies.locate({ account: asked });
    const residencyHome = async (id: ResidencyCode) => (await residencies.get({ id })).home;

    // list the enabled shared machines alone, without the drained one and the account's machine
    expect([
        await accountHome(accountId),
        await accountHome(americanId),
        await residencyHome("eu"),
        await residencyHome("us"),
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
    const row = { id: Identifier.create("space"), scope: namedId, name: "notes" };
    const snapshot = Snapshot.live(fixture.database);
    const object = { objectId: row.id, owner: namedId };
    const claimant = await Claimant.of(place, object, row, snapshot);
    await claimNames(await homeOf(fixture, "eu"), fixture.database, claimant, fixture.clock);
    const { index, key } = nth(await Claim.list(place, row, snapshot), 0);

    // read both as a machine of the other account
    const strangers = fixture.as(elsewhere).client(accountPackage);
    expect([
        await strangers.residency.locate({ account: namedId }),
        await strangers.claim.owner({ index, key }),
    ]).toEqual([await fixture.home("eu"), { objectId: row.id, owner: namedId }]);
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
            requestId: RequestId.create(fixture.store.clock.now()),
        });
    }
    expect([serving, replicated, await served()]).toEqual([
        ["eu", "us"],
        ["done", ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"]],
        [undefined, undefined],
    ]);
});

test("relate an organisation's residents again when one of its accounts lives in another residency", async () => {
    // create an eu organisation
    await using fixture = await TestAccountStore.open();
    const founder = await fixture.signIn("founder@example.com");
    const created = await founder.client(accountPackage).organisation.create({
        requestId: RequestId.create(fixture.store.clock.now()),
        name: "Initech",
        handle: "initech",
        residencyId: "eu",
    });
    const object = organisation.reference(Scope.universe.id, created.id);
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
        requestId: RequestId.create(fixture.store.clock.now()),
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
    const [europe, america] = [await fixture.home("eu"), await fixture.home("us")];
    const asking = fixture.as(
        (await fixture.declareCloudMachine(fixture.platform, "eu")).machineId,
    );
    const publisher = (handle: string, home: string) =>
        asking
            .client(accountPackage)
            .residency.publisher({ handle, home: schema.identifier("space").parse(home) });

    // sell a person's packages through their account, and the platform's through each home's account
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
