import { reconciliation, testCallKey } from "@destack/service/test";
import { ControlLoop, controllerLease } from "@destack/service/control";
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
} from "@destack/access";
import { journal } from "@destack/audit";
import { copyOwner, copyRole, copyScope } from "@destack/access/test";
import { asc, defineDatabase, eq, TABLE } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";

import { ServiceError } from "@destack/service/error";
import { RequestId } from "@destack/service/request";
import { Scope, Feed, Replica, type Page, type Subscription } from "@destack/sync";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectServer, Subscriber, SystemAuthorization } from "../src/server/index.ts";
import { userContext } from "./fixture/user.ts";

/** The account with the space. */
const accountId = schema
    .identifier("account")
    .parse("account-01996ab0-0000-7000-8000-000000000001");

/** The empty space the documents go into. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** Another space of the account outside every restriction here. */
const otherSpaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** A third space of the account, which only a cell's own scope rows name. */
const thirdSpaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000004");

/** The placement of a workload copying the spaces' chains. */
const placementId = "placement-01996ab0-0000-7000-8000-000000000005";

/** Another account, holding none of the spaces. */
const otherAccountId = schema
    .identifier("account")
    .parse("account-01996ab0-0000-7000-8000-000000000006");

/** Accounts with members that roles bind to. */
const account = defineObject({
    name: "account",
    plural: "accounts",
    scope: "universe",
    isScope: true,
    fields: {},
    relations: { member: { subjects: [principal.user] } },
    permissions: { read: relation("member") },
});

/** Spaces within accounts, which workloads read and replicate as granted. */
const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: account,
    isScope: true,
    fields: {},
    relations: {
        reader: { subjects: [principal.workload], grantedBy: null },
        replicator: { subjects: [principal.workload], grantedBy: null },
    },
    permissions: { read: relation("reader"), replicate: relation("replicator") },
    methods: (method) => ({ list: method.list("read") }),
});

/** Documents that only roles grant. */
const document = defineObject({
    name: "document",
    plural: "documents",
    scope: space,
    fields: { title: field.string(schema.string().min(1)) },
    permissions: { read: none(), write: none() },
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("write"),
    }),
});

