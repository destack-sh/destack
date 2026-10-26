import { testSimulation } from "../src/test/index.ts";
import { SimulatedClock } from "@destack/environment/simulation";
import { subjectContext, testCallKey } from "@destack/object/test";
import { reconciliation, TestDatabase } from "@destack/db/test";
import { FeedPublisher, testEvents, testServerContext, serversOf } from "../src/test/index.ts";
import {
    ControlLoop,
    type DatabaseConnection,
    defineDatabase,
    eq,
    Key,
    lease,
    ReadSet,
    Snapshot,
    TABLE,
} from "@destack/db";
import { definePackage } from "@destack/object";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    AccessContext,
    accessGrant,
    accessTables,
    Caller,
    contained,
    accessRelationship,
    none,
    principal,
    relation,
    Relationship,
    through,
    union,
} from "@destack/access";

import { AccessFixture } from "@destack/access/test";
import { canonicalize, Identifier, nth, present, schema, Uuid } from "@destack/schema";
import { followKey } from "../src/replica/key.ts";

import { CallError } from "@destack/object";
import {
    RequestId,
    Scope,
    Feed,
    Replica,
    type Page,
    type Subscription,
    type Watermark,
} from "@destack/sync";
import { until, yieldToEventLoop } from "@destack/environment";
import * as sync from "@destack/sync";
import { defineObject, defineShape, field, Follow } from "../src/index.ts";
import { ObjectSink, ObjectStore } from "../src/server/index.ts";
import { userContext, users } from "./fixture/user.ts";
import { spacePrincipal } from "./fixture/space.ts";
import { tablesOf } from "../src/stack/db.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The event loop turns a woken store gets to report ready early, ten times one check. */
const READY_TURNS = 20;

/** The source rows the incremental listing test follows: a thousand. */
const STRESS_ROWS = 1000;

/** How long a replica may take to arrive at its home or relist: 5 s, far above its milliseconds. */
const REPLICA_MILLISECONDS = 5000;

/** Refuse a change sent over the uplink, as these replicas receive none. */
const refuseChanges = () => Promise.reject(new Error("the fixture receives no changes"));

/** Refuse a query sent over the uplink, as these replicas send none. */
const refuseQueries = () => Promise.reject(new Error("the fixture answers no queries"));

/** The account with the space. */
const accountId = schema
    .identifier("account")
    .parse("account-01996ab0-0000-7000-8000-000000000001");

/** The empty space the documents go into. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** Another space of the account outside every restriction here. */
const otherSpaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** The space of an installation replicating the spaces' chains. */
const followingSpaceId = "space-01996ab0-0000-7000-8000-000000000005";

/** The installation replicating the spaces' chains, living in its space. */
const followingId = "installation-01996ab0-0000-7000-8000-000000000006";

/** Another account, without the spaces. */
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
});

/** Spaces within accounts, which installations read and replicate as granted. */
const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: account,
    isScope: true,
    partition: true,
    fields: {},
    relations: {
        reader: { subjects: [principal.installation], grantedBy: null },
        replicator: { subjects: [principal.installation], grantedBy: null },
    },
    permissions: { read: relation("reader"), replicate: relation("replicator") },
});

/** Comments an installation stores in the spaces it follows. */
const comment = defineObject({
    name: "comment",
    plural: "comments",
    scope: space,
    fields: { body: field.string() },
    permissions: { read: none() },
});

/** Annotations an installation stores in its space, each readable through its account. */
const annotation = defineObject({
    name: "annotation",
    plural: "annotations",
    scope: space,
    fields: { account: field.reference(account).relate("account"), body: field.string() },
    permissions: { read: through("account", "read") },
});

/** Documents that only roles grant. */
const document = defineObject(
    {
        name: "document",
        plural: "documents",
        scope: space,
        fields: { title: field.string(schema.string().min(1)) },
        permissions: { read: none(), write: none() },
    },
    (method) => ({
        create: method.create("write"),
    }),
);

/** The documents of a scope. */
const documentShape = defineShape({
    name: "documents",
    parameters: schema.object({}),
    objects: [document],
    queries: () => [document.query()],
});

/** The accounts, every row at the universe, which installations relate to. */
const ACCOUNTS = defineShape({
    name: "accounts",
    parameters: schema.object({}),
    objects: [account],
    queries: () => [account.query({})],
});

/** The spaces in every account. */
const SPACES = defineShape({
    name: "spaces",
    parameters: schema.object({}),
    objects: [space],
    queries: () => [space.query({ scopes: "every" })],
});

/** The follow of the accounts at the universe. */
const ACCOUNTS_FOLLOW = Follow.of(ACCOUNTS, { keys: [{ key: { scope: Scope.universe.id } }] });

/** The spaces and their documents in every account. */
const DOCUMENTS = defineShape({
    name: "documents",
    parameters: schema.object({}),
    objects: [space, document],
    queries: () => [space.query({ scopes: "every" }), document.query({ scopes: "every" })],
});

/** The spaces a follow on a person's behalf brings into an installation. */
const ACTIVITY = defineShape({
    name: "activity",
    parameters: schema.object({}),
    objects: [space],
    queries: () => [space.query({ scopes: "every" })],
});

/** The follow of the spaces at the universe. */
const SPACES_FOLLOW = Follow.of(SPACES, { keys: [{ key: { scope: Scope.universe.id } }] });

/** The follow of the spaces and their documents at the universe. */
const DOCUMENTS_FOLLOW = Follow.of(DOCUMENTS, { keys: [{ key: { scope: Scope.universe.id } }] });

