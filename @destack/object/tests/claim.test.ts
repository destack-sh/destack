import { bearerUser } from "./fixture/user.ts";
import { ObjectExtension } from "@destack/object/server";
import { outbox } from "@destack/service/outbox";
import { Scope } from "@destack/sync";
import { Change, Snapshot, type Dialect, type Row } from "@destack/db";
import { ClaimController, type ClaimStore, DatabaseClaimStore } from "../src/server/index.ts";
import {
    Claim,
    Claimant,
    claimTable,
    defineObject,
    field,
    RESERVATION_MILLISECONDS,
} from "../src/index.ts";
import { expect, onTestFinished, test, TestClock } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { aligned, Clock, Identifier, present, schema } from "@destack/schema";
import { createClient } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";
import { v7 } from "uuid";
import { profile, profilesDatabase, profilesService } from "./fixture/profile.ts";
import { none, principal, universe } from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";

/** The package serving the profiles. */
const audience = PackageId.parse("package-01a0d5eb-fb8a-74f4-ba37-8a4d6970e239");

test.each(TEST_DIALECTS)(
    "keep handles unique across spaces in separate databases on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [outbox, claimTable], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        const claims = new DatabaseClaimStore(storage.database, Clock.system);
        const east = await serveProfiles(dialect, claims);
        const west = await serveProfiles(dialect, claims);
        const handle = async (name: string) =>
            (await Claim.lookup(profile, claims, "handle", [name]))?.id;

        // let one space claim a handle, and refuse it to another space
        const ada = await east.alice.profile.create({
            spaceId: east.spaceId,
            requestId: RequestId.create(Date.now()),
            handle: "ada",
        });
        expect(await handle("ada")).toBe(ada.id);
        await expect(
            west.bob.profile.create({
                spaceId: west.spaceId,
                requestId: RequestId.create(Date.now()),
                handle: "ada",
            }),
        ).rejects.toMatchObject({ code: "CONFLICT", message: "handle is taken" });

        // release the handle a profile moves away from, and the handle of a deleted profile
        await east.alice.profile.update({
            spaceId: east.spaceId,
            id: ada.id,
            requestId: RequestId.create(Date.now()),
            revision: ada.revision,
            handle: "lovelace",
        });
        const bob = await west.bob.profile.create({
            spaceId: west.spaceId,
            requestId: RequestId.create(Date.now()),
            handle: "ada",
        });
        await east.alice.profile.delete({
            spaceId: east.spaceId,
            id: ada.id,
            requestId: RequestId.create(Date.now()),
            revision: ada.revision + 1,
        });
        expect([await handle("ada"), await handle("lovelace")]).toEqual([bob.id, undefined]);

        // release the handle a profile clears
        await west.bob.profile.update({
            spaceId: west.spaceId,
            id: bob.id,
            requestId: RequestId.create(Date.now()),
            revision: bob.revision,
            handle: null,
        });
        expect(await handle("ada")).toBeUndefined();

        // keep no reservation once every owner let go: the failed claim left none behind
        expect(await claims.expired([Claim.index(profile, "handle")])).toEqual({ claims: [] });
    },
);

