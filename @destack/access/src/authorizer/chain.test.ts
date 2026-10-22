import { ChainParameters } from "./authorizer.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { asc, eq, Snapshot, type DatabaseConnection } from "@destack/db";
import {
    Feed,
    type ObjectTypeReference,
    Replica,
    Scope,
    type Subject,
    type Subscription,
    ObjectReference,
} from "@destack/sync";
import {
    accessGrant,
    accessRelationship,
    accessRole,
    accessRolePermission,
    accessTables,
    AccessContext,
    Authorization,
    Authorizer,
    Caller,
    CHAIN_PUBLICATION,
    Relationship,
} from "../index.ts";
import {
    account,
    accountTable,
    homeMappings,
    cell,
    item,
    node,
    policies,
    policy,
    policyTable,
    region,
    regionTable,
    space,
    spaceTable,
} from "../test/fixture.ts";
import { AccessFixture } from "../test/access.ts";
import { openDatabase, openFixture, userSubject } from "../test/database.ts";
import { Clock } from "@destack/environment";
import { nth, schema, type JsonObject } from "@destack/schema";
import { v7 } from "uuid";

/** The seeds of the generated worlds the replica decides like their home. */
const SEEDS = Array.from({ length: 12 }, (_, index) => index + 1);

/** The people of a generated world. */
const PEOPLE = ["ada", "bea", "cyd", "dov"].map((id) => userSubject(id));

/** The space a generated world's replica is the home of, in the first account. */
const SPACE = "personal";

/** The chain of the space: the first account and the universe. */
const CHAIN = [SPACE, "account-0", Scope.universe.id];

/** The outside scopes of a target's chain: none. */
const OWN = { via: [], between: [] } as const;

test("replicate a space's chain up to the universe, with the policies each scope hands down, and follow a grant", async () => {
    // insert an account and its space, both of alice, in the home database
    const home = await openDatabase();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([home.close(), app.close()]);
    });
    const authorizer = new Authorizer(policies, homeMappings);
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            authorizer,
            home.database,
            () => ({ ...home.alice, now: Date.now() }),
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

    // follow each scope's access rows and handed down policies from the home database
    const feed = new Feed(home.database, [...accessTables, policyTable], { clock: Clock.system });
    const chain = ["personal", "account-1", "universe"];
    for (const scope of chain) {
        await follow(
            app.database,
            chainOf(app.authorizer, authorizer, scope, "personal", chain, [policy.typeReference]),
            feed,
        );
    }

    // replicate the account's access row for row, and only the policies the account and the universe hand down
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
    await follow(
        app.database,
        chainOf(app.authorizer, authorizer, "account-1", "personal", chain, [policy.typeReference]),
        feed,
    );
    expect(await replicated(app.database)).toEqual(await replicated(home.database));
});

test("replicate at the universe the relationships of the chain's objects alone, and the grants to the target from every scope", () => {
    // build the universe's chain replica of a space and of an outside follower
    const authorizer = new Authorizer(policies, homeMappings);
    const scoped = authorizer.replicateChain(
        { name: `${CHAIN_PUBLICATION}:personal`, scope: Scope.universe.id, target: "personal" },
        { scopes: ["personal", "account-1", "universe"], local: [], types: [], ...OWN },
    );
    const outside = authorizer.replicateChain(
        {
            name: `${CHAIN_PUBLICATION}:placement:via`,
            scope: Scope.universe.id,
            target: "placement",
        },
        { scopes: [], local: [], types: [], via: ["account-1"], between: [] },
    );

    // read the chain objects' universe relationships, and the target's grants on its chain
    expect([
        [
            scoped.tables.includes(accessGrant),
            scoped.where.get(accessRelationship),
            scoped.where.get(accessGrant),
        ],
        [outside.tables.includes(accessGrant), outside.where.get(accessRelationship)],
    ]).toEqual([
        [
            true,
            {
                OR: [
                    { scope: { ne: "universe" } },
                    { objectId: { in: ["universe", "personal", "account-1"] } },
                ],
            },
            { delegateScope: "personal" },
        ],
        [
            false,
            {
                OR: [
                    { scope: { ne: "universe" } },
                    { objectId: { in: ["universe", "account-1"] } },
                ],
            },
        ],
    ]);
});

