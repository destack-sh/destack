import { AuditOutbox } from "@destack/audit/outbox";
import { reconciliation, testJournalKey } from "@destack/service/test";
import { expect, onTestFinished, test } from "@destack/test";
import { none, principal, relation, through } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { copyScope, suspendCopy } from "@destack/access/test";
import { asc, type DatabaseConnection } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { Journal } from "@destack/service/database";
import { Replica, Scope } from "@destack/sync";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer, SystemAuthorization } from "../src/server/index.ts";
import { request } from "./schema.ts";

/** The space the copies are kept for. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000002");

/** People of the universe the spaces they joined read. */
const person = defineObject({
    name: "person",
    plural: "persons",
    tier: "global",
    scope: "universe",
    isScope: true,
    fields: { name: field.string() },
    relations: { joined: { subjects: [principal.cell], grantedBy: null } },
    permissions: { read: relation("joined") },
    methods: { list: method.list("read") },
});

/** Profiles living in their person's own scope, read by whoever reads the person. */
const profile = defineObject({
    name: "profile",
    plural: "profiles",
    tier: "global",
    scope: person,
    fields: {
        handle: field.string(),
        email: field.string().guard({ read: "manage" }),
    },
    permissions: { read: through("person", "read"), manage: none() },
    methods: { list: method.list("read") },
});

/** Serve people and profiles over a database. */
function serve(
    database: DatabaseConnection,
    replicas?: NonNullable<ConstructorParameters<typeof ObjectServer>[0]["replicas"]>,
) {
    return new ObjectServer({
        objects: { person, profile },
        database,
        ...(replicas === undefined ? {} : { replicas }),
        journal: new Journal(request, testJournalKey),
        audit: AuditRecorder.service(new AuditOutbox(database), {
            package: person.package,
            service: "test",
        }),
    });
}

test.each(TEST_DIALECTS)(
    "copy the profiles in the scopes of the people a space reads, each decided in its person's scope, on %s",
    async (dialect) => {
        const tables = [...person.tables, ...profile.tables, request];
        const home = await TestDatabase.create(dialect, tables, { isMigrated: true });
        const regional = defineDatabase({ name: "cell", tier: "regional", tables });
        const cell = await TestDatabase.create(dialect, regional, { isMigrated: true });
        onTestFinished(async () => {
            await Promise.all([home.close(), cell.close()]);
        });

        // keep three people at home with a profile each, one of them suspended
        const now = 1;
        const [ada, bob, cy] = [1, 2, 3].map((index) =>
            identifier("person").parse(`person-01996ab0-0000-7000-8000-00000000000${index}`),
        );
        const people = [ada!, bob!, cy!];
        const profileOf = (id: string) =>
            identifier("profile").parse(id.replace("person", "profile"));
        for (const id of people) {
            await copyScope(home.database, person.reference(Scope.universe.id, id));
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
        await suspendCopy(home.database, cy!, now);

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
        const joined = await join(ada!);
        await join(cy!);

        // keep the space's copy of the universe on the cell, decided at home for the space
        const copying: ObjectServer<{ person: typeof person; profile: typeof profile }> = serve(
            cell.database,
            {
                source: {
                    stream: (asked, signal) =>
                        served.source.replicate(
                            asked,
                            { subject: principal.cell.reference(Scope.universe.id, asked.below) },
                            signal,
                        ),
                },
                requests: async () => [copying.source.universeRequest(spaceId)!],
            },
        );
        const follower = copying.controllers().find((each) => each.name === "replica")!;
        const controller = new AbortController();
        const [key] = await follower.list();
        const following = follower.reconcile(key!, reconciliation(controller.signal));
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
                persons: (
                    await cell.database.select().from(person.table).orderBy(asc(person.table.id))
                ).map((row) => row.id),
                profiles: (
                    await cell.database.select().from(profile.table).orderBy(asc(profile.table.id))
                ).map((row) => [row.id, row.scope, row.handle, row.email]),
            };
        };

        // copy the readable people, and the profile of the one whose scope is active, without its guarded field
        expect(await copied()).toEqual({
            persons: [ada, cy],
            profiles: [[profileOf(ada!), ada, "@0", null]],
        });

        // add a person joining the space with their profile
        await join(bob!);
        expect(await copied()).toEqual({
            persons: [ada, bob, cy],
            profiles: [
                [profileOf(ada!), ada, "@0", null],
                [profileOf(bob!), bob, "@1", null],
            ],
        });

        // take a person leaving the space out with their profile
        await system.revoke(person.reference(Scope.universe.id, ada!), joined.id);
        expect(await copied()).toEqual({
            persons: [bob, cy],
            profiles: [[profileOf(bob!), bob, "@1", null]],
        });
    },
);
