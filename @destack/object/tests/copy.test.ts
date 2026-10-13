import { reconciliation, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";
import { ControlLoop, controllerLease } from "@destack/runtime/control";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    accessTables,
    accessRelationship,
    accessRole,
    Authorizer,
    none,
    principal,
    relation,
    Relationship,
    through,
    union,
} from "@destack/access";

import { AccessFixture } from "@destack/access/test";
import { asc, type DatabaseConnection, defineDatabase, eq, TABLE } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Clock, Identifier, present, schema } from "@destack/schema";

import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { until } from "@destack/runtime/timer";
import { Scope, Feed, Replica, type Page, type Subscription } from "@destack/sync";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore, Subscriber } from "../src/server/index.ts";
import { userContext } from "./fixture/user.ts";
import { spacePrincipal } from "./fixture/space.ts";

/** How long a copy may take to reach its home or relist: 5 s, far above its milliseconds. */
const COPY_MILLISECONDS = 5000;

/** Refuse a change sent over the uplink, as these copies receive none. */
const refuseChanges = () => Promise.reject(new Error("the fixture receives no changes"));

/** The account with the space. */
const accountId = schema
    .identifier("account")
    .parse("account-01996ab0-0000-7000-8000-000000000001");

/** The empty space the documents go into. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** Another space of the account outside every restriction here. */
const otherSpaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** A third space of the account, which only a machine's own scope rows refer to. */
const thirdSpaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000004");

/** The space of an installation copying the spaces' chains. */
const followingSpaceId = "space-01996ab0-0000-7000-8000-000000000005";

/** The installation copying the spaces' chains, living in its space. */
const followingId = "installation-01996ab0-0000-7000-8000-000000000006";

/** Another account, holding none of the spaces. */
const otherAccountId = schema
    .identifier("account")
    .parse("account-01996ab0-0000-7000-8000-000000000006");

/** Accounts with members that roles bind to, and installations reading them. */
const account = defineObject({
    name: "account",
    plural: "accounts",
    scope: "universe",
    isScope: true,
    fields: {},
    relations: {
        member: { subjects: [principal.user] },
        reader: { subjects: [principal.installation], grantedBy: null },
    },
    permissions: { read: union(relation("member"), relation("reader")) },
}).methods();

/** Spaces within accounts, which installations read and replicate as granted. */
const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: account,
    isScope: true,
    fields: {},
    relations: {
        reader: { subjects: [principal.installation], grantedBy: null },
        replicator: { subjects: [principal.installation], grantedBy: null },
    },
    permissions: { read: relation("reader"), replicate: relation("replicator") },
}).methods();

/** Comments an installation keeps in the spaces it follows. */
const comment = defineObject({
    name: "comment",
    plural: "comments",
    scope: space,
    fields: { body: field.string() },
    permissions: { read: none() },
}).methods();

/** Annotations an installation keeps in its space, each readable through its account. */
const annotation = defineObject({
    name: "annotation",
    plural: "annotations",
    scope: space,
    fields: { account: field.reference(account).relate("account"), body: field.string() },
    permissions: { read: through("account", "read") },
}).methods();

/** Documents that only roles grant. */
const document = defineObject({
    name: "document",
    plural: "documents",
    scope: space,
    fields: { title: field.string(schema.string().min(1)) },
    permissions: { read: none(), write: none() },
}).methods((method) => ({
    create: method.create("write"),
}));

