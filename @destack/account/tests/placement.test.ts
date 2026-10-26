import { testSimulation } from "@destack/object/test";
import objectPackage from "@destack/object/package";
import {
    AccessContext,
    accessRelationship,
    accessRole,
    accessRolePermission,
    Caller,
    CHAIN_PUBLICATION,
    principal,
    Relationship,
} from "@destack/access";
import { Snapshot, TABLE } from "@destack/db";
import { nth, Identifier, present, schema } from "@destack/schema";
import { RequestId, ObjectReference, Scope } from "@destack/sync";
import { expect, test, refusal, type Refusal } from "@destack/test";
import { Directory, DOMAINS, MachineAddress } from "../src/address/index.ts";
import {
    account,
    assignment,
    directoryShape,
    endpoint,
    Endpoint,
    machine,
    machineShape,
    Placement,
    placement,
    residency,
    space,
    type Tenancy,
} from "../src/object/index.ts";
import { ACCESS_TOKEN_TYPE } from "../src/oauth/index.ts";
import { MachineIdentity } from "../src/client/index.ts";
import { MemoryKeychain } from "@destack/identity";
import accountPackage from "../src/package.ts";
import { TestAccountStore, ISSUER, ids, type TestPerson, OWNER_EMAIL } from "../src/test/index.ts";
import type { TestCaller } from "@destack/object/test";
import { claimNames, firstPage, place } from "./fixture.ts";
import { Claim, Claimant, type ClaimLookup, type ClaimOwner, relationship } from "@destack/object";

/** The machines a residency's spaces spread over. */
/** The origin of the residency's relay, which publishes the tunnels it accepts. */
const RELAY_ORIGIN = "https://eu.relay.test";

/** How long a tunnel stands before its token lapses: one minute. */
const TUNNEL_MILLISECONDS = 60_000;

/** The machines the scenario places spaces on. */
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
        requestId: RequestId.create(fixture.environment),
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
            requestId: RequestId.create(fixture.environment),
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
            transferId: Identifier.create("transfer", fixture.environment),
            requestId: RequestId.create(fixture.environment),
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

/** Publish a machine's tunnel at a relay as a caller, lapsing at a time, reporting the outcome. */
function relay(
    fixture: TestAccountStore,
    caller: TestCaller,
    target: { readonly scope: string; readonly machine: string },
    lapsesAt: number,
): Promise<Refusal> {
    return refusal(
        caller.client(accountPackage).machine.relay({
            scope: target.scope,
            id: schema.identifier("machine").parse(target.machine),
            url: RELAY_ORIGIN,
            lapsesAt,
            requestId: RequestId.create(fixture.environment),
        }),
    );
}

