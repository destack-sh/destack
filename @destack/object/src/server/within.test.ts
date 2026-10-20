import { reconciliation, testCallKey } from "@destack/runtime/test";
import { expect, onTestFinished, test } from "@destack/test";
import { accessRelationship, none, principal, relation, through } from "@destack/access";
import { call, defineObject, field, relationship } from "../index.ts";
import { testEvents, serversOf } from "../test/index.ts";
import { AccessFixture } from "@destack/access/test";
import { asc, type DatabaseConnection, defineDatabase, eq, Snapshot } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, present, type Identifier } from "@destack/schema";

import { Replica, Scope, type Uplink } from "@destack/sync";
import { ObjectSink, ObjectStore } from "./index.ts";
import { Clock } from "@destack/environment";

/** Refuse a change sent over the uplink, as these replicas receive none. */
const refuseChanges = () => Promise.reject(new Error("the fixture receives no changes"));

/** Refuse a query sent over the uplink, as these replicas send none. */
const refuseQueries = () => Promise.reject(new Error("the fixture answers no queries"));

/** The space the replicas are held for. */
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

/** Profiles living in their person's own scope, read by whoever reads the person. */
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

/** Serve people and profiles over a database. */
async function serve(database: DatabaseConnection, uplink?: Uplink) {
    return new ObjectStore({
        chain: async () =>
            uplink === undefined ? undefined : { kind: "follower", parent: Scope.universe.id },
        clock: Clock.system,
        principal: principal.space.reference(Scope.universe.id, spaceId),
        servers: serversOf([person, profile]),
        database,
        ...(uplink === undefined ? {} : { uplink }),
        callKey: testCallKey,
        events: await testEvents(database),
        origin: {
            package: person.package,
        },
    });
}

