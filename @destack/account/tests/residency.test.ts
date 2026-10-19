import { accessRelationship, Relationship } from "@destack/access";
import { and, eq, Snapshot } from "@destack/db";
import { Identifier, present, schema } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { Scope } from "@destack/sync";
import { expect, test } from "@destack/test";
import {
    Account,
    account,
    machine,
    organisation,
    Placement,
    Residency,
    type ResidencyCode,
} from "../src/object/index.ts";
import { AccountFixture, type Actor, ids, outcome } from "../src/test/index.ts";
import { firstPage, place } from "./fixture.ts";
import { Claim, Claimant, RESERVATION_MILLISECONDS } from "@destack/object";

/** The account type, as replicas name it. */
const ACCOUNT_TYPE = {
    packageId: account.policy.definition.packageId,
    type: account.policy.definition.name,
};

test("replicate the chains of a residency's accounts as its platform installation, and refuse another residency's", async () => {
    // store an eu and a us account and a platform installation serving each residency
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId } = await fixture.createSpace(owner);
    const american = await fixture.createSpace(owner, "us");
    const [europe, america] = [await installed(fixture, "eu"), await installed(fixture, "us")];
    const [europeHome, americaHome] = [europe.subject.scope, america.subject.scope];
    const snapshot = Snapshot.live(fixture.storage.database);
    const platformOf = async (home: string) =>
        (await Placement.find(snapshot, [schema.identifier("space").parse(home)]))[0]?.scope;
    const [europePlatform, americaPlatform] = [
        await platformOf(europeHome),
        await platformOf(americaHome),
    ];

    // replicate each account's chain and row as its residency's platform installation alone, the platform's account in each residency included
    expect([
        await replicas(europe, europeHome, accountId),
        await replicas(america, americaHome, accountId),
        await replicas(america, europeHome, accountId),
        await replicas(america, americaHome, american.accountId),
        await replicas(europe, europeHome, american.accountId),
        await accounts(europe, europeHome),
        await accounts(america, americaHome),
    ]).toEqual([
        "executed",
        "FORBIDDEN: permission denied: replicate",
        "FORBIDDEN: permission denied: replicate",
        "executed",
        "FORBIDDEN: permission denied: replicate",
        [ids.other, ids.account, europePlatform, owner.account, accountId],
        [americaPlatform, american.accountId],
    ]);
});

test("claim names of objects in its residency's accounts as a platform installation or a cloud machine, refuse another residency's, and list the expired reservation to both", async () => {
    // store an eu and a us account, an eu platform installation and an eu cloud machine
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const european = await fixture.createSpace(owner);
    const american = await fixture.createSpace(owner, "us");
    const installation = await installed(fixture, "eu");
    const cloud = fixture.caller(await fixture.declareCloudMachine(fixture.platform, "eu"));

    // claim a name in each account
    const claim = async (caller: Actor, accountId: string) => {
        const row = { id: Identifier.create("space"), scope: accountId, name: "notes" };
        const claimant = await Claimant.of(
            place,
            { objectId: row.id, owner: accountId },
            row,
            Snapshot.live(fixture.storage.database),
        );

        return outcome(
            caller.client.claim.reserve({
                ...claimant,
                request: RequestId.create(Date.now()),
                requestId: RequestId.create(Date.now()),
            }),
        );
    };
    expect([
        await claim(installation, european.accountId),
        await claim(installation, american.accountId),
        await claim(cloud, european.accountId),
        await claim(cloud, american.accountId),
    ]).toEqual([
        "executed",
        "FORBIDDEN: permission denied: claim",
        "executed",
        "FORBIDDEN: permission denied: claim",
    ]);

    // list a reservation of a write that never confirmed it, once expired, to the residency's claimants
    const row = { id: Identifier.create("space"), scope: european.accountId, name: "lost" };
    const object = { objectId: row.id, owner: european.accountId };
    const lost = await Claimant.of(place, object, row, Snapshot.live(fixture.database));
    await Claim.reserve(fixture.database, lost, "request", Date.now() - RESERVATION_MILLISECONDS);
    expect([await expired(installation), await expired(cloud)]).toEqual([
        [european.accountId],
        [european.accountId],
    ]);
});

