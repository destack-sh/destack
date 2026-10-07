import { reconciliation, testCallKey } from "@destack/service/test";
import { expect, onTestFinished, test } from "@destack/test";
import { none, principal, relation, through } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { AccessFixture } from "@destack/access/test";
import { asc, type DatabaseConnection, defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, present, type Identifier } from "@destack/schema";

import { Replica, Scope } from "@destack/sync";
import { defineObject, field } from "../index.ts";
import { ObjectServer, Subscriber, SystemAuthorization } from "./index.ts";

/** Refuse a change sent over the uplink, as these copies receive none. */
const refuseChanges = () => Promise.reject(new Error("the fixture receives no changes"));

/** The space the copies are kept for. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** People of the universe the spaces they joined read. */
const person = defineObject({
    name: "person",
    plural: "people",
    scope: "universe",
    isScope: true,
    fields: { name: field.string() },
    relations: { joined: { subjects: [principal.cell], grantedBy: null } },
    permissions: { read: relation("joined") },
    methods: (method) => ({ list: method.list("read") }),
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
    methods: (method) => ({ list: method.list("read") }),
});

/** Teams of the universe, which the cell keeps no copies of. */
const team = defineObject({
    name: "team",
    plural: "teams",
    scope: "universe",
    isScope: true,
    fields: { name: field.string() },
    permissions: { read: none() },
    methods: (method) => ({ list: method.list("read") }),
});

/** Badges living in a person's or a team's scope, each listed to the cells it is shown to. */
const badge = defineObject({
    name: "badge",
    plural: "badges",
    scope: [person, team],
    fields: { label: field.string() },
    relations: { shown: { subjects: [principal.cell], grantedBy: null } },
    permissions: { read: relation("shown") },
    methods: (method) => ({ list: method.list("read") }),
});

/** Serve people and profiles over a database. */
function serve(
    database: DatabaseConnection,
    subscriber?: NonNullable<ConstructorParameters<typeof ObjectServer>[0]["subscriber"]>,
) {
    return new ObjectServer({
        objects: { person, profile },
        database,
        ...(subscriber === undefined ? {} : { subscriber }),
        callKey: testCallKey,
        origin: {
            package: person.package,
            service: "test",
        },
    });
}

test.each(TEST_DIALECTS)(
    "copy the profiles in the scopes of the people a space reads, each decided in its person's scope, on %s",
    async (dialect) => {
        const tables = [...person.tables, ...profile.tables, journal];
        const home = await TestDatabase.create(dialect, tables, { isMigrated: true });
        const regional = defineDatabase({
            name: "cell",
            tables: [journal],
            copies: [...person.tables, ...profile.tables],
        });
        const cell = await TestDatabase.create(dialect, regional, { isMigrated: true });
        onTestFinished(async () => {
            await Promise.all([home.close(), cell.close()]);
        });

        // keep three people at home with a profile each, one of them suspended
        const now = 1;
        const ada = personOf(1);
        const bob = personOf(2);
        const cy = personOf(3);
        const people = [ada, bob, cy];
        for (const id of people) {
            await new AccessFixture(home.database).copyScope(
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
                id: profileOf(id),
                handle: `@${index}`,
                email: `${index}@example.com`,
                scope: id,
                createdAt: now,
                updatedAt: now,
            })),
        );
        await new AccessFixture(home.database).suspendCopy(cy, now);

        // let the space read ada and the suspended cy
        const served = serve(home.database);
        const system = await SystemAuthorization.open(
            served.authorizer,
            home.database,
            Scope.universe.id,
            now,
        );
        const join = (id: string) =>
            system.grant({
                object: person.reference(Scope.universe.id, id),
                relation: "joined",
                subject: principal.cell.reference(Scope.universe.id, spaceId),
            });
        const joined = await join(ada);
        await join(cy);

        // keep the space's copy of the universe on the cell, decided at home for the space
        const copying: ObjectServer<{ person: typeof person; profile: typeof profile }> = serve(
            cell.database,
            Subscriber.of(
                {
                    receive: refuseChanges,
                    stream: (asked, signal) =>
                        served.source.replicate(
                            asked,
                            asked.after,
                            { subject: principal.cell.reference(Scope.universe.id, asked.below) },
                            signal,
                        ),
                },
                async () => [
                    present(copying.source.universeSubscription(spaceId), "the universe request"),
                ],
            ),
        );
        const follower = present(
            copying.controllers().find((each) => each.name === "replica"),
            "the replica controller",
        );
        const controller = new AbortController();
        const [key] = await follower.list();
        const following = follower.reconcile(
            present(key, "the replica key"),
            reconciliation(controller.signal),
        );
        onTestFinished(async () => {
            controller.abort();
            await following.catch(() => undefined);
        });
        const copied = async () => {
            await Replica.reach(
                cell.database,
                Scope.universe.id,
                await home.database.log.position(),
                AbortSignal.timeout(5000),
            );

            return {
                people: (
                    await cell.database.select().from(person.table).orderBy(asc(person.table.id))
                ).map((row) => row.id),
                profiles: (
                    await cell.database.select().from(profile.table).orderBy(asc(profile.table.id))
                ).map((row) => [row.id, row.scope, row.handle, row.email]),
            };
        };

        // copy the readable people, and the profile of the one whose scope is active, without its guarded field
        expect(await copied()).toEqual({
            people: [ada, cy],
            profiles: [[profileOf(ada), ada, "@0", null]],
        });

        // add a person joining the space with their profile
        await join(bob);
        expect(await copied()).toEqual({
            people: [ada, bob, cy],
            profiles: [
                [profileOf(ada), ada, "@0", null],
                [profileOf(bob), bob, "@1", null],
            ],
        });

        // take a person leaving the space out with their profile
        await system.revoke(person.reference(Scope.universe.id, ada), joined.id);
        expect(await copied()).toEqual({
            people: [bob, cy],
            profiles: [[profileOf(bob), bob, "@1", null]],
        });
    },
);

