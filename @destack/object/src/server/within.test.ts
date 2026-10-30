import type { Environment } from "@destack/environment";
import { testSimulation } from "../test/index.ts";
import { testCallKey } from "../test/context.ts";
import { reconciliation, TestDatabase } from "@destack/db/test";
import { expect, onTestFinished, test } from "@destack/test";
import { accessRelationship, none, principal, relation, through } from "@destack/access";
import { call, defineObject, defineShape, field, Follow, relationship } from "../index.ts";
import { testEvents, serversOf } from "../test/index.ts";
import { TestAccess } from "@destack/access/test";
import { asc, type DatabaseConnection, defineDatabase, eq, Snapshot } from "@destack/db";
import { schema, present, type Identifier } from "@destack/schema";

import { Replica, Scope, type Uplink } from "@destack/sync";
import { ObjectStore } from "./index.ts";
import { tablesOf } from "../stack/db.ts";
import objectPackage from "../package.ts";
import { TestObjectStore } from "../test/index.ts";

/** Refuse a change sent over the uplink, as these replicas receive none. */
const refuseChanges = () => Promise.reject(new Error("the fixture receives no changes"));

/** Refuse a query sent over the uplink, as these replicas send none. */
const refuseQueries = () => Promise.reject(new Error("the fixture answers no queries"));

/** The space the replicas are stored for. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** People of the universe the spaces they joined read. */
const person = defineObject({
    name: "person",
    plural: "people",
    scope: "universe",
    isScope: true,
    fields: { name: field.string() },
    relations: { joined: { subjects: [principal.space], grantedBy: null } },
    permissions: { read: relation("joined") },
});

/** Profiles living in their person's scope, read by whoever reads the person. */
const profile = defineObject({
    name: "profile",
    plural: "profiles",
    scope: person,
    fields: {
        handle: field.string(),
        email: field.string().guard({ read: "manage" }),
    },
    permissions: { read: through("person", "read"), manage: none() },
});

/** Teams of the universe, with no replicas in the space. */
const team = defineObject({
    name: "team",
    plural: "teams",
    scope: "universe",
    isScope: true,
    fields: { name: field.string() },
    permissions: { read: none() },
});

/** Badges living in a person's or a team's scope, each listed to the spaces it is shown to. */
const badge = defineObject({
    name: "badge",
    plural: "badges",
    scope: [person, team],
    fields: { label: field.string() },
    relations: { shown: { subjects: [principal.space], grantedBy: null } },
    permissions: { read: relation("shown") },
});

/** The people and their profiles a space reads. */
const PEOPLE = Follow.of(
    defineShape({
        name: "people",
        parameters: schema.object({}),
        objects: [person, profile],
        queries: () => [person.query({}), profile.query({ scopes: "every" })],
    }),
    { keys: [{ key: { scope: Scope.universe.id } }] },
);

/** The people and the badges a space is shown. */
const SHOWN = Follow.of(
    defineShape({
        name: "shown",
        parameters: schema.object({}),
        objects: [person, badge],
        queries: () => [person.query({}), badge.query({ scopes: "every" })],
    }),
    { keys: [{ key: { scope: Scope.universe.id } }] },
);

/** The people a space reads. */
const MEMBERS = Follow.of(
    defineShape({
        name: "members",
        parameters: schema.object({}),
        objects: [person],
        queries: () => [person.query({})],
    }),
    { keys: [{ key: { scope: Scope.universe.id } }] },
);

/** Serve people and profiles over a database, following them at the universe with an uplink. */
async function serve(world: Environment, database: DatabaseConnection, uplink?: Uplink) {
    return await TestObjectStore.at(principal.space.reference(Scope.universe.id, spaceId), {
        chain: async () =>
            uplink === undefined ? undefined : { kind: "follower", parent: Scope.universe.id },
        environment: world,
        principal: principal.space.reference(Scope.universe.id, spaceId),
        servers: serversOf([person, profile], uplink === undefined ? {} : { follows: [PEOPLE] }),
        database,
        ...(uplink === undefined ? {} : { uplink }),
        callKey: testCallKey,
        events: await testEvents(database, world),
        origin: {
            package: person.package,
        },
    });
}

