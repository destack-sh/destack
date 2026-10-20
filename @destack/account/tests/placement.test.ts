import objectPackage from "@destack/object/package";
import { AccessContext, Caller, CHAIN_PUBLICATION, principal } from "@destack/access";
import { Snapshot, TABLE } from "@destack/db";
import { nth, Identifier, present, schema } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { expect, test } from "@destack/test";
import { ObjectReference, Scope } from "@destack/sync";
import { Directory } from "../src/address/index.ts";
import { isCallError } from "@destack/runtime/error";
import { account, assignment, placement, Placement, placementShape } from "../src/object/index.ts";
import * as accountObject from "../src/object/index.ts";
import { ACCESS_TOKEN_TYPE } from "../src/oauth/index.ts";
import { sudo } from "../src/authentication/sudo.ts";
import accountPackage from "../src/package.ts";
import { AccountFixture, type Actor, ISSUER, outcome } from "../src/test/index.ts";
import { claimNames, firstPage, place } from "./fixture.ts";
import { Claim, Claimant } from "@destack/object";

test("place spaces where they are created, store names in them, let exactly a space replicate its own access, and refuse machines exchanging tokens or replicating access", async () => {
    // store two machines and a cloud machine of the owner's account, and a machine and a cloud machine of another account
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId, handle } = await fixture.createSpace(owner);
    const other = await fixture.createSpace(owner);
    const enroll = async (enrolledIn: Identifier<"account">) =>
        fixture.caller(await fixture.enroll(enrolledIn, { client: owner.client }));
    const identity = await fixture.enroll(accountId, { client: owner.client });
    const [local, peer, outsider, cloud, rival] = [
        fixture.caller(identity),
        await enroll(accountId),
        await enroll(other.accountId),
        fixture.caller(await fixture.declareCloudMachine(accountId, "eu")),
        fixture.caller(await fixture.declareCloudMachine(other.accountId, "eu")),
    ];
    const create = async (name: string, machine?: Identifier<"machine">) =>
        (
            await owner.client.space.create({
                accountId,
                requestId: RequestId.create(Date.now()),
                name,
                ...(machine === undefined ? {} : { machine }),
            })
        ).id;
    const laptop = await create("notes", schema.identifier("machine").parse(local.id));
    const served = await create("served");
    const foreign = Identifier.create("space");
    const at = (machine: string, epoch = 1) => ({
        id: laptop,
        scope: accountId,
        machine: schema.identifier("machine").parse(machine),
        epoch,
    });
    const serve = (machine: Actor, epoch: number, id: Identifier<"space"> = laptop) =>
        outcome(
            machine.client.placement.place({
                accountId,
                id,
                epoch,
                requestId: RequestId.create(Date.now()),
            }),
        );

    // place again only as the machine serving a created space, never a space nobody created, refusing another account's machine, which reads every placement through the directory
    expect([
        await serve(local, 1),
        await serve(peer, 1),
        await serve(peer, 1, foreign),
        await serve(outsider, 1),
    ]).toEqual([
        "executed",
        "FORBIDDEN: permission denied: place",
        `NOT_FOUND: no placement ${foreign}`,
        "FORBIDDEN: permission denied: place",
    ]);
    expect(await locate(peer, accountId, laptop)).toEqual(at(local.id));

    // reserve and confirm the laptop place's name as its machine and refuse the peer
    const row = { id: laptop, scope: accountId, name: "notes" };
    const snapshot = Snapshot.live(fixture.storage.database);
    const object = { objectId: laptop, owner: accountId };
    const claimant = await Claimant.of(place, object, row, snapshot);
    const reserve = (machine: Actor) => claimNames(machine, fixture.storage.database, claimant);
    expect(await outcome(reserve(peer))).toBe("FORBIDDEN: permission denied: claim");
    await reserve(local);
    expect(await Claim.lookup(place, claims(outsider), "name", ["notes"], accountId)).toEqual({
        scope: accountId,
        id: laptop,
    });

    // let exactly each space replica the chain above it, and no machine replica it
    const [laptopSpace, servedSpace] = [fixture.space(laptop), fixture.space(served)];
    expect([
        [
            await replicas(laptopSpace, accountId, laptop),
            await replicas(laptopSpace, accountId, served),
        ],
        [await replicas(local, accountId, laptop), await replicas(servedSpace, accountId, served)],
        [await replicas(peer, accountId), await replicas(outsider, other.accountId)],
    ]).toEqual([
        ["executed", unfollowed(served)],
        [unfollowed(laptop), "executed"],
        [unfollowed(accountId), unfollowed(other.accountId)],
    ]);

    // store the account's row in the chain replica of a space storing accounts
    const following = new AbortController();
    const chain = await laptopSpace.client.store.stream(
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
    expect(
        first.changes
            .filter((change) => change.table === "destack__account__account")
            .map((change) => [change.row["id"], change.concealed]),
    ).toEqual([[accountId, undefined]]);

    // refuse exchanging a machine's own token, which it asks with its assertion
    const accessToken = await identity.token(accountPackage.id, { url: ISSUER }, (request) =>
        fixture.fetch(request),
    );
    expect(
        await outcome(
            fixture.exchangeToken(accessToken, ACCESS_TOKEN_TYPE, {
                audience: accountPackage.id,
                space: laptop,
            }),
        ),
    ).toBe("BAD_REQUEST: invalid_grant: a machine asks its tokens with its assertion");

    // store a disabled and a revoked machine of the account, the revoked one without its published URL
    const [disabled, revoked] = [await enroll(accountId), await enroll(accountId)];
    const target = (machine: Actor) => ({
        scope: accountId,
        id: machine.id,
        requestId: RequestId.create(Date.now()),
    });
    await owner.client.machine.disable(target(disabled));
    await publish(revoked, revoked.id, accountId, "https://revoked.test/");
    await owner.client.machine.revoke(target(revoked));

    // refuse moving the laptop place to another account's machines, a disabled or a revoked one, then move it to the account's cloud machine at the next epoch
    const move = (machine: Actor, epoch: number, to: string) =>
        outcome(
            machine.client.placement.move({
                accountId,
                id: laptop,
                epoch,
                target: schema.identifier("machine").parse(to),
                transferId: Identifier.create("transfer"),
                requestId: RequestId.create(Date.now()),
            }),
        );
    expect([
        await move(local, 1, outsider.id),
        await move(local, 1, rival.id),
        await move(local, 1, disabled.id),
        await move(local, 1, revoked.id),
        await url(outsider, accountId, revoked.id),
    ]).toEqual([refused(outsider), refused(rival), refused(disabled), refused(revoked), undefined]);
    await move(local, 1, cloud.id);
    await serve(cloud, 2);
    expect([
        await serve(local, 3),
        await replicas(local, laptop),
        await replicas(cloud, laptop),
        await replicas(laptopSpace, laptop),
    ]).toEqual([
        "FORBIDDEN: permission denied: place",
        unfollowed(laptop),
        unfollowed(laptop),
        "executed",
    ]);

    // publish URLs only as the calling machine
    expect([
        await publish(local, local.id, accountId, "https://local.test/"),
        await publish(local, cloud.id, accountId, "https://cloud.test/"),
        await publish(cloud, cloud.id, accountId, "https://cloud.test/"),
    ]).toEqual(["executed", `NOT_FOUND: no machine ${cloud.id}`, "executed"]);
    // read URLs as any signed-in machine, as the directory gives them
    const resolver = new Directory(Snapshot.live(fixture.storage.database));
    expect([
        await url(peer, accountId, local.id),
        await url(peer, accountId, cloud.id),
        await url(outsider, accountId, local.id),
        await resolver.resolve(place, `notes--${handle}`, Date.now()),
        await outcome(resolver.resolve(place, `missing--${handle}`, Date.now())),
    ]).toEqual([
        "https://local.test/",
        "https://cloud.test/",
        "https://local.test/",
        { id: laptop, placement: at(cloud.id, 2), url: "https://cloud.test/" },
        `NOT_FOUND: nothing answers at missing--${handle}`,
    ]);

    // move and withdraw only the spaces a machine serves, at the epoch it serves them
    const withdraw = (machine: Actor, epoch: number) =>
        outcome(
            machine.client.placement.withdraw({
                accountId,
                id: laptop,
                epoch,
                requestId: RequestId.create(Date.now()),
            }),
        );
    expect([
        await move(local, 2, local.id),
        await move(cloud, 1, local.id),
        await move(cloud, 2, local.id),
        await placements(local),
        await placements(peer),
        await withdraw(peer, 2),
        await withdraw(cloud, 2),
    ]).toEqual([
        "FORBIDDEN: permission denied: move",
        `CONFLICT: ${laptop} is no longer placed at epoch 1`,
        "executed",
        [[laptop, local.id]],
        [],
        "FORBIDDEN: permission denied: move",
        "executed",
    ]);
    expect(await locate(local, accountId, laptop)).toBeUndefined();
});