test.each(TEST_DIALECTS)(
    "decide calls over a relayed copy of access as the holder does, through the account's members and restricted credentials, on %s",
    async (dialect) => {
        const home = await TestDatabase.create(dialect, accessTables, { isMigrated: true });
        const workload = await TestDatabase.create(dialect, [...document.tables, journal], {
            isMigrated: true,
        });
        onTestFinished(async () => {
            await Promise.all([home.close(), workload.close()]);
        });

        // insert an account owned by alice, a member role carol has through the account, and its empty space at home
        const alice = principal.user.reference("universe", "alice");
        const carol = principal.user.reference("universe", "carol");
        const accountObject = account.reference(Scope.universe.id, accountId);
        await copyScope(home.database, accountObject);
        await copyScope(home.database, space.reference(accountId, spaceId));
        await copyOwner(home.database, accountObject, alice);
        await copyRole(
            home.database,
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

        // serve the documents of one space over the workload's database to each call's user, copying its chain from the home database
        const feed = new Feed(home.database, accessTables);
        const server: ObjectServer<{ document: typeof document }> = new ObjectServer({
            objects: { document },
            database: workload.database,
            subscriber: Subscriber.of(
                {
                    stream: (subscription, signal) =>
                        feed.subscribe(
                            server.source.replicaOf(subscription).queries,
                            subscription.after,
                            signal,
                        ),
                },
                () => server.authorizer.chain(workload.database, spaceId, { isHome: false }),
            ),
            callKey: testCallKey,
            origin: {
                package: document.package,
                service: "test",
            },
        });

        // follow the space's chain up to the universe
        const follower = present(
            server.controllers().find((each) => each.name === "replica"),
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
            await Replica.reach(workload.database, scope, head, controller.signal);
        }
        expect((await follower.list()).map(scopeOf)).toEqual([
            spaceId,
            accountId,
            Scope.universe.id,
        ]);

        // create through credentials restricted to the space, to another one and unrestricted, and list as alice
        const create = async (user: string, title: string, restricted?: string) => {
            // restrict the caller's permissions to one scope when the credential asks
            const context = userContext(
                user,
                spaceId,
                restricted === undefined
                    ? {}
                    : {
                          permissions: [
                              document.permission("read"),
                              document.permission("write"),
                          ].map((permission) => ({ ...permission, scope: restricted })),
                      },
            );

            const created = server.call(
                document,
                "create",
                { spaceId, requestId: RequestId.create(), title },
                context,
            );
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
        const listed = await server.call(
            document,
            "list",
            { spaceId },
            userContext("alice", spaceId),
        );

        // admit the owner through a credential for the empty space and hide the space from the rest
        const hidden = ["NOT_FOUND", `no scope ${spaceId}`];
        expect(outcomes).toEqual(["Plan", hidden, hidden, hidden, "Budget"]);
        expect(listed.items.map((item) => item.title).toSorted()).toEqual(["Budget", "Plan"]);
    },
);

test("keep one copy record per followed space of one account, and drop one space's copies while the other's keep the shared rows", async () => {
    // keep an account with a member role and two spaces at home, and a cell following both spaces' chains
    const { home, cell, server, requested } = await openCopies();
    requested.add(spaceId).add(otherSpaceId);
    const controller = new AbortController();
    const loop = new ControlLoop(
        cell.database,
        server.controllers().filter((each) => each.name === "replica"),
        { report: failing },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const head = await home.database.log.position();
    await cell.database.log.until(
        async () => (await Replica.subscriptions(cell.database)).length === 6,
        controller.signal,
    );
    for (const scope of [spaceId, otherSpaceId, accountId, Scope.universe.id]) {
        await Replica.reach(cell.database, scope, head, controller.signal);
    }

    // record each space's copy of the account and the universe apart
    const records = async () =>
        (await Replica.subscriptions(cell.database))
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
    await copyScope(cell.database, space.reference(accountId, thirdSpaceId));
    await cell.database.log.until(
        async () => (await Replica.subscriptions(cell.database)).length === 3,
        controller.signal,
    );
    const roles = await cell.database
        .select({ scope: accessRole.scope, name: accessRole.name })
        .from(accessRole);
    const scopes = await cell.database
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
    // follow a space's chain on a cell through one control loop and start another over the same database
    const { home, cell, server, requested } = await openCopies();
    requested.add(spaceId);
    const controller = new AbortController();
    const loop = (holder: string) =>
        new ControlLoop(
            cell.database,
            server.controllers().filter((each) => each.name === "replica"),
            { report: failing, lease: { holder } },
        );
    const [first, second] = [loop("first"), loop("second")];
    const running = [first.run(controller.signal)];
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(running);
    });
    const head = await home.database.log.position();
    for (const scope of [spaceId, accountId, Scope.universe.id]) {
        await Replica.reach(cell.database, scope, head, controller.signal);
    }
    running.push(second.run(controller.signal));
    await second.idle();

    // stop requesting the space's chain once the scope rows change, and wait for its copies to drop
    requested.delete(spaceId);
    await copyScope(cell.database, space.reference(accountId, otherSpaceId));
    await cell.database.log.until(
        async () => (await Replica.subscriptions(cell.database)).length === 0,
        controller.signal,
    );

    // drop every copy and its rows under the first controller's leases, which the second never took
    const leases = await cell.database
        .select({ key: controllerLease.key, holder: controllerLease.holder })
        .from(controllerLease)
        .orderBy(asc(controllerLease.key));
    const roles = await cell.database.select({ id: accessRole.id }).from(accessRole);
    expect([leases.map((lease) => lease.holder), roles]).toEqual([["first", "first", "first"], []]);
});

test("drop a copy once its source refuses its follow and no request names it any longer, without a change of the rows the requests read", async () => {
    // follow a space's chain on a cell
    const { home, cell, server, requested, refuse } = await openCopies();
    requested.add(spaceId);
    const controller = new AbortController();
    const failures: unknown[] = [];
    const loop = new ControlLoop(
        cell.database,
        server.controllers().filter((each) => each.name === "replica"),
        { report: (_controller, _key, error) => failures.push(error) },
    );
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const head = await home.database.log.position();
    for (const scope of [spaceId, accountId, Scope.universe.id]) {
        await Replica.reach(cell.database, scope, head, controller.signal);
    }

    // stop requesting the chain without a change the controller watches, and refuse the cell at home
    requested.delete(spaceId);
    refuse();
    await cell.database.log.until(
        async () => (await Replica.subscriptions(cell.database)).length === 0,
        controller.signal,
    );
    const roles = await cell.database.select({ id: accessRole.id }).from(accessRole);
    expect([roles, failures]).toEqual([[], []]);
});

test("follow copies as a server serving no methods, which keeps no journal and takes no call key", async () => {
    // keep an account and its space at home, and a database keeping only access copies
    const home = await TestDatabase.create("sqlite", accessTables, { isMigrated: true });
    const follower = await TestDatabase.create("sqlite", accessTables, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), follower.close()]);
    });
    await copyScope(home.database, account.reference(Scope.universe.id, accountId));
    await copyScope(home.database, space.reference(accountId, spaceId));

    // follow the space's chain without a journal
    const feed = new Feed(home.database, accessTables);
    const server: ObjectServer = new ObjectServer({
        objects: {},
        policies: [document],
        database: follower.database,
        subscriber: Subscriber.of(
            {
                stream: (subscription, signal) =>
                    feed.subscribe(
                        server.source.replicaOf(subscription).queries,
                        subscription.after,
                        signal,
                    ),
            },
            () => server.authorizer.chain(follower.database, spaceId, { isHome: false }),
        ),
        origin: { package: document.package, service: "test" },
    });
    const controller = new AbortController();
    const loop = new ControlLoop(follower.database, server.controllers(), { report: failing });
    const running = loop.run(controller.signal);
    onTestFinished(async () => {
        controller.abort();
        await running;
    });
    const head = await home.database.log.position();
    for (const scope of [spaceId, accountId, Scope.universe.id]) {
        await Replica.reach(follower.database, scope, head, controller.signal);
    }

    // refuse a call key without methods, and methods without a call key
    const origin = { package: document.package, service: "test" };
    const refused = (options: { readonly callKey?: typeof testCallKey }, objects = {}) => {
        try {
            const served = new ObjectServer({
                objects,
                database: follower.database,
                origin,
                ...options,
            });

            return served.objects.length;
        } catch (error) {
            return error instanceof TypeError ? error.message : "unexpected";
        }
    };
    expect([
        server.controllers().map((each) => each.name),
        refused({ callKey: testCallKey }),
        refused({}, { document }),
    ]).toEqual([
        ["compaction", "replica"],
        "a server serving no methods keeps no journal and takes no call key",
        "a server serving methods needs the call key of their journal",
    ]);
});

