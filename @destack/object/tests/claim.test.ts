import { outbox } from "@destack/service/outbox";
import { Scope } from "@destack/sync";
import { Change, Snapshot, type Dialect } from "@destack/db";
import { ClaimController, ObjectServer } from "../src/server/index.ts";
import { defineObject, field } from "../src/index.ts";
import {
    directoryTables,
    DirectoryStore,
    RESERVATION_MILLISECONDS,
    type Directory,
} from "@destack/directory";
import { expect, onTestFinished, test } from "@destack/test";
import { vi } from "vitest";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { aligned, schema, present } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { createClient } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";
import { v7 } from "uuid";
import { profile, profilesDatabase, profilesService } from "./fixture/profile.ts";
import { none, principal } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { openSpace } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";

/** The package serving the profiles. */
const audience = PackageId.parse("package-01a0d5eb-fb8a-74f4-ba37-8a4d6970e239");

test.each(TEST_DIALECTS)(
    "keep handles unique across spaces in separate databases on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [outbox, ...directoryTables], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const east = await serveProfiles(dialect, directory);
        const west = await serveProfiles(dialect, directory);
        const handle = async (name: string) =>
            (await profile.lookup(directory, "handle", [name]))?.id;

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

        // keep no reservation once every owner let go: the failed claim left none behind
        expect(await directory.expired([profile.index("handle")])).toEqual({ claims: [] });
    },
);

test.each(TEST_DIALECTS)(
    "claim the names of writes outside the object server and finish expired reservations on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [outbox, ...directoryTables], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        const directory = new DirectoryStore(storage.database);
        const east = await serveProfiles(dialect, directory);
        const handle = async (name: string) =>
            (await profile.lookup(directory, "handle", [name]))?.id;

        // follow a profile a controller writes directly, and refuse a second holder of its handle
        const after = (await east.database.log.position()).sequence;
        const now = Date.now();
        const row = {
            scope: east.spaceId,
            ownerId: "carol",
            handle: "carol",
            createdAt: now,
            updatedAt: now,
        };
        await east.database
            .insert(profile.table)
            .values({ ...row, id: schema.identifier("profile").parse(`profile-${v7()}`) });
        await east.database
            .insert(profile.table)
            .values({ ...row, id: schema.identifier("profile").parse(`profile-${v7()}`) });
        const { changes } = await east.database.log.read({ tables: [profile.table], after });
        const replace = async (position: number) => {
            const change = aligned(changes, position);
            const image = Change.image(change);
            const owned = await profile.claimsOf(
                String(change.key.id),
                image,
                Snapshot.live(east.database),
            );

            return directory.replace(owned, `change-${change.sequence}`);
        };
        await replace(0);
        expect(await handle("carol")).toBe(aligned(changes, 0).key.id);
        expect(await replace(1)).toEqual([
            {
                index: profile.index("handle"),
                key: '["universe","carol"]',
                packageId: profile.policy.definition.packageId,
                objectId: aligned(changes, 1).key.id,
                scope: east.spaceId,
            },
        ]);

        // keep an existing object's claim, release a failed one, and wait for the later one
        const kept = schema.identifier("profile").parse(`profile-${v7()}`);
        await east.database.insert(profile.table).values({ ...row, id: kept, handle: "kept" });
        const reserved = {
            index: profile.index("handle"),
            packageId: profile.policy.definition.packageId,
            scope: east.spaceId,
        };
        vi.useFakeTimers({ toFake: ["Date"], now: now - RESERVATION_MILLISECONDS });
        await directory.claim(
            [
                { ...reserved, key: '["universe","kept"]', objectId: kept },
                { ...reserved, key: '["universe","lost"]', objectId: `profile-${v7()}` },
            ],
            "request",
        );
        vi.setSystemTime(now - RESERVATION_MILLISECONDS + 1000);
        await directory.claim(
            [{ ...reserved, key: '["universe","later"]', objectId: `profile-${v7()}` }],
            "request",
        );
        vi.useRealTimers();

        // finish the expired reservations through the controller, looking again at the later one
        const controller = new ClaimController(directory, east.database, [profile]);
        const next = await controller.reconcile();
        expect([
            controller.watches,
            await controller.list(),
            next !== undefined && next > 0 && next <= 1000,
            await handle("kept"),
            await handle("lost"),
        ]).toEqual([[profile.table], ["reservations"], true, kept, undefined]);
    },
);