test.each(TEST_DIALECTS)(
    "replicate the profiles in the scopes of the people a space reads, each decided in its person's scope, on %s",
    async (dialect) => {
        const tables = [...person.tables, ...profile.tables, ...call.tables];
        const home = await TestDatabase.create(dialect, tables, { isMigrated: true });
        const declared = defineDatabase({
            name: "space",
            tables: call.tables,
            replicated: [...person.tables, ...profile.tables],
        });
        const space = await TestDatabase.create(dialect, declared, { isMigrated: true });
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
            await new AccessFixture(home.database).replicateScope(
                person.reference(Scope.universe.id, id),
            );
        }
        await home.database.insert(person.table).values(
            people.map((id) => ({
                id,
                name: id,
                scope: "universe",
                createdAt: now,
                updatedAt: now,
            })),
        );
        await home.database.insert(profile.table).values(
            people.map((id, index) => ({
                id: profileId(id),
                handle: `@${index}`,
                email: `${index}@example.com`,
                scope: id,
                createdAt: now,
                updatedAt: now,
            })),
        );
        await new AccessFixture(home.database).suspendReplica(cy, now);

        // let the space read ada and the suspended cy
        const served = await serve(home.database);
        const join = (id: string) =>
            new AccessFixture(home.database).replicateRelationship(
                id,
                person.reference(Scope.universe.id, id),
                "joined",
                principal.space.reference(Scope.universe.id, spaceId),
            );
        const joined = await join(ada);
        await join(cy);

        // store the space's replica of the universe in its database, decided at home for the space
        const replicating: ObjectStore = await serve(space.database, {
            receive: refuseChanges,
            call: refuseQueries,
            confirm: async () => {},
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
                AbortSignal.timeout(5000),
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
        await home.database.delete(accessRelationship).where(eq(accessRelationship.id, joined));
        expect(await replicated()).toEqual({
            people: [bob, cy],
            profiles: [[profileId(bob), bob, "@1", null]],
        });
    },
);

test.each(TEST_DIALECTS)(
    "replicate the badges a space is shown from the scopes of teams without replicas here, and withhold the badges it is not shown, on %s",
    async (dialect) => {
        // create ada and a team at home, the team's badges and ada's own
        const home = await TestDatabase.create(
            dialect,
            [...person.tables, ...team.tables, ...badge.tables, ...call.tables],
            { isMigrated: true },
        );
        const space = await TestDatabase.create(
            dialect,
            defineDatabase({
                name: "space",
                tables: call.tables,
                replicated: [...person.tables, ...badge.tables],
            }),
            { isMigrated: true },
        );
        onTestFinished(async () => {
            await Promise.all([home.close(), space.close()]);
        });
        const now = 1;
        const ada = personId(1);
        const crew = schema.identifier("team").parse("team-01996ab0-0000-7000-8000-000000000001");
        await new AccessFixture(home.database).replicateScope(
            person.reference(Scope.universe.id, ada),
        );
        await new AccessFixture(home.database).replicateScope(
            team.reference(Scope.universe.id, crew),
        );
        const at = { createdAt: now, updatedAt: now };
        await home.database
            .insert(person.table)
            .values({ id: ada, name: "Ada", scope: "universe", ...at });
        await home.database
            .insert(team.table)
            .values({ id: crew, name: "Crew", scope: "universe", ...at });
        const badges = [
            [badgeId(1), crew, "shown in the team"],
            [badgeId(2), crew, "hidden in the team"],
            [badgeId(3), ada, "shown in ada's scope"],
        ] as const;
        await home.database
            .insert(badge.table)
            .values(badges.map(([id, scope, label]) => ({ id, scope, label, ...at })));

        // let the space read ada, and show it the first and the last badge
        const served = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: principal.space.reference(Scope.universe.id, spaceId),
            servers: serversOf([person, team, badge]),
            database: home.database,
            callKey: testCallKey,
            events: await testEvents(home.database),
            origin: { package: person.package },
        });
        const fixture = new AccessFixture(home.database);
        const spaceSubject = principal.space.reference(Scope.universe.id, spaceId);
        await fixture.replicateRelationship(
            ada,
            person.reference(Scope.universe.id, ada),
            "joined",
            spaceSubject,
        );
        for (const [id, scope] of [badges[0], badges[2]]) {
            await fixture.replicateRelationship(
                scope,
                badge.reference(scope, id),
                "shown",
                spaceSubject,
            );
        }

        // replicate the universe the space reads into its database, which serves only people and badges
        const replicating: ObjectStore = new ObjectStore({
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            clock: Clock.system,
            principal: principal.space.reference(Scope.universe.id, spaceId),
            servers: serversOf([person, badge]),
            database: space.database,
            uplink: {
                receive: refuseChanges,
                call: refuseQueries,
                confirm: async () => {},
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
            events: await testEvents(space.database),
            origin: { package: person.package },
        });
        await follow(replicating, home.database, space.database);

        // list the shown badges of the team and of ada, and withhold the hidden one
        const replicated = await space.database
            .select()
            .from(badge.table)
            .orderBy(asc(badge.table.id));
        expect(replicated.map((row) => [row.id, row.scope, row.label])).toEqual([
            [badgeId(1), crew, "shown in the team"],
            [badgeId(3), ada, "shown in ada's scope"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "replicate each person once the space may read them at the change's position, though the home lists relationships as objects, on %s",
    async (dialect) => {
        const home = await TestDatabase.create(dialect, [...person.tables, ...call.tables], {
            isMigrated: true,
        });
        const space = await TestDatabase.create(
            dialect,
            defineDatabase({ name: "space", tables: call.tables, replicated: person.tables }),
            { isMigrated: true },
        );
        onTestFinished(async () => {
            await Promise.all([home.close(), space.close()]);
        });

        // create ada, who joined the space, and bob, who did not yet
        const at = { scope: "universe", createdAt: 1, updatedAt: 1 };
        const spaceSubject = principal.space.reference(Scope.universe.id, spaceId);
        const [ada, bob, cy] = [personId(1), personId(2), personId(3)];
        const join = (database: DatabaseConnection, id: string) =>
            new AccessFixture(database).replicateRelationship(
                id,
                person.reference(Scope.universe.id, id),
                "joined",
                spaceSubject,
            );
        for (const id of [ada, bob]) {
            await new AccessFixture(home.database).replicateScope(
                person.reference(Scope.universe.id, id),
            );
            await home.database.insert(person.table).values({ id, name: id, ...at });
        }
        const joined = await join(home.database, ada);

        // serve the people and their relationships as objects, and replica what the space reads
        const served = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spaceSubject,
            servers: serversOf([person, relationship]),
            database: home.database,
            callKey: testCallKey,
            events: await testEvents(home.database),
            origin: { package: person.package },
        });
        const replicating: ObjectStore = new ObjectStore({
            chain: async () => ({ kind: "follower", parent: Scope.universe.id }),
            clock: Clock.system,
            principal: spaceSubject,
            servers: serversOf([person]),
            database: space.database,
            uplink: {
                receive: refuseChanges,
                call: refuseQueries,
                confirm: async () => {},
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
            events: await testEvents(space.database),
            origin: { package: person.package },
        });
        await follow(replicating, home.database, space.database);
        const replicated = async () => {
            await Replica.wait(
                space.database,
                Scope.universe.id,
                await home.database.log.position(),
                AbortSignal.timeout(5000),
            );
            const rows = await space.database
                .select()
                .from(person.table)
                .orderBy(asc(person.table.id));

            return rows.map((row) => row.id);
        };
        expect(await replicated()).toEqual([ada]);

        // let bob join the space, then create cy joining it in one commit
        await join(home.database, bob);
        await home.database.transaction(async (transaction) => {
            await new AccessFixture(transaction).replicateScope(
                person.reference(Scope.universe.id, cy),
            );
            await transaction.insert(person.table).values({ id: cy, name: cy, ...at });
            await join(transaction, cy);
        });
        expect(await replicated()).toEqual([ada, bob, cy]);

        // take ada out once she leaves the space
        await home.database.delete(accessRelationship).where(eq(accessRelationship.id, joined));
        expect(await replicated()).toEqual([bob, cy]);
    },
);

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

    // wait for the replica to reach home
    await Replica.wait(
        space,
        Scope.universe.id,
        await home.log.position(),
        AbortSignal.timeout(5000),
    );
}

/** Report whether a replica key follows the space's query of the universe's rows. */
function isUniverseKey(key: string): boolean {
    return key === `${ObjectSink.queriesName(spaceId)} ${Scope.universe.id}`;
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