test("copy the chains of a workload's two spaces in one copy via both, requiring it to replicate each and refusing a scope outside them", async () => {
    // keep an account with two spaces at home, and another account
    const home = await TestDatabase.create("sqlite", [...space.tables, journal], {
        isMigrated: true,
    });
    const placed = defineDatabase({
        name: "workload",
        tables: [...document.tables, journal],
        copies: [space.table],
    });
    const workload = await TestDatabase.create("sqlite", placed, { isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), workload.close()]);
    });
    await copyScope(home.database, account.reference(Scope.universe.id, accountId));
    await copyScope(home.database, account.reference(Scope.universe.id, otherAccountId));
    for (const id of [spaceId, otherSpaceId]) {
        await copyScope(home.database, space.reference(accountId, id));
        await home.database
            .insert(space.table)
            .values({ id, scope: accountId, createdAt: 1, updatedAt: 1 });
    }

    // let the workload read both spaces and replicate the first
    const source = new ObjectServer({
        objects: { space },
        policies: [document],
        database: home.database,
        callKey: testCallKey,
        origin: { package: document.package, service: "test" },
    });
    const system = await SystemAuthorization.open(
        source.authorizer,
        home.database,
        Scope.universe.id,
        1,
    );
    const subject = principal.workload.reference(Scope.universe.id, placementId);
    const grant = (id: string, granted: "reader" | "replicator") =>
        system.grant({ object: space.reference(accountId, id), relation: granted, subject });
    await grant(spaceId, "reader");
    await grant(otherSpaceId, "reader");
    await grant(spaceId, "replicator");

    // copy the universe's spaces to the workload, decided at home for it
    const follower = { subject, parent: Scope.universe.id };
    const copying: ObjectServer<{ document: typeof document }> = new ObjectServer({
        objects: { document },
        policies: [space],
        database: workload.database,
        subscriber: Subscriber.of(
            {
                stream: (asked, signal) =>
                    source.source.replicate(asked, asked.after, follower, signal),
            },
            async () => [
                present(copying.source.universeSubscription(placementId), "the universe request"),
            ],
        ),
        callKey: testCallKey,
        origin: { package: document.package, service: "test" },
    });
    const replica = present(
        copying.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );

    // follow the universe's copy until it reaches the home's head
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
    const head = await home.database.log.position();
    await Replica.reach(workload.database, Scope.universe.id, head, controller.signal);

    // request the universe's rows and one copy of both spaces' chains via them
    const requests = await copying.source.workloadSubscriptions(placementId);
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
            [placementId, Scope.universe.id, placementId],
            [Authorizer.chainCopy(placementId), Scope.universe.id, placementId],
        ],
        parameters: [[spaceId, otherSpaceId], [accountId]],
        refused: ["FORBIDDEN", "permission denied: replicate"],
        scopes: new Set([accountId, spaceId, otherSpaceId]),
        outside: ["NOT_FOUND", `${otherAccountId} contains none of the replicated scopes`],
    });
});

