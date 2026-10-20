import { VirtualClock } from "@destack/environment/simulation";
import { reconciliation, subjectContext, testCallKey } from "@destack/runtime/test";
import { callTables, FeedPublisher, testEvents, serversOf } from "../src/test/index.ts";
import { ControlLoop } from "@destack/runtime/control";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    accessGrant,
    accessTables,
    accessRelationship,
    none,
    principal,
    relation,
    Relationship,
    through,
    union,
} from "@destack/access";

import { AccessFixture } from "@destack/access/test";
import {
    type DatabaseConnection,
    defineDatabase,
    eq,
    Key,
    lease,
    ReadSet,
    Snapshot,
    TABLE,
} from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Identifier, nth, present, schema } from "@destack/schema";

import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { until } from "@destack/runtime/timer";
import { Scope, Feed, Replica, type Page, type Subscription, type Watermark } from "@destack/sync";
import * as sync from "@destack/sync";
import { v7 } from "uuid";
import { defineObject, defineShape, field, Follow } from "../src/index.ts";
import { ObjectSink, ObjectStore } from "../src/server/index.ts";
import { userContext, users } from "./fixture/user.ts";
import { spacePrincipal } from "./fixture/space.ts";
import { Clock } from "@destack/environment";

/** The source rows the incremental listing test follows: a thousand. */
const STRESS_ROWS = 1000;

/** How long a replica may take to reach its home or relist: 5 s, far above its milliseconds. */
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