test("replicate the profiles in the scopes of the people a space reads, each decided in its person's scope", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const tables = [...person.tables, ...profile.tables, ...call.tables];
    const home = await TestDatabase.create(tables, { environment: world, isMigrated: true });
    const declared = defineDatabase({
        name: "space",
        ...tablesOf({ packages: [objectPackage], follows: [PEOPLE], tables: [] }),
    });
    const space = await TestDatabase.create(declared, { environment: world, isMigrated: true });
    onTestFinished(async () => {
        await Promise.all([home.close(), space.close()]);
    });

    // create three people at home with a profile each, one of them suspended
    const now = 1;
    const ada = personId(1);
    const bob = personId(2);
    const cy = personId(3);
    const people = [ada, bob, cy];
    for (const id of people) {
        await new TestAccess(home.database, world).replicateScope(
            person.reference(Scope.universe.id, id),
            person.policy,
        );
    }
    await home.database.transaction(
        async (transaction) =>
            await transaction.insert(person.table).values(
                people.map((id) => ({
                    id,
                    name: id,
                    scope: "universe",
                    createdAt: now,
                    updatedAt: now,
                })),
            ),
    );
    await home.database.transaction(
        async (transaction) =>
            await transaction.insert(profile.table).values(
                people.map((id, index) => ({
                    id: profileId(id),
                    handle: `@${index}`,
                    email: `${index}@example.com`,
                    scope: id,
                    createdAt: now,
                    updatedAt: now,
                })),
            ),
    );
    await new TestAccess(home.database, world).suspendReplica(cy, now);

    // let the space read ada and the suspended cy
    const served = await serve(world, home.database);
    const join = (id: string) =>
        new TestAccess(home.database, world).replicateRelationship(
            person.reference(Scope.universe.id, id),
            "joined",
            principal.space.reference(Scope.universe.id, spaceId),
        );
    const joined = await join(ada);
    await join(cy);

    // store the space's replica of the universe in its database, decided at home for the space
    const replicating: ObjectStore = await serve(world, space.database, {
        receive: refuseChanges,
        call: refuseQueries,
        request: async () => {},
        stream: (asked, signal) =>
            served.source.replicate(
                asked,
                asked.after,
                {
                    subject: principal.space.reference(Scope.universe.id, asked.target),
                },
                signal,
            ),
    });
    const follower = present(
        replicating.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const keys = await follower.list(Snapshot.live(replicating.database));
    const following = keys
        .filter(isUniverseKey)
        .map((key) => follower.reconcile(key, reconciliation(controller.signal)));
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(following);
    });
    const replicated = async () => {
        await Replica.wait(
            space.database,
            Scope.universe.id,
            await home.database.log.position(),
            world.clock.timeout(5000),
        );

        return {
            people: (
                await space.database.select().from(person.table).orderBy(asc(person.table.id))
            ).map((row) => row.id),
            profiles: (
                await space.database.select().from(profile.table).orderBy(asc(profile.table.id))
            ).map((row) => [row.id, row.scope, row.handle, row.email]),
        };
    };

    // replicate the readable people, and the active scope's profile without its guarded field
    expect(await replicated()).toEqual({
        people: [ada, cy],
        profiles: [[profileId(ada), ada, "@0", null]],
    });

    // add a person joining the space with their profile
    await join(bob);
    expect(await replicated()).toEqual({
        people: [ada, bob, cy],
        profiles: [
            [profileId(ada), ada, "@0", null],
            [profileId(bob), bob, "@1", null],
        ],
    });

    // take a person leaving the space out with their profile
    await home.database.transaction(
        async (transaction) =>
            await transaction.delete(accessRelationship).where(eq(accessRelationship.id, joined)),
    );
    expect(await replicated()).toEqual({
        people: [bob, cy],
        profiles: [[profileId(bob), bob, "@1", null]],
    });
});