test.each(TEST_DIALECTS)(
    "decide calls over a relayed copy of access as the holder does, through the account's members and restricted credentials, on %s",
    async (dialect) => {
        const home = await TestDatabase.create(dialect, accessTables, { isMigrated: true });
        const installed = await TestDatabase.create(dialect, [...document.tables, ...callTables], {
            isMigrated: true,
        });
        onTestFinished(async () => {
            await Promise.all([home.close(), installed.close()]);
        });

        // insert an account owned by alice, a member role carol has through the account, and its empty space at home
        const alice = principal.user.reference("universe", "alice");
        const carol = principal.user.reference("universe", "carol");
        const accountObject = account.reference(Scope.universe.id, accountId);
        const accessCopies = new AccessFixture(home.database);
        await accessCopies.copyScope(accountObject);
        await accessCopies.copyScope(space.reference(accountId, spaceId), "own");
        await accessCopies.copyOwner(accountObject, alice);
        await accessCopies.copyRole(
            accountObject,
            {
                name: "member",
                description: "Writes documents in the account's spaces",
                permissions: [
                    space.permission("read"),
                    document.permission("read"),
                    document.permission("write"),
                ],
            },
            { ...accountObject, relation: "member" },
        );
        await home.database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                    object: accountObject,
                    relation: "member",
                    subject: carol,
                    createdAt: 0,
                    expiresAt: null,
                },
                accountId,
            ),
        );

        // serve the documents of one space over the installation's database to each call's user, copying its chain from the home database
        const feed = new Feed(home.database, accessTables, { clock: Clock.system });
        const store: ObjectStore = new ObjectStore({
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: [document],
            database: installed.database,
            subscriber: Subscriber.of(
                {
                    receive: refuseChanges,
                    stream: (subscription, signal) =>
                        feed.subscribe(
                            store.source.replicaOf(subscription).queries,
                            subscription.after,
                            signal,
                        ),
                },
                () => store.authorizer.chain(installed.database, spaceId, { kind: "follower" }),
            ),
            callKey: testCallKey,
            events: await testEvents(installed.database),
            origin: {
                package: document.package,
            },
        });

        // follow the space's chain up to the universe
        const follower = present(
            store.controllers().find((each) => each.name === "replica"),
            "the replica controller",
        );
        const controller = new AbortController();
        const following: Promise<void>[] = [];
        onTestFinished(async () => {
            controller.abort();
            await Promise.allSettled(following);
        });
        const head = await home.database.log.position();
        for (const scope of [spaceId, accountId, Scope.universe.id]) {
            const key = present(
                (await follower.list()).find((entry) => scopeOf(entry) === scope),
                `the replica key of ${scope}`,
            );
            following.push(
                follower.reconcile(key, reconciliation(controller.signal)).then(() => undefined),
            );
            await Replica.wait(installed.database, scope, head, controller.signal);
        }
        expect((await follower.list()).map(scopeOf)).toEqual([
            spaceId,
            accountId,
            Scope.universe.id,
        ]);

        // create through tokens whose grants give the space, another one or everything, and list as alice
        const create = async (user: string, title: string, restricted?: string) => {
            // limit the caller's token to one scope by its grant when asked
            const context = userContext(
                user,
                spaceId,
                restricted === undefined
                    ? {}
                    : {
                          delegates: [
                              principal.personalAccessToken.reference(user, `token-${user}`),
                          ],
                          grant: {
                              id: Identifier.create("grant"),
                              scope: user,
                              permissions: [
                                  document.permission("read"),
                                  document.permission("write"),
                              ].map((permission) => ({ ...permission, scope: restricted })),
                          },
                      },
            );

            const created = store
                .mutate(context.caller(), {
                    id: RequestId.create(Date.now()),
                    calls: [document.call("create", { spaceId, title })],
                })
                .then(([written]) => written);
            const refused = await refusal(created);

            return refused === "done" ? (await created).title : refused;
        };
        const outcomes = [
            await create("alice", "Plan", spaceId),
            await create("alice", "Elsewhere", otherSpaceId),
            await create("dave", "Leak", spaceId),
            await create("dave", "Leak"),
            await create("carol", "Budget"),
        ];
        const listed = await store.query(
            userContext("alice", spaceId).caller(),
            document.call("list", { spaceId }),
        );

        // admit the owner through a credential for the empty space and hide the space from the rest
        const hidden = ["NOT_FOUND", `no scope ${spaceId}`];
        expect(outcomes).toEqual(["Plan", hidden, hidden, hidden, "Budget"]);
        expect(listed.items.map((item) => item.title).toSorted()).toEqual(["Budget", "Plan"]);
    },
);