test("replicate the spaces giving a machine work to that machine alone, until the work or the placement ends", async () => {
    // store a machine serving a space, a machine running its work and another machine
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId } = await fixture.createSpace(owner);
    const enroll = async () =>
        fixture.caller(await fixture.enroll(accountId, { client: owner.client }));
    const [serving, runner, other] = [await enroll(), await enroll(), await enroll()];
    const created = await owner.client.space.create({
        accountId,
        requestId: RequestId.create(Date.now()),
        name: "work",
        machine: schema.identifier("machine").parse(serving.id),
    });
    const placed = present(await locate(serving, accountId, created.id), "the space's placement");

    // give the runner work in the space, then withdraw it
    const machine = schema.identifier("machine").parse(runner.id);
    const work = { accountId, id: created.id, epoch: placed.epoch, machine };
    await serving.client.placement.assign({ ...work, requestId: RequestId.create(Date.now()) });
    const assigned = [await assignments(runner), await assignments(other)];
    await serving.client.placement.unassign({ ...work, requestId: RequestId.create(Date.now()) });
    const unassigned = await assignments(runner);

    // give the work again, then withdraw the space's placement
    await serving.client.placement.assign({ ...work, requestId: RequestId.create(Date.now()) });
    await serving.client.placement.withdraw({
        accountId,
        id: created.id,
        epoch: placed.epoch,
        requestId: RequestId.create(Date.now()),
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
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId, spaceId } = await fixture.createSpace(owner);
    const created = await owner.client.space.create({
        accountId,
        requestId: RequestId.create(Date.now()),
        name: "work",
    });

    // grant an installation of each space and another person
    const delegates = [
        principal.installation.reference(created.id, Identifier.create("installation")),
        principal.installation.reference(spaceId, Identifier.create("installation")),
        principal.user.reference(Scope.universe.id, Identifier.create("user")),
    ];
    for (const delegate of delegates) {
        await fixture.sessionClient(owner.session, objectPackage).grant.create({
            scope: owner.id,
            requestId: RequestId.create(Date.now()),
            delegate,
            permissions: [],
        });
    }

    // delete the space as its owner just authenticated with a second factor, then finalize its removal as the account store
    const person = principal.user.reference(Scope.universe.id, owner.id);
    const now = Date.now();
    await fixture.objects.mutate(
        AccessContext.of(Caller.of(person), now, {
            credential: {
                kind: "session",
                id: "session-1",
                audience: accountPackage.id,
                verifiedAt: now,
                expiresAt: now + 60_000,
                assurance: { level: sudo.assurance, authenticatedAt: now },
            },
        }),
        {
            id: RequestId.create(Date.now()),
            calls: [accountObject.space.call("delete", { accountId, id: created.id })],
        },
    );
    const deleting = present(
        await Snapshot.live(fixture.database)
            .read(accountObject.space.table)
            .findFirst({ where: { id: created.id } }),
        "the deleting space",
    );
    await fixture.objects.mutate(fixture.objects.serving(accountObject.space), {
        id: RequestId.create(Date.now()),
        calls: [accountObject.space.call("finalize", accountObject.space.target(deleting))],
    });

    // leave the grants to the other space's installation and the person
    const grants = (
        await fixture.sessionClient(owner.session, objectPackage).grant.list({ scope: owner.id })
    ).items;
    expect(grants.map((grant) => ObjectReference.at("delegate", grant))).toEqual(
        delegates.slice(1),
    );
});

