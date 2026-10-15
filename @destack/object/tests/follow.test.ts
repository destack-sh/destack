import { reconciliation, subjectContext, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";
import { ControlLoop, controllerLease } from "@destack/runtime/control";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    accessTables,
    accessRelationship,
    Authorizer,
    none,
    principal,
    relation,
    Relationship,
    through,
    union,
} from "@destack/access";

import { AccessFixture } from "@destack/access/test";
import { type DatabaseConnection, defineDatabase, eq, Key, TABLE } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Clock, Identifier, present, schema } from "@destack/schema";

import { CallError } from "@destack/runtime/error";
import { RequestId } from "@destack/runtime/request";
import { until } from "@destack/runtime/timer";
import { Scope, Feed, Replica, Subject, type Page, type Subscription } from "@destack/sync";
import * as sync from "@destack/sync";
import { v7 } from "uuid";
import { defineObject, defineShape, field, Follow, Shape } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { userContext } from "./fixture/user.ts";
import { spacePrincipal } from "./fixture/space.ts";

/** How long a replica may take to reach its home or relist: 5 s, far above its milliseconds. */
const REPLICA_MILLISECONDS = 5000;

/** Refuse a change sent over the uplink, as these replicas receive none. */
const refuseChanges = () => Promise.reject(new Error("the fixture receives no changes"));

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
        const alice = principal.user.reference("universe", "alice");
        const carol = principal.user.reference("universe", "carol");
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
            chain: async () => ({ kind: "follower" }),
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: [document],
            database: installed.database,
            subscriber: {
                uplink: {
                    receive: refuseChanges,
                    stream: (subscription, signal) =>
                        feed.subscribe(
                            store.source.buildReplica(subscription).queries,
                            subscription.after,
                            signal,
                        ),
                },
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
            const key = present(
                (await follower.list()).find((entry) => readScope(entry) === scope),
                `the replica key of ${scope}`,
            );
            following.push(
                follower.reconcile(key, reconciliation(controller.signal)).then(() => undefined),
            );
            await Replica.wait(installed.database, scope, head, controller.signal);
        }
        expect((await follower.list()).map(readScope)).toEqual([
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

test("drop a replica once its source refuses its follow and no follow names it any longer, without a change of the rows the follow reads", async () => {
    // keep an account and its two spaces at home, and a document of the first space on a machine
    const home = await TestDatabase.create("sqlite", [...accessTables, ...document.tables], {
        isMigrated: true,
    });
    const machine = await TestDatabase.create(
        "sqlite",
        [...document.tables, ...callTables, controllerLease],
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), machine.close()]);
    });
    const access = new AccessFixture(home.database);
    await access.replicateScope(account.reference(Scope.universe.id, accountId));
    await access.replicateScope(space.reference(accountId, spaceId), "own");
    await access.replicateScope(space.reference(accountId, otherSpaceId), "own");
    await machine.database.insert(document.table).values({
        id: Identifier.create("document"),
        scope: spaceId,
        title: "Notes",
        createdAt: 1,
        updatedAt: 1,
    });

    // follow the other space's documents beside the store's own chain while requested, failing a refused scope's streams
    const feed = new Feed(home.database, [...accessTables, ...document.tables], {
        clock: Clock.system,
    });
    const requested = new Set([otherSpaceId]);
    const refusals = new Map<string, AbortController>();
    const closing = (scope: string) => {
        const kept = refusals.get(scope) ?? new AbortController();
        refusals.set(scope, kept);

        return kept;
    };
    const store: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [document],
        database: machine.database,
        follows: [
            Follow.of(documentShape, {
                rows: () => document.query(),
                key: async () =>
                    requested.has(otherSpaceId) ? { key: { scope: otherSpaceId } } : undefined,
            }),
        ],
        publisher: (scope) => ({
            stream: async function* (subscription, signal) {
                const { signal: closed } = closing(scope);
                yield* feed.subscribe(
                    store.source.buildReplica(subscription).queries,
                    subscription.after,
                    AbortSignal.any([signal, closed]),
                );
                closed.throwIfAborted();
            },
        }),
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
        { clock: Clock.system, report: (_controller, _key, error) => failures.push(error) },
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

    // stop requesting the other space's documents without a change the controller watches, and refuse them at home
    requested.delete(otherSpaceId);
    closing(otherSpaceId).abort(new CallError("FORBIDDEN", { message: "refused" }));
    await machine.database.log.until(
        async () => (await ownChain(machine.database)) === OWN_CHAIN,
        controller.signal,
    );

    // drop the documents' replica without reporting the refusal
    expect(failures).toEqual([]);
});