test("keep one copy record per followed space of one account, and drop one space's copies while the other's keep the shared rows", async () => {
    // keep an account with a member role and two spaces at home, and a machine following both spaces' chains
    const { home, machine, store, requested } = await openCopies();
    requested.add(spaceId).add(otherSpaceId);
    const controller = new AbortController();
    const loop = new ControlLoop(
        machine.database,
        store.controllers().filter((each) => each.name === "replica"),
        { clock: Clock.system, report: failing },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const head = await home.database.log.position();
    await machine.database.log.until(
        async () => (await Replica.subscriptions(machine.database)).length === 6,
        controller.signal,
    );
    for (const scope of [spaceId, otherSpaceId, accountId, Scope.universe.id]) {
        await Replica.wait(machine.database, scope, head, controller.signal);
    }

    // record each space's copy of the account and the universe apart
    const records = async () =>
        (await Replica.subscriptions(machine.database))
            .map((copy) => [copy.below, copy.scope])
            .toSorted((left, right) => left.join().localeCompare(right.join()));
    expect(await records()).toEqual(
        [spaceId, otherSpaceId]
            .flatMap((below) => [
                [below, accountId],
                [below, below],
                [below, Scope.universe.id],
            ])
            .toSorted((left, right) => left.join().localeCompare(right.join())),
    );

    // stop following the other space once the scope rows change, dropping its copies alone, and keep the account's role
    requested.delete(otherSpaceId);
    await new AccessFixture(machine.database).copyScope(
        space.reference(accountId, thirdSpaceId),
        "own",
    );
    await machine.database.log.until(
        async () => (await Replica.subscriptions(machine.database)).length === 3,
        controller.signal,
    );
    const roles = await machine.database
        .select({ scope: accessRole.scope, name: accessRole.name })
        .from(accessRole);
    const scopes = await machine.database
        .select({ scope: Scope.table.scope })
        .from(Scope.table)
        .where(eq(Scope.table.scope, otherSpaceId));
    expect([await records(), roles, scopes]).toEqual([
        [
            [spaceId, accountId],
            [spaceId, spaceId],
            [spaceId, Scope.universe.id],
        ],
        [{ scope: accountId, name: "member" }],
        [],
    ]);
});

test("drop a copy no request names under the lease its follow holds, whichever of two controllers on the database follows it", async () => {
    // follow a space's chain on a machine through one control loop and start another over the same database
    const { home, machine, store, requested } = await openCopies();
    requested.add(spaceId);
    const controller = new AbortController();
    const loop = (holder: string) =>
        new ControlLoop(
            machine.database,
            store.controllers().filter((each) => each.name === "replica"),
            { clock: Clock.system, report: failing, lease: { holder } },
        );
    const [first, second] = [loop("first"), loop("second")];
    const running = [first.run(controller.signal)];
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(running);
    });
    const head = await home.database.log.position();
    for (const scope of [spaceId, accountId, Scope.universe.id]) {
        await Replica.wait(machine.database, scope, head, controller.signal);
    }
    running.push(second.run(controller.signal));
    await second.idle();

    // stop requesting the space's chain once the scope rows change, and wait for its copies to drop
    requested.delete(spaceId);
    await new AccessFixture(machine.database).copyScope(
        space.reference(accountId, otherSpaceId),
        "own",
    );
    await machine.database.log.until(
        async () => (await Replica.subscriptions(machine.database)).length === 0,
        controller.signal,
    );

    // drop every copy and its rows under the first controller's leases, which the second never took
    const leases = await machine.database
        .select({ key: controllerLease.key, holder: controllerLease.holder })
        .from(controllerLease)
        .orderBy(asc(controllerLease.key));
    const roles = await machine.database.select({ id: accessRole.id }).from(accessRole);
    expect([leases.map((lease) => lease.holder), roles]).toEqual([["first", "first", "first"], []]);
});

