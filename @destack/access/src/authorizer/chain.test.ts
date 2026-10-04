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
    CHAIN_SHAPE,
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

test("copy a space's chain up to the universe from a relay, with the policies each scope hands down, and follow a grant", async () => {
    // insert an account and its space, both owned by alice, in the home database
    const home = await openFixture();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([home.close(), app.close()]);
    });
    const authorizer = new Authorizer(policies, homeMappings);
    const owner = new Authorization(authorizer, home.database, () => ({
        ...home.alice,
        now: Date.now(),
    }));
    await home.database.insert(accountTable).values({ id: "account-1", scope: "universe" });
    await owner.create(account.reference("universe", "account-1"), { owner: userSubject("alice") });
    await home.database.insert(spaceTable).values({ id: "personal", account: "account-1" });
    await owner.create(space.reference("account-1", "personal"), { owner: userSubject("alice") });

    // require a value everywhere and of the account's spaces, and set one for the account alone
    await home.database.insert(policyTable).values([
        { id: "everywhere", scope: "universe", mode: "require", value: "on" },
        { id: "required", scope: "account-1", mode: "require", value: "on" },
        { id: "own", scope: "account-1", mode: "set", value: "off" },
    ]);

    // relay the space's chain from the home database
    const feed = new Feed(home.database, [...accessTables, policyTable]);
    const requests = () => app.authorizer.chain(app.database, "personal", { isHome: false });
    const scopes = async () => (await requests()).map((request) => request.scope);
    const follow = async (scope: string) => {
        const request = (await requests()).find((entry) => entry.scope === scope);
        if (request === undefined) {
            throw new Error(`no chain request for ${scope}`);
        }
        following.push(
            app.authorizer.chainShape
                .replica(request)
                .follow(
                    app.database,
                    ({ after }, signal) =>
                        feed.subscribe(
                            authorizer.chainShape.replica(request).queries,
                            after,
                            signal,
                        ),
                    controller.signal,
                    { subscription: request },
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
        await Replica.reach(app.database, scope, head, controller.signal);
    };

    // list the space alone until its copy lists the account containing it, up to the universe
    expect(await scopes()).toEqual(["personal"]);
    await follow("personal");
    await reach("personal");
    expect(await scopes()).toEqual(["personal", "account-1", "universe"]);

    // copy the account's access row for row, and only the policies the account and the universe hand down
    await follow("account-1");
    await follow("universe");
    await reach("account-1");
    await reach("universe");
    expect([
        await copied(app.database),
        await app.database.select().from(policyTable).orderBy(asc(policyTable.id)),
    ]).toEqual([
        await copied(home.database),
        [
            { id: "everywhere", scope: "universe", mode: "require", value: "on" },
            { id: "required", scope: "account-1", mode: "require", value: "on" },
        ],
    ]);

    // leave the access rows of local types and of universe-living types out of every copy
    const types = [...app.authorizer.local, ...authorizer.universal];
    expect(
        authorizer.chainShape
            .replica(
                authorizer.chainShape.subscription({
                    name: "chain",
                    scope: "universe",
                    below: "personal",
                    parameters: { local: [...app.authorizer.local], copied: [] },
                }),
            )
            .where.get(accessRelationship),
    ).toEqual({
        NOT: { OR: types.map((type) => ({ packageId: type.packageId, type: type.type })) },
    });
    expect(authorizer.universal.map((type) => type.type)).toEqual(["region", "user"]);

    // follow a grant on the account into the copy
    await owner.grant({
        object: account.reference("universe", "account-1"),
        relation: "member",
        subject: userSubject("carol"),
    });
    await reach("account-1");
    expect(await copied(app.database)).toEqual(await copied(home.database));
});

test("copy the chains of the accounts a follower outside them replicates in one copy at the universe, and let go of an account it stops replicating", async () => {
    // open the home and app databases
    const home = await openFixture();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([home.close(), app.close()]);
    });

    // keep two accounts owned by alice with one member each in the home database
    const authorizer = new Authorizer(policies, homeMappings);
    const owner = new Authorization(authorizer, home.database, () => ({
        ...home.alice,
        now: Date.now(),
    }));
    for (const [id, member] of [
        ["account-1", "carol"],
        ["account-2", "dave"],
    ] as const) {
        await home.database.insert(accountTable).values({ id, scope: "universe" });
        const object = account.reference("universe", id);
        await owner.create(object, { owner: userSubject("alice") });
        await owner.grant({ object, relation: "member", subject: userSubject(member) });
    }
    const feed = new Feed(home.database, [...accessTables, policyTable]);

    // request one copy of both chains for the follower, recorded at the universe
    const both = present(
        await app.authorizer.chainVia(app.database, "placement", [
            account.reference("universe", "account-2"),
            account.reference("universe", "account-1"),
        ]),
        "the chains' request",
    );
    expect(both).toEqual({
        name: Authorizer.chainCopy("placement"),
        shape: CHAIN_SHAPE,
        scope: "universe",
        below: "placement",
        parameters: {
            local: [...app.authorizer.local],
            copied: [{ packageId: policy.definition.packageId, type: policy.definition.name }],
            via: ["account-1", "account-2"],
            between: [],
        },
    });

    // copy both accounts' access rows and the universe's in one stream, kept at each scope by the one record
    await followChain(app.database, app.authorizer, both, feed);
    const head = await home.database.log.position();
    const scopes = ["account-1", "account-2", "universe"];
    const origins = await Replica.origins(app.database, CHAIN_SHAPE, scopes);
    expect([
        await copied(app.database),
        [...origins.keys()].toSorted(),
        [...origins.values()].map((origin) => origin.position),
    ]).toEqual([await copied(home.database), scopes, [head, head, head]]);

    // reshape the copy once the follower replicates the first account alone, letting go of the second's rows
    const first = present(
        await app.authorizer.chainVia(app.database, "placement", [
            account.reference("universe", "account-1"),
        ]),
        "the chain's request",
    );
    await followChain(app.database, app.authorizer, first, feed);
    const kept = await copied(home.database);
    expect([
        await copied(app.database),
        [...(await Replica.origins(app.database, CHAIN_SHAPE, scopes)).keys()].toSorted(),
    ]).toEqual([
        {
            scopes: kept.scopes.filter(isOutsideSecond),
            roles: kept.roles.filter(isOutsideSecond),
            permissions: kept.permissions,
            relationships: kept.relationships.filter(isOutsideSecond),
        },
        ["account-1", "universe"],
    ]);
});

/** Apply a chain request's pages to the follower until it reaches the home's head, reshaping from the parameters the copy reflects. */
async function followChain(
    database: DatabaseConnection,
    authorizer: Authorizer,
    request: Subscription,
    feed: Feed,
): Promise<void> {
    // resume from the copy's position, reshaping from its reflected parameters
    const replica = authorizer.chainShape.replica(request);
    await replica.register(database, request);
    const { after, previous } = await replica.resume(database, request);
    const reflected =
        previous === undefined
            ? {}
            : {
                  previous: authorizer.chainShape.replica({ ...request, parameters: previous })
                      .queries,
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
    await Array.fromAsync(replica.apply(database, pages, { subscription: request }));
}

/** Report whether a row lives outside the second account. */
function isOutsideSecond(row: { readonly scope: string }): boolean {
    return row.scope !== "account-2";
}

/** Read the decision rows about the account and the space, skipping the nodes' own relationships. */
async function copied(database: DatabaseConnection) {
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
        relationships: relationships.filter((row) => row.type !== node.name),
    };
}