test("follow replicates as a server without methods, journal or call key", async () => {
    // keep an account and its space at home, and a database keeping access replicas
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
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [],
        policies: [document],
        database: follower.database,
        subscriber: {
            uplink: {
                receive: refuseChanges,
                stream: (subscription, signal) =>
                    feed.subscribe(
                        store.source.buildReplica(subscription).queries,
                        subscription.after,
                        signal,
                    ),
            },
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

    // run only compaction and the replica follow, keeping no event log
    expect(store.controllers().map((each) => each.name)).toEqual(["compaction", "replica"]);
});

test("replicate the chains of an installation's two served spaces in one replica via both, requiring it to replicate each and refusing a scope outside them", async () => {
    // keep an account with two spaces at home, and another account
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
        fixture.replicateRelationship(id, space.reference(accountId, id), granted, subject, 1);
    await grant(spaceId, "reader");
    await grant(otherSpaceId, "reader");
    await grant(spaceId, "replicator");

    // replicate the universe's spaces to the installation, decided at home for it
    const follower = { subject, parent: Scope.universe.id };
    const replicating: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [document],
        policies: [space],
        database: installed.database,
        subscriber: {
            uplink: {
                receive: refuseChanges,
                stream: (asked, signal) =>
                    source.source.replicate(asked, asked.after, follower, signal),
            },
        },
        callKey: testCallKey,
        events: await testEvents(installed.database),
        origin: { package: document.package },
    });
    const controller = await followUniverse(replicating, home.database);

    // request the universe's rows and one replica of both spaces' chains via them
    const requests = (await replicating.source.chains(followingSpaceId)).map(
        (entry) => entry.subscription,
    );
    const chains = present(
        requests.find((request) => request.name === Authorizer.viaReplica(followingSpaceId)),
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
            [Authorizer.viaReplica(followingSpaceId), Scope.universe.id, followingSpaceId],
        ],
        parameters: [[spaceId, otherSpaceId], [accountId]],
        refused: ["FORBIDDEN", "permission denied: replicate"],
        scopes: new Set([accountId, spaceId, otherSpaceId]),
        outside: ["NOT_FOUND", `${otherAccountId} contains none of the replicated scopes`],
    });
});

test("refuse a chain follow with no via to a caller without replicate on the scope", async () => {
    // keep an account with a space at home
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
        objects: [space],
        policies: [document],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: document.package },
    });

    // request the account's chain below the space as an installation living in it, with no via
    const subject = principal.installation.reference(spaceId, followingId);
    const request = Authorizer.chainSubscription({
        name: Authorizer.chainReplica(spaceId),
        scope: accountId,
        below: spaceId,
        parameters: { local: [], replicated: [], via: [], between: [], types: [] },
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
    expect({
        refused,
        scopes: new Set(
            page.changes
                .filter((change) => change.table === Scope.table[TABLE].sqlName)
                .map((change) => change.row["scope"]),
        ),
    }).toEqual({
        refused: ["FORBIDDEN", "permission denied: replicate"],
        scopes: new Set([accountId]),
    });
});

test("decide a via chain for the subject whose follow brought its rows", async () => {
    // keep a space replicated into an installation's database by a follow on alice's behalf
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
    await database.insert(sync.replica).values({
        name,
        scope: spaceId,
        ...sync.Replica.subscriptionColumns({
            name,
            shape: "queries",
            scope: spaceId,
            below: followingSpaceId,
            parameters: {},
            subject: alice,
        }),
        tables: [space.table[TABLE].sqlName],
    });
    await database.insert(sync.replicaRow).values({
        name,
        scope: spaceId,
        table: space.table[TABLE].sqlName,
        key: Key.name(space.table, { id: spaceId, scope: accountId }),
        concealed: [],
    });

    // list the installation's chains below its own space
    const store = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        objects: [document],
        policies: [space],
        database,
        subscriber: { uplink: { receive: refuseChanges, stream: async function* () {} } },
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: document.package },
    });
    const vias = (await store.source.chains(followingSpaceId))
        .map((entry) => entry.subscription)
        .filter((subscription) =>
            subscription.name.startsWith(Authorizer.viaReplica(followingSpaceId)),
        )
        .map((subscription) => [
            subscription.name,
            subscription.subject,
            subscription.parameters["via"],
        ]);

    // request the space's chain via it decided for alice, beside the installation's own universe replica
    expect(vias).toEqual([
        [Authorizer.viaReplica(followingSpaceId), undefined, []],
        [`${Authorizer.viaReplica(followingSpaceId)}:${Subject.key(alice)}`, alice, [spaceId]],
    ]);
});