test("read an account's residency home, the residencies' homes and the enabled shared cloud machines serving them with their endpoints", async () => {
    // store an eu and a us account, and cloud machines of the platform's account serving each residency
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId } = await fixture.createSpace(owner);
    const american = await fixture.createSpace(owner, "us");
    const { platform } = fixture;
    const [eu, us, unpublished] = [
        fixture.caller(await fixture.declareCloudMachine(platform, "eu")),
        fixture.caller(await fixture.declareCloudMachine(platform, "us")),
        fixture.caller(await fixture.declareCloudMachine(platform, "eu")),
    ];
    const homes = [await fixture.home("eu"), await fixture.home("us")];
    const euMachine = schema.identifier("machine").parse(eu.id);
    const usMachine = schema.identifier("machine").parse(us.id);
    // publish each machine's endpoint as the machine itself
    await eu.client.machine.publish({
        ...machine.target({ scope: platform, id: euMachine }),
        url: "https://eu.destack.test/",
        relay: null,
        requestId: RequestId.create(Date.now()),
    });
    await us.client.machine.publish({
        ...machine.target({ scope: platform, id: usMachine }),
        url: "https://us.destack.test/",
        relay: null,
        requestId: RequestId.create(Date.now()),
    });

    // drain the eu residency's shared cloud machine
    await fixture.objects.mutate(fixture.objects.serving(machine), {
        id: RequestId.create(Date.now()),
        calls: [
            machine.call("drain", machine.target({ scope: Scope.universe.id, id: ids.machine })),
        ],
    });

    // read each account's residency home, each residency's home, and the machines answering at a URL
    const residencies = unpublished.client.residency;
    const accountHome = (asked: Identifier<"account">) => residencies.locate({ account: asked });
    const homeOf = async (id: ResidencyCode) => (await residencies.get({ id })).home;
    const placed = outcome(Residency.home(residencies, "us"));
    expect([
        await accountHome(accountId),
        await accountHome(american.accountId),
        await homeOf("eu"),
        await homeOf("us"),
        await residencies.machines({ id: "eu" }),
        await residencies.machines({ id: "us" }),
        await placed,
    ]).toEqual([
        ...homes,
        ...homes,
        // list the enabled shared machines alone, leaving out the drained one and the account's own
        [],
        [{ machine: ids.usMachine, url: "https://us-east.machine.test/" }],
        "executed",
    ]);
});

test("read the residency home and the owners of names for another account's objects as a machine", async () => {
    // store two accounts and a name claimed in the first
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const named = await fixture.createSpace(owner);
    const stranger = await fixture.createSpace(await fixture.person("stranger@example.com"));
    const row = { id: Identifier.create("space"), scope: named.accountId, name: "notes" };
    const [claim] = await Claim.list(place, row, Snapshot.live(fixture.storage.database));
    const { index, key } = present(claim, "the claim");
    await Claim.replace(fixture.database, {
        indexes: [index],
        objectId: row.id,
        owner: named.accountId,
        claims: [present(claim, "the claim")],
        state: "active",
    });

    // read both as a machine of the other account
    const strangers = stranger.machine.client;
    expect([
        await strangers.residency.locate({ account: named.accountId }),
        await strangers.claim.owner({ index, key }),
    ]).toEqual([await fixture.home("eu"), { objectId: row.id, owner: named.accountId }]);
});