test.each(TEST_DIALECTS)(
    "decide calls over a relayed replica of access as the holder does, through the account's members and restricted credentials, on %s",
    async (dialect) => {
        const home = await TestDatabase.create(dialect, accessTables, { isMigrated: true });
        const installed = await TestDatabase.create(dialect, [...document.tables, ...callTables], {
            isMigrated: true,
        });
        onTestFinished(async () => {
            await Promise.all([home.close(), installed.close()]);
        });

        // insert an account owned by alice, a member role carol has through the account, and its empty space at home
        const alice = principal.user.reference("universe", users.alice);
        const carol = principal.user.reference("universe", users.carol);
        const accountObject = account.reference(Scope.universe.id, accountId);
        const access = new AccessFixture(home.database);
        await access.replicateScope(accountObject);
        await access.replicateScope(space.reference(accountId, spaceId), "own");
        await access.bindOwner(accountObject, alice);
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

        // serve the documents of one space over the installation's database to each call's user, replicating its chain from the home database
        const feed = new Feed(home.database, accessTables, { clock: Clock.system });
        const store: ObjectStore = new ObjectStore({
            chain: async () => ({ kind: "follower", parent: accountId }),
            clock: Clock.system,
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
            // follow each scope's chain, which at the universe has the grants delegated in
            const keys = (await follower.list(Snapshot.live(store.database))).filter(
                (entry) => readScope(entry) === scope,
            );
            for (const key of keys) {
                following.push(
                    follower
                        .reconcile(key, reconciliation(controller.signal))
                        .then(() => undefined),
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
                    : await new AccessFixture(installed.database).replicateGrant(
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
                    ? {}
                    : { actors: [token], grants: [{ id: grantId, scope: user }] },
            );

            const created = store
                .mutate(context.access(), {
                    id: RequestId.create(Date.now()),
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
            userContext(users.alice, spaceId).access(),
            document.call("list", { spaceId }),
        );

        // allow the owner through a credential for the empty space and hide the space from the rest
        const hidden = ["NOT_FOUND", `no scope ${spaceId}`];
        expect(outcomes).toEqual(["Plan", hidden, hidden, hidden, "Budget"]);
        expect(listed.items.map((item) => item.title).toSorted()).toEqual(["Budget", "Plan"]);
    },
);

/** An account outside every chain the tests' followers have. */
const FOREIGN_ACCOUNT = "account-01996ab0-0000-7000-8000-0000000000ff";

test("record the reads of a replica listing, so only a change of a followed source row touches it", async () => {
    // follow another space's annotations for each document titled to ask for them
    const machine = await TestDatabase.create(
        "sqlite",
        [...document.tables, ...annotation.tables, ...callTables, lease],
        { isMigrated: true },
    );
    onTestFinished(() => machine.close());
    const annotations = defineShape({
        name: "annotations",
        parameters: schema.object({}),
        objects: [annotation],
        queries: () => [annotation.query()],
    });
    const store = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        clock: Clock.system,
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
        events: await testEvents(machine.database),
        origin: { package: document.package },
    });
    const id = Identifier.create("document");
    await machine.database
        .insert(document.table)
        .values({ id, scope: spaceId, title: "Follow", createdAt: 1, updatedAt: 1 });
    const replica = present(
        store.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const reads = new ReadSet();
    await replica.list(Snapshot.live(machine.database).recording(reads));

    // add a document the follow leaves out, then retitle the followed one out of the follow
    const after = (await machine.database.log.position()).sequence;
    await machine.database.insert(document.table).values({
        id: Identifier.create("document"),
        scope: spaceId,
        title: "Draft",
        createdAt: 1,
        updatedAt: 1,
    });
    await machine.database
        .update(document.table)
        .set({ title: "Other" })
        .where(eq(document.table.id, id));
    const changes = (await machine.database.log.read({ tables: [document.table], after })).changes;

    // touch the listing with the retitle alone
    expect(changes.map((change) => [change.operation, reads.touches(change)])).toEqual([
        ["insert", false],
        ["update", true],
    ]);
});

test("read a row follow's keys again only for the source rows a change touched, across many rows", async () => {
    // follow another space's annotations for each document, counting the key reads
    const machine = await TestDatabase.create(
        "sqlite",
        [...document.tables, ...annotation.tables, ...callTables, lease],
        { isMigrated: true },
    );
    onTestFinished(() => machine.close());
    const annotations = defineShape({
        name: "annotations",
        parameters: schema.object({}),
        objects: [annotation],
        queries: () => [annotation.query()],
    });
    let keyReads = 0;
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
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
        events: await testEvents(machine.database),
        origin: { package: document.package },
    });
    const ids = Array.from({ length: STRESS_ROWS }, () => Identifier.create("document"));
    await machine.database
        .insert(document.table)
        .values(
            ids.map((id) => ({ id, scope: spaceId, title: "Notes", createdAt: 1, updatedAt: 1 })),
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
        machine.database
            .update(document.table)
            .set({ title: "Plans" })
            .where(eq(document.table.id, nth(ids, 0))),
    );
    const added = await relist(() =>
        machine.database.insert(document.table).values({
            id: Identifier.create("document"),
            scope: spaceId,
            title: "Added",
            createdAt: 1,
            updatedAt: 1,
        }),
    );
    const peered = await relist(() =>
        machine.database.insert(document.table).values({
            id: Identifier.create("document"),
            scope: spaceId,
            title: `peer of ${nth(ids, 1)}`,
            createdAt: 1,
            updatedAt: 1,
        }),
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
    // follow nothing for any document, with no chain above the store
    const machine = await TestDatabase.create(
        "sqlite",
        [...document.tables, ...callTables, lease],
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => machine.close());
    const annotations = defineShape({
        name: "annotations",
        parameters: schema.object({}),
        objects: [annotation],
        queries: () => [annotation.query()],
    });
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
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
        events: await testEvents(machine.database),
        origin: { package: document.package },
    });

    // list the replicas for readiness itself, finding none to wait for
    const ready = present(store.packages[0]?.ready, "the package's readiness");
    const waited = AbortSignal.timeout(1000);
    await ready(waited);

    // become ready before the wait runs out, following nothing
    expect({ isTimedOut: waited.aborted, followed: await store.sink.followed() }).toEqual({
        isTimedOut: false,
        followed: [],
    });
});

test("report a follow its source refuses, and drop its replica once a change of the rows the follow reads stops requesting it", async () => {
    // store an account and its two spaces at home, and a document of the first space on a machine
    const home = await TestDatabase.create("sqlite", [...accessTables, ...document.tables], {
        isMigrated: true,
    });
    const machine = await TestDatabase.create(
        "sqlite",
        [...document.tables, ...callTables, lease],
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), machine.close()]);
    });
    const access = new AccessFixture(home.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(space.reference(accountId, spaceId), "own");
    await access.replicateScope(space.reference(accountId, otherSpaceId), "own");
    const requesting = Identifier.create("document");
    await machine.database.insert(document.table).values({
        id: requesting,
        scope: spaceId,
        title: "Notes",
        createdAt: 1,
        updatedAt: 1,
    });

    // follow the other space's documents beside the store's own chain while a document asks, failing a refused scope's streams
    const feed = new Feed(home.database, [...accessTables, ...document.tables], {
        clock: Clock.system,
    });
    const refusals = new Map<string, AbortController>();
    const closing = (scope: string) => {
        const refused = refusals.get(scope) ?? new AbortController();
        refusals.set(scope, refused);

        return refused;
    };
    const store: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: accountId }),
        clock: Clock.system,
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
        events: await testEvents(machine.database),
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
            clock: new VirtualClock(Date.now()),
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
    const refused = new CallError("FORBIDDEN", { message: "refused" });
    closing(otherSpaceId).abort(refused);
    await expect.poll(() => failures).toEqual([refused]);

    // stop asking for them, which drops their replica
    await machine.database
        .update(document.table)
        .set({ title: "Archived" })
        .where(eq(document.table.id, requesting));
    await machine.database.log.until(
        async () => (await ownReplicas(machine.database)) === OWN_REPLICAS,
        controller.signal,
    );
    expect(failures).toEqual([refused]);
});