/** Read the URL a personal machine answers at through its relay, below its account's handle. */
async function relayed(
    fixture: TestAccountStore,
    machineId: string,
    handle: string,
): Promise<string> {
    const named = await Snapshot.live(fixture.database)
        .read(machine.table)
        .findFirst({ where: { id: schema.identifier("machine").parse(machineId) } });
    const address = { machine: present(named, `machine ${machineId}`).name, handle };

    return `https://${MachineAddress.format(address, DOMAINS.machine)}`;
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
function claims(caller: TestCaller): ClaimLookup {
    return {
        resolve: async ({ index, keys }) => {
            // ask for each name's owner, keeping the claimed ones
            const owners = new Map<string, ClaimOwner>();
            for (const key of keys) {
                const owner = await caller.client(accountPackage).claim.owner({ index, key });
                if (owner !== null) {
                    owners.set(key, owner);
                }
            }

            return owners;
        },
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
        "MACHINE_UNKNOWN",
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
    const foreign = Identifier.create("space", fixture.environment);
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
        refusal(claimNames(caller, fixture.database, claimant, fixture.environment));

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

test("follow a machine's chain as a space of its account, and refuse a space of another account", async () => {
    // place a space of another account on that account's machine, beside the laptop's space
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, otherId, owner, identity, outsider, laptop } = opened;
    const rival = await owner.client(accountPackage).space.create({
        accountId: otherId,
        requestId: RequestId.create(fixture.environment),
        name: "rival",
        machine: outsider.subject.id,
    });
    await fixture.startIdentity(laptop, opened.local);
    await fixture.startIdentity(rival.id, outsider);

    // follow the laptop machine's chain as each space
    const follow = async (spaceId: string) => {
        const following = new AbortController();
        const chain = fixture
            .as(spaceId)
            .client(accountPackage)
            .store.stream(
                {
                    name: CHAIN_PUBLICATION,
                    scope: accountId,
                    target: identity.machineId,
                    publication: CHAIN_PUBLICATION,
                    parameters: {
                        scopes: [],
                        local: [],
                        types: [
                            {
                                packageId: machine.policy.definition.packageId,
                                type: machine.policy.definition.name,
                            },
                        ],
                        via: [],
                        between: [],
                    },
                },
                { signal: following.signal },
            );
        try {
            const first = await firstPage(await chain);

            return [...new Set(first.changes.map((change) => change.table))].toSorted();
        } finally {
            following.abort();
        }
    };

    // replicate the machine's scope row to the laptop's space, refusing the rival account's
    expect([await follow(laptop), await refusal(follow(rival.id))]).toEqual([
        [Scope.table[TABLE].sqlName],
        [
            "FORBIDDEN",
            "FOLLOWER_OUTSIDE_TARGET",
            `only ${identity.machineId} and its contents can follow its replicas`,
        ],
    ]);
});

test("follow a machine's chain via the universe as a machine of its account, and refuse a machine of another account", async () => {
    // follow the laptop machine's chain via the universe as its account's peer and as another account's machine
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, identity, peer, outsider } = opened;
    const follow = async (caller: typeof peer) => {
        const following = new AbortController();
        const chain = caller.client(accountPackage).store.stream(
            {
                name: CHAIN_PUBLICATION,
                scope: Scope.universe.id,
                target: identity.machineId,
                publication: CHAIN_PUBLICATION,
                parameters: {
                    scopes: [],
                    local: [],
                    types: [
                        {
                            packageId: machine.policy.definition.packageId,
                            type: machine.policy.definition.name,
                        },
                    ],
                    via: [identity.machineId],
                    between: [accountId],
                },
            },
            { signal: following.signal },
        );
        try {
            const first = await firstPage(await chain);

            return [...new Set(first.changes.map((change) => change.table))].toSorted();
        } finally {
            following.abort();
        }
    };

    // replicate the machine's chain to its account's peer, refusing the other account's machine
    expect([await follow(peer), await refusal(follow(outsider))]).toEqual([
        [
            accessRelationship[TABLE].sqlName,
            accessRole[TABLE].sqlName,
            accessRolePermission[TABLE].sqlName,
            machine.table[TABLE].sqlName,
            Scope.table[TABLE].sqlName,
        ],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"],
    ]);
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
                { partition: laptop },
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
        requestId: RequestId.create(fixture.environment),
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

test("publish a personal machine's tunnel only as its residency's relay, and read it as any machine and through the directory", async () => {
    // claim the laptop space's name, and relate an installation of the residency's home as its relay
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, handle, laptop, local, peer, outsider } = opened;
    const row = { id: laptop, scope: accountId, name: "notes" };
    const object = { objectId: laptop, owner: accountId };
    const claimant = await Claimant.of(place, object, row, Snapshot.live(fixture.database));
    await claimNames(local, fixture.database, claimant, fixture.environment);
    const relaying = fixture.as(await fixture.relay("eu"));
    const home = await fixture.residencySpace("eu");
    const other = principal.installation.reference(
        home,
        Identifier.create("installation", fixture.environment),
    );

    // publish the laptop's tunnel as the relay, the home's other installation, the residency's cloud machine and the laptop itself
    const target = { scope: accountId, machine: local.subject.id };
    const lapsesAt = fixture.clock.now() + TUNNEL_MILLISECONDS;
    const published = [
        await relay(fixture, relaying, target, lapsesAt),
        await relay(fixture, fixture.as(other), target, lapsesAt),
        await relay(fixture, fixture.as(fixture.cloud(ids.machine)), target, lapsesAt),
        await relay(fixture, local, target, lapsesAt),
    ];

    // refuse the laptop publishing a URL of its own, and the other installation relating itself as the relay
    const own = await refusal(
        local.client(accountPackage).machine.publish({
            scope: accountId,
            id: schema.identifier("machine").parse(local.subject.id),
            url: "https://local.test/",
            requestId: RequestId.create(fixture.environment),
        }),
    );
    const now = fixture.clock.now();
    const related = await refusal(
        fixture.store.mutate(AccessContext.of(Caller.of(other), now), {
            id: RequestId.create(fixture.environment),
            calls: [
                relationship.call("create", {
                    ...relationship.in(Scope.universe.id),
                    object: residency.reference(Scope.universe.id, "eu"),
                    relation: "relay",
                    subject: other,
                }),
            ],
        }),
    );

    // read it as the peer and another account's machine, and resolve the laptop's space through the directory
    const address = await relayed(fixture, local.subject.id, handle);
    const resolver = new Directory(Snapshot.live(fixture.database));
    expect([
        published,
        own,
        related,
        await url(fixture, peer, accountId, local.subject.id),
        await url(fixture, outsider, accountId, local.subject.id),
        await resolver.resolve(place, `notes--${handle}`, fixture.clock.now()),
        await refusal(resolver.resolve(place, `missing--${handle}`, fixture.clock.now())),
    ]).toEqual([
        [
            "done",
            ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${accountId} not found`],
            ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: relay"],
            ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: relay"],
        ],
        [
            "FORBIDDEN",
            "MACHINE_PERSONAL",
            `machine ${local.subject.id} is personal: it answers only through a relay`,
        ],
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: operate"],
        address,
        address,
        {
            id: laptop,
            placement: { id: laptop, scope: accountId, machine: local.subject.id, epoch: 1 },
            url: address,
        },
        ["NOT_FOUND", "ADDRESS_NOT_FOUND", `nothing responds at missing--${handle}`],
    ]);
});

test("withdraw a revoked machine's published tunnel", async () => {
    // publish a machine's tunnel as its relay, then revoke the machine as the owner
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { owner, accountId, peer } = opened;
    const revoked = await enroll(fixture, accountId, owner);
    await relay(
        fixture,
        fixture.as(await fixture.relay("eu")),
        { scope: accountId, machine: revoked.subject.id },
        fixture.clock.now() + TUNNEL_MILLISECONDS,
    );
    await owner.client(accountPackage).machine.revoke({
        scope: accountId,
        id: revoked.subject.id,
        requestId: RequestId.create(fixture.environment),
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
                requestId: RequestId.create(fixture.environment),
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
            ["CONFLICT", "EPOCH_STALE", `${laptop} is no longer placed at epoch 2`],
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
        requestId: RequestId.create(fixture.environment),
        name: "work",
        machine: schema.identifier("machine").parse(serving.subject.id),
    });
    const placed = await locate(serving, accountId, created.id);

    // give the runner work in the space, then withdraw it
    const machineId = schema.identifier("machine").parse(runner.subject.id);
    const work = { accountId, id: created.id, epoch: placed.epoch, machine: machineId };
    await serving
        .client(accountPackage)
        .placement.assign({ ...work, requestId: RequestId.create(fixture.environment) });
    const assigned = [await assignments(runner), await assignments(other)];
    await serving
        .client(accountPackage)
        .placement.unassign({ ...work, requestId: RequestId.create(fixture.environment) });
    const unassigned = await assignments(runner);

    // give the work again, then withdraw the space's placement
    await serving
        .client(accountPackage)
        .placement.assign({ ...work, requestId: RequestId.create(fixture.environment) });
    await serving.client(accountPackage).placement.withdraw({
        accountId,
        id: created.id,
        epoch: placed.epoch,
        requestId: RequestId.create(fixture.environment),
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
        requestId: RequestId.create(fixture.environment),
        name: "work",
    });

    // grant an installation of each space and another person
    const delegates = [
        principal.installation.reference(
            created.id,
            Identifier.create("installation", fixture.environment),
        ),
        principal.installation.reference(
            spaceId,
            Identifier.create("installation", fixture.environment),
        ),
        principal.user.reference(Scope.universe.id, Identifier.create("user", fixture.environment)),
    ];
    for (const delegate of delegates) {
        // grant each a millisecond after the last, so the grants list in the order granted
        fixture.clock.advance(1);
        await owner.client(objectPackage).grant.create({
            scope: owner.id,
            requestId: RequestId.create(fixture.environment),
            delegate,
            permissions: [],
        });
    }

    // delete the space as its owner stepped up, then finalize its removal as the account store
    const elevated = await fixture.elevate(owner);
    await elevated.client(accountPackage).space.delete({
        accountId,
        id: created.id,
        requestId: RequestId.create(fixture.environment),
    });
    const deleting = present(
        await Snapshot.live(fixture.database)
            .read(space.table)
            .findFirst({ where: { id: created.id } }),
        "the deleting space",
    );
    await fixture.store.mutate(fixture.store.as(space), {
        id: RequestId.create(fixture.environment),
        calls: [space.call("finalize", space.target(deleting))],
    });

    // leave the grants to the other space's installation and the person
    const grants = (await owner.client(objectPackage).grant.list({ scope: owner.id })).items;
    expect(grants.map((grant) => ObjectReference.at("delegate", grant))).toEqual(
        delegates.slice(1),
    );
});

test("replicate a drained machine's status to every machine through the directory", async () => {
    // store two machines a millisecond apart, so their identifiers order as they enroll, and drain the first
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const drained = await enroll(fixture, accountId, owner);
    fixture.clock.advance(1);
    const other = await enroll(fixture, accountId, owner);
    await owner.client(accountPackage).machine.drain({
        scope: accountId,
        id: drained.subject.id,
        requestId: RequestId.create(fixture.environment),
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

test("read a personal machine's tunnel its relay published as absent once its token lapses", async () => {
    // store a personal machine of a person's account, and the residency's relay
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const accountId = owner.account;
    const enrolled = await enroll(fixture, accountId, owner);
    const relaying = fixture.as(await fixture.relay("eu"));
    const target = { scope: accountId, machine: enrolled.subject.id };
    const published = () => url(fixture, enrolled, accountId, enrolled.subject.id);

    // publish a tunnel lapsing later, then one that lapsed
    const later = fixture.clock.now() + TUNNEL_MILLISECONDS;
    const first = [await relay(fixture, relaying, target, later), await published()];
    const lapsed = [
        await relay(fixture, relaying, target, fixture.clock.now() - 1),
        await published(),
    ];
    expect({ first, lapsed }).toEqual({
        first: ["done", await relayed(fixture, enrolled.subject.id, owner.handle)],
        lapsed: ["done", null],
    });
});

test("pick each space's machine regardless of the machines' order, and move only the spaces of a machine that leaves", async () => {
    // pick every space's machine among the machines, and again among them reversed and without one
    await using simulation = testSimulation();
    const { crypto } = simulation.environment("placement");
    const picked = await Promise.all(
        SPACES.map(async (spaceId) =>
            present(await Placement.pick(spaceId, MACHINES, crypto), spaceId),
        ),
    );
    const reversed = await Promise.all(
        SPACES.map((spaceId) => Placement.pick(spaceId, MACHINES.toReversed(), crypto)),
    );
    const leaving = nth(MACHINES, 1);
    const remaining = MACHINES.filter((machineId) => machineId !== leaving);
    const after = await Promise.all(
        SPACES.map((spaceId) => Placement.pick(spaceId, remaining, crypto)),
    );

    // make every choice again under another order, move exactly the spaces the leaving machine had, and use every machine
    expect({
        reversed,
        moved: SPACES.filter((_, index) => picked[index] !== after[index]),
        used: [...new Set(picked)].toSorted((left, right) => left.localeCompare(right)),
        none: await Placement.pick(nth(SPACES, 0), [], crypto),
    }).toEqual({
        reversed: picked,
        moved: SPACES.filter((_, index) => picked[index] === leaving),
        used: MACHINES,
        none: undefined,
    });
});

test("refuse declaring an account's dedicated cloud machine its plan does not unlock", async () => {
    // declare a cloud machine into a new account without a plan unlocking dedicated tenancy, as the platform does
    await using fixture = await TestAccountStore.open();
    const owner = await fixture.signIn(OWNER_EMAIL);
    const { id: accountId } = await fixture.createAccount(owner);
    const identity = new MachineIdentity(
        Identifier.create("machine", fixture.environment),
        new MemoryKeychain(),
        fixture.store.environment,
    );
    const { publicKey } = await identity.generate();
    const declared = fixture.store.mutate(fixture.store.as(machine), {
        id: RequestId.create(fixture.store.environment),
        calls: [
            machine.call("declare", {
                ...machine.target({ scope: accountId, id: identity.machineId }),
                name: "dedicated",
                kind: "cloud",
                residencyId: "eu",
                providerCode: "test",
                location: "test-1",
                publicKey,
            }),
        ],
    });

    expect(await refusal(declared)).toEqual([
        "PAYMENT_REQUIRED",
        "DEDICATED_TENANCY_UNENTITLED",
        `${accountId} has no dedicated tenancy: no plan of the account grants dedicated tenancy`,
    ]);
});

test("schedule a space where its tenancy keeps it: unset, unentitled, placed, moving, without a machine, or ambiguous", async () => {
    // place the laptop's space on the laptop, a personal machine of the account, with no tenancy set
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, laptop, local, owner } = opened;
    const decided = async () => {
        const row = await Snapshot.live(fixture.database)
            .read(placement.table)
            .findFirst({ where: { id: laptop } });

        return present(row, "the laptop's placement").schedule;
    };
    const unset = await decided();

    // set the space's tenancy as its owner, reading the decision its update records
    const schedule = async (tenancy: Tenancy | null) => {
        const row = await Snapshot.live(fixture.database)
            .read(space.table)
            .findFirst({ where: { id: laptop } });
        await owner.client(accountPackage).space.update({
            accountId,
            id: laptop,
            requestId: RequestId.create(fixture.environment),
            revision: present(row, "the laptop's space").revision,
            tenancy,
        });

        return await decided();
    };

    // refuse any tenancy no plan of the account unlocks
    const unentitled = await schedule("local");

    // keep it on the laptop once the plan unlocks its tenancy, move it to the residency's cloud machine, and keep it once unset
    await fixture.entitle(accountId, "tenancy", {
        kind: "static",
        value: ["shared", "dedicated", "local"],
    });
    await fixture.served.settle();
    const kept = [await decided(), await schedule("shared"), await schedule(null)];

    // find no dedicated machine of the account, then decide again once one and then two are declared
    const none = await schedule("dedicated");
    const first = await fixture.declareCloudMachine(accountId, "eu");
    await fixture.served.settle();
    const one = await decided();
    const second = await fixture.declareCloudMachine(accountId, "eu");
    await fixture.served.settle();
    const two = await decided();

    // compare every decision
    const machineId = local.subject.id;
    const unsetCondition = {
        status: "true",
        reason: "Unset",
        message: `placed local on ${machineId}`,
    };
    expect([unset, unentitled, kept, none, one, two]).toEqual([
        { kind: "stay", condition: unsetCondition },
        stay("Unentitled", "no plan of the account grants local tenancy"),
        [
            {
                kind: "stay",
                condition: {
                    status: "true",
                    reason: "Placed",
                    message: `placed local on ${machineId}`,
                },
            },
            { kind: "move", machine: ids.machine },
            { kind: "stay", condition: unsetCondition },
        ],
        stay("NoMachine", "the account has no cloud machine"),
        { kind: "move", machine: first.machineId },
        stay(
            "Ambiguous",
            `transfer the space to one of the account's machines: ${[first.machineId, second.machineId].toSorted().join(", ")}`,
        ),
    ]);
});

test("move a space off a draining machine, decide again once it is placed, and keep its placement when nothing changed", async () => {
    // place the laptop's space on the laptop beside the peer, both personal machines of the account
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, laptop, local, peer, owner } = opened;
    const read = async () =>
        present(
            await Snapshot.live(fixture.database)
                .read(placement.table)
                .findFirst({ where: { id: laptop } }),
            "the laptop's placement",
        );
    const placed = (await read()).schedule;

    // drain the laptop, which moves the space to the peer
    await owner.client(accountPackage).machine.drain({
        scope: accountId,
        id: local.subject.id,
        requestId: RequestId.create(fixture.environment),
    });
    await fixture.served.settle();
    const draining = (await read()).schedule;

    // place the space on the peer, which decides again in the same call
    await move(fixture, local, { accountId, id: laptop }, 1, peer.subject.id);
    await serve(fixture, peer, accountId, laptop, 2);
    const moved = await read();

    // change the plan, which decides again without a new decision
    await fixture.entitle(accountId, "tenancy", { kind: "static", value: ["local"] });
    await fixture.served.settle();
    const kept = await read();

    // compare each decision, and the revision a decision without change keeps
    const unset = (machineId: string) => ({
        kind: "stay",
        condition: { status: "true", reason: "Unset", message: `placed local on ${machineId}` },
    });
    expect([placed, draining, moved.schedule, kept.revision]).toEqual([
        unset(local.subject.id),
        { kind: "move", machine: peer.subject.id },
        unset(peer.subject.id),
        moved.revision,
    ]);
});

test("relate a move's target as the space's one receiver, keep it when asked again, replace it on the next move, and release it once the space is placed again", async () => {
    // move the laptop's space toward the peer, then toward a third machine of the account
    const opened = await openPlacement();
    await using fixture = opened.fixture;
    const { accountId, laptop, local, peer, owner } = opened;
    const third = fixture.as(await fixture.enroll(accountId, owner));
    const target = { accountId, id: laptop };
    const related = () =>
        Relationship.readOn(Snapshot.live(fixture.database), space.reference(accountId, laptop), {
            relation: "receiver",
        });
    const receivers = async () => (await related()).map((row) => row.subjectId);

    // relate the peer, then the third machine in its place, keep it when asked again, then none once the laptop places it again
    const moved = [await move(fixture, local, target, 1, peer.subject.id)];
    const first = await receivers();
    moved.push(await move(fixture, local, target, 1, third.subject.id));
    const second = await related();
    moved.push(await move(fixture, local, target, 1, third.subject.id));
    const again = await related();
    moved.push(await serve(fixture, local, accountId, laptop, 1));

    // compare the relationships by identifier, as asking again keeps the same one
    expect([moved, first, second.map((row) => row.subjectId), again, await receivers()]).toEqual([
        ["done", "done", "done", "done"],
        [peer.subject.id],
        [third.subject.id],
        second,
        [],
    ]);
});

/** Describe a placement staying put for a reason, as its unmet condition reads. */
function stay(reason: string, message: string) {
    return { kind: "stay", condition: { status: "false", reason, message } };
}