test.each(TEST_DIALECTS)(
    "copy the badges a space is shown from the scopes of teams it keeps no copies of, and withhold the badges it is not shown, on %s",
    async (dialect) => {
        // keep ada and a team at home, the team's badges and ada's own
        const home = await TestDatabase.create(
            dialect,
            [...person.tables, ...team.tables, ...badge.tables, journal],
            { isMigrated: true },
        );
        const cell = await TestDatabase.create(
            dialect,
            defineDatabase({
                name: "cell",
                tables: [journal],
                copies: [...person.tables, ...badge.tables],
            }),
            { isMigrated: true },
        );
        onTestFinished(async () => {
            await Promise.all([home.close(), cell.close()]);
        });
        const now = 1;
        const ada = personOf(1);
        const crew = schema.identifier("team").parse("team-01996ab0-0000-7000-8000-000000000001");
        await new AccessFixture(home.database).copyScope(person.reference(Scope.universe.id, ada));
        await new AccessFixture(home.database).copyScope(team.reference(Scope.universe.id, crew));
        const at = { createdAt: now, updatedAt: now };
        await home.database
            .insert(person.table)
            .values({ id: ada, name: "Ada", scope: "universe", ...at });
        await home.database
            .insert(team.table)
            .values({ id: crew, name: "Crew", scope: "universe", ...at });
        const badges = [
            [badgeOf(1), crew, "shown in the team"],
            [badgeOf(2), crew, "hidden in the team"],
            [badgeOf(3), ada, "shown in ada's scope"],
        ] as const;
        await home.database
            .insert(badge.table)
            .values(badges.map(([id, scope, label]) => ({ id, scope, label, ...at })));

        // let the space read ada, and show it the first and the last badge
        const served = new ObjectServer({
            objects: { person, team, badge },
            database: home.database,
            callKey: testCallKey,
            origin: { package: person.package, service: "test" },
        });
        const system = await SystemAuthorization.open(
            served.authorizer,
            home.database,
            Scope.universe.id,
            now,
        );
        const cellOfSpace = principal.cell.reference(Scope.universe.id, spaceId);
        await system.grant({
            object: person.reference(Scope.universe.id, ada),
            relation: "joined",
            subject: cellOfSpace,
        });
        for (const [id, scope] of [badges[0], badges[2]]) {
            await system.grant({
                object: badge.reference(scope, id),
                relation: "shown",
                subject: cellOfSpace,
            });
        }

        // copy the universe the space reads onto the cell, which serves people and badges alone
        const copying: ObjectServer<{ person: typeof person; badge: typeof badge }> =
            new ObjectServer({
                objects: { person, badge },
                database: cell.database,
                subscriber: Subscriber.of(
                    {
                        receive: refuseChanges,
                        stream: (asked, signal) =>
                            served.source.replicate(
                                asked,
                                asked.after,
                                {
                                    subject: principal.cell.reference(
                                        Scope.universe.id,
                                        asked.below,
                                    ),
                                },
                                signal,
                            ),
                    },
                    async () => [
                        present(
                            copying.source.universeSubscription(spaceId),
                            "the universe request",
                        ),
                    ],
                ),
                callKey: testCallKey,
                origin: { package: person.package, service: "test" },
            });
        await follow(copying, home.database, cell.database);

        // keep the shown badges of the team and of ada, and withhold the hidden one
        const copied = await cell.database.select().from(badge.table).orderBy(asc(badge.table.id));
        expect(copied.map((row) => [row.id, row.scope, row.label])).toEqual([
            [badgeOf(1), crew, "shown in the team"],
            [badgeOf(3), ada, "shown in ada's scope"],
        ]);
    },
);

/** Follow a server's copies until they reach the home database's position, stopping once the test finishes. */
async function follow(
    copying: ObjectServer,
    home: DatabaseConnection,
    cell: DatabaseConnection,
): Promise<void> {
    // run the replica controller's one key
    const follower = present(
        copying.controllers().find((each) => each.name === "replica"),
        "the replica controller",
    );
    const controller = new AbortController();
    const [key] = await follower.list();
    const following = follower.reconcile(
        present(key, "the replica key"),
        reconciliation(controller.signal),
    );
    onTestFinished(async () => {
        controller.abort();
        await following.catch(() => undefined);
    });

    // wait for the copy to reach home
    await Replica.reach(
        cell,
        Scope.universe.id,
        await home.log.position(),
        AbortSignal.timeout(5000),
    );
}

/** Build the identifier of a numbered badge. */
function badgeOf(index: number): Identifier<"badge"> {
    return schema.identifier("badge").parse(`badge-01996ab0-0000-7000-8000-00000000000${index}`);
}

/** Build the identifier of a numbered person. */
function personOf(index: number): Identifier<"person"> {
    return schema.identifier("person").parse(`person-01996ab0-0000-7000-8000-00000000000${index}`);
}

/** Build the identifier of a person's profile. */
function profileOf(id: string): Identifier<"profile"> {
    return schema.identifier("profile").parse(id.replace("person", "profile"));
}
