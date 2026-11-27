import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import type { Environment } from "@destack/environment";
import { argument, plural, t } from "@destack/locale";
import { PackageVersion } from "@destack/package";
import { type Identifier, schema } from "@destack/schema";
import { ObjectReference, RequestId, Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { testCallKey } from "@destack/object/test";
import { type ObjectConnection } from "../src/client/index.ts";
import { ObjectClient } from "../src/device/index.ts";
import { defineObject, definePackage, field, tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { implementPackage } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "../src/test/space.ts";
import { testEvents, testIdentity, testSimulation, TestObjectStore } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";

/** The space. */
const spaceId = "space-01996ab0-0000-7000-8000-000000000015";

/** A page its owner reads. */
const page = defineObject(
    {
        name: "page",
        plural: "pages",
        title: t`${plural(argument("count"), { one: "Page", other: "Pages" })}`,
        scope: space,
        fields: { owner: field.reference(principal.user).caller(), title: field.string() },
        access: { permissions: { read: relation("owner"), write: relation("owner") } },
    },
    (method) => ({ create: method.create("write"), list: method.list("read") }),
);

/** A task its owner reads. */
const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
        scope: space,
        fields: { owner: field.reference(principal.user).caller(), title: field.string() },
        access: { permissions: { read: relation("owner"), write: relation("owner") } },
    },
    (method) => ({ create: method.create("write"), list: method.list("read") }),
);

/** A bookmark of a page or a task. */
const bookmark = defineObject(
    {
        name: "bookmark",
        plural: "bookmarks",
        title: t`${plural(argument("count"), { one: "Bookmark", other: "Bookmarks" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            /** The bookmarked page or task. */
            target: field.object(page, task),
        },
        access: { permissions: { read: relation("owner"), write: relation("owner") } },
    },
    (method) => ({ create: method.create("write"), list: method.list("read") }),
);

/** The package. */
const bookmarksPackage = definePackage({ ...testIdentity, objects: [page, task, bookmark] });

/** The types a client follows. */
const OBJECTS = { page, task, bookmark };

/** A row with a title, or none. */
const TITLED = schema.looseObject({ title: schema.string().nullable() });

/** The space's database. */
const bookmarksDatabase = defineDatabase({
    name: "main",
    ...tablesOf({
        packages: [objectPackage, { objects: OBJECTS, tables: [] }],
        follows: [],
        tables: [],
    }),
});

test("include and get references to several types", async () => {
    // create alice's page and task, and bob's page
    const world = testSimulation().environment("test");
    const { connect, connection } = await serveBookmarks(world);
    const alice = connect(users.alice);
    const bob = connect(users.bob);
    const create = () => ({ spaceId, requestId: RequestId.create(world) });
    const plan = await alice.page.create({ ...create(), title: "Plan" });
    const ship = await alice.task.create({ ...create(), title: "Ship" });
    const secret = await bob.page.create({ ...create(), title: "Secret" });

    // bookmark all three as alice
    const references = [
        page.reference(spaceId, plan.id),
        task.reference(spaceId, ship.id),
        page.reference(spaceId, secret.id),
    ];
    for (const reference of references) {
        await alice.bookmark.create({
            spaceId,
            requestId: RequestId.create(world),
            ...ObjectReference.columns("target", reference),
        });
    }

    // read the targets through the include and through get
    const device = await openClient(world, users.alice, connection(users.alice));
    const included = device.query.bookmark.findMany({ with: { target: true } }).subscribe();
    const got = device.get(references);
    onTestFinished(async () => {
        await included.close();
        await got.close();
    });
    await Promise.all([included.ready, got.ready]);
    const titles = (await included.read()).map((item) => TITLED.parse(item.target ?? { title: null }).title);
    const rows = await got.read();

    // expect bob's page absent from both
    expect([
        titles,
        references.map((reference) => rows.get(ObjectReference.key(reference))?.["title"] ?? null),
    ]).toEqual([
        ["Plan", "Ship", null],
        ["Plan", "Ship", null],
    ]);
});

/** Serve the space to its users. */
async function serveBookmarks(world: Environment) {
    // store the space's objects
    const storage = await TestDatabase.create(bookmarksDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);

    // authenticate each request as the user its bearer credential identifies
    const served = TestObjectStore.serve(
        await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            servers: [implementPackage(bookmarksPackage, {})],
            chain: async () => undefined,
            environment: world,
            origin: { package: PackageVersion.of(bookmarksPackage) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(storage.database, world),
            database: storage.database,
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());
    const connection = (name: Identifier<"user">) => served.as(name).connect(bookmarksPackage);
    const connect = (name: Identifier<"user">) => served.as(name).client(bookmarksPackage);

    return { connect, connection };
}

/** Open a user's device client of the space. */
async function openClient(world: Environment, name: Identifier<"user">, remote: ObjectConnection) {
    // create the local database
    const storage = await TestDatabase.create(ObjectClient.tables(OBJECTS), {
        environment: world,
        dialect: "sqlite",
        storage: "file",
    });
    onTestFinished(() => storage.close());
    const client = await ObjectClient.open({
        environment: world,
        database: storage.database,
        objects: OBJECTS,
        scope: spaceId,
        caller: principal.user.reference("universe", name),
        remote,
    });

    // push and follow until the test finishes
    const stop = new AbortController();
    const running = client.run(stop.signal);
    onTestFinished(async () => {
        stop.abort();
        await running;
        await client.close();
    });

    return client;
}