test("replicate the badges a space is shown from the scopes of teams without replicas here, and withhold the badges it is not shown", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // create ada and a team at home, the team's badges and ada's badges
    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { person, team, badge }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    const space = await TestDatabase.create(
        defineDatabase({
            name: "space",
            ...tablesOf({ packages: [objectPackage], follows: [SHOWN], tables: [] }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), space.close()]);
    });
    const now = 1;
    const ada = personId(1);
    const crew = schema.identifier("team").parse("team-01996ab0-0000-7000-8000-000000000001");
    await new TestAccess(home.database, world).replicateScope(
        person.reference(Scope.universe.id, ada),
        person.policy,
    );
    await new TestAccess(home.database, world).replicateScope(
        team.reference(Scope.universe.id, crew),
        team.policy,
    );
    const at = { createdAt: now, updatedAt: now };
    await home.database.transaction(
        async (transaction) =>
            await transaction
                .insert(person.table)
                .values({ id: ada, name: "Ada", scope: "universe", ...at }),
    );
    await home.database.transaction(
        async (transaction) =>
            await transaction
                .insert(team.table)
                .values({ id: crew, name: "Crew", scope: "universe", ...at }),
    );
    const badges = [
        [badgeId(1), crew, "shown in the team"],
        [badgeId(2), crew, "hidden in the team"],
        [badgeId(3), ada, "shown in ada's scope"],
    ] as const;
    await home.database.transaction(
        async (transaction) =>
            await transaction
                .insert(badge.table)
                .values(badges.map(([id, scope, label]) => ({ id, scope, label, ...at }))),
    );

    // let the space read ada, and show it the first and the last badge
    const served = await TestObjectStore.at(principal.space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: principal.space.reference(Scope.universe.id, spaceId),
        servers: serversOf([person, team, badge]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: person.package },
    });
    const fixture = new TestAccess(home.database, world);
    const spaceSubject = principal.space.reference(Scope.universe.id, spaceId);
    await fixture.replicateRelationship(
        person.reference(Scope.universe.id, ada),
        "joined",
        spaceSubject,
    );
    for (const [id, scope] of [badges[0], badges[2]]) {
        await fixture.replicateRelationship(badge.reference(scope, id), "shown", spaceSubject);
    }

    // replicate the universe the space reads into its database, which serves only people and badges
    const replicating: ObjectStore = await TestObjectStore.at(
        principal.space.reference(Scope.universe.id, spaceId),
        {
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            environment: world,
            principal: principal.space.reference(Scope.universe.id, spaceId),
            servers: serversOf([person, badge], { follows: [SHOWN] }),
            database: space.database,
            uplink: {
                receive: refuseChanges,
                call: refuseQueries,
                request: async () => {},
                stream: (asked, signal) =>
                    served.source.replicate(
                        asked,
                        asked.after,
                        {
                            subject: principal.space.reference(Scope.universe.id, asked.target),
                        },
                        signal,
                    ),
            },
            callKey: testCallKey,
            events: await testEvents(space.database, world),
            origin: { package: person.package },
        },
    );
    await follow(replicating, home.database, space.database);

    // list the shown badges of the team and of ada, and withhold the hidden one
    const replicated = await space.database.select().from(badge.table).orderBy(asc(badge.table.id));
    expect(replicated.map((row) => [row.id, row.scope, row.label])).toEqual([
        [badgeId(1), crew, "shown in the team"],
        [badgeId(3), ada, "shown in ada's scope"],
    ]);
});