test("replicate a machine its own row with its status, and no other machine's", async () => {
    // store two machines and drain the first as the owner
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const { accountId } = await fixture.createSpace(owner);
    const enroll = async () =>
        fixture.caller(await fixture.enroll(accountId, { client: owner.client }));
    const [drained, other] = [await enroll(), await enroll()];
    await owner.client.machine.drain({
        scope: accountId,
        id: drained.id,
        requestId: RequestId.create(Date.now()),
    });

    // replicate each machine its own row alone
    expect([await machines(drained), await machines(other)]).toEqual([
        [[drained.id, "draining"]],
        [[other.id, "enabled"]],
    ]);
});

/** Read the machine rows a machine replicates, with their statuses. */
async function machines(copier: Actor): Promise<unknown[][]> {
    const changes = await placementChanges(copier);

    return changes
        .filter((change) => change.table === accountObject.machine.table[TABLE].sqlName)
        .map((change) => [change.row["id"], change.row["status"]]);
}

/** Publish the URL a machine answers at as a caller, reporting the outcome. */
function publish(
    caller: Actor,
    machine: string,
    scope: string,
    published: string,
): Promise<string> {
    return outcome(
        caller.client.machine.publish({
            scope,
            id: schema.identifier("machine").parse(machine),
            url: published,
            relay: null,
            requestId: RequestId.create(Date.now()),
        }),
    );
}