test("drop a copy once its source refuses its follow and no request names it any longer, without a change of the rows the requests read", async () => {
    // follow a space's chain on a machine
    const { home, machine, store, requested, refuse } = await openCopies();
    requested.add(spaceId);
    const controller = new AbortController();
    const failures: unknown[] = [];
    const loop = new ControlLoop(
        machine.database,
        store.controllers().filter((each) => each.name === "replica"),
        { clock: Clock.system, report: (_controller, _key, error) => failures.push(error) },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const head = await home.database.log.position();
    for (const scope of [spaceId, accountId, Scope.universe.id]) {
        await Replica.wait(machine.database, scope, head, controller.signal);
    }

    // stop requesting the chain without a change the controller watches, and refuse the machine at home
    requested.delete(spaceId);
    refuse();
    await machine.database.log.until(
        async () => (await Replica.subscriptions(machine.database)).length === 0,
        controller.signal,
    );
    const roles = await machine.database.select({ id: accessRole.id }).from(accessRole);
    expect([roles, failures]).toEqual([[], []]);
});

test("follow copies as a server serving no methods, which keeps no journal and takes no call key", async () => {
    // keep an account and its space at home, and a database keeping only access copies
    const home = await TestDatabase.create("sqlite", accessTables, { isMigrated: true });
    const follower = await TestDatabase.create("sqlite", accessTables, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), follower.close()]);
    });
    const accessCopies = new AccessFixture(home.database);
    await accessCopies.copyScope(account.reference(Scope.universe.id, accountId));
    await accessCopies.copyScope(space.reference(accountId, spaceId), "own");

    // follow the space's chain without a journal
    const feed = new Feed(home.database, accessTables, { clock: Clock.system });
    const store: ObjectStore = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [],
        policies: [document],
        database: follower.database,
        subscriber: Subscriber.of(
            {
                receive: refuseChanges,
                stream: (subscription, signal) =>
                    feed.subscribe(
                        store.source.replicaOf(subscription).queries,
                        subscription.after,
                        signal,
                    ),
            },
            () => store.authorizer.chain(follower.database, spaceId, { kind: "follower" }),
        ),
        origin: { package: document.package },
    });
    const controller = new AbortController();
    const loop = new ControlLoop(follower.database, store.controllers(), {
        clock: Clock.system,
        report: failing,
    });
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const head = await home.database.log.position();
    for (const scope of [spaceId, accountId, Scope.universe.id]) {
        await Replica.wait(follower.database, scope, head, controller.signal);
    }

    // run only compaction and the replica follow
    expect(store.controllers().map((each) => each.name)).toEqual(["compaction", "replica"]);
});

test("copy the chains of an installation's two served spaces in one copy via both, requiring it to replicate each and refusing a scope outside them", async () => {
    // keep an account with two spaces at home, and another account
    const home = await TestDatabase.create("sqlite", [...space.tables, ...callTables], {
        isMigrated: true,
    });
    const declared = defineDatabase({
        name: "installed",
        tables: [...document.tables, ...callTables],
        copies: [space.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const accessCopies = new AccessFixture(home.database);
    await accessCopies.copyScope(account.reference(Scope.universe.id, accountId));
    await accessCopies.copyScope(account.reference(Scope.universe.id, otherAccountId));
    for (const id of [spaceId, otherSpaceId]) {
        await new AccessFixture(home.database).copyScope(space.reference(accountId, id), "own");
        await home.database
            .insert(space.table)
            .values({ id, scope: accountId, createdAt: 1, updatedAt: 1 });
    }

    // let the installation read both spaces and replicate the first
    const source = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [space],
        policies: [document],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: document.package },
    });
    const fixture = new AccessFixture(home.database);
    const subject = principal.installation.reference(followingSpaceId, followingId);
    const grant = (id: string, granted: "reader" | "replicator") =>
        fixture.copyRelationship(id, space.reference(accountId, id), granted, subject, 1);
    await grant(spaceId, "reader");
    await grant(otherSpaceId, "reader");
    await grant(spaceId, "replicator");

    // copy the universe's spaces to the installation, decided at home for it
    const follower = { subject, parent: Scope.universe.id };
    const copying: ObjectStore = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [document],
        policies: [space],
        database: installed.database,
        subscriber: Subscriber.of(
            {
                receive: refuseChanges,
                stream: (asked, signal) =>
                    source.source.replicate(asked, asked.after, follower, signal),
            },
            async () => [
                present(
                    copying.source.universeSubscription(followingSpaceId),
                    "the universe request",
                ),
            ],
        ),
        callKey: testCallKey,
        events: await testEvents(installed.database),
        origin: { package: document.package },
    });
    const controller = await followUniverse(copying, home.database);

    // request the universe's rows and one copy of both spaces' chains via them
    const requests = await copying.source.servedSubscriptions(followingSpaceId);
    const chains = present(requests[1], "the chains' request");

    // stream the first page of a request at home, refusing the second space until replicable
    const first = async (request: Subscription): Promise<Page | undefined> => {
        const pages = source.source.replicate(request, undefined, follower, controller.signal);
        const page = await pages.next();
        await pages.return(undefined);

        return page.done === true ? undefined : page.value;
    };
    const refused = await refusal(first(chains));
    await grant(otherSpaceId, "replicator");

    // copy both chains, and refuse a scope between outside them
    const page = present(await first(chains), "the chains' first page");
    const outside = await refusal(
        first({ ...chains, parameters: { ...chains.parameters, between: [otherAccountId] } }),
    );
    expect({
        requests: requests.map((request) => [request.name, request.scope, request.below]),
        parameters: [chains.parameters["via"], chains.parameters["between"]],
        refused,
        scopes: new Set(
            page.changes
                .filter((change) => change.table === Scope.table[TABLE].sqlName)
                .map((change) => change.row["scope"]),
        ),
        outside,
    }).toEqual({
        requests: [
            [followingSpaceId, Scope.universe.id, followingSpaceId],
            [Authorizer.chainCopy(followingSpaceId), Scope.universe.id, followingSpaceId],
        ],
        parameters: [[spaceId, otherSpaceId], [accountId]],
        refused: ["FORBIDDEN", "permission denied: replicate"],
        scopes: new Set([accountId, spaceId, otherSpaceId]),
        outside: ["NOT_FOUND", `${otherAccountId} contains none of the replicated scopes`],
    });
});

