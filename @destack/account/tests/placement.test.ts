import objectPackage from "@destack/object/package";
import { CHAIN_PUBLICATION, principal } from "@destack/access";
import { Snapshot, TABLE } from "@destack/db";
import { nth, Identifier, present, schema } from "@destack/schema";
import { RequestId, ObjectReference, Scope } from "@destack/sync";
import { expect, test, refusal, type Refusal } from "@destack/test";
import { Directory } from "../src/address/index.ts";
import {
    account,
    assignment,
    directoryShape,
    endpoint,
    Endpoint,
    machine,
    machineShape,
    Placement,
    space,
} from "../src/object/index.ts";
import { ACCESS_TOKEN_TYPE } from "../src/oauth/index.ts";
import accountPackage from "../src/package.ts";
import { TestAccountStore, ISSUER, type TestPerson, OWNER_EMAIL } from "../src/test/index.ts";
import type { TestCaller } from "@destack/object/test";
import { claimNames, firstPage, place } from "./fixture.ts";
import { Claim, Claimant } from "@destack/object";

/** The machines a residency's spaces spread over. */
const MACHINES = [1, 2, 3, 4].map((index) =>
    schema
        .identifier("machine")
        .parse(`machine-01996ab0-0000-7000-8000-${String(index).padStart(12, "0")}`),
);

/** The spaces placed in the scenario. */
const SPACES = Array.from({ length: 200 }, (_, index) =>
    schema
        .identifier("space")
        .parse(`space-01996ab0-0000-7000-8000-${String(index).padStart(12, "0")}`),
);

/** Serve an account with a laptop serving a space called notes, a peer and an outside machine. */
async function openPlacement() {
    // enroll two machines of the owner's account and a machine of another account
    const fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId, handle } = await fixture.createAccount(owner);
    const other = await fixture.createAccount(owner);
    const identity = await fixture.enroll(accountId, owner);
    const [local, peer, outsider] = [
        fixture.as(identity),
        fixture.as(await fixture.enroll(accountId, owner)),
        fixture.as(await fixture.enroll(other.id, owner)),
    ];

    // create the notes space on the laptop
    const created = await owner.client(accountPackage).space.create({
        accountId,
        requestId: RequestId.create(fixture.clock.now()),
        name: "notes",
        machine: identity.machineId,
    });

    return {
        fixture,
        owner,
        accountId,
        handle,
        otherId: other.id,
        identity,
        local,
        peer,
        outsider,
        laptop: created.id,
    };
}

/** Place a space again at an epoch as a machine, reporting the outcome. */
function serve(
    fixture: TestAccountStore,
    caller: TestCaller,
    accountId: Identifier<"account">,
    id: Identifier<"space">,
    epoch: number,
): Promise<Refusal> {
    return refusal(
        caller.client(accountPackage).placement.place({
            accountId,
            id,
            epoch,
            requestId: RequestId.create(fixture.clock.now()),
        }),
    );
}

/** Move a space at an epoch to a machine as a machine, reporting the outcome. */
function move(
    fixture: TestAccountStore,
    caller: TestCaller,
    target: { readonly accountId: Identifier<"account">; readonly id: Identifier<"space"> },
    epoch: number,
    to: string,
): Promise<Refusal> {
    return refusal(
        caller.client(accountPackage).placement.move({
            ...target,
            epoch,
            target: schema.identifier("machine").parse(to),
            transferId: Identifier.create("transfer"),
            requestId: RequestId.create(fixture.clock.now()),
        }),
    );
}

/** Read the machine rows a machine replicates of the directory, with their statuses. */
async function machines(copier: TestCaller): Promise<unknown[][]> {
    const controller = new AbortController();
    const pages = await copier
        .client(accountPackage)
        .store.stream(
            directoryShape.subscription({ scope: Scope.universe.id }, copier.subject.id),
            { signal: controller.signal },
        );
    const page = await firstPage(pages);
    controller.abort();

    return page.changes
        .filter((change) => change.table === machine.table[TABLE].sqlName)
        .map((change) => [change.row["id"], change.row["status"]]);
}

/** Publish the URL a machine answers at as a caller, reporting the outcome. */
function publish(
    fixture: TestAccountStore,
    caller: TestCaller,
    target: { readonly scope: string; readonly machine: string },
    published: string,
): Promise<Refusal> {
    return refusal(
        caller.client(accountPackage).machine.publish({
            scope: target.scope,
            id: schema.identifier("machine").parse(target.machine),
            url: published,
            relay: null,
            requestId: RequestId.create(fixture.clock.now()),
        }),
    );
}

