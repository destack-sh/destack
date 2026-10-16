import { expect, onTestFinished, test } from "@destack/test";
import { asc, type DatabaseConnection } from "@destack/db";
import { Feed, Replica, Scope, type Subscription } from "@destack/sync";
import { present } from "@destack/schema";
import {
    accessRelationship,
    accessRole,
    accessRolePermission,
    accessTables,
    Authorization,
    Authorizer,
    CHAIN_PUBLICATION,
    Relationship,
} from "../index.ts";
import {
    account,
    accountTable,
    homeMappings,
    node,
    policies,
    policy,
    policyTable,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { openFixture, userSubject } from "../test/database.ts";
import { Clock } from "@destack/environment";

/** The replica of the accounts' chains stored for the placement follower. */
const PLACEMENT_REPLICA = { name: Authorizer.chainReplica("placement"), below: "placement" };

/** The object types whose universe relationships the tests' decisions read: every test type's. */
const TYPES = policies.map((each) => each.typeReference);

test("replicate a space's chain up to the universe from a relay, with the policies each scope hands down, and follow a grant", async () => {
    // insert an account and its space, both owned by alice, in the home database
    const home = await openFixture();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([home.close(), app.close()]);
    });
    const authorizer = new Authorizer(policies, homeMappings);
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            authorizer,
            home.database,
            () => ({
                ...home.alice,
                now: Date.now(),
            }),
            call,
        );
    await home.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await owner((authorization) =>
        authorization.create(account.reference("universe", "account-1"), {
            owner: userSubject("alice"),
        }),
    );
    await home.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await owner((authorization) =>
        authorization.create(space.reference("account-1", "personal"), {
            owner: userSubject("alice"),
        }),
    );

    // require a value everywhere and in the account's spaces, and set one for the account only
    await home.database.insert(policyTable).values([
        { id: "everywhere", scope: "universe", mode: "require", value: "on" },
        { id: "required", scope: "account-1", mode: "require", value: "on" },
        { id: "own", scope: "account-1", mode: "set", value: "off" },
    ]);

    // relay the space's chain from the home database
    const feed = new Feed(home.database, [...accessTables, policyTable], { clock: Clock.system });
    const requests = () =>
        app.authorizer.chain(app.database, "personal", { kind: "follower" }, TYPES);
    const scopes = async () => (await requests()).map((request) => request.scope);
    const follow = async (scope: string) => {
        const request = (await requests()).find((entry) => entry.scope === scope);
        if (request === undefined) {
            throw new Error(`no chain request for ${scope}`);
        }
        following.push(
            app.authorizer
                .replicateChain(request)
                .follow(
                    app.database,
                    ({ after }, signal) =>
                        feed.subscribe(authorizer.replicateChain(request).queries, after, signal),
                    controller.signal,
                    { clock: Clock.system, subscription: request },
                ),
        );
    };
    const controller = new AbortController();
    const following: Promise<void>[] = [];
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(following);
    });
    const reach = async (scope: string) => {
        const head = await home.database.log.position();
        await Replica.wait(app.database, scope, head, controller.signal);
    };

    // list only the space until its replica lists the account containing it, up to the universe
    expect(await scopes()).toEqual(["personal"]);
    await follow("personal");
    await reach("personal");
    expect(await scopes()).toEqual(["personal", "account-1", "universe"]);

    // replicate the account's access row for row, and only the policies the account and the universe hand down
    await follow("account-1");
    await follow("universe");
    await reach("account-1");
    await reach("universe");
    expect([
        await replicated(app.database),
        await app.database.select().from(policyTable).orderBy(asc(policyTable.id)),
    ]).toEqual([
        await replicated(home.database),
        [
            { id: "everywhere", scope: "universe", mode: "require", value: "on" },
            { id: "required", scope: "account-1", mode: "require", value: "on" },
        ],
    ]);

    // leave out local types and universe principals, and take the universe rows of the read types
    expect(
        authorizer
            .replicateChain(
                Authorizer.chainSubscription({
                    name: "chain",
                    scope: "universe",
                    below: "personal",
                    parameters: {
                        local: [...app.authorizer.local],
                        replicated: [],
                        via: [],
                        between: [],
                        types: TYPES,
                    },
                }),
            )
            .where.get(accessRelationship),
    ).toEqual({
        AND: [
            {
                NOT: {
                    OR: [
                        ...app.authorizer.local.map((type) => ({
                            objectPackageId: type.packageId,
                            objectType: type.type,
                        })),
                        ...authorizer.global.map((type) => ({
                            objectPackageId: type.packageId,
                            objectType: type.type,
                            objectScope: "universe",
                        })),
                    ],
                },
            },
            {
                OR: [
                    { scope: { ne: "universe" } },
                    ...TYPES.map((type) => ({
                        scope: "universe",
                        objectPackageId: type.packageId,
                        objectType: type.type,
                    })),
                ],
            },
        ],
    });
    expect(authorizer.global.map((type) => type.type)).toEqual([
        "machine",
        "client",
        "space",
        "anonymous",
        "user",
    ]);

    // follow a grant on the account into the replica
    await owner((authorization) =>
        Relationship.grant(
            {
                object: account.reference("universe", "account-1"),
                relation: "member",
                subject: userSubject("carol"),
            },
            authorization,
        ),
    );
    await reach("account-1");
    expect(await replicated(app.database)).toEqual(await replicated(home.database));
});