test("relate a cloud machine as a server of its residency when declared or enrolled, as a resident of the residency's accounts, and release it when revoked", async () => {
    // store an eu and enroll a us cloud machine of the platform
    await using fixture = await AccountFixture.open();
    const declared = await fixture.declareCloudMachine(fixture.platform, "eu");
    const enrolled = await fixture.enroll(fixture.platform, { residencyId: "us" });
    const served = async () => [
        await Residency.of(fixture.database, fixture.caller(declared).subject),
        await Residency.of(fixture.database, fixture.caller(enrolled).subject),
    ];
    const serving = await served();

    // replicate an eu account's chain as the eu cloud machine, and refuse a us account's
    const owner = await fixture.person("owner@example.com");
    const american = await fixture.createSpace(owner, "us");
    const cloud = fixture.caller(declared);
    const replicated = [
        await replicas(cloud, cloud.id, ids.account),
        await replicas(cloud, cloud.id, american.accountId),
    ];

    // release both once the operator revokes them
    const operator = fixture.user(ids.operator);
    for (const identity of [declared, enrolled]) {
        await operator.machine.revoke({
            scope: fixture.platform,
            id: identity.machineId,
            requestId: RequestId.create(Date.now()),
        });
    }
    expect([serving, replicated, await served()]).toEqual([
        ["eu", "us"],
        ["executed", "FORBIDDEN: permission denied: replicate"],
        [undefined, undefined],
    ]);
});

test("relate an organisation's residents again when one of its accounts lives in another residency", async () => {
    // create an eu organisation
    await using fixture = await AccountFixture.open();
    const founder = await fixture.person("founder@example.com");
    const created = await founder.client.organisation.create({
        requestId: RequestId.create(Date.now()),
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
    await founder.client.account.create({
        scope: created.id,
        requestId: RequestId.create(Date.now()),
        handle: "initech-us",
        name: "Initech US",
        kind: "shared",
        residencyId: "us",
    });
    expect([before, await residents()]).toEqual([["eu"], ["eu", "us"]]);
});

/** Request the replica of an account's chain a platform installation stores for its space. */
function chain(space: string, accountId: string) {
    return {
        name: "chain",
        scope: accountId,
        below: space,
        publication: "chain",
        parameters: {
            local: [],
            replicated: [ACCOUNT_TYPE],
            via: [accountId],
            between: [],
            types: [],
        },
    };
}

/** List the owners of the expired reservations an installation claims names for. */
async function expired(caller: Actor): Promise<unknown[]> {
    const listed = await caller.client.claim.expired({ indexes: [Claim.index(place, "name")] });

    return listed.claims.map((entry) => entry.owner);
}

/** Replicate an account's chain as a platform installation for its space, reporting the refusal. */
async function replicas(caller: Actor, space: string, accountId: string): Promise<string> {
    const controller = new AbortController();
    const replicated = caller.client.store
        .stream(chain(space, accountId), { signal: controller.signal })
        .then((pages) => firstPage(pages));

    return outcome(replicated).finally(() => controller.abort());
}

/** Read the accounts a platform installation reads among the universe's rows. */
async function accounts(caller: Actor, space: string): Promise<unknown[]> {
    const controller = new AbortController();
    const pages = await caller.client.store.stream(
        {
            name: space,
            scope: Scope.universe.id,
            below: space,
            publication: "universe",
            parameters: { rows: [{ type: ACCOUNT_TYPE, where: {}, isEverywhere: true }] },
        },
        { signal: controller.signal },
    );
    const page = await firstPage(pages);
    controller.abort();

    return page.changes
        .filter((change) => change.table === "destack__account__account")
        .map((change) => change.row["id"]);
}

/** Install a platform installation serving a residency and connect a caller acting as it. */
async function installed(fixture: AccountFixture, residencyId: ResidencyCode): Promise<Actor> {
    const subject = await fixture.install([], residencyId);

    return { id: subject.id, subject, client: fixture.installation(subject) };
}

test("find the account selling a handle's packages to a residency home's buyers: the handle's own account, the home's platform account for the platform's handles, none otherwise", async () => {
    // store the residencies' homes and a cloud machine asking
    await using fixture = await AccountFixture.open();
    const [europe, america] = [await fixture.home("eu"), await fixture.home("us")];
    const asking = fixture.caller(await fixture.declareCloudMachine(fixture.platform, "eu"));
    const publisher = (handle: string, home: string) =>
        asking.client.residency.publisher({ handle, home: schema.identifier("space").parse(home) });

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