/** Serve one space's profiles in its own database, returning a client per user. */
async function serveProfiles(dialect: Dialect, directory: Directory) {
    // keep the space's profiles in a database of its own
    const spaceId = schema.identifier("space").parse(`space-${v7()}`);
    const storage = await TestDatabase.create(dialect, profilesDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // authenticate each request as the user its bearer credential names
    const server = Server.start({
        ...ObjectServer.serve(profilesService, {
            callKey: testCallKey,
            database: storage.database,
            directory,
        }),
        audience,
        scope: spaceId,
        resources: new ResourceContext(),
        health: new Health("profiles"),
        drainTimeout: 1000,
        authorizeHost: async () => {},
        authenticate: async (request) => {
            const id = present(
                request.headers.get("authorization"),
                "the authorization header",
            ).slice("Bearer ".length);
            const subject = principal.user.reference("universe", id);
            const now = Date.now();

            return new Authentication({
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
        createClient(profilesService, {
            url: "https://profiles.test",
            headers: { authorization: `Bearer ${user}` },
            fetch: (request: Request) => server.fetch(request),
        });

    return { spaceId, database: storage.database, alice: connect("alice"), bob: connect("bob") };
}

test.each(TEST_DIALECTS)(
    "key an index within the enclosing scope it is unique across, and refuse one across an unrelated scope, on %s",
    async (dialect) => {
        // declare routes in folders of accounts, unique per account
        const account = defineObject({
            name: "account",
            plural: "accounts",
            scope: "universe",
            isScope: true,
            fields: {},
            permissions: { read: none() },
        });
        const folder = defineObject({
            name: "folder",
            plural: "folders",
            scope: account,
            isScope: true,
            fields: {},
            permissions: { read: none() },
        });
        const route = defineObject({
            name: "route",
            plural: "routes",
            scope: folder,
            fields: { path: field.string() },
            indexes: { path: { on: ["path"], unique: true, across: () => account } },
            permissions: { read: none() },
        });

        // record two folders of one account and one of another
        const storage = await TestDatabase.create(dialect, [...account.tables, ...folder.tables], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        const folders: readonly (readonly [string, string])[] = [
            ["account-1", "folder-1"],
            ["account-1", "folder-2"],
            ["account-2", "folder-3"],
        ];
        for (const [accountId, folderId] of folders) {
            if (folderId !== "folder-2") {
                await new AccessFixture(storage.database).copyScope(
                    account.reference(Scope.universe.id, accountId),
                );
            }
            await new AccessFixture(storage.database).copyScope(
                folder.reference(accountId, folderId),
            );
        }

        // claim the same path in each folder
        const snapshot = Snapshot.live(storage.database);
        const keys = await Promise.all(
            ["folder-1", "folder-2", "folder-3"].map(async (scope, position) =>
                route.claims({ id: `route-${position}`, scope, path: "/home" }, snapshot),
            ),
        );

        // refuse an index across a scope that does not enclose the objects
        const unrelated = () =>
            defineObject({
                name: "stray",
                plural: "strays",
                scope: account,
                fields: { path: field.string() },
                indexes: { path: { on: ["path"], unique: true, across: () => folder } },
                permissions: { read: none() },
            });

        // one account's folders share a key, another account's folder claims apart
        const index = present(keys[0]?.[0], "the first folder's claim").index;
        const { packageId } = route.policy.definition;
        expect(keys).toEqual([
            [
                {
                    index,
                    packageId,
                    key: '["account-1","/home"]',
                    objectId: "route-0",
                    scope: "folder-1",
                },
            ],
            [
                {
                    index,
                    packageId,
                    key: '["account-1","/home"]',
                    objectId: "route-1",
                    scope: "folder-2",
                },
            ],
            [
                {
                    index,
                    packageId,
                    key: '["account-2","/home"]',
                    objectId: "route-2",
                    scope: "folder-3",
                },
            ],
        ]);
        expect(unrelated).toThrow(
            new TypeError(
                "index path of stray must be unique within the universe or a scope type enclosing its objects",
            ),
        );
    },
);