/** Read where a space runs as a caller, none for a space it does not read. */
async function locate(caller: Actor, accountId: string, spaceId: string) {
    return caller.client.placement
        .get({
            accountId: schema.identifier("account").parse(accountId),
            id: schema.identifier("space").parse(spaceId),
        })
        .then(({ id, scope, machineId, epoch }) => ({ id, scope, machine: machineId, epoch }))
        .catch((error: unknown) => {
            if (isCallError(error) && error.code === "NOT_FOUND") {
                return undefined;
            }
            throw error;
        });
}

/** Read the URL a machine answers at as a caller, none before it publishes or after it lapses. */
async function url(caller: Actor, scope: string, machine: string) {
    const id = schema.identifier("machine").parse(machine);
    const found = await caller.client.endpoint
        .get({ ...accountObject.endpoint.target({ scope, id }) })
        .catch((error: unknown) => {
            if (isCallError(error) && error.code === "NOT_FOUND") {
                return undefined;
            }
            throw error;
        });

    return found === undefined || !accountObject.Endpoint.isAnswering(found, Date.now())
        ? undefined
        : found.url;
}

/** Find the owners of names as a caller. */
function claims(caller: Actor) {
    return {
        owner: async (index: string, key: string) =>
            (await caller.client.claim.owner({ index, key })) ?? undefined,
    };
}

/** Replicate a scope's chain as a machine, for a target space if given, reporting a refusal. */
async function replicas(machine: Actor, scope: string, spaceId?: string): Promise<string> {
    const controller = new AbortController();
    const target = spaceId ?? scope;
    const replicated = machine.client.store
        .stream(
            {
                name: CHAIN_PUBLICATION,
                scope,
                target,
                publication: CHAIN_PUBLICATION,
                parameters: { scopes: [target], local: [], types: [], via: [], between: [] },
            },
            { signal: controller.signal },
        )
        .then((pages) => firstPage(pages));

    return outcome(replicated).finally(() => controller.abort());
}

/** Read the placements a machine replicates of the spaces it runs or receives, with targets. */
async function placements(machine: Actor): Promise<unknown[][]> {
    const changes = await placementChanges(machine);

    return changes
        .filter((change) => change.table === placement.table[TABLE].sqlName)
        .map((change) => [change.row["id"], change.row["targetId"]]);
}

