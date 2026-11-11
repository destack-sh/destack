import type { Environment } from "@destack/environment";
import { testSimulation } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { Scope, RequestId } from "@destack/sync";
import {
    Change,
    type DatabaseConnection,
    Snapshot,
    type Row,
    type RelationalQueryBuilder,
    defineDatabase,
    sql,
} from "@destack/db";
import { claimReservation, type ClaimPrimary, implementPackage } from "../src/server/index.ts";
import {
    Claim,
    ClaimLookup,
    ClaimRefusal,
    Claimant,
    claimTable,
    defineObject,
    definePackage,
    field,
    RESERVATION_MILLISECONDS,
} from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { PackageVersion } from "@destack/package";
import { nth, Identifier, present, schema, Uuid } from "@destack/schema";
import { profile, profilesDatabase, profilesPackage } from "./fixture/profile.ts";
import { none, principal, relation, universe } from "@destack/access";
import { TestAccess } from "@destack/access/test";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { settleControl, testEvents } from "../src/test/index.ts";

import { claim, serveClaims } from "./fixture/claim.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";
import { recoverable } from "../src/index.ts";
import { argument, plural, t } from "@destack/locale";

/** Profiles as first released, before their type declared the handle index. */
const unindexed = defineObject(
    {
        name: "profile",
        plural: "profiles",
        title: t`${plural(argument("count"), { one: "Profile", other: "Profiles" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            handle: field.string(schema.string().min(1).max(40)).optional(),
        },
        access: {
            permissions: { read: relation("owner"), manage: relation("owner") },
        },
    },
    (method) => ({ create: method.create("manage") }),
);

/** The package of the profiles as first released. */
const unindexedPackage = definePackage({ objects: [unindexed] });

/** The database of one space's profiles as first released. */
const unindexedDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: { profile: unindexed }, tables: [] }],
        follows: [],
        tables: [],
    }),
});

test("enforce unique handles across spaces in separate databases", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await universeStorage(world);
    const claims = await serveClaims(storage.database, world);
    const east = await serveProfiles(world, claims);
    const west = await serveProfiles(world, claims);
    const handle = async (name: string) =>
        (await Claim.lookup(profile, names(storage.database), "handle", [name]))?.id;

    // let one space claim a handle, and refuse it to another space
    const ada = await east.alice.profile.create({
        spaceId: east.spaceId,
        requestId: RequestId.create(world),
        handle: "ada",
    });
    expect(await handle("ada")).toBe(ada.id);
    expect(
        await refusal(
            west.bob.profile.create({
                spaceId: west.spaceId,
                requestId: RequestId.create(world),
                handle: "ada",
            }),
        ),
    ).toEqual(["CONFLICT", "NAME_EXISTS", "'handle' already exists"]);

    // release the handle a profile moves away from, and the handle of a deleted profile
    await east.alice.profile.update({
        spaceId: east.spaceId,
        id: ada.id,
        requestId: RequestId.create(world),
        revision: ada.revision,
        handle: "lovelace",
    });
    const bob = await west.bob.profile.create({
        spaceId: west.spaceId,
        requestId: RequestId.create(world),
        handle: "ada",
    });
    await east.alice.profile.delete({
        spaceId: east.spaceId,
        id: ada.id,
        requestId: RequestId.create(world),
        revision: ada.revision + 1,
    });
    expect([await handle("ada"), await handle("lovelace")]).toEqual([bob.id, undefined]);

    // release the handle a profile clears
    await west.bob.profile.update({
        spaceId: west.spaceId,
        id: bob.id,
        requestId: RequestId.create(world),
        revision: bob.revision,
        handle: null,
    });
    expect(await handle("ada")).toBeUndefined();

    // leave no reservation once every owner let go: the failed claim left none behind
    const index = Claim.index(profile, "handle");
    expect(
        await readClaims(storage.database).findMany({ where: { state: "pending", index } }),
    ).toEqual([]);
});

test("reserve a write's names before it commits and confirm them once it committed", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // record each claim call with the profiles the space committed when it arrived
    const storage = await universeStorage(world);
    const served = await serveClaims(storage.database, world);
    const home = present(served.home, "the universe's claims");
    const seen: (readonly [string, number])[] = [];
    let spaceDatabase: DatabaseConnection | undefined;
    const claims: ClaimPrimary = {
        object: served.object,
        home: {
            mutate: async (mutation) => {
                const database = present(spaceDatabase, "the space's database");
                const rows = await database.select().from(profile.table);
                seen.push(...mutation.calls.map((call) => [call.method, rows.length] as const));

                return home.mutate(mutation);
            },
            query: (call) => home.query(call),
        },
    };
    const east = await serveProfiles(world, claims);
    spaceDatabase = east.database;

    // create a profile claiming a handle
    await east.alice.profile.create({
        spaceId: east.spaceId,
        requestId: RequestId.create(world),
        handle: "ada",
    });

    expect(seen).toEqual([
        ["reserve", 0],
        ["confirm", 1],
    ]);
});

test("claim the names of writes outside the object store and finish expired reservations", async () => {
    const now = 1_800_000_000_000;
    const simulation = testSimulation(now - RESERVATION_MILLISECONDS);
    const world = simulation.environment("test");
    const storage = await universeStorage(world);
    const claims = await serveClaims(storage.database, world);
    const east = await receiveProfiles(world, claims);
    const handle = async (name: string) =>
        (await Claim.lookup(profile, names(storage.database), "handle", [name]))?.id;

    // follow a profile a controller writes directly, and refuse a second owner of its handle
    const after = (await east.database.log.position()).sequence;
    const row = {
        scope: east.spaceId,
        ownerId: users.carol,
        handle: "carol",
        createdAt: now,
        updatedAt: now,
    };
    await east.database.transaction(
        async (transaction) =>
            await transaction.insert(profile.table).values({
                ...row,
                id: schema.identifier("profile").parse(`profile-${Uuid.v7(world)}`),
            }),
    );
    await east.database.transaction(
        async (transaction) =>
            await transaction.insert(profile.table).values({
                ...row,
                id: schema.identifier("profile").parse(`profile-${Uuid.v7(world)}`),
            }),
    );
    const { changes } = await east.database.log.read({ tables: [profile.table], after });
    const replace = async (position: number) => {
        const change = nth(changes, position);
        const image = Change.image(change);
        const object = { objectId: schema.string().parse(change.key.id), owner: east.spaceId };
        const claimant = await Claimant.of(profile, object, image, Snapshot.live(east.database));
        const [replaced] = await present(claims.home, "the universe's claims").mutate({
            id: RequestId.create(world),
            calls: [claim.call("replace", claimant)],
        });

        return ClaimRefusal.parse(replaced).refused;
    };
    await replace(0);
    expect(await handle("carol")).toBe(nth(changes, 0).key.id);
    expect(await replace(1)).toEqual([
        {
            index: Claim.index(profile, "handle"),
            key: '["universe","carol"]',
            packageId: profile.policy.definition.packageId,
            objectId: nth(changes, 1).key.id,
            owner: east.spaceId,
        },
    ]);

    // confirm an existing object's claim, release a failed one, and wait for the later one
    const existing = schema.identifier("profile").parse(`profile-${Uuid.v7(world)}`);
    await east.database.transaction(
        async (transaction) =>
            await transaction
                .insert(profile.table)
                .values({ ...row, id: existing, handle: "existing" }),
    );
    const reserve = async (key: string, objectId: string, at: number) => {
        // reserve at the home and record the reservation here, as a write that then crashed would
        const owner = east.spaceId;
        const index = Claim.index(profile, "handle");
        const { packageId } = profile.policy.definition;
        const claimed = { index, key, packageId, objectId, owner };
        const claimant = { indexes: [index], objectId, owner, claims: [claimed] };
        await storage.database.transaction((transaction) =>
            Claim.reserve(
                transaction,
                { ...claimant, state: "active" },
                "request",
                at - RESERVATION_MILLISECONDS,
                world,
            ),
        );
        await east.database.transaction(
            async (transaction) =>
                await transaction
                    .insert(claimReservation)
                    .values({ requestId: "request", objectId, owner, index, expiresAt: at }),
        );
    };
    await reserve('["universe","existing"]', existing, now);
    await reserve('["universe","lost"]', `profile-${Uuid.v7(world)}`, now);
    await reserve('["universe","later"]', `profile-${Uuid.v7(world)}`, now + 1000);
    await simulation.jump(now);

    // finish the expired reservations through the controller, keeping the later one
    await settleControl(east.store);
    const recorded = await east.database.select().from(claimReservation);
    expect([
        await handle("existing"),
        await handle("lost"),
        recorded.map((reservation) => reservation.expiresAt),
    ]).toEqual([existing, undefined, [now + 1000]]);
});

test("resolve a confirmed name to its object alone, and refuse it to others while trashed", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        tablesOf({ packages: [], follows: [], tables: [claimTable] }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const [first, second] = [`place-${Uuid.v7(world)}`, `place-${Uuid.v7(world)}`];
    const index = "package-1/place/name";
    const owner = "account-1";
    const entry = (key: string, objectId: string) => ({
        index,
        key,
        packageId: "package-1",
        objectId,
        owner,
    });
    const claimant = (key: string, state: "active" | "trashed", objectId = first) => ({
        indexes: [index],
        objectId,
        owner,
        claims: [entry(key, objectId)],
        state,
    });
    const now = world.clock.now();

    // reserve a name and confirm it once the write committed
    expect([
        await database.transaction((transaction) =>
            Claim.reserve(transaction, claimant("notes", "active"), "request-1", now, world),
        ),
        await database.transaction((transaction) =>
            Claim.reserve(
                transaction,
                claimant("notes", "active", second),
                "request-2",
                now,
                world,
            ),
        ),
    ]).toEqual([[], [entry("notes", second)]]);
    const reserved = await Claim.owner(readClaims(database), { index, key: "notes" });
    await database.transaction((transaction) =>
        Claim.confirm(transaction, claimant("notes", "active"), "request-1"),
    );
    const active = await Claim.owner(readClaims(database), { index, key: "notes" });

    // retain the name while its object sits in the trash, still refusing it to another object
    await database.transaction((transaction) =>
        Claim.replace(transaction, claimant("notes", "trashed"), world),
    );
    const trashed = await Claim.owner(readClaims(database), { index, key: "notes" });
    const taken = await database.transaction((transaction) =>
        Claim.replace(transaction, claimant("notes", "active", second), world),
    );

    // restore and rename the object
    await database.transaction((transaction) =>
        Claim.replace(transaction, claimant("notes", "active"), world),
    );
    const restored = await Claim.owner(readClaims(database), { index, key: "notes" });
    await database.transaction((transaction) =>
        Claim.replace(transaction, claimant("archive", "active"), world),
    );
    expect([
        reserved,
        active,
        trashed,
        taken,
        restored,
        await Claim.owner(readClaims(database), { index, key: "notes" }),
        await Claim.owner(readClaims(database), { index, key: "archive" }),
    ]).toEqual([
        undefined,
        { objectId: first, owner },
        undefined,
        [entry("notes", second)],
        { objectId: first, owner },
        undefined,
        { objectId: first, owner },
    ]);
});

/** Serve one space's profiles in a separate database, returning a client per user. */
test("refuse a name a postgresql write took after a commit's owners read as a lost race, and plainly once run again", async () => {
    // run in a world of its own on postgresql, where writes interleave
    const world = testSimulation().environment("test");
    const storage = await TestDatabase.create(
        tablesOf({ packages: [], follows: [], tables: [claimTable] }).tables,
        { environment: world, isMigrated: true, dialect: "postgresql" },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const index = "package-1/place/name";
    const claimant = (objectId: string) => ({
        indexes: [index],
        objectId,
        owner: "account-1",
        claims: [{ index, key: "notes", packageId: "package-1", objectId, owner: "account-1" }],
        state: "active" as const,
    });
    const replace = (objectId: string) =>
        database.transaction(
            (transaction) => Claim.replace(transaction, claimant(objectId), world),
            { isolationLevel: "read committed" },
        );

    // hold a write's name uncommitted, and replace another object's names until its insert waits on it
    const written = Promise.withResolvers<number>();
    const released = Promise.withResolvers<void>();
    const holding = database.transaction(async (transaction) => {
        await Claim.replace(transaction, claimant("place-1"), world);
        const [row] = await transaction.all(sql`SELECT pg_backend_pid() AS pid`);
        written.resolve(Number(row?.["pid"]));
        await released.promise;
    });
    const holder = await written.promise;
    const late = refusal(replace("place-2"));
    for (let waiting = 0; waiting === 0;) {
        const [row] = await database.all(
            sql`SELECT count(*) AS waiting FROM pg_stat_activity WHERE ${holder}::INTEGER = ANY(pg_blocking_pids(pid))`,
        );
        waiting = Number(row?.["waiting"]);
    }

    // refuse the late insert as a lost race, then plainly once the commit shows the owner
    released.resolve();
    await holding;
    expect([await late, await replace("place-2")]).toEqual([
        [
            "CONFLICT",
            "UPDATE_CONFLICT",
            "cannot claim the name: another write claimed it meanwhile",
        ],
        [{ index, key: "notes", packageId: "package-1", objectId: "place-2", owner: "account-1" }],
    ]);
});

/** Serve a space's profiles from their own database, claiming names through a primary. */
test("build a global index a migration declares from the rows stored before it, refusing a name another database's rows hold", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");
    const storage = await universeStorage(world);
    const claims = await serveClaims(storage.database, world);
    const handle = async (name: string) =>
        (await Claim.lookup(profile, names(storage.database), "handle", [name]))?.id;

    // store profiles in two spaces before their type declares the handle index
    const east = await storeUnindexed(world, ["ada", "grace"]);
    const west = await storeUnindexed(world, ["ada"]);

    // migrate each database to the indexed type, which claims the rows stored before
    const eastReceived = await receiveProfiles(world, claims, east);
    await settleControl(eastReceived.store);
    const westReceived = await receiveProfiles(world, claims, west);
    const refused = await settleControl(westReceived.store).then(
        () => undefined,
        (error: unknown) => (error instanceof AggregateError ? error.errors.map(String) : error),
    );

    // resolve the east's rows by their handles, and refuse the west's ada as a duplicate
    const [ada, grace] = await east.select().from(profile.table).orderBy(profile.table.handle);
    expect({
        ada: await handle("ada"),
        grace: await handle("grace"),
        refused,
    }).toEqual({
        ada: present(ada, "east's ada").id,
        grace: present(grace, "east's grace").id,
        refused: ["FamilyError: 'handle' already exists"],
    });
});

/** Store profiles with handles in a space's database while their type declares no index. */
async function storeUnindexed(world: Environment, handles: readonly string[]) {
    // store the space's profiles under the type without the index, claiming nothing
    const spaceId = schema.identifier("space").parse(`space-01996ab0-0000-7000-8000-00000000a006`);
    const storage = await TestDatabase.create(unindexedDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const served = TestObjectStore.serve(
        await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            servers: [implementPackage(unindexedPackage, {})],
            chain: async () => undefined,
            environment: world,
            origin: { package: PackageVersion.of(unindexedPackage) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            database: storage.database,
        }),
    );

    // create one profile per handle, then stop serving the type
    const alice = served.as(users.alice).client(unindexedPackage);
    for (const name of handles) {
        await alice.profile.create({ spaceId, requestId: RequestId.create(world), handle: name });
    }
    await served[Symbol.asyncDispose]();

    return storage.database;
}

/** Receive the profiles package's store on a world's database, a new one unless given. */
async function receiveProfiles(
    world: Environment,
    claims: ClaimPrimary,
    stored?: DatabaseConnection,
) {
    // store the space's profiles in a separate database, or migrate the one storing them already
    const spaceId = schema.identifier("space").parse(`space-01996ab0-0000-7000-8000-00000000a006`);
    const database = stored ?? (await profilesStorage(world, spaceId));
    if (stored !== undefined) {
        await stored.migrate(profilesDatabase.tables);
    }

    // receive the space's store, whose controllers the test runs
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        servers: [implementPackage(profilesPackage, {})],
        chain: async () => undefined,
        environment: world,
        origin: { package: PackageVersion.of(profilesPackage) },
        principal: spacePrincipal(spaceId),
        callKey: testCallKey,
        events: await testEvents(database, world),
        database,
        claims,
    });

    return { environment: world, spaceId, store, database };
}

/** Serve the profiles package on a world's database, running its controllers, under some claims. */
async function serveProfiles(world: Environment, claims: ClaimPrimary) {
    // authenticate each request as the user its bearer credential identifies
    const received = await receiveProfiles(world, claims);
    const served = TestObjectStore.serve(received.store);
    onTestFinished(() => served[Symbol.asyncDispose]());
    const connect = (user: Identifier<"user">) => served.as(user).client(profilesPackage);

    return { ...received, alice: connect(users.alice), bob: connect(users.bob) };
}

/** Store a space's profiles in a fresh database of their own. */
async function profilesStorage(world: Environment, spaceId: Identifier<"space">) {
    const storage = await TestDatabase.create(profilesDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);

    return storage.database;
}

test("key an index within the enclosing scope it is unique across", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // declare routes in folders of accounts, unique per account
    const account = defineObject({
        name: "account",
        plural: "accounts",
        title: t`${plural(argument("count"), { one: "Account", other: "Accounts" })}`,
        scope: "universe",
        fields: {},
        container: "scope",
        access: {
            permissions: { read: none() },
        },
    });
    const folder = defineObject({
        name: "folder",
        plural: "folders",
        title: t`${plural(argument("count"), { one: "Folder", other: "Folders" })}`,
        scope: account,
        fields: {},
        container: "scope",
        access: {
            permissions: { read: none() },
        },
    });
    const route = defineObject({
        name: "route",
        plural: "routes",
        title: t`${plural(argument("count"), { one: "Route", other: "Routes" })}`,
        scope: folder,
        fields: { path: field.string() },
        indexes: { path: { on: ["path"], across: () => account } },
        access: {
            permissions: { read: none() },
        },
    });

    // record two folders of one account and one of another
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [{ objects: { account, folder }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const folders: readonly (readonly [string, string])[] = [
        ["account-1", "folder-1"],
        ["account-1", "folder-2"],
        ["account-2", "folder-3"],
    ];
    for (const [accountId, folderId] of folders) {
        if (folderId !== "folder-2") {
            await new TestAccess(storage.database, world).replicateScope(
                account.reference(Scope.universe.id, accountId),
                account.policy,
            );
        }
        await new TestAccess(storage.database, world).replicateScope(
            folder.reference(accountId, folderId),
            folder.policy,
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
                owner: "folder-1",
            },
        ],
        [
            {
                index,
                packageId,
                key: '["account-1","/home"]',
                objectId: "route-1",
                owner: "folder-2",
            },
        ],
        [
            {
                index,
                packageId,
                key: '["account-2","/home"]',
                objectId: "route-2",
                owner: "folder-3",
            },
        ],
    ]);
});

test("retain a trashed object's names, activate them on restore, and release them on purge", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // declare tags whose names are unique everywhere and whose purge retains the record
    const tag = defineObject({
        name: "tag",
        plural: "tags",
        title: t`${plural(argument("count"), { one: "Tag", other: "Tags" })}`,
        scope: space,
        fields: { name: field.string() },
        indexes: { name: { on: ["name"], across: () => universe } },
        access: {
            permissions: { read: none(), manage: none() },
        },
        traits: [recoverable({ within: { days: 30 }, by: "manage", retain: "record" })],
    });
    const storage = await TestDatabase.create(
        tablesOf({ packages: [], follows: [], tables: [claimTable] }).tables,
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    const snapshot = Snapshot.live(database);

    // write the tag's row as it stands, sits in the trash, is restored and is purged
    const [first, second] = [Identifier.create("tag", world), Identifier.create("tag", world)];
    const standing = { id: first, scope: "space-1", name: "urgent", deletedAt: null };
    const trashed = { ...standing, deletedAt: 1, purgedAt: null };
    const purged = { ...trashed, purgedAt: 2 };
    const other = { ...standing, id: second };
    const write = async (row: Row & { readonly id: string }) => {
        const object = { objectId: row.id, owner: "space-1" };

        const claimant = await Claimant.of(tag, object, row, snapshot);

        return database.transaction((transaction) => Claim.replace(transaction, claimant, world));
    };
    const owner = async () => (await Claim.lookup(tag, names(database), "name", ["urgent"]))?.id;
    const states = [];
    for (const row of [standing, trashed, other, standing, purged, other]) {
        states.push([await write(row), await owner()]);
    }

    // resolve the name while it stands, retain it in the trash, and free it once purged
    const refused = nth(await Claim.list(tag, other, snapshot), 0);
    expect(states).toEqual([
        [[], first],
        [[], undefined],
        [[refused], undefined],
        [[], first],
        [[], undefined],
        [[], second],
    ]);
});

/** Open the universe's database with its claims and the journal of their calls. */
async function universeStorage(world: Environment) {
    const tables = [
        ...new Set(
            tablesOf({
                packages: [objectPackage, { objects: { claim }, tables: [] }],
                follows: [],
                tables: [],
            }).tables,
        ),
    ];
    const storage = await TestDatabase.create(tables, { environment: world, isMigrated: true });
    onTestFinished(() => storage.close());

    return storage;
}

/** Find the objects active names resolve to in the universe's database. */
function names(database: DatabaseConnection): ClaimLookup {
    return ClaimLookup.of(readClaims(database));
}

/** Read the claims in a database, as store code reads them. */
function readClaims(database: DatabaseConnection): RelationalQueryBuilder<typeof claimTable> {
    return Snapshot.live(database).read(claimTable);
}

test("read a claim's indexed values back from its key, without the scope it is unique within", () => {
    expect(Claim.values(Claim.key(profile, "handle", ["ada"]))).toEqual(["ada"]);
});
