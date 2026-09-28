import { KeyIndex, ObjectServer } from "../src/server/index.ts";
import { keyEntry } from "../src/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import type { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection, Dialect } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { identifier } from "@destack/schema";
import { Caller } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";
import { v7 } from "uuid";
import { profile, profilesDatabase, profilesJournal, profilesService } from "./fixture/profiles.ts";
import { principal } from "@destack/access";
import { openSpace } from "./fixture/space.ts";

/** The package serving the profiles. */
const audience = PackageId.parse("package-01a0d5eb-fb8a-74f4-ba37-8a4d6970e239");

test.each(TEST_DIALECTS)(
    "keep handles unique across spaces in separate databases on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [keyEntry], { isMigrated: true });
        onTestFinished(() => storage.close());
        const index = new KeyIndex(storage.database);
        const east = await serveProfiles(dialect, index);
        const west = await serveProfiles(dialect, index);
        const handle = async (name: string) => (await index.resolve(profile, "handle", [name]))?.id;

        // let one space claim a handle, and refuse it to another space
        const ada = await east.alice.profile.create({
            spaceId: east.spaceId,
            requestId: RequestId.create(),
            handle: "ada",
        });
        expect(await handle("ada")).toBe(ada.id);
        await expect(
            west.bob.profile.create({
                spaceId: west.spaceId,
                requestId: RequestId.create(),
                handle: "ada",
            }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "handle is taken" });

        // release the handle a profile moves away from, and the handle of a deleted profile
        await east.alice.profile.update({
            spaceId: east.spaceId,
            id: ada.id,
            requestId: RequestId.create(),
            revision: ada.revision,
            handle: "lovelace",
        });
        const bob = await west.bob.profile.create({
            spaceId: west.spaceId,
            requestId: RequestId.create(),
            handle: "ada",
        });
        await east.alice.profile.delete({
            spaceId: east.spaceId,
            id: ada.id,
            requestId: RequestId.create(),
            revision: ada.revision + 1,
        });
        expect([await handle("ada"), await handle("lovelace")]).toEqual([bob.id, undefined]);

        // release the handle a profile clears
        await west.bob.profile.update({
            spaceId: west.spaceId,
            id: bob.id,
            requestId: RequestId.create(),
            revision: bob.revision,
            handle: null,
        });
        expect(await handle("ada")).toBeUndefined();

        // hold no key once every holder let go: the failed claim left no reservation behind
        expect(await storage.database.select().from(keyEntry)).toEqual([]);
    },
);

test.each(TEST_DIALECTS)(
    "follow writes outside the object server and finish expired reservations on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [keyEntry], { isMigrated: true });
        onTestFinished(() => storage.close());
        const index = new KeyIndex(storage.database);
        const east = await serveProfiles(dialect, index);
        const handle = async (name: string) => (await index.resolve(profile, "handle", [name]))?.id;

        // follow a profile a controller writes directly, and refuse a second holder of its handle
        const after = (await east.database.log.position()).sequence;
        const now = Date.now();
        const row = {
            scope: east.spaceId,
            owner: "carol",
            handle: "carol",
            createdAt: now,
            updatedAt: now,
        };
        await east.database
            .insert(profile.table)
            .values({ ...row, id: identifier("profile").parse(`profile-${v7()}`) });
        await east.database
            .insert(profile.table)
            .values({ ...row, id: identifier("profile").parse(`profile-${v7()}`) });
        const { changes } = await east.database.log.read({ tables: [profile.table], after });
        await index.apply(profile, changes[0]!);
        expect(await handle("carol")).toBe(changes[0]!.key.id);
        await expect(index.apply(profile, changes[1]!)).rejects.toMatchObject({
            code: "CONFLICT",
            message: `handle of profile ${changes[1]!.key.id} is taken by ${changes[0]!.key.id}`,
        });

        // keep an existing object's key, release a failed one, and wait for the later one
        const kept = identifier("profile").parse(`profile-${v7()}`);
        await east.database.insert(profile.table).values({ ...row, id: kept, handle: "kept" });
        const reserved = {
            index: `${profile.policy.definition.packageId}/profile/handle`,
            scope: east.spaceId,
            state: "reserved" as const,
            requestId: "request",
            expiresAt: now,
        };
        await storage.database.insert(keyEntry).values([
            { ...reserved, key: '[null,"kept"]', objectId: kept },
            { ...reserved, key: '[null,"lost"]', objectId: `profile-${v7()}` },
            {
                ...reserved,
                key: '[null,"later"]',
                objectId: `profile-${v7()}`,
                expiresAt: now + 1000,
            },
        ]);
        const next = await index.finish([profile], east.database, now);
        expect([next, await handle("kept"), await handle("lost")]).toEqual([1000, kept, undefined]);

        // finish through the controller
        const controller = index.controller({ database: east.database, objects: [profile] });
        const again = await controller.reconcile("reservations");
        expect([controller.watches, await controller.list(), again! > 0 && again! <= 1000]).toEqual(
            [[profile.table], ["reservations"], true],
        );
    },
);

/** Serve one space's profiles in its own database, returning a client per user. */
async function serveProfiles(dialect: Dialect, index: KeyIndex) {
    // hold the space's profiles in a database of its own
    const spaceId = identifier("space").parse(`space-${v7()}`);
    const storage = await TestDatabase.create(dialect, profilesDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // authenticate each request as the user its bearer credential names
    const server = Server.start({
        ...ObjectServer.serve(profilesService, {
            journal: profilesJournal,
            database: storage.database,
            index,
            audit: () =>
                ({
                    begin: () => ({}),
                    append: async () => {},
                    complete: () => ({}),
                    record: async () => {},
                }) as unknown as AuditRecorder<DatabaseConnection>,
        }),
        audience,
        scope: spaceId,
        resources: new ResourceContext(),
        health: new Health("profiles"),
        drainTimeout: 1000,
        authorizeHost: async () => {},
        authenticate: async (request) => {
            const id = request.headers.get("authorization")!.slice("Bearer ".length);
            const subject = principal.user.reference("global", id);
            const now = Date.now();

            return new Caller({
                subject,
                subjects: [subject],
                credential: { kind: "user", id },
                audience,
                scope: spaceId,
                verifiedAt: now,
                expiresAt: now + 60_000,
            });
        },
    });
    onTestFinished(() => server.close());
    const connect = (user: string) =>
        createClient(profilesService.router, {
            url: "https://profiles.test",
            headers: { authorization: `Bearer ${user}` },
            fetch: (request: Request) => server.fetch(request),
        });

    return { spaceId, database: storage.database, alice: connect("alice"), bob: connect("bob") };
}