/** Read the spaces a machine replicates as giving it work. */
async function assignments(machine: Actor): Promise<unknown[]> {
    const changes = await placementChanges(machine);

    return changes
        .filter((change) => change.table === assignment.table[TABLE].sqlName)
        .map((change) => change.row["placementId"]);
}

/** Read the first page of a machine's replica of its placements and work. */
async function placementChanges(machine: Actor) {
    const controller = new AbortController();
    const pages = await machine.client.store.stream(
        placementShape.subscription({ scope: Scope.universe.id, machine: machine.id }, machine.id),
        { signal: controller.signal },
    );
    const page = await firstPage(pages);
    controller.abort();

    return page.changes;
}

test("publish a personal machine's URL as the machine alone, and read it as absent once its tunnel lapses", async () => {
    // store a personal machine of a person's account and an installation of the residency
    await using fixture = await AccountFixture.open();
    const owner = await fixture.person("owner@example.com");
    const accountId = owner.account;
    const machine = fixture.caller(await fixture.enroll(accountId, { client: owner.client }));
    const other = fixture.installation(await fixture.install());
    const address = "https://laptop--acme.destack.test/";
    const machineId = schema.identifier("machine").parse(machine.id);
    const publishVia = (client: typeof other, lapsesAt: number) =>
        outcome(
            client.machine.publish({
                scope: accountId,
                id: machineId,
                url: address,
                relay: { url: "https://relay.test/", lapsesAt },
                requestId: RequestId.create(Date.now()),
            }),
        );
    const published = () => url(machine, accountId, machine.id);

    // refuse the installation, and publish as the machine through a tunnel lapsing later
    const later = Date.now() + 60_000;
    expect([
        await publishVia(other, later),
        await publishVia(machine.client, later),
        await published(),
    ]).toEqual(["FORBIDDEN: permission denied: publish", "executed", address]);

    // read nothing once the machine publishes a tunnel that lapsed
    expect([await publishVia(machine.client, Date.now() - 1), await published()]).toEqual([
        "executed",
        undefined,
    ]);
});

/** Refuse following the replicas stored for a scope as anyone but it and what lives in it. */
function unfollowed(target: string): string {
    return `FORBIDDEN: only ${target} and its contents can follow the replicas held for it`;
}

/** Refuse moving a space to a machine that serves neither its account nor its residency. */
function refused(machine: Actor): string {
    return `BAD_REQUEST: ${machine.id} is no standing machine of the space's account nor of its residency`;
}

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

test("pick each space's machine regardless of the machines' order, and move only the spaces of a machine that leaves", async () => {
    // pick every space's machine among the machines, and again among them reversed and without one
    const picked = await Promise.all(
        SPACES.map(async (space) => present(await Placement.pick(space, MACHINES), space)),
    );
    const reversed = await Promise.all(
        SPACES.map((space) => Placement.pick(space, MACHINES.toReversed())),
    );
    const leaving = nth(MACHINES, 1);
    const remaining = MACHINES.filter((machine) => machine !== leaving);
    const after = await Promise.all(SPACES.map((space) => Placement.pick(space, remaining)));

    // make every choice again under another order, move exactly the spaces the leaving machine had, and use every machine
    const moved = SPACES.filter((_, index) => picked[index] !== after[index]);
    const leavingSpaces = SPACES.filter((_, index) => picked[index] === leaving);
    expect({
        isOrderless: reversed.every((machine, index) => machine === picked[index]),
        isMovingOnlyHeld:
            moved.length === leavingSpaces.length &&
            moved.every((space) => leavingSpaces.includes(space)),
        used: [...new Set(picked)].toSorted((left, right) => left.localeCompare(right)),
        none: await Placement.pick(nth(SPACES, 0), []),
    }).toEqual({ isOrderless: true, isMovingOnlyHeld: true, used: MACHINES, none: undefined });
});