/** Read where a space runs as a caller. */
async function locate(caller: TestCaller, accountId: string, spaceId: string) {
    const { id, scope, machineId, epoch } = await caller.client(accountPackage).placement.get({
        accountId: schema.identifier("account").parse(accountId),
        id: schema.identifier("space").parse(spaceId),
    });

    return { id, scope, machine: machineId, epoch };
}

/** Read the URL a machine answers at as a caller, none once its tunnel lapsed. */
async function url(fixture: TestAccountStore, caller: TestCaller, scope: string, id: string) {
    const target = endpoint.target({ scope, id: schema.identifier("machine").parse(id) });
    const found = await caller.client(accountPackage).endpoint.get(target);

    return Endpoint.isAnswering(found, fixture.clock.now()) ? found.url : null;
}

/** Find the owners of names as a caller. */
function claims(caller: TestCaller) {
    return {
        owner: async (index: string, key: string) =>
            (await caller.client(accountPackage).claim.owner({ index, key })) ?? undefined,
    };
}

/** Read the spaces a machine replicates as giving it work. */
async function assignments(caller: TestCaller): Promise<unknown[]> {
    const controller = new AbortController();
    const pages = await caller.client(accountPackage).store.stream(
        machineShape.subscription(
            {
                scope: Scope.universe.id,
                machine: schema.identifier("machine").parse(caller.subject.id),
            },
            caller.subject.id,
        ),
        { signal: controller.signal },
    );
    const page = await firstPage(pages);
    controller.abort();

    return page.changes
        .filter((change) => change.table === assignment.table[TABLE].sqlName)
        .map((change) => change.row["placementId"]);
}

/** Refuse moving a space to a machine that serves neither its account nor its residency. */
function unknownMachine(caller: TestCaller): Refusal {
    return [
        "BAD_REQUEST",
        "UNKNOWN_MACHINE",
        `${caller.subject.id} is not a standing machine of the space's account or residency`,
    ];
}

/** Enroll a machine of an account as a person, and act as it. */
async function enroll(
    fixture: TestAccountStore,
    accountId: Identifier<"account">,
    person: TestPerson,
): Promise<TestCaller> {
    return fixture.as(await fixture.enroll(accountId, person));
}

test("place a created space again only as the machine serving it, and let the account's other machines read where it runs", async () => {
    // place the laptop's space again as each machine, and a space nobody created
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, laptop, local, peer, outsider } = opened;
    const foreign = Identifier.create("space");
    const placedOn = { id: laptop, scope: accountId, machine: local.subject.id, epoch: 1 };

    // place it again as its machine alone, refusing the peer, an unknown space and another account's machine
    expect([
        await serve(fixture, local, accountId, laptop, 1),
        await serve(fixture, peer, accountId, laptop, 1),
        await serve(fixture, peer, accountId, foreign, 1),
        await serve(fixture, outsider, accountId, laptop, 1),
        await locate(peer, accountId, laptop),
    ]).toEqual([
        "done",
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: place"],
        ["NOT_FOUND", "NOT_FOUND", `placement ${foreign} not found`],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: place"],
        placedOn,
    ]);
});

test("reserve and confirm a space's names only as the machine serving it", async () => {
    // reserve the laptop space's name as the peer and as the laptop
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, laptop, local, peer, outsider } = opened;
    const row = { id: laptop, scope: accountId, name: "notes" };
    const object = { objectId: laptop, owner: accountId };
    const claimant = await Claimant.of(place, object, row, Snapshot.live(fixture.database));
    const reserve = (caller: TestCaller) =>
        refusal(claimNames(caller, fixture.database, claimant, fixture.clock));

    // refuse the peer, confirm it as the laptop, and find it as any machine
    expect([
        await reserve(peer),
        await reserve(local),
        await Claim.lookup(place, claims(outsider), "name", ["notes"], accountId),
    ]).toEqual([
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: claim"],
        "done",
        { scope: accountId, id: laptop },
    ]);
});

test("store the account's row in the chain replica of a space storing accounts", async () => {
    // start the laptop space's identity as its machine, and follow its account's chain as the space
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, laptop, local } = opened;
    await fixture.startIdentity(laptop, local);
    const following = new AbortController();
    const chain = await fixture
        .as(laptop)
        .client(accountPackage)
        .store.stream(
            {
                name: CHAIN_PUBLICATION,
                scope: accountId,
                target: laptop,
                publication: CHAIN_PUBLICATION,
                parameters: {
                    scopes: [],
                    local: [],
                    types: [
                        {
                            packageId: account.policy.definition.packageId,
                            type: account.policy.definition.name,
                        },
                    ],
                    via: [],
                    between: [],
                },
            },
            { signal: following.signal },
        );
    const first = await firstPage(chain);
    following.abort();

    // store the account's row unconcealed
    expect(
        first.changes
            .filter((change) => change.table === account.table[TABLE].sqlName)
            .map((change) => [change.row["id"], change.concealed]),
    ).toEqual([[accountId, undefined]]);
});