test("replicate the chains of the accounts a follower outside them replicates in one replica at the universe, and let go of an account it stops replicating", async () => {
    // open the home and app databases
    const home = await openDatabase();
    const app = await openFixture();
    onTestFinished(async () => {
        await Promise.all([home.close(), app.close()]);
    });

    // store two accounts of alice with one member each in the home database
    const authorizer = new Authorizer(policies, homeMappings);
    const owner = <Value>(call: (authorization: Authorization) => Promise<Value>) =>
        Authorization.run(
            authorizer,
            home.database,
            () => ({ ...home.alice, now: Date.now() }),
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

    // replicate both accounts' access rows and the universe's in one stream, stored at each scope by the one record
    const via = (accounts: readonly string[]) =>
        chainOf(app.authorizer, authorizer, Scope.universe.id, "placement", [], [], {
            via: accounts,
            between: [],
        });
    await follow(app.database, via(["account-1", "account-2"]), feed);
    const head = await home.database.log.position();
    const scopes = ["account-1", "account-2", "universe"];
    const origins = await Replica.origins(app.database, CHAIN_PUBLICATION, scopes);
    expect([
        await replicated(app.database),
        [...origins.keys()].toSorted(),
        [...origins.values()].map((origin) => origin.position),
    ]).toEqual([await replicated(home.database), scopes, [head, head, head]]);

    // reshape the replica once the follower replicates only the first account, dropping the second's rows
    await follow(app.database, via(["account-1"]), feed);
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

test("replicate a scope object's row from wherever it lives, and only from a source storing its type", () => {
    // build the account's chain replica of its type and policies, at every source
    const storing = new Authorizer(policies, homeMappings);
    const relating = new Authorizer(policies, homeMappings, {
        tables: new Set(
            homeMappings.map((mapping) => mapping.table).filter((table) => table !== policyTable),
        ),
    });
    const chained = (authorizer: Authorizer, type: ObjectTypeReference) =>
        authorizer.replicateChain(
            { name: CHAIN_PUBLICATION, scope: "account-1", target: "personal" },
            { scopes: ["personal", "account-1", "universe"], local: [], types: [type], ...OWN },
        );
    const chain = chained(storing, account.typeReference);

    // replicate the account's row everywhere, and policy rows from their source alone
    expect([
        chain.where.get(accountTable),
        chain.everywhere.has(accountTable),
        chained(storing, policy.typeReference).tables.includes(policyTable),
        chained(relating, policy.typeReference).tables.includes(policyTable),
    ]).toEqual([{ id: { in: ["account-1"] } }, true, true, false]);
});

test("read one table of several types once in a chain replica, in the scopes of every type", () => {
    // replicate node and cell rows, which share one table, via two outside scopes
    const authorizer = new Authorizer(policies, homeMappings, { replicated: new Set([item]) });
    const outside = authorizer.replicateChain(
        {
            name: `${CHAIN_PUBLICATION}:placement:via`,
            scope: Scope.universe.id,
            target: "placement",
        },
        {
            scopes: [],
            local: [],
            types: [node.typeReference, cell.typeReference],
            via: ["personal", "account-1"],
            between: [],
        },
    );

    // read the table once, without a condition, in both scopes
    expect([
        outside.tables.filter((table) => table === item).length,
        outside.where.has(item),
        outside.scopes.get(item),
    ]).toEqual([1, false, ["personal", "account-1"]]);
});

/** A replica a test follows, with the subscription it registers under. */
interface Followed {
    /** The follower's replica. */
    readonly replica: Replica;
    /** The source's replica of the same request. */
    readonly source: Replica;
    /** Build the source's replica of the outside scopes a follower replicated before. */
    readonly reflected: (previous: JsonObject) => Replica;
    /** The subscription the follower registers. */
    readonly subscription: Subscription;
}

/** Build one scope's chain replica for a target at the follower and the source. */
function chainOf(
    follower: Authorizer,
    source: Authorizer,
    scope: string,
    target: string,
    scopes: readonly string[],
    types: ChainParameters["types"] = [],
    outside: Pick<ChainParameters, "via" | "between"> = OWN,
): Followed {
    const request = {
        name: `${CHAIN_PUBLICATION}:${target}${outside.via.length === 0 ? "" : ":via"}`,
        scope,
        target,
    };
    const parameters = { scopes, local: [], types, ...outside };

    return {
        replica: follower.replicateChain(request, parameters),
        source: source.replicateChain(request, parameters),
        reflected: (previous) => source.replicateChain(request, ChainParameters.parse(previous)),
        subscription: {
            ...request,
            publication: CHAIN_PUBLICATION,
            parameters: {
                scopes: [...scopes],
                local: [],
                types: types.map((type) => ({ ...type })),
                via: [...outside.via],
                between: [...outside.between],
            },
        },
    };
}

/** Apply a source's pages to the follower until it is at the source's head. */
async function follow(database: DatabaseConnection, followed: Followed, feed: Feed): Promise<void> {
    // resume from the replica's position, reshaping from the outside scopes it reflects
    const { replica, source, subscription } = followed;
    const options = { subscription, audience: "contained" as const, clock: Clock.system };
    await replica.register(database, options);
    const { after, previous } = await replica.resume(database, subscription);
    const reflected =
        previous === undefined
            ? {}
            : {
                  previous: followed.reflected(previous).queries,
              };

    // read the pages up to the source's head and apply them in order
    const head = await feed.database.log.position();
    const pages = [];
    for await (const page of feed.subscribe(
        source.queries,
        after,
        AbortSignal.timeout(5000),
        reflected,
    )) {
        pages.push(page);
        if (page.complete && page.position.sequence >= head.sequence) {
            break;
        }
    }
    await Array.fromAsync(replica.apply(database, pages, options));
}

/** Read a database's relationships in key order. */
async function relationshipsOf(database: DatabaseConnection) {
    return await database.select().from(accessRelationship).orderBy(asc(accessRelationship.id));
}

/** Report whether a row lives outside the second account. */
function isOutsideSecond(row: { readonly scope: string }): boolean {
    return row.scope !== "account-2";
}

/** Read the decision rows of the account and the space, skipping node relationships. */
async function replicated(database: DatabaseConnection) {
    // read each decision table in key order
    const related = await database
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
        relationships: related.filter((row) => row.objectType !== node.name),
    };
}

test("decide who serves the account above a space in its replica exactly as at the home, across generated regions, residents and members", async () => {
    // decide every person in each generated world at the home and in the space's replica
    const decisions: [boolean, boolean][] = [];
    for (const seed of SEEDS) {
        const world = await openWorld(generator(seed));
        for (const person of PEOPLE) {
            decisions.push([await serves(world.home, person), await serves(world.app, person)]);
        }
    }

    // decide alike in every world, allowing some people and refusing others
    expect([
        decisions.filter(([home, replica]) => home !== replica),
        decisions.some(([home]) => home),
        decisions.some(([home]) => !home),
    ]).toEqual([[], true, true]);
});

test("revoke a region's server from the account it resides in at once in the space's replica, through the included relationship", async () => {
    // let ada serve the first account as the server of the region it resides in
    const world = await openWorld(() => 0);
    const ada = nth(PEOPLE, 0);
    const before = await serves(world.app, ada);

    // remove her as the region's server at the home and follow the universe's chain again
    await world.home.database
        .delete(accessRelationship)
        .where(eq(accessRelationship.subjectId, ada.id));
    await world.follow(Scope.universe.id);

    // refuse her in the replica, which stores no relationship of hers
    expect([
        before,
        await serves(world.app, ada),
        await relationships(world.app.database, ada),
    ]).toEqual([true, false, []]);
});

test("replicate the universe's relationships of a space's chain alone, beside thousands of other regions and accounts", async () => {
    // store 2,000 other regions with a server and 2,000 other accounts with a member at the home
    const world = await openWorld(
        () => 0,
        async (fixture) => {
            const now = Date.now();
            const others = Array.from({ length: 2000 }, (_, index) => index);
            await fixture.database
                .insert(regionTable)
                .values(others.map((index) => ({ id: `other-${index}` })));
            const rows = others.flatMap((index) => [
                Relationship.encode(
                    {
                        id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                        object: region.reference(Scope.universe.id, `other-${index}`),
                        relation: "machine",
                        subject: userSubject(`server-${index}`),
                        createdAt: now,
                        expiresAt: null,
                    },
                    Scope.universe.id,
                ),
                Relationship.encode(
                    {
                        id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                        object: account.reference(Scope.universe.id, `account-other-${index}`),
                        relation: "member",
                        subject: userSubject(`member-${index}`),
                        createdAt: now,
                        expiresAt: null,
                    },
                    Scope.universe.id,
                ),
            ]);

            // relate the people otherwise, whom the chain lists directly, and its regions otherwise
            const unnamed = [
                ...PEOPLE.map((person) => ({
                    object: person,
                    relation: "friend",
                    subject: userSubject("zed"),
                })),
                ...[0, 1].map((index) => ({
                    object: region.reference(Scope.universe.id, `region-${index}`),
                    relation: "home",
                    subject: userSubject("zed"),
                })),
            ];
            rows.push(
                ...unnamed.map((related) =>
                    Relationship.encode(
                        {
                            id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                            ...related,
                            createdAt: now,
                            expiresAt: null,
                        },
                        Scope.universe.id,
                    ),
                ),
            );

            // insert them in batches below PostgreSQL's 65,535 bound parameters, 21 per row
            for (let start = 0; start < rows.length; start += 1000) {
                await fixture.database
                    .insert(accessRelationship)
                    .values(rows.slice(start, start + 1000));
            }
        },
    );

    // replicate the relationships of the chain and of the objects whose sets it lists, over 4,000
    const stored = await relationshipsOf(world.home.database);
    const chained = stored.filter((row) => CHAIN.includes(row.objectId));
    const sets = new Set(
        chained.flatMap((row) =>
            row.subjectRelation === null ? [] : [ObjectReference.key(Relationship.subjectOf(row))],
        ),
    );
    const expected = stored.filter(
        (row) =>
            CHAIN.includes(row.objectId) ||
            sets.has(ObjectReference.key(ObjectReference.at("object", row))),
    );
    const remaining = await relationshipsOf(world.app.database);
    expect([
        remaining.map((row) => row.id),
        stored.length > 4000,
        sets.size > 0,
        expected.length < 40,
    ]).toEqual([expected.map((row) => row.id), true, true, true]);
});

/** A generated world of regions, accounts and a space, at a home and the space's replica. */
interface World {
    /** The home database deciding everything. */
    readonly home: Side;
    /** The space's database, replicating the chain above it. */
    readonly app: Side;
    /** Follow one scope of the space's chain to the home's head. */
    readonly follow: (scope: string) => Promise<void>;
}

/** A database and the authorizer deciding over it. */
interface Side {
    /** The database. */
    readonly database: DatabaseConnection;
    /** The authorizer. */
    readonly authorizer: Authorizer;
}

/** Open a world of two regions and two accounts drawn from a generator. */
async function openWorld(
    random: () => number,
    populate: (home: AccessFixture) => Promise<void> = async () => {},
): Promise<World> {
    // open the home and the space's database
    const home = await openDatabase();
    const app = await openDatabase();
    onTestFinished(async () => {
        await Promise.all([home.close(), app.close()]);
    });
    const access = new AccessFixture(home.database);
    await populate(access);

    // store two regions, each person serving one as its machine or home with a generated chance
    for (const index of [0, 1]) {
        const id = `region-${index}`;
        await home.database.insert(regionTable).values({ id });
        for (const person of PEOPLE) {
            if (random() < 0.4) {
                await access.replicateRelationship(
                    Scope.universe.id,
                    region.reference(Scope.universe.id, id),
                    random() < 0.5 ? "machine" : "home",
                    person,
                );
            }
        }
    }

    // store two accounts in generated regions with generated members, the space in the first
    for (const index of [0, 1]) {
        const id = `account-${index}`;
        const object = account.reference(Scope.universe.id, id);
        await home.database.insert(accountTable).values({ id, scope: Scope.universe.id });
        await access.replicateScope(object);
        for (const resided of [0, 1]) {
            if (random() < 0.5) {
                const servers = {
                    ...region.reference(Scope.universe.id, `region-${resided}`),
                    relation: "server",
                };
                await access.replicateRelationship(Scope.universe.id, object, "resident", servers);
            }
        }
        for (const person of PEOPLE) {
            if (random() < 0.3) {
                await access.replicateRelationship(Scope.universe.id, object, "member", person);
            }
        }
    }
    await home.database.insert(spaceTable).values({ id: SPACE, account: "account-0" });
    await access.replicateScope(space.reference("account-0", SPACE), "self");

    // follow the space's chain above it, then record the space as its home
    const authorizer = new Authorizer(policies, homeMappings);
    const feed = new Feed(home.database, [...accessTables, policyTable], { clock: Clock.system });
    const followScope = (scope: string) =>
        follow(app.database, chainOf(app.authorizer, authorizer, scope, SPACE, CHAIN), feed);
    await followScope("account-0");
    await followScope(Scope.universe.id);
    await new AccessFixture(app.database).replicateScope(
        space.reference("account-0", SPACE),
        "self",
    );

    return {
        home: { database: home.database, authorizer },
        app: { database: app.database, authorizer: app.authorizer },
        follow: followScope,
    };
}

/** Decide whether a person serves the space's account, resolved in the space. */
async function serves(side: Side, person: Subject): Promise<boolean> {
    const snapshot = Snapshot.live(side.database);
    const resolved = await side.authorizer.resolve(
        snapshot,
        SPACE,
        AccessContext.of(Caller.of(person), Date.now()),
    );
    const decided = await side.authorizer.check(
        snapshot,
        account.permission("serve"),
        account.reference(Scope.universe.id, "account-0"),
        resolved,
    );

    return decided.isAllowed;
}

/** Read the relationships of a subject a database stores. */
async function relationships(database: DatabaseConnection, subject: Subject): Promise<unknown[]> {
    return database
        .select()
        .from(accessRelationship)
        .where(eq(accessRelationship.subjectId, subject.id));
}

/** Generate numbers between 0 and 1 from a seed, the same for the same seed (mulberry32). */
function generator(seed: number): () => number {
    let state = seed;

    return () => {
        // mix the advanced state into 32 bits
        state = (state + 0x6d2b79f5) | 0;
        let mixed = Math.imul(state ^ (state >>> 15), 1 | state);
        mixed ^= mixed + Math.imul(mixed ^ (mixed >>> 7), 61 | mixed);

        return ((mixed ^ (mixed >>> 14)) >>> 0) / 4_294_967_296;
    };
}