test("replicate a person's grant to an installation of the space from the universe into the space", async () => {
    // keep alice's grant to an installation of the space, and another one's to an installation elsewhere, at home
    const home = await TestDatabase.create("sqlite", accessTables, { isMigrated: true });
    const machine = await TestDatabase.create(
        "sqlite",
        [...document.tables, ...callTables, lease],
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), machine.close()]);
    });
    const access = new AccessFixture(home.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(space.reference(accountId, spaceId), "own");
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
    const feed = new Feed(home.database, accessTables, { clock: Clock.system });
    const store: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: accountId }),
        clock: Clock.system,
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
        events: await testEvents(machine.database),
        origin: { package: document.package },
    });
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
    await Replica.wait(machine.database, Scope.universe.id, head, controller.signal);

    // keep only the grant to the space's installation
    const kept = await machine.database.select({ id: accessGrant.id }).from(accessGrant);
    expect(kept.map((row) => row.id)).toEqual([granted]);
});

test("follow replicates as a server without methods, journal or call key", async () => {
    // store an account and its space at home, and a database holding access replicas
    const home = await TestDatabase.create("sqlite", accessTables, { isMigrated: true });
    const follower = await TestDatabase.create("sqlite", accessTables, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), follower.close()]);
    });
    const access = new AccessFixture(home.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(space.reference(accountId, spaceId), "own");

    // follow the universe's rows the space reads, without a journal
    const feed = new Feed(home.database, accessTables, { clock: Clock.system });
    const store: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: accountId }),
        clock: Clock.system,
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
        clock: Clock.system,
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
    // store an account with two spaces at home, and another account
    const home = await TestDatabase.create("sqlite", [...space.tables, ...callTables], {
        isMigrated: true,
    });
    const declared = defineDatabase({
        name: "installed",
        tables: [...document.tables, ...callTables],
        replicated: [space.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(account.reference(Scope.universe.id, otherAccountId));
    for (const id of [spaceId, otherSpaceId]) {
        await new AccessFixture(home.database).replicateScope(
            space.reference(accountId, id),
            "own",
        );
        await home.database
            .insert(space.table)
            .values({ id, scope: accountId, createdAt: 1, updatedAt: 1 });
    }

    // let the installation read both spaces and replicate the first
    const source = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([space]),
        policies: [document],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: document.package },
    });
    const fixture = new AccessFixture(home.database);
    const subject = principal.installation.reference(followingSpaceId, followingId);
    const grant = (id: string, granted: "reader" | "replicator") =>
        fixture.replicateRelationship(id, space.reference(accountId, id), granted, subject, 1);
    await grant(spaceId, "reader");
    await grant(otherSpaceId, "reader");
    await grant(spaceId, "replicator");

    // replicate the universe's spaces to the installation, decided at home for it
    const follower = { subject, parent: Scope.universe.id };
    const replicating: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document]),
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
        events: await testEvents(installed.database),
        origin: { package: document.package },
    });
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
            [ObjectSink.queriesName(followingSpaceId), Scope.universe.id, followingSpaceId],
            [ObjectSink.viaName(followingSpaceId), Scope.universe.id, followingSpaceId],
        ],
        parameters: [[spaceId, otherSpaceId], [accountId]],
        refused: ["FORBIDDEN", "permission denied: replicate"],
        scopes: new Set([accountId, spaceId, otherSpaceId]),
        outside: ["NOT_FOUND", `${otherAccountId} contains none of the replicated scopes`],
    });
});