test("refuse a machine exchanging a token for itself, which it asks for with its assertion", async () => {
    // take the laptop machine's token for the account package
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { identity, laptop } = opened;
    const accessToken = await identity.token(accountPackage.id, { url: ISSUER }, (request) =>
        fixture.fetch(request),
    );

    // refuse exchanging it for a token in its space
    expect(
        await refusal(
            fixture.issuer.exchange(
                { token: accessToken, type: ACCESS_TOKEN_TYPE },
                accountPackage.id,
                { spaceId: laptop },
            ),
        ),
    ).toEqual([
        "BAD_REQUEST",
        "INVALID_GRANT",
        "the token endpoint refused the request: invalid_grant: a machine must request its tokens with its assertion",
    ]);
});

test("move a space only to a standing machine of its account, which serves it from the next epoch on", async () => {
    // store a cloud machine of the account, one of another account, and a disabled and a revoked machine of the account
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { owner, accountId, otherId, laptop, local, outsider } = opened;
    const cloud = fixture.as(await fixture.declareCloudMachine(accountId, "eu"));
    const rival = fixture.as(await fixture.declareCloudMachine(otherId, "eu"));
    const [disabled, revoked] = [
        await enroll(fixture, accountId, owner),
        await enroll(fixture, accountId, owner),
    ];
    const status = (caller: TestCaller) => ({
        scope: accountId,
        id: caller.subject.id,
        requestId: RequestId.create(fixture.clock.now()),
    });
    await owner.client(accountPackage).machine.disable(status(disabled));
    await owner.client(accountPackage).machine.revoke(status(revoked));

    // refuse moving it to the other account's machines, the disabled and the revoked one, then move it to the cloud machine
    const target = { accountId, id: laptop };
    const refused = [
        await move(fixture, local, target, 1, outsider.subject.id),
        await move(fixture, local, target, 1, rival.subject.id),
        await move(fixture, local, target, 1, disabled.subject.id),
        await move(fixture, local, target, 1, revoked.subject.id),
    ];
    await move(fixture, local, target, 1, cloud.subject.id);

    // serve it at the next epoch as the cloud machine alone
    expect([
        refused,
        await serve(fixture, cloud, accountId, laptop, 2),
        await serve(fixture, local, accountId, laptop, 3),
        await locate(local, accountId, laptop),
    ]).toEqual([
        [
            unknownMachine(outsider),
            unknownMachine(rival),
            unknownMachine(disabled),
            unknownMachine(revoked),
        ],
        "done",
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: place"],
        { id: laptop, scope: accountId, machine: cloud.subject.id, epoch: 2 },
    ]);
});

test("publish a machine's URL only as the machine, and read it as any machine and through the directory", async () => {
    // claim the laptop space's name, and publish the laptop's URL as itself and as the peer
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, handle, laptop, local, peer, outsider } = opened;
    const row = { id: laptop, scope: accountId, name: "notes" };
    const object = { objectId: laptop, owner: accountId };
    const claimant = await Claimant.of(place, object, row, Snapshot.live(fixture.database));
    await claimNames(local, fixture.database, claimant, fixture.clock);
    const published = [
        await publish(
            fixture,
            local,
            { scope: accountId, machine: local.subject.id },
            "https://local.test/",
        ),
        await publish(
            fixture,
            peer,
            { scope: accountId, machine: local.subject.id },
            "https://peer.test/",
        ),
    ];

    // read it as the peer and another account's machine, and resolve the laptop's space through the directory
    const resolver = new Directory(Snapshot.live(fixture.database));
    expect([
        published,
        await url(fixture, peer, accountId, local.subject.id),
        await url(fixture, outsider, accountId, local.subject.id),
        await resolver.resolve(place, `notes--${handle}`, fixture.clock.now()),
        await refusal(resolver.resolve(place, `missing--${handle}`, fixture.clock.now())),
    ]).toEqual([
        ["done", ["NOT_FOUND", "NOT_FOUND", `machine ${local.subject.id} not found`]],
        "https://local.test/",
        "https://local.test/",
        {
            id: laptop,
            placement: { id: laptop, scope: accountId, machine: local.subject.id, epoch: 1 },
            url: "https://local.test/",
        },
        ["NOT_FOUND", "ADDRESS_NOT_FOUND", `nothing responds at missing--${handle}`],
    ]);
});