test("replicate the chains of the accounts a follower outside them replicates in one replica at the universe, and let go of an account it stops replicating", async () => {
    // open the home and app databases
    const home = await openFixture();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([home.close(), app.close()]);
    });

    // store two accounts owned by alice with one member each in the home database
    const authorizer = new Authorizer(policies, homeMappings);
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            authorizer,
            home.database,
            () => ({
                ...home.alice,
                now: Date.now(),
            }),
            call,
        );
    for (const [id, member] of [
        ["account-1", "carol"],
        ["account-2", "dave"],
    ] as const) {
        await home.database.insert(accountTable).values({ id, scope: "universe" });
        const object = account.reference("universe", id);
        await owner((authorization) =>
            authorization.create(object, { owner: userSubject("alice") }),
        );
        await owner((authorization) =>
            Relationship.grant(
                { object, relation: "member", subject: userSubject(member) },
                authorization,
            ),
        );
    }
    const feed = new Feed(home.database, [...accessTables, policyTable], { clock: Clock.system });

    // request one replica of both chains for the follower, recorded at the universe
    const both = present(
        await app.authorizer.chainVia(
            app.database,
            PLACEMENT_REPLICA,
            [
                account.reference("universe", "account-2"),
                account.reference("universe", "account-1"),
            ],
            TYPES,
        ),
        "the chains' request",
    );
    expect(both).toEqual({
        name: Authorizer.chainReplica("placement"),
        publication: CHAIN_PUBLICATION,
        scope: "universe",
        below: "placement",
        parameters: {
            local: [...app.authorizer.local],
            replicated: [{ packageId: policy.definition.packageId, type: policy.definition.name }],
            via: ["account-1", "account-2"],
            between: [],
            types: TYPES,
        },
    });

    // leave out the replicated types another publisher replicas
    const policyType = { packageId: policy.definition.packageId, type: policy.definition.name };
    const elsewhere = await app.authorizer.chainVia(
        app.database,
        PLACEMENT_REPLICA,
        [account.reference("universe", "account-1")],
        TYPES,
        app.authorizer.scoped,
        [policyType],
    );
    expect(elsewhere?.parameters).toEqual({
        local: [...app.authorizer.local],
        replicated: [],
        via: ["account-1"],
        between: [],
        types: TYPES,
    });

    // replicate both accounts' access rows and the universe's in one stream, stored at each scope by the one record
    await followChain(app.database, app.authorizer, both, feed);
    const head = await home.database.log.position();
    const scopes = ["account-1", "account-2", "universe"];
    const origins = await Replica.origins(app.database, CHAIN_PUBLICATION, scopes);
    expect([
        await replicated(app.database),
        [...origins.keys()].toSorted(),
        [...origins.values()].map((origin) => origin.position),
    ]).toEqual([await replicated(home.database), scopes, [head, head, head]]);

    // reshape the replica once the follower replicates only the first account, dropping the second's rows
    const first = present(
        await app.authorizer.chainVia(
            app.database,
            PLACEMENT_REPLICA,
            [account.reference("universe", "account-1")],
            TYPES,
        ),
        "the chain's request",
    );
    await followChain(app.database, app.authorizer, first, feed);
    const stored = await replicated(home.database);
    expect([
        await replicated(app.database),
        [...(await Replica.origins(app.database, CHAIN_PUBLICATION, scopes)).keys()].toSorted(),
    ]).toEqual([
        {
            scopes: stored.scopes.filter(isOutsideSecond),
            roles: stored.roles.filter(isOutsideSecond),
            permissions: stored.permissions,
            relationships: stored.relationships.filter(isOutsideSecond),
        },
        ["account-1", "universe"],
    ]);
});