test("copy a scoped table an installation's database declares beside its followed space's chain, and drop its rows once the installation may no longer read the space", async () => {
    // keep an account with two spaces and a document in each at home
    const home = await TestDatabase.create(
        "sqlite",
        [...space.tables, ...document.tables, ...callTables],
        { isMigrated: true },
    );
    const declared = defineDatabase({
        name: "installed",
        tables: [...comment.tables, ...callTables, controllerLease],
        copies: [space.table, document.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const accessCopies = new AccessFixture(home.database);
    await accessCopies.copyScope(account.reference(Scope.universe.id, accountId));
    for (const id of [spaceId, otherSpaceId]) {
        await accessCopies.copyScope(space.reference(accountId, id), "own");
        await home.database
            .insert(space.table)
            .values({ id, scope: accountId, createdAt: 1, updatedAt: 1 });
        await home.database.insert(document.table).values({
            id: schema.identifier("document").parse(`document-${id.slice("space-".length)}`),
            scope: id,
            title: id,
            createdAt: 1,
            updatedAt: 1,
        });
    }

    // let the installation read and replicate the first space only
    const source = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [space, document],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: document.package },
    });
    const fixture = new AccessFixture(home.database);
    const subject = principal.installation.reference(followingSpaceId, followingId);
    const followedSpace = space.reference(accountId, spaceId);
    const reading = await fixture.copyRelationship(spaceId, followedSpace, "reader", subject, 1);
    await fixture.copyRelationship(spaceId, followedSpace, "replicator", subject, 1);

    // follow the universe's rows and the followed space's chain from home
    const follower = { subject, parent: Scope.universe.id };
    const copying: ObjectStore = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [comment],
        policies: [space, document],
        database: installed.database,
        subscriber: Subscriber.of(
            {
                receive: refuseChanges,
                stream: (asked, signal) =>
                    source.source.replicate(asked, asked.after, follower, signal),
            },
            () => copying.source.servedSubscriptions(followingSpaceId),
        ),
        callKey: testCallKey,
        events: await testEvents(installed.database),
        origin: { package: document.package },
    });
    const replica = present(
        copying.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const loop = new ControlLoop(installed.database, [replica], {
        clock: Clock.system,
        report: failing,
    }).run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await loop;
    });

    // receive the followed space's document, then lose it once the installation may no longer read the space
    const documents = () =>
        installed.database.select({ scope: document.table.scope }).from(document.table);
    await installed.database.log.until(
        async () => (await documents()).length > 0,
        AbortSignal.timeout(COPY_MILLISECONDS),
    );
    const followed = await documents();
    await home.database.delete(accessRelationship).where(eq(accessRelationship.id, reading));
    await installed.database.log.until(
        async () => (await documents()).length === 0,
        AbortSignal.timeout(COPY_MILLISECONDS),
    );
    expect({ followed, left: await documents() }).toEqual({
        followed: [{ scope: spaceId }],
        left: [],
    });
});