test("refuse a chain follow with no via to a caller without replicate on the scope", async () => {
    // store an account with a space at home
    const home = await TestDatabase.create("sqlite", [...space.tables, ...callTables], {
        isMigrated: true,
    });
    onTestFinished(() => home.close());
    const access = new AccessFixture(home.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(space.reference(accountId, spaceId), "own");
    await home.database
        .insert(space.table)
        .values({ id: spaceId, scope: accountId, createdAt: 1, updatedAt: 1 });
    const source = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([space]),
        policies: [document],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
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
        const pages = source.source.relayed(request, subjectContext(subject, spaceId));
        const page = await pages.next();
        await pages.return(undefined);

        return page.done === true ? undefined : page.value;
    };

    // refuse it until the installation may replicate the space, then replicate the account's chain
    const refused = await refusal(first());
    await access.replicateRelationship(
        spaceId,
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
        source.source.relayed(claiming, subjectContext(subject, spaceId)).next(),
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
        refused: ["FORBIDDEN", "permission denied: replicate"],
        claimed: ["NOT_FOUND", `${FOREIGN_ACCOUNT} does not contain ${spaceId}`],
        scopes: new Set([accountId]),
    });
});

test("replicate a partition's chain via its installation's space, which the source holds no row of the partition below", async () => {
    // store an account with a space at home, which holds no row of the installation's partition
    const home = await TestDatabase.create("sqlite", [...space.tables, ...callTables], {
        isMigrated: true,
    });
    onTestFinished(() => home.close());
    const access = new AccessFixture(home.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(space.reference(accountId, spaceId), "own");
    await home.database
        .insert(space.table)
        .values({ id: spaceId, scope: accountId, createdAt: 1, updatedAt: 1 });
    const source = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([space]),
        policies: [document],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
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
        const pages = source.source.relayed(request, subjectContext(subject, spaceId));
        const page = await pages.next();
        await pages.return(undefined);

        return page.done === true ? undefined : page.value;
    };

    // refuse it until the installation may replicate the space, then replicate the space's chain
    const refused = await refusal(first());
    await access.replicateRelationship(
        spaceId,
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
        source.source.relayed(claiming, subjectContext(subject, spaceId)).next(),
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
        refused: ["FORBIDDEN", "permission denied: replicate"],
        claimed: ["NOT_FOUND", `${FOREIGN_ACCOUNT} does not contain ${partition}`],
        scopes: new Set([accountId, spaceId]),
    });
});