/** Apply a chain request's pages to the follower until it is at the home's head. */
async function followChain(
    database: DatabaseConnection,
    authorizer: Authorizer,
    request: Subscription,
    feed: Feed,
): Promise<void> {
    // resume from the replica's position, reshaping from its reflected parameters
    const replica = authorizer.replicateChain(request);
    await replica.register(database, { subscription: request, clock: Clock.system });
    const { after, previous } = await replica.resume(database, request);
    const reflected =
        previous === undefined
            ? {}
            : {
                  previous: authorizer.replicateChain({ ...request, parameters: previous }).queries,
              };

    // read the pages up to the home's head and apply them in order
    const head = await feed.database.log.position();
    const pages = [];
    for await (const page of feed.subscribe(
        replica.queries,
        after,
        AbortSignal.timeout(5000),
        reflected,
    )) {
        pages.push(page);
        if (page.complete && page.position.sequence >= head.sequence) {
            break;
        }
    }
    await Array.fromAsync(
        replica.apply(database, pages, { clock: Clock.system, subscription: request }),
    );
}

/** Report whether a row lives outside the second account. */
function isOutsideSecond(row: { readonly scope: string }): boolean {
    return row.scope !== "account-2";
}

/** Read the decision rows of the account and the space, skipping node relationships. */
async function replicated(database: DatabaseConnection) {
    // read each decision table in key order
    const relationships = await database
        .select()
        .from(accessRelationship)
        .orderBy(asc(accessRelationship.id));

    return {
        scopes: await database.select().from(Scope.table).orderBy(asc(Scope.table.scope)),
        roles: await database.select().from(accessRole).orderBy(asc(accessRole.id)),
        permissions: await database
            .select()
            .from(accessRolePermission)
            .orderBy(asc(accessRolePermission.id)),
        relationships: relationships.filter((row) => row.objectType !== node.name),
    };
}

test("replicate the scope object row of a home's parent before the home is born", async () => {
    // store the home's chain in a database replicating accounts and spaces
    const home = await openFixture();
    onTestFinished(() => home.close());
    const authorizer = new Authorizer(policies, homeMappings, {
        partition: "personal",
        replicates: (table) => table === accountTable || table === spaceTable,
    });

    // request the chain of a home before it is born, its account alone
    const requests = await authorizer.chain(
        home.database,
        "personal",
        { kind: "home", parent: "account-1" },
        TYPES,
    );
    const replicas = requests.map((request) => authorizer.replicateChain(request));

    // replicate the account's own row alone, the home's own row arriving with its follows
    expect(replicas.map((replica) => replica.where.get(spaceTable))).toEqual([
        { id: { in: ["account-1"] } },
    ]);
});