/** Keep an account and two spaces at home, and serve a machine following their chains. */
async function openCopies() {
    // keep the account, its member role and its two spaces at home
    const home = await TestDatabase.create("sqlite", accessTables, { isMigrated: true });
    const machine = await TestDatabase.create(
        "sqlite",
        [...document.tables, ...callTables, controllerLease],
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), machine.close()]);
    });
    const accountObject = account.reference(Scope.universe.id, accountId);
    const accessCopies = new AccessFixture(home.database);
    await accessCopies.copyScope(accountObject);
    await accessCopies.copyScope(space.reference(accountId, spaceId), "own");
    await accessCopies.copyScope(space.reference(accountId, otherSpaceId), "own");
    await accessCopies.copyRole(
        accountObject,
        { name: "member", description: "Reads the account's spaces", permissions: [] },
        { ...accountObject, relation: "member" },
    );

    // follow each requested space's chain from home, failing every stream once home refuses the machine
    const feed = new Feed(home.database, accessTables, { clock: Clock.system });
    const requested = new Set<string>();
    const refused = new AbortController();
    const refuse = () => refused.abort(new CallError("FORBIDDEN", { message: "refused" }));
    const store: ObjectStore = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [document],
        database: machine.database,
        subscriber: Subscriber.of(
            {
                receive: refuseChanges,
                stream: async function* (subscription, signal) {
                    yield* feed.subscribe(
                        store.source.replicaOf(subscription).queries,
                        subscription.after,
                        AbortSignal.any([signal, refused.signal]),
                    );
                    refused.signal.throwIfAborted();
                },
            },
            async () =>
                (
                    await Promise.all(
                        [...requested].map((below): Promise<Subscription[]> =>
                            store.authorizer.chain(machine.database, below, { kind: "follower" }),
                        ),
                    )
                ).flat(),
        ),
        callKey: testCallKey,
        events: await testEvents(machine.database),
        origin: { package: document.package },
    });

    return { home, machine, store, requested, refuse };
}

/** Fail a test on a failed reconciliation. */
function failing(_controller: unknown, key: string, error: unknown): never {
    throw new Error(`reconciling ${key} failed`, { cause: error });
}

/** Read the scope a replica controller's key follows. */
function scopeOf(key: string): string | undefined {
    return key.split(" ")[1];
}

test("copy the chains of the accounts an installation's rows relate to, beside its own space where those rows live", async () => {
    // keep two accounts at home, the first readable by the installation
    const home = await TestDatabase.create("sqlite", [...account.tables, ...callTables], {
        isMigrated: true,
    });
    const declared = defineDatabase({
        name: "installed",
        tables: [...annotation.tables, ...callTables],
        copies: [account.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const accessCopies = new AccessFixture(home.database);
    for (const id of [accountId, otherAccountId]) {
        await accessCopies.copyScope(account.reference(Scope.universe.id, id));
        await home.database
            .insert(account.table)
            .values({ id, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 });
    }
    const subject = principal.installation.reference(followingSpaceId, followingId);
    await accessCopies.copyRelationship(
        accountId,
        account.reference(Scope.universe.id, accountId),
        "reader",
        subject,
        1,
    );

    // copy the universe's accounts to the installation, decided at home for it
    const source = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [account],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: account.package },
    });
    const follower = { subject, parent: Scope.universe.id };
    const copying: ObjectStore = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        objects: [annotation],
        policies: [account],
        database: installed.database,
        subscriber: Subscriber.of(
            {
                receive: refuseChanges,
                stream: (asked, signal) =>
                    source.source.replicate(asked, asked.after, follower, signal),
            },
            async () => [
                present(
                    copying.source.universeSubscription(followingSpaceId),
                    "the universe request",
                ),
            ],
        ),
        callKey: testCallKey,
        events: await testEvents(installed.database),
        origin: { package: annotation.package },
    });
    await followUniverse(copying, home.database);

    // request one copy of the readable account's chain and the installation's own space
    const requests = await copying.source.servedSubscriptions(followingSpaceId);
    const chains = present(requests[1], "the chains' request");
    expect({
        requests: requests.map((request) => [request.name, request.scope, request.below]),
        via: chains.parameters["via"],
    }).toEqual({
        requests: [
            [followingSpaceId, Scope.universe.id, followingSpaceId],
            [Authorizer.chainCopy(followingSpaceId), Scope.universe.id, followingSpaceId],
        ],
        via: [accountId, followingSpaceId],
    });
});