test("replicate each person once the space may read them at the change's position, though the home lists relationships as objects", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const home = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { person }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    const space = await TestDatabase.create(
        defineDatabase({
            name: "space",
            ...tablesOf({ packages: [objectPackage], follows: [MEMBERS], tables: [] }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(async () => {
        await Promise.all([home.close(), space.close()]);
    });

    // create ada, who joined the space, and bob, who did not yet
    const at = { scope: "universe", createdAt: 1, updatedAt: 1 };
    const spaceSubject = principal.space.reference(Scope.universe.id, spaceId);
    const [ada, bob, cy] = [personId(1), personId(2), personId(3)];
    const join = (database: DatabaseConnection, id: string) =>
        new TestAccess(database, world).replicateRelationship(
            person.reference(Scope.universe.id, id),
            "joined",
            spaceSubject,
        );
    for (const id of [ada, bob]) {
        await new TestAccess(home.database, world).replicateScope(
            person.reference(Scope.universe.id, id),
            person.policy,
        );
        await home.database.transaction(
            async (transaction) =>
                await transaction.insert(person.table).values({ id, name: id, ...at }),
        );
    }
    const joined = await join(home.database, ada);

    // serve the people and their relationships as objects, and replica what the space reads
    const served = await TestObjectStore.at(spaceSubject, {
        chain: async () => undefined,
        environment: world,
        principal: spaceSubject,
        servers: serversOf([person, relationship]),
        database: home.database,
        callKey: testCallKey,
        events: await testEvents(home.database, world),
        origin: { package: person.package },
    });
    const replicating: ObjectStore = await TestObjectStore.at(spaceSubject, {
        chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
        environment: world,
        principal: spaceSubject,
        servers: serversOf([person], { follows: [MEMBERS] }),
        database: space.database,
        uplink: {
            receive: refuseChanges,
            call: refuseQueries,
            request: async () => {},
            stream: (asked, signal) =>
                served.source.replicate(
                    asked,
                    asked.after,
                    {
                        subject: principal.space.reference(Scope.universe.id, asked.target),
                    },
                    signal,
                ),
        },
        callKey: testCallKey,
        events: await testEvents(space.database, world),
        origin: { package: person.package },
    });
    await follow(replicating, home.database, space.database);
    const replicated = async () => {
        await Replica.wait(
            space.database,
            Scope.universe.id,
            await home.database.log.position(),
            world.clock.timeout(5000),
        );
        const rows = await space.database.select().from(person.table).orderBy(asc(person.table.id));

        return rows.map((row) => row.id);
    };
    expect(await replicated()).toEqual([ada]);

    // let bob join the space, then create cy joining it in one commit
    await join(home.database, bob);
    await home.database.transaction(async (transaction) => {
        await new TestAccess(transaction, world).replicateScope(
            person.reference(Scope.universe.id, cy),
            person.policy,
        );
        await transaction.insert(person.table).values({ id: cy, name: cy, ...at });
        await join(transaction, cy);
    });
    expect(await replicated()).toEqual([ada, bob, cy]);

    // take ada out once she leaves the space
    await home.database.transaction(
        async (transaction) =>
            await transaction.delete(accessRelationship).where(eq(accessRelationship.id, joined)),
    );
    expect(await replicated()).toEqual([bob, cy]);
});

/** Follow a server's replicas until they are at the home database's position, until test end. */
async function follow(
    replicating: ObjectStore,
    home: DatabaseConnection,
    space: DatabaseConnection,
): Promise<void> {
    // run the replica controller's key of the universe's rows
    const follower = present(
        replicating.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const keys = await follower.list(Snapshot.live(replicating.database));
    const following = keys
        .filter(isUniverseKey)
        .map((key) => follower.reconcile(key, reconciliation(controller.signal)));
    onTestFinished(async () => {
        controller.abort();
        await Promise.allSettled(following);
    });

    // wait for the replica to arrive at home
    await Replica.wait(
        space,
        Scope.universe.id,
        await home.log.position(),
        replicating.environment.clock.timeout(5000),
    );
}

/** Report whether a replica key follows a shape at the universe. */
function isUniverseKey(key: string): boolean {
    return key.endsWith(`:${Scope.universe.id} ${Scope.universe.id}`);
}

/** Build the identifier of a numbered badge. */
function badgeId(index: number): Identifier<"badge"> {
    return schema.identifier("badge").parse(`badge-01996ab0-0000-7000-8000-00000000000${index}`);
}

/** Build the identifier of a numbered person. */
function personId(index: number): Identifier<"person"> {
    return schema.identifier("person").parse(`person-01996ab0-0000-7000-8000-00000000000${index}`);
}

/** Build the identifier of a person's profile. */
function profileId(id: string): Identifier<"profile"> {
    return schema.identifier("profile").parse(id.replace("person", "profile"));
}