test("withdraw a revoked machine's published URL", async () => {
    // publish a machine's URL, then revoke it as the owner
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { owner, accountId, peer } = opened;
    const revoked = await enroll(fixture, accountId, owner);
    await publish(
        fixture,
        revoked,
        { scope: accountId, machine: revoked.subject.id },
        "https://revoked.test/",
    );
    await owner.client(accountPackage).machine.revoke({
        scope: accountId,
        id: revoked.subject.id,
        requestId: RequestId.create(fixture.clock.now()),
    });

    // find no endpoint of it
    expect(await refusal(url(fixture, peer, accountId, revoked.subject.id))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `endpoint ${revoked.subject.id} not found`,
    ]);
});

test("move and withdraw a space only as the machine serving it, at the epoch it serves it", async () => {
    // move the laptop's space toward the peer
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, laptop, local, peer } = opened;
    const target = { accountId, id: laptop };
    const withdraw = (caller: TestCaller, epoch: number) =>
        refusal(
            caller.client(accountPackage).placement.withdraw({
                ...target,
                epoch,
                requestId: RequestId.create(fixture.clock.now()),
            }),
        );

    // refuse the peer and a stale epoch, move it as the laptop, and withdraw it as the laptop alone
    const moves = [
        await move(fixture, peer, target, 1, peer.subject.id),
        await move(fixture, local, target, 2, peer.subject.id),
        await move(fixture, local, target, 1, peer.subject.id),
    ];
    const moving = await peer.client(accountPackage).placement.get(target);
    expect([
        moves,
        [moving.machineId, moving.targetId],
        await withdraw(peer, 1),
        await withdraw(local, 1),
        await refusal(locate(local, accountId, laptop)),
    ]).toEqual([
        [
            ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: move"],
            ["CONFLICT", "STALE_EPOCH", `${laptop} is no longer placed at epoch 2`],
            "done",
        ],
        [local.subject.id, peer.subject.id],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: move"],
        "done",
        ["NOT_FOUND", "NOT_FOUND", `placement ${laptop} not found`],
    ]);
});

test("replicate the spaces giving a machine work to that machine alone, until the work or the placement ends", async () => {
    // store a machine serving a space, a machine running its work and another machine
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const [serving, runner, other] = [
        await enroll(fixture, accountId, owner),
        await enroll(fixture, accountId, owner),
        await enroll(fixture, accountId, owner),
    ];
    const created = await owner.client(accountPackage).space.create({
        accountId,
        requestId: RequestId.create(fixture.clock.now()),
        name: "work",
        machine: schema.identifier("machine").parse(serving.subject.id),
    });
    const placed = await locate(serving, accountId, created.id);

    // give the runner work in the space, then withdraw it
    const machineId = schema.identifier("machine").parse(runner.subject.id);
    const work = { accountId, id: created.id, epoch: placed.epoch, machine: machineId };
    await serving
        .client(accountPackage)
        .placement.assign({ ...work, requestId: RequestId.create(fixture.clock.now()) });
    const assigned = [await assignments(runner), await assignments(other)];
    await serving
        .client(accountPackage)
        .placement.unassign({ ...work, requestId: RequestId.create(fixture.clock.now()) });
    const unassigned = await assignments(runner);

    // give the work again, then withdraw the space's placement
    await serving
        .client(accountPackage)
        .placement.assign({ ...work, requestId: RequestId.create(fixture.clock.now()) });
    await serving.client(accountPackage).placement.withdraw({
        accountId,
        id: created.id,
        epoch: placed.epoch,
        requestId: RequestId.create(fixture.clock.now()),
    });
    const remaining = await fixture.database
        .select({ id: assignment.table.id })
        .from(assignment.table);

    // replicate the assignment to the runner alone while the work and the placement last
    expect([...assigned, unassigned, await assignments(runner), remaining]).toEqual([
        [created.id],
        [],
        [],
        [],
        [],
    ]);
});