/** Keep an account with a member role and two spaces at home, and serve a cell following the chains of the requested spaces from it. */
async function openCopies() {
    // keep the account, its member role and its two spaces at home
    const home = await TestDatabase.create("sqlite", accessTables, { isMigrated: true });
    const cell = await TestDatabase.create(
        "sqlite",
        [...document.tables, journal, controllerLease],
        { isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), cell.close()]);
    });
    const accountObject = account.reference(Scope.universe.id, accountId);
    await copyScope(home.database, accountObject);
    await copyScope(home.database, space.reference(accountId, spaceId));
    await copyScope(home.database, space.reference(accountId, otherSpaceId));
    await copyRole(
        home.database,
        accountObject,
        { name: "member", description: "Reads the account's spaces", permissions: [] },
        { ...accountObject, relation: "member" },
    );

    // follow each requested space's chain from home, failing every stream once home refuses the cell
    const feed = new Feed(home.database, accessTables);
    const requested = new Set<string>();
    const refused = new AbortController();
    const refuse = () => refused.abort(new ServiceError("FORBIDDEN", { message: "refused" }));
    const server: ObjectServer<{ document: typeof document }> = new ObjectServer({
        objects: { document },
        database: cell.database,
        subscriber: Subscriber.of(
            {
                stream: async function* (subscription, signal) {
                    yield* feed.subscribe(
                        server.source.replicaOf(subscription).queries,
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
                            server.authorizer.chain(cell.database, below, { isHome: false }),
                        ),
                    )
                ).flat(),
        ),
        callKey: testCallKey,
        origin: { package: document.package, service: "test" },
    });

    return { home, cell, server, requested, refuse };
}

/** Fail a test on a failed reconciliation. */
function failing(_controller: unknown, key: string, error: unknown): never {
    throw new Error(`reconciling ${key} failed`, { cause: error });
}

/** Read the scope a replica controller's key follows. */
function scopeOf(key: string): string | undefined {
    return key.split(" ")[1];
}