test("serve a store whose on-behalf follow is refused, without gating its readiness", async () => {
    // keep a space replicated into an installation's database by a best-effort follow on alice's behalf
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
    const brought = Shape.subscription(activity, { scope: spaceId }, followingSpaceId, alice);
    await database
        .insert(space.table)
        .values({ id: spaceId, scope: accountId, createdAt: 1, updatedAt: 1 });
    await database.insert(sync.replica).values({
        name: brought.name,
        scope: spaceId,
        ...sync.Replica.subscriptionColumns(brought),
        tables: [space.table[TABLE].sqlName],
    });
    await database.insert(sync.replicaRow).values({
        name: brought.name,
        scope: spaceId,
        table: space.table[TABLE].sqlName,
        key: Key.name(space.table, { id: spaceId, scope: accountId }),
        concealed: [],
    });

    // serve the installation's store, whose publisher refuses every follow
    const store = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        objects: [document],
        policies: [space],
        database,
        follows: [follow],
        publisher: () => ({
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
    const gates = (await subscriber.subscriptions())
        .filter(({ subscription }) => subscription.subject !== undefined)
        .map(({ subscription, isReadinessGate }) => [subscription.name, isReadinessGate]);

    // gate neither the follow nor the via chain its rows brought
    const via = `${Authorizer.viaReplica(followingSpaceId)}:${Subject.key(alice)}`;
    expect(gates).toEqual([
        [via, false],
        [brought.name, false],
    ]);
});

test("replicate a scoped table an installation's database declares beside its followed space's chain, and drop its rows once the installation may no longer read the space", async () => {
    // keep an account with two spaces and a document in each at home
    const home = await TestDatabase.create(
        "sqlite",
        [...space.tables, ...document.tables, ...callTables],
        { isMigrated: true },
    );
    const declared = defineDatabase({
        name: "installed",
        tables: [...comment.tables, ...callTables, controllerLease],
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
        objects: [space, document],
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
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: subject,
        objects: [comment],
        policies: [space, document],
        database: installed.database,
        subscriber: {
            uplink: {
                receive: refuseChanges,
                stream: (asked, signal) =>
                    source.source.replicate(asked, asked.after, follower, signal),
            },
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

/** The replicas of the store's own chain, each as its name and scope. */
const OWN_CHAIN = JSON.stringify(
    [spaceId, accountId, Scope.universe.id]
        .map((scope) => [Authorizer.chainReplica(spaceId), scope])
        .toSorted((left, right) => left.join().localeCompare(right.join())),
);

/** List a database's replicas as their names and scopes, to compare with the own chain. */
async function ownChain(database: DatabaseConnection): Promise<string> {
    const replicas = (await Replica.subscriptions(database))
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
    // keep two accounts at home, the first readable by the installation
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
        objects: [account],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: account.package },
    });
    const follower = { subject, parent: Scope.universe.id };
    const replicating: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        objects: [annotation],
        policies: [account],
        database: installed.database,
        subscriber: {
            uplink: {
                receive: refuseChanges,
                stream: (asked, signal) =>
                    source.source.replicate(asked, asked.after, follower, signal),
            },
        },
        callKey: testCallKey,
        events: await testEvents(installed.database),
        origin: { package: annotation.package },
    });
    await followUniverse(replicating, home.database);

    // request one replica of the readable account's chain and the installation's own space
    const requests = (await replicating.source.chains(followingSpaceId)).map(
        (entry) => entry.subscription,
    );
    const chains = present(
        requests.find((request) => request.name === Authorizer.viaReplica(followingSpaceId)),
        "the chains' request",
    );
    expect({
        requests: requests.map((request) => [request.name, request.scope, request.below]),
        via: chains.parameters["via"],
    }).toEqual({
        requests: [
            [Authorizer.chainReplica(followingSpaceId), followingSpaceId, followingSpaceId],
            [followingSpaceId, Scope.universe.id, followingSpaceId],
            [Authorizer.viaReplica(followingSpaceId), Scope.universe.id, followingSpaceId],
        ],
        via: [accountId],
    });
});

test("replicate the chain of an account another replica brought first, once the universe replica includes it", async () => {
    // keep an account at home that the installation reads
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
        objects: [account],
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database),
        origin: { package: account.package },
    });
    const follower = { subject, parent: Scope.universe.id };
    const chain = Authorizer.viaReplica(followingSpaceId);
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

    // replicate the universe's rows under another name first, as a residents replica does
    const replicating: ObjectStore = new ObjectStore({
        chain: async () => ({ kind: "follower" }),
        clock: Clock.system,
        principal: spacePrincipal(followingSpaceId),
        objects: [annotation],
        policies: [account],
        database: installed.database,
        subscriber: { uplink: publisher },
        callKey: testCallKey,
        events: await testEvents(installed.database),
        origin: { package: annotation.package },
    });
    const residents = {
        ...present(
            replicating.source.universeSubscription(followingSpaceId),
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
            { subscription: residents, clock: Clock.system },
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

/** Follow a store's replica of the universe until the test finishes. */
async function followUniverse(
    replicating: ObjectStore,
    home: DatabaseConnection,
): Promise<AbortController> {
    // follow the store's replica of the universe's rows
    const replica = present(
        replicating.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const key = (await replica.list()).find((each) => readScope(each) === Scope.universe.id);
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
    await Replica.wait(replicating.database, Scope.universe.id, head, controller.signal);

    return controller;
}