test("remove a space's delegated grants when the space is finalized", async () => {
    // store a space and the owner's space beside it
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { accountId, spaceId } = await fixture.createSpace(owner);
    const created = await owner.client(accountPackage).space.create({
        accountId,
        requestId: RequestId.create(fixture.clock.now()),
        name: "work",
    });

    // grant an installation of each space and another person
    const delegates = [
        principal.installation.reference(created.id, Identifier.create("installation")),
        principal.installation.reference(spaceId, Identifier.create("installation")),
        principal.user.reference(Scope.universe.id, Identifier.create("user")),
    ];
    for (const delegate of delegates) {
        await owner.client(objectPackage).grant.create({
            scope: owner.id,
            requestId: RequestId.create(fixture.clock.now()),
            delegate,
            permissions: [],
        });
    }

    // delete the space as its owner stepped up, then finalize its removal as the account store
    const elevated = await fixture.elevate(owner);
    await elevated.client(accountPackage).space.delete({
        accountId,
        id: created.id,
        requestId: RequestId.create(fixture.clock.now()),
    });
    const deleting = present(
        await Snapshot.live(fixture.database)
            .read(space.table)
            .findFirst({ where: { id: created.id } }),
        "the deleting space",
    );
    await fixture.store.mutate(fixture.store.serving(space), {
        id: RequestId.create(fixture.clock.now()),
        calls: [space.call("finalize", space.target(deleting))],
    });

    // leave the grants to the other space's installation and the person
    const grants = (await owner.client(objectPackage).grant.list({ scope: owner.id })).items;
    expect(grants.map((grant) => ObjectReference.at("delegate", grant))).toEqual(
        delegates.slice(1),
    );
});

test("replicate a drained machine's status to every machine through the directory", async () => {
    // store two machines and drain the first as the owner
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const [drained, other] = [
        await enroll(fixture, accountId, owner),
        await enroll(fixture, accountId, owner),
    ];
    await owner.client(accountPackage).machine.drain({
        scope: accountId,
        id: drained.subject.id,
        requestId: RequestId.create(fixture.clock.now()),
    });

    // replicate both machines' statuses to each machine
    const statuses = [
        [drained.subject.id, "draining"],
        [other.subject.id, "enabled"],
    ];
    const both = async (copier: TestCaller) =>
        (await machines(copier)).filter(
            ([id]) => id === drained.subject.id || id === other.subject.id,
        );
    expect([await both(drained), await both(other)]).toEqual([statuses, statuses]);
});

test("publish a personal machine's URL as the machine alone, and read it as absent once its tunnel lapses", async () => {
    // store a personal machine of a person's account, and the residency's home
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const accountId = owner.account;
    const enrolled = await enroll(fixture, accountId, owner);
    const home = fixture.as(await fixture.home("eu")).client(accountPackage);
    const address = "https://laptop--acme.destack.test/";
    const machineId = schema.identifier("machine").parse(enrolled.subject.id);
    const publishVia = (client: typeof home, lapsesAt: number) =>
        refusal(
            client.machine.publish({
                scope: accountId,
                id: machineId,
                url: address,
                relay: { url: "https://relay.test/", lapsesAt },
                requestId: RequestId.create(fixture.clock.now()),
            }),
        );
    const published = () => url(fixture, enrolled, accountId, enrolled.subject.id);

    // refuse the installation, and publish as the machine through a tunnel lapsing later
    const later = fixture.clock.now() + 60_000;
    expect([
        await publishVia(home, later),
        await publishVia(enrolled.client(accountPackage), later),
        await published(),
    ]).toEqual([["FORBIDDEN", "PERMISSION_DENIED", "permission denied: publish"], "done", address]);

    // read nothing once the machine publishes a tunnel that lapsed
    expect([
        await publishVia(enrolled.client(accountPackage), fixture.clock.now() - 1),
        await published(),
    ]).toEqual(["done", null]);
});

test("pick each space's machine regardless of the machines' order, and move only the spaces of a machine that leaves", async () => {
    // pick every space's machine among the machines, and again among them reversed and without one
    const picked = await Promise.all(
        SPACES.map(async (spaceId) => present(await Placement.pick(spaceId, MACHINES), spaceId)),
    );
    const reversed = await Promise.all(
        SPACES.map((spaceId) => Placement.pick(spaceId, MACHINES.toReversed())),
    );
    const leaving = nth(MACHINES, 1);
    const remaining = MACHINES.filter((machineId) => machineId !== leaving);
    const after = await Promise.all(SPACES.map((spaceId) => Placement.pick(spaceId, remaining)));

    // make every choice again under another order, move exactly the spaces the leaving machine had, and use every machine
    expect({
        reversed,
        moved: SPACES.filter((_, index) => picked[index] !== after[index]),
        used: [...new Set(picked)].toSorted((left, right) => left.localeCompare(right)),
        none: await Placement.pick(nth(SPACES, 0), []),
    }).toEqual({
        reversed: picked,
        moved: SPACES.filter((_, index) => picked[index] === leaving),
        used: MACHINES,
        none: undefined,
    });
});