test("request no via chain for the rows a subject's follow brought, which their source decided for that subject", async () => {
    // store a space replicated into an installation's database by a follow on alice's behalf
    const declared = defineDatabase({
        name: "installed",
        tables: [...document.tables, ...callTables],
        replicated: [space.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(() => installed.close());
    const { database } = installed;
    const alice = principal.user.reference(Scope.universe.id, Identifier.create("user"));
    await database
        .insert(space.table)
        .values({ id: spaceId, scope: accountId, createdAt: 1, updatedAt: 1 });
    const name = `activity:${spaceId}`;
    await database
        .insert(sync.replica)
        .values({ name, scope: spaceId, tables: [space.table[TABLE].sqlName] });
    await database.insert(sync.replicaSubscription).values({
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
    });
    await database.insert(sync.replicaKey).values({
        name,
        scope: spaceId,
        table: space.table[TABLE].sqlName,
        key: Key.name(space.table, { id: spaceId, scope: accountId }),
        concealed: [],
    });

    // list the installation's chains below its own space
    const store = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        servers: serversOf([document]),
        policies: [space],
        database,
        uplink: {
            receive: refuseChanges,
            call: refuseQueries,
            stream: async function* () {},
            confirm: async () => {},
        },
        callKey: testCallKey,
        events: await testEvents(database),
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

test("serve a store whose on-behalf follow is refused, without gating its readiness", async () => {
    // store a space replicated into an installation's database by a best-effort follow on alice's behalf
    const declared = defineDatabase({
        name: "installed",
        tables: [...document.tables, ...callTables],
        replicated: [space.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(() => installed.close());
    const { database } = installed;
    const alice = principal.user.reference(Scope.universe.id, Identifier.create("user"));
    const activity = defineShape({
        name: "activity",
        parameters: schema.object({}),
        objects: [document],
        queries: () => [document.query()],
    });
    const follow = Follow.of(activity, {
        keys: [{ key: { scope: spaceId }, subject: alice }],
        isReadinessGate: false,
    });
    const brought = activity.subscription({ scope: spaceId }, followingSpaceId, alice);
    await database
        .insert(space.table)
        .values({ id: spaceId, scope: accountId, createdAt: 1, updatedAt: 1 });
    await database
        .insert(sync.replica)
        .values({ name: brought.name, scope: spaceId, tables: [space.table[TABLE].sqlName] });
    await database.insert(sync.replicaSubscription).values({
        name: brought.name,
        scope: spaceId,
        ...sync.Replica.subscriptionColumns(brought, "caller"),
    });
    await database.insert(sync.replicaKey).values({
        name: brought.name,
        scope: spaceId,
        table: space.table[TABLE].sqlName,
        key: Key.name(space.table, { id: spaceId, scope: accountId }),
        concealed: [],
    });

    // serve the installation's store, whose publisher refuses every follow
    const store = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        servers: serversOf([document], { follows: [follow] }),
        policies: [space],
        database,
        publisher: () => ({
            confirm: async () => {},
            stream: async function* () {
                yield* [];
                throw new CallError("FORBIDDEN", { message: "permission denied: follow" });
            },
        }),
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: document.package },
    });
    const subscriber = present(store.subscriber, "the store's subscriber");
    const gates = (await subscriber.subscriptions(Snapshot.live(database)))
        .filter(({ subscription }) => subscription.subject !== undefined)
        .map(({ subscription, isReadinessGate }) => [subscription.name, isReadinessGate]);

    // leave the best-effort follow ungated
    expect(gates).toEqual([[brought.name, false]]);
});

test("replicate a scoped table an installation's database declares beside its followed space's chain, and drop its rows once the installation may no longer read the space", async () => {
    // store an account with two spaces and a document in each at home
    const home = await TestDatabase.create(
        "sqlite",
        [...space.tables, ...document.tables, ...callTables],
        { isMigrated: true },
    );
    const declared = defineDatabase({
        name: "installed",
        tables: [...comment.tables, ...callTables, lease],
        replicated: [space.table, document.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    for (const id of [spaceId, otherSpaceId]) {
        await access.replicateScope(space.reference(accountId, id), "own");
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
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([space, document]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: document.package },
    });
    const fixture = new AccessFixture(home.database);
    const subject = principal.installation.reference(followingSpaceId, followingId);
    const followedSpace = space.reference(accountId, spaceId);
    const reading = await fixture.replicateRelationship(
        spaceId,
        followedSpace,
        "reader",
        subject,
        1,
    );
    await fixture.replicateRelationship(spaceId, followedSpace, "replicator", subject, 1);

    // follow the universe's rows and the followed space's chain from home, as the installation
    const follower = { subject, parent: Scope.universe.id };
    const replicating: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        clock: Clock.system,
        principal: subject,
        servers: serversOf([comment]),
        policies: [space, document],
        database: installed.database,
        uplink: {
            receive: refuseChanges,
            call: refuseQueries,
            confirm: (asked, watermark) => source.source.confirm(asked, watermark),
            stream: (asked, signal) =>
                source.source.replicate(asked, asked.after, follower, signal),
        },
        callKey: testCallKey,
        events: await testEvents(installed.database),
        origin: { package: document.package },
    });
    const replica = present(
        replicating.controllers().find((each) => each.name === "replica"),
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
        AbortSignal.timeout(REPLICA_MILLISECONDS),
    );
    const followed = await documents();
    await home.database.delete(accessRelationship).where(eq(accessRelationship.id, reading));
    await installed.database.log.until(
        async () => (await documents()).length === 0,
        AbortSignal.timeout(REPLICA_MILLISECONDS),
    );
    expect({ followed, left: await documents() }).toEqual({
        followed: [{ scope: spaceId }],
        left: [],
    });
});

/** The store's own replicas, its chain with the grants into its space, as names and scopes. */
const OWN_REPLICAS = JSON.stringify(
    [spaceId, accountId, Scope.universe.id]
        .map((scope) => [ObjectSink.chainName(spaceId), scope])
        .toSorted((left, right) => left.join().localeCompare(right.join())),
);

/** List a database's replicas as their names and scopes, to compare with the own ones. */
async function ownReplicas(database: DatabaseConnection): Promise<string> {
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

test("replicate the chains of the accounts an installation's rows relate to, beside its own space where those rows live", async () => {
    // store two accounts at home, the first readable by the installation
    const home = await TestDatabase.create("sqlite", [...account.tables, ...callTables], {
        isMigrated: true,
    });
    const declared = defineDatabase({
        name: "installed",
        tables: [...annotation.tables, ...callTables],
        replicated: [account.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database);
    for (const id of [accountId, otherAccountId]) {
        await access.replicateScope(account.reference(Scope.universe.id, id));
        await home.database
            .insert(account.table)
            .values({ id, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 });
    }
    const subject = principal.installation.reference(followingSpaceId, followingId);
    await access.replicateRelationship(
        accountId,
        account.reference(Scope.universe.id, accountId),
        "reader",
        subject,
        1,
    );

    // replicate the universe's accounts to the installation, decided at home for it
    const source = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([account]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: account.package },
    });
    const follower = { subject, parent: Scope.universe.id };
    const replicating: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        servers: serversOf([annotation]),
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
        events: await testEvents(installed.database),
        origin: { package: annotation.package },
    });
    await followUniverse(replicating, home.database);

    // request one replica of the readable account's chain and the installation's own space
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
            [ObjectSink.queriesName(followingSpaceId), Scope.universe.id, followingSpaceId],
            [ObjectSink.viaName(followingSpaceId), Scope.universe.id, followingSpaceId],
        ],
        via: [accountId],
    });
});

test("replicate the chain of an account another replica brought first, once the universe replica includes it", async () => {
    // store an account at home that the installation reads
    const home = await TestDatabase.create("sqlite", [...account.tables, ...callTables], {
        isMigrated: true,
    });
    const declared = defineDatabase({
        name: "installed",
        tables: [...annotation.tables, ...callTables],
        replicated: [account.table],
    });
    const installed = await TestDatabase.create("sqlite", declared, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), installed.close()]);
    });
    const access = new AccessFixture(home.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await home.database
        .insert(account.table)
        .values({ id: accountId, scope: Scope.universe.id, createdAt: 1, updatedAt: 1 });
    const subject = principal.installation.reference(followingSpaceId, followingId);
    await access.replicateRelationship(
        accountId,
        account.reference(Scope.universe.id, accountId),
        "reader",
        subject,
        1,
    );

    // serve the accounts at home, and record the first chain request via the account
    const source = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([account]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
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
            // record a chain request via the account and hold it open
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
    const replicating: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        servers: serversOf([annotation]),
        policies: [account],
        database: installed.database,
        uplink: publisher,
        callKey: testCallKey,
        events: await testEvents(installed.database),
        origin: { package: annotation.package },
    });
    const listed = await replicating.sink.chains(
        followingSpaceId,
        Snapshot.live(installed.database),
    );
    const residents = {
        ...present(
            listed.find((entry) => entry.subscription.publication === "queries")?.subscription,
            "the universe request",
        ),
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
                clock: Clock.system,
            },
        );
    const head = await home.database.log.position();
    await Replica.wait(
        installed.database,
        Scope.universe.id,
        head,
        AbortSignal.timeout(REPLICA_MILLISECONDS),
    );
    early.abort();
    await Promise.allSettled([replicated]);

    // run the replicas, whose universe replica then only records the account it shares with the other replica
    const controller = new AbortController();
    const loop = new ControlLoop(
        installed.database,
        replicating.controllers().filter((each) => each.name === "replica"),
        { clock: Clock.system, report: failing },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });

    // list the replicas again and follow the chain via the account
    const timeout = until(AbortSignal.timeout(REPLICA_MILLISECONDS)).then(
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

    // wait for it to reach the home's head
    const head = await home.log.position();
    await Replica.wait(replicating.database, Scope.universe.id, head, controller.signal);

    return controller;
}