test.each(TEST_DIALECTS)(
    "claim the names of writes outside the object store and finish expired reservations on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [outbox, claimTable], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        const claims = new DatabaseClaimStore(storage.database, Clock.system);
        const east = await serveProfiles(dialect, claims);
        const handle = async (name: string) =>
            (await Claim.lookup(profile, claims, "handle", [name]))?.id;

        // follow a profile a controller writes directly, and refuse a second owner of its handle
        const after = (await east.database.log.position()).sequence;
        const now = 1_800_000_000_000;
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
            const owned = await Claimant.of(
                profile,
                schema.string().parse(change.key.id),
                image,
                Snapshot.live(east.database),
            );

            return claims.replace(owned);
        };
        await replace(0);
        expect(await handle("carol")).toBe(aligned(changes, 0).key.id);
        expect(await replace(1)).toEqual([
            {
                index: Claim.index(profile, "handle"),
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
            index: Claim.index(profile, "handle"),
            packageId: profile.policy.definition.packageId,
            scope: east.spaceId,
        };
        const clock = new TestClock(now - RESERVATION_MILLISECONDS);
        const reserving = new DatabaseClaimStore(storage.database, clock);
        await reserving.claim(
            [
                { ...reserved, key: '["universe","kept"]', objectId: kept },
                { ...reserved, key: '["universe","lost"]', objectId: `profile-${v7()}` },
            ],
            "request",
        );
        clock.set(now - RESERVATION_MILLISECONDS + 1000);
        await reserving.claim(
            [{ ...reserved, key: '["universe","later"]', objectId: `profile-${v7()}` }],
            "request",
        );
        clock.set(now);

        // finish the expired reservations through the controller, looking again at the later one
        const controller = new ClaimController(reserving, east.database, [profile], clock);
        expect([await controller.reconcile(), await handle("kept"), await handle("lost")]).toEqual([
            1000,
            kept,
            undefined,
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "resolve a confirmed name to its object alone, and refuse it to others while trashed, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [claimTable], { isMigrated: true });
        onTestFinished(() => storage.close());
        const claims = new DatabaseClaimStore(storage.database, Clock.system);
        const [first, second] = [`place-${v7()}`, `place-${v7()}`];
        const index = "package-1/place/name";
        const claim = (key: string, objectId: string) => ({
            index,
            key,
            packageId: "package-1",
            objectId,
            scope: "account-1",
        });
        const owned = (key: string, state: "active" | "trashed") => ({
            indexes: [index],
            objectId: first,
            claims: [claim(key, first)],
            state,
        });

        // reserve a name and confirm it once the write committed
        expect([
            await claims.claim([claim("notes", first)], "request-1"),
            await claims.claim([claim("notes", second)], "request-2"),
        ]).toEqual([[], [claim("notes", second)]]);
        const reserved = await claims.owner(index, "notes");
        await claims.confirm("request-1", [owned("notes", "active")]);
        const active = await claims.owner(index, "notes");

        // keep the name while its object sits in the trash, still refusing it to another object
        await claims.replace(owned("notes", "trashed"));
        const trashed = await claims.owner(index, "notes");
        const taken = await claims.replace({
            indexes: [index],
            objectId: second,
            claims: [claim("notes", second)],
            state: "active",
        });

        // restore and rename the object
        await claims.replace(owned("notes", "active"));
        const restored = await claims.owner(index, "notes");
        await claims.replace(owned("archive", "active"));
        expect([
            reserved,
            active,
            trashed,
            taken,
            restored,
            await claims.owner(index, "notes"),
            await claims.owner(index, "archive"),
        ]).toEqual([
            undefined,
            { objectId: first, scope: "account-1" },
            undefined,
            [claim("notes", second)],
            { objectId: first, scope: "account-1" },
            undefined,
            { objectId: first, scope: "account-1" },
        ]);
    },
);

/** Serve one space's profiles in its own database, returning a client per user. */
async function serveProfiles(dialect: Dialect, claims: ClaimStore) {
    // keep the space's profiles in a database of its own
    const spaceId = schema.identifier("space").parse(`space-${v7()}`);
    const storage = await TestDatabase.create(dialect, profilesDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // authenticate each request as the user its bearer credential identifies
    const server = Server.start({
        clock: Clock.system,
        ...ObjectExtension.compose([ObjectExtension.of(profilesService, [])], {
            clock: Clock.system,
            origin: { package: profilesService.package, service: profilesService.name },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            database: storage.database,
            claims,
        }).services[0],
        audience,
        scope: spaceId,
        resources: new ResourceContext(),
        health: new Health("profiles"),
        drainTimeout: 1000,
        authorizeMachine: async () => {},
        authenticate: bearerUser(audience, spaceId),
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
    "key an index within the enclosing scope it is unique across on %s",
    async (dialect) => {
        // declare routes in folders of accounts, unique per account
        const account = defineObject({
            name: "account",
            plural: "accounts",
            scope: "universe",
            isScope: true,
            fields: {},
            permissions: { read: none() },
        }).methods();
        const folder = defineObject({
            name: "folder",
            plural: "folders",
            scope: account,
            isScope: true,
            fields: {},
            permissions: { read: none() },
        }).methods();
        const route = defineObject({
            name: "route",
            plural: "routes",
            scope: folder,
            fields: { path: field.string() },
            indexes: { path: { on: ["path"], unique: true, across: () => account } },
            permissions: { read: none() },
        }).methods();

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
                Claim.list(route, { id: `route-${position}`, scope, path: "/home" }, snapshot),
            ),
        );

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
    },
);

test("keep a trashed object's names, activate them on restore, and release them on purge", async () => {
    // declare tags whose names are unique everywhere and whose purge keeps the record
    const tag = defineObject({
        name: "tag",
        plural: "tags",
        scope: space,
        fields: { name: field.string() },
        indexes: { name: { on: ["name"], unique: true, across: () => universe } },
        recoverable: { within: { days: 30 }, by: "manage", keep: "record" },
        permissions: { read: none(), manage: none() },
    }).methods();
    const storage = await TestDatabase.create("sqlite", [claimTable], { isMigrated: true });
    onTestFinished(() => storage.close());
    const claims = new DatabaseClaimStore(storage.database, Clock.system);
    const snapshot = Snapshot.live(storage.database);

    // write the tag's row as it stands, sits in the trash, is restored and is purged
    const [first, second] = [Identifier.create("tag"), Identifier.create("tag")];
    const standing = { id: first, scope: "space-1", name: "urgent", deletionRequestedAt: null };
    const trashed = { ...standing, deletionRequestedAt: 1, purgedAt: null };
    const purged = { ...trashed, purgedAt: 2 };
    const other = { ...standing, id: second };
    const write = async (row: Row & { readonly id: string }, requestId: string) =>
        claims.replace(await Claimant.of(tag, row.id, row, snapshot));
    const owner = async () => (await Claim.lookup(tag, claims, "name", ["urgent"]))?.id;
    const states = [];
    for (const [row, requestId] of [
        [standing, "request-1"],
        [trashed, "request-2"],
        [other, "request-3"],
        [standing, "request-4"],
        [purged, "request-5"],
        [other, "request-6"],
    ] as const) {
        states.push([await write(row, requestId), await owner()]);
    }

    // resolve the name while it stands, keep it in the trash, and free it once purged
    const refused = aligned(await Claim.list(tag, other, snapshot), 0);
    expect(states).toEqual([
        [[], first],
        [[], undefined],
        [[refused], undefined],
        [[], first],
        [[], undefined],
        [[], second],
    ]);
});