test("copy the chain of an account another copy brought first, once the universe copy includes it", async () => {
    // keep an account at home that the installation reads
    const home = await TestDatabase.create("sqlite", [...account.tables, ...callTables], {
        isMigrated: true,
    });
    const declared = defineDatabase({
        name: "installed",
        tables: [...annotation.tables, ...callTables],
        copies: [account.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const accessCopies = new AccessFixture(home.database);
    await accessCopies.copyScope(account.reference(Scope.universe.id, accountId));
    await home.database
        .insert(account.table)
        .values({ id: accountId, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 });
    const subject = principal.installation.reference(followingSpaceId, followingId);
    await accessCopies.copyRelationship(
        accountId,
        account.reference(Scope.universe.id, accountId),
        "reader",
        subject,
        1,
    );

    // serve the accounts at home, and record the first chain request via the account
    const source = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [account],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: account.package },
    });
    const follower = { subject, parent: Scope.universe.id };
    const chain = Authorizer.chainCopy(followingSpaceId);
    const followed = Promise.withResolvers<unknown>();
    const publisher = {
        receive: refuseChanges,
        stream: async function* (asked: Subscription, signal: AbortSignal) {
            // stream the universe's rows from home
            if (asked.name !== chain) {
                yield* source.source.replicate(asked, asked.after, follower, signal);
            }
            // record a chain request via the account and keep it open
            else {
                const via = asked.parameters["via"];
                if (Array.isArray(via) && via.includes(accountId)) {
                    followed.resolve(via);
                }
                await until(signal);
            }
        },
    };

    // copy the universe's rows under another name first, as a residents copy does
    const copying: ObjectStore = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        objects: [annotation],
        policies: [account],
        database: installed.database,
        subscriber: Subscriber.of(publisher, async () => [
            residents,
            ...(await copying.source.servedSubscriptions(followingSpaceId)),
        ]),
        callKey: testCallKey,
        events: await testEvents(installed.database),
        origin: { package: annotation.package },
    });
    const residents = {
        ...present(copying.source.universeSubscription(followingSpaceId), "the universe request"),
        name: "residents",
    };
    const early = new AbortController();
    const copied = copying.source
        .replicaOf(residents)
        .follow(
            installed.database,
            (from, signal) => publisher.stream({ ...residents, ...from }, signal),
            early.signal,
            { subscription: residents, clock: Clock.system },
        );
    const head = await home.database.log.position();
    await Replica.wait(
        installed.database,
        Scope.universe.id,
        head,
        AbortSignal.timeout(COPY_MILLISECONDS),
    );
    early.abort();
    await Promise.allSettled([copied]);

    // run the copies, whose universe copy then only records the account it shares with the other copy
    const controller = new AbortController();
    const loop = new ControlLoop(
        installed.database,
        copying.controllers().filter((each) => each.name === "replica"),
        { clock: Clock.system, report: failing },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });

    // list the copies again and follow the chain via the account
    const timeout = until(AbortSignal.timeout(COPY_MILLISECONDS)).then(
        () => "no chain via the account",
    );
    expect(await Promise.race([followed.promise, timeout])).toEqual([accountId, followingSpaceId]);
});

/** Follow a store's copy of the universe until the test finishes. */
async function followUniverse(
    copying: ObjectStore,
    home: DatabaseConnection,
): Promise<AbortController> {
    // follow the store's one copy of the universe
    const replica = present(
        copying.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const [key] = await replica.list();
    const following = replica.reconcile(
        present(key, "the universe's key"),
        reconciliation(controller.signal),
    );
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled([following]);
    });

    // wait for it to reach the home's head
    const head = await home.log.position();
    await Replica.wait(copying.database, Scope.universe.id, head, controller.signal);

    return controller;
}