test("decide calls over a relayed replica of access as the holder does, through the account's members and restricted credentials", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const home = await TestDatabase.create(accessTables, { environment: world, isMigrated: true });
    const installed = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });

    // insert an account of alice, a member role carol has through the account, and its empty space at home
    const alice = principal.user.reference("universe", users.alice);
    const carol = principal.user.reference("universe", users.carol);
    const accountObject = account.reference(Scope.universe.id, accountId);
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(accountObject, account.policy);
    await access.replicateScope(space.reference(accountId, spaceId), space.policy);
    await access.replicateOwner(accountObject, alice);
    await access.replicateRole(
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
    await home.database.transaction(
        async (transaction) =>
            await transaction.insert(accessRelationship).values(
                Relationship.encode({
                    id: schema.identifier("relationship").parse(`relationship-${Uuid.v7(world)}`),
                    object: accountObject,
                    relation: "member",
                    subject: carol,
                    createdAt: 0,
                    expiresAt: null,
                }),
            ),
    );

    // serve the documents of one space over the installation's database to each call's user, replicating its chain from the home database
    const feed = new Feed(home.database, accessTables);
    const store: ObjectStore = await TestObjectStore.at(space.reference(accountId, spaceId), {
        chain: async () => ({ kind: "follower", parent: accountId }),
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document]),
        database: installed.database,
        uplink: {
            ...FeedPublisher.of(
                feed,
                (subscription) => store.source.buildReplica(subscription).queries,
            ),
            receive: refuseChanges,
            call: refuseQueries,
        },
        callKey: testCallKey,
        events: await testEvents(installed.database, world),
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
        // follow each scope's chain, which at the universe has the grants delegated in
        const keys = (await follower.list(Snapshot.live(store.database))).filter(
            (entry) => readScope(entry) === scope,
        );
        for (const key of keys) {
            following.push(
                follower.reconcile(key, reconciliation(controller.signal)).then(() => undefined),
            );
        }
        await Replica.wait(installed.database, scope, head, controller.signal);
    }
    expect((await follower.list(Snapshot.live(store.database))).map(readScope)).toEqual([
        spaceId,
        accountId,
        Scope.universe.id,
    ]);

    // create through tokens whose grants give the space, another one or everything, and list as alice
    const create = async (user: Identifier<"user">, title: string, restricted?: string) => {
        // limit the caller's token to one scope by its grant when asked
        const token = principal.client.reference(Scope.universe.id, `client-${user}`);
        const grantId =
            restricted === undefined
                ? undefined
                : await new AccessFixture(installed.database, world).replicateGrant(
                      principal.user.reference("universe", user),
                      token,
                      [document.permission("read"), document.permission("write")].map(
                          (permission) => ({ ...permission, scope: restricted }),
                      ),
                  );
        const context = userContext(
            user,
            spaceId,
            grantId === undefined
                ? { environment: world }
                : {
                      environment: world,
                      actors: [token],
                      grants: [{ id: grantId, scope: user }],
                  },
        );

        const created = store
            .mutate(context.access(), {
                id: RequestId.create(world),
                calls: [document.call("create", { spaceId, title })],
            })
            .then(([written]) => written);
        const refused = await refusal(created);

        return refused === "done" ? (await created).title : refused;
    };
    const outcomes = [
        await create(users.alice, "Plan", spaceId),
        await create(users.alice, "Elsewhere", otherSpaceId),
        await create(users.dave, "Leak", spaceId),
        await create(users.dave, "Leak"),
        await create(users.carol, "Budget"),
    ];
    const listed = await store.query(
        userContext(users.alice, spaceId, { environment: world }).access(),
        document.call("list", { spaceId }),
    );

    // allow the owner through a credential for the empty space and hide the space from the rest
    const hidden = ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${spaceId} not found`];
    expect(outcomes).toEqual(["Plan", hidden, hidden, hidden, "Budget"]);
    expect(listed.items.map((item) => item.title).toSorted()).toEqual(["Budget", "Plan"]);
});

/** An account outside every chain the tests' followers have. */
const FOREIGN_ACCOUNT = "account-01996ab0-0000-7000-8000-0000000000ff";

test("record the reads of a replica listing, so only a change of a followed source row touches it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // follow another space's annotations for each document titled to ask for them
    const machine = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document, annotation }, tables: [] }],
            follows: [],
            tables: [lease],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => machine.close());
    const annotations = defineShape({
        name: "annotations",
        parameters: schema.object({}),
        objects: [annotation],
        queries: () => [annotation.query()],
    });
    const store = await TestObjectStore.at(space.reference(Scope.universe.id, spaceId), {
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document], {
            follows: [
                document.follow(annotations, {
                    where: { title: "Follow" },
                    key: async () => ({ key: { scope: otherSpaceId } }),
                }),
            ],
        }),
        database: machine.database,
        uplink: {
            receive: refuseChanges,
            call: refuseQueries,
            stream: async function* () {},
            confirm: async () => {},
        },
        callKey: testCallKey,
        events: await testEvents(machine.database, world),
        origin: { package: document.package },
    });
    const id = Identifier.create("document", world);
    await machine.database.transaction(
        async (transaction) =>
            await transaction
                .insert(document.table)
                .values({ id, scope: spaceId, title: "Follow", createdAt: 1, updatedAt: 1 }),
    );
    const replica = present(
        store.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const reads = new ReadSet();
    await replica.list(Snapshot.live(machine.database).recording(reads));

    // add a document the follow leaves out, then retitle the followed one out of the follow
    const after = (await machine.database.log.position()).sequence;
    await machine.database.transaction(
        async (transaction) =>
            await transaction.insert(document.table).values({
                id: Identifier.create("document", world),
                scope: spaceId,
                title: "Draft",
                createdAt: 1,
                updatedAt: 1,
            }),
    );
    await machine.database.transaction(
        async (transaction) =>
            await transaction
                .update(document.table)
                .set({ title: "Other" })
                .where(eq(document.table.id, id)),
    );
    const changes = (await machine.database.log.read({ tables: [document.table], after })).changes;

    // touch the listing with the retitle alone
    expect(changes.map((change) => [change.operation, reads.touches(change)])).toEqual([
        ["insert", false],
        ["update", true],
    ]);
});

test("read a row follow's keys again only for the source rows a change touched, across many rows", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // follow another space's annotations for each document, counting the key reads
    const machine = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document, annotation }, tables: [] }],
            follows: [],
            tables: [lease],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => machine.close());
    const annotations = defineShape({
        name: "annotations",
        parameters: schema.object({}),
        objects: [annotation],
        queries: () => [annotation.query()],
    });
    let keyReads = 0;
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document], {
            follows: [
                document.follow(annotations, {
                    key: async (row, view) => {
                        // read the row's peers too, so a peer's change touches the row's key
                        keyReads += 1;
                        await view
                            .read(document.table)
                            .findMany({ where: { title: `peer of ${row.id}` } });

                        return { key: { scope: otherSpaceId } };
                    },
                }),
            ],
        }),
        database: machine.database,
        uplink: {
            receive: refuseChanges,
            call: refuseQueries,
            stream: async function* () {},
            confirm: async () => {},
        },
        callKey: testCallKey,
        events: await testEvents(machine.database, world),
        origin: { package: document.package },
    });
    const ids = Array.from({ length: STRESS_ROWS }, () => Identifier.create("document", world));
    await machine.database.transaction(
        async (transaction) =>
            await transaction.insert(document.table).values(
                ids.map((id) => ({
                    id,
                    scope: spaceId,
                    title: "Notes",
                    createdAt: 1,
                    updatedAt: 1,
                })),
            ),
    );
    const replica = present(
        store.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );

    // list every row's key, then list again for a retitled row and for an added one
    await replica.list(Snapshot.live(machine.database));
    const listed = keyReads;
    const relist = async (change: () => PromiseLike<unknown>) => {
        const after = (await machine.database.log.position()).sequence;
        await change();
        const { changes } = await machine.database.log.read({ tables: [document.table], after });
        const before = keyReads;
        await replica.list(Snapshot.live(machine.database).recording(new ReadSet()), {
            changes,
            unlogged: new Set(),
        });

        return keyReads - before;
    };
    const retitled = await relist(() =>
        machine.database.transaction(
            async (transaction) =>
                await transaction
                    .update(document.table)
                    .set({ title: "Plans" })
                    .where(eq(document.table.id, nth(ids, 0))),
        ),
    );
    const added = await relist(() =>
        machine.database.transaction(
            async (transaction) =>
                await transaction.insert(document.table).values({
                    id: Identifier.create("document", world),
                    scope: spaceId,
                    title: "Added",
                    createdAt: 1,
                    updatedAt: 1,
                }),
        ),
    );
    const peered = await relist(() =>
        machine.database.transaction(
            async (transaction) =>
                await transaction.insert(document.table).values({
                    id: Identifier.create("document", world),
                    scope: spaceId,
                    title: `peer of ${nth(ids, 1)}`,
                    createdAt: 1,
                    updatedAt: 1,
                }),
        ),
    );

    // read each row's key once, then the retitled, the added, and the peered row's with its peer's
    expect({ listed, retitled, added, peered }).toEqual({
        listed: STRESS_ROWS,
        retitled: 1,
        added: 1,
        peered: 2,
    });
});

test("serve a store whose follows ask for nothing before its replica controller lists them", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // follow nothing for any document, with no chain above the store
    const machine = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows: [],
            tables: [lease],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => machine.close());
    const annotations = defineShape({
        name: "annotations",
        parameters: schema.object({}),
        objects: [annotation],
        queries: () => [annotation.query()],
    });
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document], {
            follows: [document.follow(annotations, { key: async () => undefined })],
        }),
        database: machine.database,
        uplink: {
            receive: refuseChanges,
            call: refuseQueries,
            stream: async function* () {},
            confirm: async () => {},
        },
        callKey: testCallKey,
        events: await testEvents(machine.database, world),
        origin: { package: document.package },
    });

    // list the replicas for readiness itself, finding none to wait for
    const ready = present(store.packages[0]?.ready, "the package's readiness");
    const waited = world.clock.timeout(1000);
    await ready(waited);

    // become ready before the wait runs out, following nothing
    expect({ isTimedOut: waited.aborted, followed: await store.sink.followed() }).toEqual({
        isTimedOut: false,
        followed: [],
    });
});

test("report a follow its source refuses, and drop its replica once a change of the rows the follow reads stops requesting it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store an account and its two spaces at home, and a document of the first space on a machine
    const home = await TestDatabase.create(
        tablesOf({ packages: [{ objects: { document }, tables: [] }], follows: [], tables: [] })
            .tables,
        { environment: world, isMigrated: true },
    );
    const machine = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows: [],
            tables: [lease],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), machine.close()]);
    });
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await access.replicateScope(space.reference(accountId, spaceId), space.policy);
    await access.replicateScope(space.reference(accountId, otherSpaceId), space.policy);
    const requesting = Identifier.create("document", world);
    await machine.database.transaction(
        async (transaction) =>
            await transaction.insert(document.table).values({
                id: requesting,
                scope: spaceId,
                title: "Notes",
                createdAt: 1,
                updatedAt: 1,
            }),
    );

    // follow the other space's documents beside the scope chain while a document asks, failing refused streams
    const feed = new Feed(home.database, [...accessTables, ...document.tables]);
    const refusals = new Map<string, AbortController>();
    const closing = (scope: string) => {
        const refused = refusals.get(scope) ?? new AbortController();
        refusals.set(scope, refused);

        return refused;
    };
    const store: ObjectStore = await TestObjectStore.at(space.reference(accountId, spaceId), {
        chain: async () => ({ kind: "follower", parent: accountId }),
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document], {
            follows: [
                document.follow(documentShape, {
                    key: async (row) =>
                        row.title === "Notes" ? { key: { scope: otherSpaceId } } : undefined,
                }),
            ],
        }),
        database: machine.database,
        publisher: (scope) => {
            // publish the feed, ending each stream once its scope's publisher closes
            const published = FeedPublisher.of(
                feed,
                (subscription) => store.source.buildReplica(subscription).queries,
            );

            return {
                confirm: published.confirm,
                stream: async function* (subscription, signal) {
                    const { signal: closed } = closing(scope);
                    yield* published.stream(subscription, AbortSignal.any([signal, closed]));
                    closed.throwIfAborted();
                },
            };
        },
        callKey: testCallKey,
        events: await testEvents(machine.database, world),
        origin: { package: document.package },
    });

    // follow the replicas to the home's head
    const controller = new AbortController();
    const failures: unknown[] = [];
    const loop = new ControlLoop(
        machine.database,
        store.controllers().filter((each) => each.name === "replica"),
        // run the loop at a frozen clock, so a drop that waits out a failure's backoff never comes
        {
            clock: new SimulatedClock(world.clock.now()),
            report: (_controller, _key, error) => failures.push(error),
        },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const head = await home.database.log.position();
    for (const scope of [spaceId, otherSpaceId, accountId, Scope.universe.id]) {
        await Replica.wait(machine.database, scope, head, controller.signal);
    }

    // refuse the other space's documents at home, reported while a document still asks for them
    const refused = CallError.http("FORBIDDEN", { message: "refused" });
    closing(otherSpaceId).abort(refused);
    await expect.poll(() => failures).toEqual([refused]);

    // stop asking for them, which drops their replica
    await machine.database.transaction(
        async (transaction) =>
            await transaction
                .update(document.table)
                .set({ title: "Archived" })
                .where(eq(document.table.id, requesting)),
    );
    await machine.database.log.until(
        async () => (await localReplicas(machine.database)) === LOCAL_REPLICAS,
        controller.signal,
    );
    expect(failures).toEqual([refused]);
});

test("finish a wait through the replicas a store still follows once its sink drops another replica of the scope", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // follow the other space's activity at a fixed key
    const activity = Follow.of(ACTIVITY, { keys: [{ key: { scope: otherSpaceId } }] });

    // store an account and its two spaces at home, and a document of the first space on a machine
    const home = await TestDatabase.create(
        tablesOf({
            packages: [{ objects: { space, document }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    const machine = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows: [activity],
            tables: [lease],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), machine.close()]);
    });
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await access.replicateScope(space.reference(accountId, spaceId), space.policy);
    await access.replicateScope(space.reference(accountId, otherSpaceId), space.policy);
    const requesting = Identifier.create("document", world);
    await machine.database.transaction(
        async (transaction) =>
            await transaction.insert(document.table).values({
                id: requesting,
                scope: spaceId,
                title: "Notes",
                createdAt: 1,
                updatedAt: 1,
            }),
    );

    // publish the feed, confirming positions to every replica except the documents one
    const feed = new Feed(home.database, [...accessTables, space.table, ...document.tables]);
    const published = FeedPublisher.of(
        feed,
        (subscription) => store.source.buildReplica(subscription).queries,
    );
    const documents = documentShape.replicaName({ scope: otherSpaceId });
    const publisher: sync.Publisher = {
        stream: published.stream,
        confirm: async (subscription, watermark) => {
            if (subscription.name !== documents) {
                await published.confirm(subscription, watermark);
            }
        },
    };

    // follow the other space's activity, and its documents while the document asks for them
    const store: ObjectStore = await TestObjectStore.at(space.reference(accountId, spaceId), {
        chain: async () => ({ kind: "follower", parent: accountId }),
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document], {
            follows: [
                activity,
                document.follow(documentShape, {
                    key: async (row) =>
                        row.title === "Notes" ? { key: { scope: otherSpaceId } } : undefined,
                }),
            ],
        }),
        database: machine.database,
        publisher: () => publisher,
        callKey: testCallKey,
        events: await testEvents(machine.database, world),
        origin: { package: document.package },
    });

    // follow the replicas to the home's head
    const controller = new AbortController();
    const loop = new ControlLoop(
        machine.database,
        store.controllers().filter((each) => each.name === "replica"),
        { clock: world.clock, report: failing },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const followed = await home.database.log.position();
    for (const scope of [spaceId, otherSpaceId, accountId, Scope.universe.id]) {
        await Replica.wait(machine.database, scope, followed, controller.signal);
    }

    // commit a document of the first space at home, outside both replicas of the other space
    await home.database.transaction(
        async (transaction) =>
            await transaction.insert(document.table).values({
                id: Identifier.create("document", world),
                scope: spaceId,
                title: "Elsewhere",
                createdAt: 1,
                updatedAt: 1,
            }),
    );
    const head = await home.database.log.position();

    // wait for the head at the other space, held back by its unconfirmed documents replica
    const waiting = store.wait(
        { scope: otherSpaceId, ...head },
        world.clock.timeout(REPLICA_MILLISECONDS),
    );

    // stop asking for the other space's documents, which drops their replica
    await machine.database.transaction(
        async (transaction) =>
            await transaction
                .update(document.table)
                .set({ title: "Archived" })
                .where(eq(document.table.id, requesting)),
    );

    // reach the head through the activity replica the store still follows
    expect({
        isReached: await waiting,
        followed: (await store.sink.followed()).map(({ subscription }) => [
            subscription.name,
            subscription.scope,
        ]),
    }).toEqual({
        isReached: true,
        followed: [
            [ObjectSink.chainName(spaceId), spaceId],
            [ObjectSink.chainName(spaceId), accountId],
            [ObjectSink.chainName(spaceId), Scope.universe.id],
            [ACTIVITY.replicaName({ scope: otherSpaceId }), otherSpaceId],
        ],
    });
});

test("replicate a person's grant to an installation of the space from the universe into the space", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store alice's grant to the space's installation, and another's grant to one elsewhere, at home
    const home = await TestDatabase.create(accessTables, { environment: world, isMigrated: true });
    const machine = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows: [],
            tables: [lease],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), machine.close()]);
    });
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await access.replicateScope(space.reference(accountId, spaceId), space.policy);
    const alice = principal.user.reference(Scope.universe.id, users.alice);
    const granted = await access.replicateGrant(
        alice,
        principal.installation.reference(
            spaceId,
            "installation-01996ab0-0000-7000-8000-0000000000b1",
        ),
        [{ ...document.permission("read"), scope: spaceId }],
    );
    await access.replicateGrant(
        alice,
        principal.installation.reference(
            otherSpaceId,
            "installation-01996ab0-0000-7000-8000-0000000000b2",
        ),
        [{ ...document.permission("read"), scope: otherSpaceId }],
    );

    // follow the space's replicas from home
    const feed = new Feed(home.database, accessTables);
    const store: ObjectStore = await TestObjectStore.at(space.reference(accountId, spaceId), {
        chain: async () => ({ kind: "follower", parent: accountId }),
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document]),
        database: machine.database,
        uplink: {
            ...FeedPublisher.of(
                feed,
                (subscription) => store.source.buildReplica(subscription).queries,
            ),
            receive: refuseChanges,
            call: refuseQueries,
        },
        callKey: testCallKey,
        events: await testEvents(machine.database, world),
        origin: { package: document.package },
    });
    const controller = new AbortController();
    const loop = new ControlLoop(
        machine.database,
        store.controllers().filter((each) => each.name === "replica"),
        { clock: world.clock, report: failing },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const head = await home.database.log.position();
    await Replica.wait(machine.database, Scope.universe.id, head, controller.signal);

    // retain only the grant to the space's installation
    const remaining = await machine.database.select({ id: accessGrant.id }).from(accessGrant);
    expect(remaining.map((row) => row.id)).toEqual([granted]);
});

test("serve a woken store and let its controllers act only once its home confirms the replicas it stored unheard past the lag", async () => {
    // insert an account and its space at home, both databases on the world's clock
    const simulation = testSimulation();
    const { clock } = simulation;
    const world = simulation.environment("test");
    const home = await TestDatabase.create(accessTables, { environment: world, isMigrated: true });
    const installed = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await access.replicateScope(space.reference(accountId, spaceId), space.policy);

    // serve the space over the installation's database under an hour's heartbeat
    const events = await testEvents(installed.database, world);
    const feed = new Feed(home.database, accessTables, { heartbeat: 3_600_000 });
    const open = async (pages: Promise<unknown>): Promise<ObjectStore> => {
        const published = FeedPublisher.of(
            feed,
            (subscription) => store.source.buildReplica(subscription).queries,
        );
        const store: ObjectStore = await TestObjectStore.at(space.reference(accountId, spaceId), {
            chain: async () => ({ kind: "follower", parent: accountId }),
            environment: world,
            principal: spacePrincipal(spaceId),
            servers: serversOf([document]),
            database: installed.database,
            uplink: {
                ...published,
                // block the universe chain's pages until released
                async *stream(subscription, signal) {
                    if (subscription.scope === Scope.universe.id) {
                        await pages;
                    }
                    yield* published.stream(subscription, signal);
                },
                receive: refuseChanges,
                call: refuseQueries,
            },
            callKey: testCallKey,
            events,
            origin: { package: document.package },
        });

        return store;
    };
    const follow = (store: ObjectStore, signal: AbortSignal) => {
        const loop = new ControlLoop(
            installed.database,
            store.controllers().filter((each) => each.name === "replica"),
            { clock, report: failing },
        );

        return loop.run(signal);
    };

    // follow the chain once, then stop as an evicted worker does and let a minute pass
    const first = new AbortController();
    const started = clock.now();
    const followed = await open(Promise.resolve());
    const firstFollow = follow(followed, first.signal);
    const isComplete = async () => {
        const gates = await followed.sink.gates();
        const current = await Promise.all(
            gates.map(({ subscription }) =>
                Replica.confirmedAt(installed.database, subscription).then(
                    (at) => at !== undefined && at >= started,
                ),
            ),
        );

        return gates.length > 0 && current.every(Boolean);
    };
    await installed.database.log.until(isComplete, world.clock.timeout(5000));
    first.abort();
    await firstFollow;
    clock.advance(60_000);

    // wake a store over the same database, blocking the universe's pages while the others arrive
    const pages = Promise.withResolvers<void>();
    const woken = await open(pages.promise);
    const handle = definePackage({ objects: { document } });
    const implemented = woken.implement(handle, [], []);
    const ready = present(implemented.ready, "the readiness wait");
    const confirmed = present(implemented.confirmed, "the confirmation wait");
    const waking = new AbortController();
    onTestFinished(() => waking.abort());
    let isReady = false;
    let isConfirmed = false;
    const readying = ready(waking.signal).then(() => {
        isReady = true;
    });
    const confirming = confirmed(waking.signal).then(() => {
        isConfirmed = true;
    });
    const following = follow(woken, waking.signal);
    const now = clock.now();
    const isOthersFresh = async () => {
        const gates = (await woken.sink.gates()).filter(
            ({ subscription }) => subscription.scope !== Scope.universe.id,
        );
        const fresh = await Promise.all(
            gates.map(({ subscription }) =>
                Replica.confirmedAt(installed.database, subscription).then(
                    (at) => at !== undefined && at >= now,
                ),
            ),
        );

        return gates.length > 0 && fresh.every(Boolean);
    };
    await installed.database.log.until(isOthersFresh, world.clock.timeout(5000));
    for (let turn = 0; turn < READY_TURNS; turn++) {
        await yieldToEventLoop();
    }
    const before = { isReady, isConfirmed };

    // release the pages, which confirm the replicas fresh and make the store ready
    pages.resolve();
    await Promise.all([readying, confirming]);
    waking.abort();
    await following;
    expect({ before, after: { isReady, isConfirmed } }).toEqual({
        before: { isReady: false, isConfirmed: false },
        after: { isReady: true, isConfirmed: true },
    });
});

test("follow replicates as a server without methods, journal or call key", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store an account and its space at home, and a database with access replicas
    const home = await TestDatabase.create(accessTables, { environment: world, isMigrated: true });
    const follower = await TestDatabase.create(accessTables, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(async () => {
        await Promise.all([home.close(), follower.close()]);
    });
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await access.replicateScope(space.reference(accountId, spaceId), space.policy);

    // follow the universe's rows the space reads, without a journal
    const feed = new Feed(home.database, accessTables);
    const store: ObjectStore = await TestObjectStore.at(space.reference(accountId, spaceId), {
        chain: async () => ({ kind: "follower", parent: accountId }),
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([]),
        policies: [document],
        database: follower.database,
        uplink: {
            ...FeedPublisher.of(
                feed,
                (subscription) => store.source.buildReplica(subscription).queries,
            ),
            receive: refuseChanges,
            call: refuseQueries,
        },
        origin: { package: document.package },
    });
    const controller = new AbortController();
    const loop = new ControlLoop(follower.database, store.controllers(), {
        clock: world.clock,
        report: failing,
    });
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const head = await home.database.log.position();
    await Replica.wait(follower.database, Scope.universe.id, head, controller.signal);

    // run only compaction and the replica follow, with no event log
    expect(store.controllers().map((each) => each.name)).toEqual(["compaction", "replica"]);
});

test("replicate the chains of an installation's two served spaces in one replica via both, requiring it to replicate each and refusing a scope outside them", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store an account with two spaces at home, and another account
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { space }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    const declared = defineDatabase({
        name: "installed",
        ...tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows: [SPACES_FOLLOW],
            tables: [],
        }),
    });
    const installed = await TestDatabase.create(declared, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await access.replicateScope(
        account.reference(Scope.universe.id, otherAccountId),
        account.policy,
    );
    for (const id of [spaceId, otherSpaceId]) {
        await new AccessFixture(home.database, world).replicateScope(
            space.reference(accountId, id),
            space.policy,
        );
        await home.database.transaction(
            async (transaction) =>
                await transaction
                    .insert(space.table)
                    .values({ id, scope: accountId, createdAt: 1, updatedAt: 1 }),
        );
    }

    // let the installation read both spaces and replicate the first
    const source = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([space]),
        policies: [document],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: document.package },
    });
    const fixture = new AccessFixture(home.database, world);
    const subject = principal.installation.reference(followingSpaceId, followingId);
    const grant = (id: string, granted: "reader" | "replicator") =>
        fixture.replicateRelationship(space.reference(accountId, id), granted, subject, 1);
    await grant(spaceId, "reader");
    await grant(otherSpaceId, "reader");
    await grant(spaceId, "replicator");

    // replicate the universe's spaces to the installation, decided at home for it
    const follower = { subject, parent: Scope.universe.id };
    const replicating: ObjectStore = await TestObjectStore.at(
        space.reference(Scope.universe.id, spaceId),
        {
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            environment: world,
            principal: spacePrincipal(spaceId),
            servers: serversOf([document], {
                follows: [Follow.of(SPACES, { keys: [{ key: { scope: Scope.universe.id } }] })],
            }),
            policies: [space],
            database: installed.database,
            uplink: {
                receive: refuseChanges,
                call: refuseQueries,
                confirm: (asked, watermark) => source.source.confirm(asked, watermark),
                stream: (asked, signal) =>
                    source.source.replicate(asked, asked.after, follower, signal),
            },
            callKey: testCallKey,
            events: await testEvents(installed.database, world),
            origin: { package: document.package },
        },
    );
    const controller = await followUniverse(replicating, home.database);

    // request the universe's rows and one replica of both spaces' chains via them
    const requests = (
        await replicating.sink.chains(followingSpaceId, Snapshot.live(replicating.database))
    ).map((entry) => entry.subscription);
    const chains = present(
        requests.find((request) => request.name === ObjectSink.viaName(followingSpaceId)),
        "the chains' request",
    );

    // stream the first page of a request at home, refusing the second space until replicable
    const first = async (request: Subscription): Promise<Page | undefined> => {
        const pages = source.source.replicate(request, undefined, follower, controller.signal);
        const page = await pages.next();
        await pages.return(undefined);

        return page.done === true ? undefined : page.value;
    };
    const refused = await refusal(first(chains));
    await grant(otherSpaceId, "replicator");

    // replicate both chains, and refuse a scope between outside them
    const page = present(await first(chains), "the chains' first page");
    const outside = await refusal(
        first({ ...chains, parameters: { ...chains.parameters, between: [otherAccountId] } }),
    );
    expect({
        requests: requests.map((request) => [request.name, request.scope, request.target]),
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
            [ObjectSink.chainName(followingSpaceId), Scope.universe.id, followingSpaceId],
            [ObjectSink.viaName(followingSpaceId), Scope.universe.id, followingSpaceId],
        ],
        parameters: [[spaceId, otherSpaceId], [accountId]],
        refused: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"],
        scopes: new Set([accountId, spaceId, otherSpaceId]),
        outside: [
            "NOT_FOUND",
            "SCOPE_NOT_CONTAINED",
            `${otherAccountId} contains none of the replicated scopes`,
        ],
    });
});

test("refuse a chain follow with no via to a caller without replicate on the scope", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store an account with a space at home
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { space }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => home.close());
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await access.replicateScope(space.reference(accountId, spaceId), space.policy);
    await home.database.transaction(
        async (transaction) =>
            await transaction
                .insert(space.table)
                .values({ id: spaceId, scope: accountId, createdAt: 1, updatedAt: 1 }),
    );
    const source = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([space]),
        policies: [document],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: document.package },
    });

    // request the account's chain below the space as an installation living in it, with no via
    const subject = principal.installation.reference(spaceId, followingId);
    const request = source.source.chainPublication.subscription({
        name: ObjectSink.chainName(spaceId),
        scope: accountId,
        target: spaceId,
        parameters: {
            scopes: [spaceId, accountId, Scope.universe.id],
            local: [],
            types: [],
            via: [],
            between: [],
        },
    });
    const first = async (): Promise<Page | undefined> => {
        const pages = source.source.relayed(
            request,
            subjectContext(subject, spaceId, { environment: world }),
        );
        const page = await pages.next();
        await pages.return(undefined);

        return page.done === true ? undefined : page.value;
    };

    // refuse it until the installation may replicate the space, then replicate the account's chain
    const refused = await refusal(first());
    await access.replicateRelationship(
        space.reference(accountId, spaceId),
        "replicator",
        subject,
        1,
    );
    const page = present(await first(), "the chain's first page");

    // refuse claiming a chain beyond the scopes containing the follower's
    const claiming = source.source.chainPublication.subscription({
        name: ObjectSink.chainName(spaceId),
        scope: accountId,
        target: spaceId,
        parameters: {
            scopes: [spaceId, FOREIGN_ACCOUNT],
            local: [],
            types: [],
            via: [],
            between: [],
        },
    });
    const claimed = await refusal(
        source.source
            .relayed(claiming, subjectContext(subject, spaceId, { environment: world }))
            .next(),
    );
    expect({
        refused,
        claimed,
        scopes: new Set(
            page.changes
                .filter((change) => change.table === Scope.table[TABLE].sqlName)
                .map((change) => change.row["scope"]),
        ),
    }).toEqual({
        refused: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"],
        claimed: [
            "NOT_FOUND",
            "SCOPE_NOT_CONTAINED",
            `${FOREIGN_ACCOUNT} does not contain ${spaceId}`,
        ],
        scopes: new Set([accountId]),
    });
});

test("replicate a partition's chain via its installation's space, whose source has no row of the partition below", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store an account with a space at home, which has no row of the installation's partition
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { space }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => home.close());
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await access.replicateScope(space.reference(accountId, spaceId), space.policy);
    await home.database.transaction(
        async (transaction) =>
            await transaction
                .insert(space.table)
                .values({ id: spaceId, scope: accountId, createdAt: 1, updatedAt: 1 }),
    );
    const source = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([space]),
        policies: [document],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: document.package },
    });

    // request the chain via the space below a partition of the installation living in it
    const subject = principal.installation.reference(spaceId, followingId);
    const partition = "tunnel-01996ab0-0000-7000-8000-000000000007";
    const request = source.source.chainPublication.subscription({
        name: ObjectSink.viaName(partition),
        scope: Scope.universe.id,
        target: partition,
        parameters: { scopes: [], local: [], types: [], via: [spaceId], between: [accountId] },
    });
    const first = async (): Promise<Page | undefined> => {
        const pages = source.source.relayed(
            request,
            subjectContext(subject, spaceId, { environment: world }),
        );
        const page = await pages.next();
        await pages.return(undefined);

        return page.done === true ? undefined : page.value;
    };

    // refuse it until the installation may replicate the space, then replicate the space's chain
    const refused = await refusal(first());
    await access.replicateRelationship(
        space.reference(accountId, spaceId),
        "replicator",
        subject,
        1,
    );
    const page = present(await first(), "the chain's first page");

    // refuse claiming a chain beyond the via scopes' chains
    const claiming = source.source.chainPublication.subscription({
        name: ObjectSink.viaName(partition),
        scope: Scope.universe.id,
        target: partition,
        parameters: {
            scopes: [FOREIGN_ACCOUNT],
            local: [],
            types: [],
            via: [spaceId],
            between: [accountId],
        },
    });
    const claimed = await refusal(
        source.source
            .relayed(claiming, subjectContext(subject, spaceId, { environment: world }))
            .next(),
    );
    expect({
        refused,
        claimed,
        scopes: new Set(
            page.changes
                .filter((change) => change.table === Scope.table[TABLE].sqlName)
                .map((change) => change.row["scope"]),
        ),
    }).toEqual({
        refused: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"],
        claimed: [
            "NOT_FOUND",
            "SCOPE_NOT_CONTAINED",
            `${FOREIGN_ACCOUNT} does not contain ${partition}`,
        ],
        scopes: new Set([accountId, spaceId]),
    });
});

test("request no via chain for the rows a subject's follow brought, which their source decided for that subject", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store a space replicated into an installation's database by a follow on alice's behalf
    const follow = Follow.of(ACTIVITY, { onDemand: { ttl: { minutes: 10 } } });
    const declared = defineDatabase({
        name: "installed",
        ...tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows: [follow],
            tables: [],
        }),
    });
    const installed = await TestDatabase.create(declared, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => installed.close());
    const { database } = installed;
    const alice = principal.user.reference(Scope.universe.id, Identifier.create("user", world));
    await database.transaction(
        async (transaction) =>
            await transaction
                .insert(space.table)
                .values({ id: spaceId, scope: accountId, createdAt: 1, updatedAt: 1 }),
    );
    const name = `activity:${spaceId}`;
    await database.transaction(
        async (transaction) =>
            await transaction.insert(sync.replica).values({
                name,
                scope: spaceId,
                tables: [space.table[TABLE].sqlName],
            }),
    );
    await database.transaction(
        async (transaction) =>
            await transaction.insert(sync.replicaSubscription).values({
                name,
                scope: spaceId,
                ...sync.Replica.subscriptionColumns(
                    {
                        name,
                        publication: "queries",
                        scope: spaceId,
                        target: followingSpaceId,
                        parameters: {},
                        subject: alice,
                    },
                    "caller",
                ),
            }),
    );
    await database.insert(sync.replicaKey).values({
        name,
        scope: spaceId,
        table: space.table[TABLE].sqlName,
        key: Key.name(space.table, { id: spaceId, scope: accountId }),
        concealed: [],
    });

    // list the installation's chains below its space
    const store = await TestObjectStore.at(space.reference(Scope.universe.id, followingSpaceId), {
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        environment: world,
        principal: spacePrincipal(followingSpaceId),
        servers: serversOf([document], { follows: [follow] }),
        policies: [space],
        database,
        uplink: {
            receive: refuseChanges,
            call: refuseQueries,
            stream: async function* () {},
            confirm: async () => {},
        },
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: document.package },
    });
    const vias = (await store.sink.chains(followingSpaceId, Snapshot.live(database)))
        .map((entry) => entry.subscription)
        .filter((subscription) =>
            subscription.name.startsWith(ObjectSink.viaName(followingSpaceId)),
        )
        .map((subscription) => [
            subscription.name,
            subscription.subject,
            subscription.parameters["via"],
        ]);

    // request no via chain, leaving alice's space to its source
    expect(vias).toEqual([]);
});

test("wait before serving for the store's chain replicas, never for its followed shapes, another principal's or an acquired one", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // follow the documents at the universe for the store, the spaces there and a space's activity for alice, and accounts on demand
    const alice = principal.user.reference(Scope.universe.id, Identifier.create("user", world));
    const declaredFollow = Follow.of(DOCUMENTS, { keys: [{ key: { scope: Scope.universe.id } }] });
    const behalf = Follow.of(SPACES, {
        keys: [{ key: { scope: Scope.universe.id }, subject: alice }],
    });
    const elsewhere = Follow.of(ACTIVITY, { keys: [{ key: { scope: spaceId }, subject: alice }] });
    const acquired = Follow.of(ACCOUNTS, { onDemand: { ttl: { minutes: 10 } } });
    const follows = [declaredFollow, behalf, elsewhere, acquired];
    const installed = await TestDatabase.create(
        defineDatabase({
            name: "installed",
            ...tablesOf({
                packages: [objectPackage, { objects: { document }, tables: [] }],
                follows,
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => installed.close());
    const { database } = installed;

    // serve the installation's store, whose publisher refuses every follow, with an accounts key a use acquired
    const store = await TestObjectStore.at(space.reference(Scope.universe.id, followingSpaceId), {
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        environment: world,
        principal: spacePrincipal(followingSpaceId),
        servers: serversOf([document], { follows }),
        policies: [space],
        database,
        publisher: () => ({
            confirm: async () => {},
            stream: async function* () {
                yield* [];
                throw CallError.http("FORBIDDEN", { message: "permission denied: follow" });
            },
        }),
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: { package: document.package },
    });
    await database.insert(followKey).values({
        shape: ACCOUNTS.name,
        scope: Scope.universe.id,
        parameters: canonicalize({}),
        subject: canonicalize(store.principal),
        expiresAt: world.clock.now() + 60_000,
    });

    // gate on the chain alone, listing the store's declared documents and the other three without gating
    const listed = await present(store.subscriber, "the store's subscriber").subscriptions(
        Snapshot.live(database),
    );
    const gates = (await store.sink.gates()).map(({ subscription }) => subscription.name);
    expect([listed.length, gates]).toEqual([5, [ObjectSink.chainName(followingSpaceId)]]);
});

test("serve a store once its chain is confirmed while the source of a followed shape never answers", async () => {
    // insert an account and its space at home, and declare the documents the store follows at the universe
    const simulation = testSimulation();
    const { clock } = simulation;
    const world = simulation.environment("test");
    const home = await TestDatabase.create(accessTables, { environment: world, isMigrated: true });
    const follows = [DOCUMENTS_FOLLOW];
    const installed = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows,
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await access.replicateScope(space.reference(accountId, spaceId), space.policy);

    // serve the space, its chain from home and its followed documents from a source that never answers
    const feed = new Feed(home.database, accessTables, { heartbeat: 3_600_000 });
    const chain = ObjectSink.chainName(spaceId);
    const store: ObjectStore = await TestObjectStore.at(space.reference(accountId, spaceId), {
        chain: async () => ({ kind: "follower", parent: accountId }),
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document], { follows }),
        database: installed.database,
        uplink: {
            ...FeedPublisher.of(
                feed,
                (subscription) => store.source.buildReplica(subscription).queries,
            ),
            async *stream(subscription, signal) {
                if (subscription.name !== chain) {
                    await until(signal);
                }
                yield* FeedPublisher.of(
                    feed,
                    (followed) => store.source.buildReplica(followed).queries,
                ).stream(subscription, signal);
            },
            receive: refuseChanges,
            call: refuseQueries,
        },
        callKey: testCallKey,
        events: await testEvents(installed.database, world),
        origin: { package: document.package },
    });

    // follow both, and serve once the chain is confirmed
    const stopping = new AbortController();
    onTestFinished(() => stopping.abort());
    const following = new ControlLoop(
        installed.database,
        store.controllers().filter((each) => each.name === "replica"),
        { clock, report: failing },
    ).run(stopping.signal);
    const ready = present(
        store.implement(definePackage({ objects: { document } }), [], []).ready,
        "the readiness wait",
    );
    await ready(stopping.signal);
    const followed = new Set(
        (await store.sink.followed()).map(({ subscription }) => subscription.name),
    );
    stopping.abort();
    await following;
    expect([...followed].toSorted()).toEqual(
        [chain, DOCUMENTS.replicaName({ scope: Scope.universe.id })].toSorted(),
    );
});

test("replicate a scoped table an installation's database declares beside its followed space's chain, and drop its rows once the installation may no longer read the space", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store an account with two spaces and a document in each at home
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { space, document }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    const declared = defineDatabase({
        name: "installed",
        ...tablesOf({
            packages: [objectPackage, { objects: { comment }, tables: [] }],
            follows: [DOCUMENTS_FOLLOW],
            tables: [],
        }),
    });
    const installed = await TestDatabase.create(declared, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    for (const id of [spaceId, otherSpaceId]) {
        await access.replicateScope(space.reference(accountId, id), space.policy);
        await home.database.transaction(
            async (transaction) =>
                await transaction
                    .insert(space.table)
                    .values({ id, scope: accountId, createdAt: 1, updatedAt: 1 }),
        );
        await home.database.transaction(
            async (transaction) =>
                await transaction.insert(document.table).values({
                    id: schema
                        .identifier("document")
                        .parse(`document-${id.slice("space-".length)}`),
                    scope: id,
                    title: id,
                    createdAt: 1,
                    updatedAt: 1,
                }),
        );
    }

    // let the installation read and replicate the first space only
    const source = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([space, document]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: document.package },
    });
    const fixture = new AccessFixture(home.database, world);
    const subject = principal.installation.reference(followingSpaceId, followingId);
    const followedSpace = space.reference(accountId, spaceId);
    const reading = await fixture.replicateRelationship(followedSpace, "reader", subject, 1);
    await fixture.replicateRelationship(followedSpace, "replicator", subject, 1);

    // follow the universe's rows and the followed space's chain from home, as the installation
    const follower = { subject, parent: Scope.universe.id };
    const uplink: sync.Uplink = {
        receive: refuseChanges,
        call: refuseQueries,
        confirm: (asked, watermark) => source.source.confirm(asked, watermark),
        stream: (asked, signal) => source.source.replicate(asked, asked.after, follower, signal),
    };
    const following = await testServerContext(comment.package, followingSpaceId, world);
    const replicating: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        environment: world,
        server: {
            ...following,
            installation: schema.identifier("installation").parse(followingId),
            publisher: uplink,
            publisherAt: () => uplink,
        },
        servers: serversOf([comment], {
            follows: [Follow.of(DOCUMENTS, { keys: [{ key: { scope: Scope.universe.id } }] })],
        }),
        policies: [space, document],
        database: installed.database,
        callKey: testCallKey,
        events: await testEvents(installed.database, world),
        origin: { package: document.package },
    });
    const replica = present(
        replicating.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const loop = new ControlLoop(installed.database, [replica], {
        clock: world.clock,
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
        world.clock.timeout(REPLICA_MILLISECONDS),
    );
    const followed = await documents();
    await home.database.transaction(
        async (transaction) =>
            await transaction.delete(accessRelationship).where(eq(accessRelationship.id, reading)),
    );
    await installed.database.log.until(
        async () => (await documents()).length === 0,
        world.clock.timeout(REPLICA_MILLISECONDS),
    );
    expect({ followed, left: await documents() }).toEqual({
        followed: [{ scope: spaceId }],
        left: [],
    });
});

/** The store's replicas, its chain with the grants into its space, as names and scopes. */
const LOCAL_REPLICAS = JSON.stringify(
    [spaceId, accountId, Scope.universe.id]
        .map((scope) => [ObjectSink.chainName(spaceId), scope])
        .toSorted((left, right) => left.join().localeCompare(right.join())),
);

/** List a database's replicas as their names and scopes, to compare with the store's. */
async function localReplicas(database: DatabaseConnection): Promise<string> {
    const replicas = (await Replica.subscriptions(Snapshot.live(database)))
        .map((replica) => [replica.name, replica.scope])
        .toSorted((left, right) => left.join().localeCompare(right.join()));

    return JSON.stringify(replicas);
}

/** Fail a test on a failed reconciliation. */
function failing(_controller: unknown, key: string, error: unknown): never {
    throw new Error(`reconciling ${key} failed`, { cause: error });
}

/** Read the scope a replica controller's key follows. */
function readScope(key: string): string | undefined {
    return key.split(" ")[1];
}

test("replicate the chains of the accounts an installation's rows relate to, beside its space where those rows live", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store two accounts at home, the first readable by the installation
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { account }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    const declared = defineDatabase({
        name: "installed",
        ...tablesOf({
            packages: [objectPackage, { objects: { annotation }, tables: [] }],
            follows: [ACCOUNTS_FOLLOW],
            tables: [],
        }),
    });
    const installed = await TestDatabase.create(declared, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database, world);
    for (const id of [accountId, otherAccountId]) {
        await access.replicateScope(account.reference(Scope.universe.id, id), account.policy);
        await home.database.transaction(
            async (transaction) =>
                await transaction
                    .insert(account.table)
                    .values({ id, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 }),
        );
    }
    const subject = principal.installation.reference(followingSpaceId, followingId);
    await access.replicateRelationship(
        account.reference(Scope.universe.id, accountId),
        "reader",
        subject,
        1,
    );

    // replicate the universe's accounts to the installation, decided at home for it
    const source = await TestObjectStore.at(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([account]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: account.package },
    });
    const follower = { subject, parent: Scope.universe.id };
    const replicating: ObjectStore = await TestObjectStore.at(
        space.reference(Scope.universe.id, followingSpaceId),
        {
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            environment: world,
            principal: spacePrincipal(followingSpaceId),
            servers: serversOf([annotation], {
                follows: [Follow.of(ACCOUNTS, { keys: [{ key: { scope: Scope.universe.id } }] })],
            }),
            policies: [account],
            database: installed.database,
            uplink: {
                receive: refuseChanges,
                call: refuseQueries,
                confirm: (asked, watermark) => source.source.confirm(asked, watermark),
                stream: (asked, signal) =>
                    source.source.replicate(asked, asked.after, follower, signal),
            },
            callKey: testCallKey,
            events: await testEvents(installed.database, world),
            origin: { package: annotation.package },
        },
    );
    await followUniverse(replicating, home.database);

    // request one replica of the readable account's chain and the installation's space
    const requests = (
        await replicating.sink.chains(followingSpaceId, Snapshot.live(replicating.database))
    ).map((entry) => entry.subscription);
    const chains = present(
        requests.find((request) => request.name === ObjectSink.viaName(followingSpaceId)),
        "the chains' request",
    );
    expect({
        requests: requests.map((request) => [request.name, request.scope, request.target]),
        via: chains.parameters["via"],
    }).toEqual({
        requests: [
            [ObjectSink.chainName(followingSpaceId), followingSpaceId, followingSpaceId],
            [ObjectSink.chainName(followingSpaceId), Scope.universe.id, followingSpaceId],
            [ObjectSink.viaName(followingSpaceId), Scope.universe.id, followingSpaceId],
        ],
        via: [accountId],
    });
});

test("replicate the chain of an account another replica brought first, once the universe replica includes it", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store an account at home that the installation reads
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { account }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    const declared = defineDatabase({
        name: "installed",
        ...tablesOf({
            packages: [objectPackage, { objects: { annotation }, tables: [] }],
            follows: [ACCOUNTS_FOLLOW],
            tables: [],
        }),
    });
    const installed = await TestDatabase.create(declared, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database, world);
    await access.replicateScope(account.reference(Scope.universe.id, accountId), account.policy);
    await home.database.transaction(
        async (transaction) =>
            await transaction
                .insert(account.table)
                .values({ id: accountId, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 }),
    );
    const subject = principal.installation.reference(followingSpaceId, followingId);
    await access.replicateRelationship(
        account.reference(Scope.universe.id, accountId),
        "reader",
        subject,
        1,
    );

    // serve the accounts at home, and record the first chain request via the account
    const source = await TestObjectStore.at(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([account]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: account.package },
    });
    const follower = { subject, parent: Scope.universe.id };
    const chain = ObjectSink.viaName(followingSpaceId);
    const followed = Promise.withResolvers<unknown>();
    const publisher = {
        confirm: (asked: Subscription, watermark: Watermark) =>
            source.source.confirm(asked, watermark),
        receive: refuseChanges,
        call: refuseQueries,
        stream: async function* (asked: Subscription, signal: AbortSignal) {
            // stream the universe's rows from home
            if (asked.name !== chain) {
                yield* source.source.replicate(asked, asked.after, follower, signal);
            }
            // record a chain request via the account and leave it open
            else {
                const via = asked.parameters["via"];
                if (Array.isArray(via) && via.includes(accountId)) {
                    followed.resolve(via);
                }
                await until(signal);
            }
        },
    };

    // replicate the universe's rows under another name first, as a residents replica does
    const replicating: ObjectStore = await TestObjectStore.at(
        space.reference(Scope.universe.id, followingSpaceId),
        {
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            environment: world,
            principal: spacePrincipal(followingSpaceId),
            servers: serversOf([annotation], { follows: [ACCOUNTS_FOLLOW] }),
            policies: [account],
            database: installed.database,
            uplink: publisher,
            callKey: testCallKey,
            events: await testEvents(installed.database, world),
            origin: { package: annotation.package },
        },
    );
    const residents = {
        ...ACCOUNTS.subscription({ scope: Scope.universe.id }, followingSpaceId),
        name: "residents",
    };
    const early = new AbortController();
    const replicated = replicating.source
        .buildReplica(residents)
        .follow(
            installed.database,
            (from, signal) => publisher.stream({ ...residents, ...from }, signal),
            early.signal,
            {
                subscription: residents,
                audience: replicating.source.queriesPublication.audience,
                synced: () => {},
            },
        );
    const head = await home.database.log.position();
    await Replica.wait(
        installed.database,
        Scope.universe.id,
        head,
        world.clock.timeout(REPLICA_MILLISECONDS),
    );
    early.abort();
    await Promise.allSettled([replicated]);

    // run the replicas, whose universe replica then only records the account it shares with the other replica
    const controller = new AbortController();
    const loop = new ControlLoop(
        installed.database,
        replicating.controllers().filter((each) => each.name === "replica"),
        { clock: world.clock, report: failing },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });

    // list the replicas again and follow the chain via the account
    const timeout = until(world.clock.timeout(REPLICA_MILLISECONDS)).then(
        () => "no chain via the account",
    );
    expect(await Promise.race([followed.promise, timeout])).toEqual([accountId]);
});

/** Follow a store's replicas of the universe until the test finishes. */
async function followUniverse(
    replicating: ObjectStore,
    home: DatabaseConnection,
): Promise<AbortController> {
    // follow the store's replicas of the universe's rows, and of the grants delegated into it
    const replica = present(
        replicating.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const keys = (await replica.list(Snapshot.live(replicating.database))).filter(
        (each) => readScope(each) === Scope.universe.id,
    );
    const following = keys.map((key) => replica.reconcile(key, reconciliation(controller.signal)));
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(following);
    });

    // wait for it to arrive at the home's head
    const head = await home.log.position();
    await Replica.wait(replicating.database, Scope.universe.id, head, controller.signal);

    return controller;
}

test("require replicate via a scope for an in-process follower inside the scopes its row places it, as for a remote one, and refuse the store's principal", async () => {
    // store a region with a machine standing inside it, which the region lets its machines replicate
    const world = testSimulation().environment("test");
    const region = defineObject({
        name: "region",
        plural: "regions",
        scope: "universe",
        isScope: true,
        fields: {},
        permissions: { read: none(), replicate: contained(principal.machine) },
    });
    const machine = defineObject({
        name: "machine",
        plural: "machines",
        scope: region,
        represents: principal.machine,
        fields: {},
        permissions: { read: none() },
    });
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { region, machine }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => home.close());
    const regionId = schema
        .identifier("region")
        .parse("region-01996ab0-0000-7000-8000-00000000000a");
    const machineId = schema
        .identifier("machine")
        .parse("machine-01996ab0-0000-7000-8000-00000000000b");
    const fixture = new AccessFixture(home.database, world);
    await fixture.replicateScope(region.reference(Scope.universe.id, regionId), region.policy);
    await home.database.transaction(
        async (transaction) =>
            await transaction
                .insert(machine.table)
                .values({ id: machineId, scope: regionId, createdAt: 1, updatedAt: 1 }),
    );
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, otherSpaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(otherSpaceId),
        servers: serversOf([space]),
        policies: [region, machine],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: space.package },
    });

    // decide for the machine calling in process and remotely, and for the store's own principal
    const now = world.clock.now();
    const inside = principal.machine.reference(Scope.universe.id, machineId);
    const decide = (context: ReturnType<typeof AccessContext.of>) =>
        refusal(store.source.requireReplicateVia(regionId, context));
    expect({
        local: await decide(AccessContext.of(Caller.of(inside), now)),
        remote: await decide({
            ...AccessContext.of(Caller.of(inside), now),
            credential: {
                kind: "fixture",
                id: "fixture-1",
                audience: region.package.id,
                verifiedAt: now,
                expiresAt: now + 60_000,
            },
        }),
        own: await decide(AccessContext.of(Caller.of(store.principal), now)),
    }).toEqual({
        local: "done",
        remote: "done",
        own: ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: replicate"],
    });
});
