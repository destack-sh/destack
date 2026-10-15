import { testCallKey } from "@destack/runtime/test";
import { expect, onTestFinished, test, TestClock } from "@destack/test";
import {
    AccessContext,
    accessRelationship,
    accessRole,
    accessRolePermission,
    Caller,
    none,
    principal,
    relation,
    Relationship,
    through,
} from "@destack/access";
import { call } from "../index.ts";
import { testEvents, serversOf } from "../test/index.ts";
import { AccessFixture } from "@destack/access/test";
import { eq, type Table } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Clock, schema, type Identifier } from "@destack/schema";
import { type Page, Scope } from "@destack/sync";
import { v7 } from "uuid";
import { defineObject, field } from "../index.ts";
import { ObjectAudience } from "./audience.ts";
import { ObjectStore } from "./index.ts";

/** The spaces the notebooks live in. */
const space = defineObject({
    name: "space",
    plural: "spaces",
    scope: "universe",
    isScope: true,
    fields: {},
    permissions: { read: none() },
}).methods();

/** Notebooks of a space, read by their viewers and by people with a role granting it. */
const notebook = defineObject({
    name: "notebook",
    plural: "notebooks",
    scope: space,
    fields: { name: field.string() },
    relations: { viewer: { subjects: [principal.user] } },
    permissions: { read: relation("viewer") },
}).methods();

/** Notes in notebooks, read by whoever reads their notebook. */
const note = defineObject({
    name: "note",
    plural: "notes",
    scope: space,
    nested: { in: notebook, delete: "cascade", receive: "read" },
    fields: { title: field.string() },
    permissions: { read: through("parent", "read") },
}).methods();

/** How long the shared notebook stays shared, in milliseconds. */
const SHARED_MILLISECONDS = 200;

/** The space the notebooks live in. */
const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** The caller following the space. */
const ada = principal.user.reference(Scope.universe.id, "ada");

test("decide again after changes of the caller's own access, and leave other identities' and other roles' changes alone", async () => {
    // keep two notebooks with a note each, a role ada has on the first and a role she lacks
    const storage = await TestDatabase.create(
        "sqlite",
        [...space.tables, ...notebook.tables, ...note.tables, ...call.tables],
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;
    await new AccessFixture(database).replicateScope(
        space.reference(Scope.universe.id, spaceId),
        "own",
    );
    const at = { scope: spaceId, createdAt: 1, updatedAt: 1 };
    await database.insert(space.table).values({ ...at, id: spaceId, scope: Scope.universe.id });
    const [first, second] = [newId("notebook"), newId("notebook")];
    await database.insert(notebook.table).values([
        { ...at, id: first, name: "First" },
        { ...at, id: second, name: "Second" },
    ]);
    const [firstNote, secondNote] = [newId("note"), newId("note")];
    await database.insert(note.table).values([
        { ...at, id: firstNote, parentId: first, title: "First" },
        { ...at, id: secondNote, parentId: second, title: "Second" },
    ]);
    const [readerRole, otherRole] = [newId("role"), newId("role")];
    await database.insert(accessRole).values(
        [readerRole, otherRole].map((id) => ({
            ...at,
            id,
            name: id,
            description: "Reads notebooks",
        })),
    );
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: newId("relationship"),
                object: notebook.reference(spaceId, first),
                role: readerRole,
                subject: ada,
                createdAt: 1,
                expiresAt: null,
            },
            spaceId,
        ),
    );

    // follow the space as ada
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: principal.space.reference(Scope.universe.id, spaceId),
        servers: serversOf([space, notebook, note]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: notebook.package },
    });
    const audience = await ObjectAudience.of(
        store,
        spaceId,
        AccessContext.of(Caller.of(ada), Date.now()),
    );
    const decided = async (write: () => PromiseLike<unknown>, tables: readonly Table[]) => {
        const { sequence } = await database.log.position();
        await write();
        const { changes } = await database.log.read({ tables, after: sequence });

        return Promise.all(changes.map((change) => audience.decides(change)));
    };

    // decide again once her role grants reading
    const granted = await decided(
        () =>
            database.insert(accessRolePermission).values({
                id: newId("role-permission"),
                roleId: readerRole,
                scope: spaceId,
                packageId: notebook.policy.definition.packageId,
                type: notebook.name,
                name: "read",
            }),
        [accessRolePermission],
    );
    expect(granted).toEqual([true]);

    // leave a role ada has no binding of, and decide again once her role changes
    const unrelated = await decided(
        () =>
            database
                .update(accessRole)
                .set({ description: "Reads every notebook" })
                .where(eq(accessRole.id, otherRole)),
        [accessRole],
    );
    const described = await decided(
        () =>
            database
                .update(accessRole)
                .set({ description: "Reads the first notebook" })
                .where(eq(accessRole.id, readerRole)),
        [accessRole],
    );
    expect([unrelated, described]).toEqual([[false], [true]]);

    // decide again once ada joins a team, which her access may read
    const team = {
        packageId: notebook.policy.definition.packageId,
        type: "team",
        scope: Scope.universe.id,
        id: "team-1",
    };
    const joined = await decided(
        () =>
            database.insert(accessRelationship).values(
                Relationship.encode(
                    {
                        id: newId("relationship"),
                        object: team,
                        relation: "member",
                        subject: ada,
                        createdAt: 1,
                        expiresAt: null,
                    },
                    Scope.universe.id,
                ),
            ),
        [accessRelationship],
    );
    expect(joined).toEqual([true]);

    // decide again once the space is suspended
    const suspended = await decided(
        () => new AccessFixture(database).suspendReplica(spaceId, 2),
        [Scope.table],
    );
    expect(suspended).toEqual([true]);
});

/** Build a fresh identifier of a type. */
function newId<Prefix extends string>(prefix: Prefix): Identifier<Prefix> {
    return schema.identifier(prefix).parse(`${prefix}-${v7()}`);
}

test("drop a notebook from a live replica once the relationship sharing it expires, deciding alike with an identical audience", async () => {
    // keep a notebook shared with ada for a moment
    const storage = await TestDatabase.create(
        "sqlite",
        [...space.tables, ...notebook.tables, ...note.tables, ...call.tables],
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;
    await new AccessFixture(database).replicateScope(
        space.reference(Scope.universe.id, spaceId),
        "own",
    );
    const at = { scope: spaceId, createdAt: 1, updatedAt: 1 };
    await database.insert(space.table).values({ ...at, id: spaceId, scope: Scope.universe.id });
    const shared = newId("notebook");
    await database.insert(notebook.table).values({ ...at, id: shared, name: "Shared" });
    const clock = new TestClock(Date.now());
    const expiresAt = clock.now() + SHARED_MILLISECONDS;
    await database.insert(accessRelationship).values(
        Relationship.encode(
            {
                id: newId("relationship"),
                object: notebook.reference(spaceId, shared),
                relation: "viewer",
                subject: ada,
                createdAt: 1,
                expiresAt,
            },
            spaceId,
        ),
    );

    // follow the notebooks as ada on the test's clock, with a second identical audience deciding alike
    const store = new ObjectStore({
        chain: async () => undefined,
        clock,
        principal: principal.space.reference(Scope.universe.id, spaceId),
        servers: serversOf([space, notebook, note]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: notebook.package },
    });
    const [following, identical] = await Promise.all([
        ObjectAudience.of(store, spaceId, AccessContext.of(Caller.of(ada), Date.now())),
        ObjectAudience.of(store, spaceId, AccessContext.of(Caller.of(ada), Date.now())),
    ]);
    const subscription = new AbortController();
    onTestFinished(() => subscription.abort());
    const pages = store.feed.subscribe(
        { notebooks: { table: notebook.table, scopes: [spaceId] } },
        undefined,
        subscription.signal,
        { audience: following },
    );
    const isShared = following.key === identical.key;

    // send the notebook, then delete it once the clock reaches the expiry
    const inserted = await take(pages, (page) =>
        page.changes.some((change) => change.operation === "insert"),
    );
    clock.advance(SHARED_MILLISECONDS);
    const deleted = await take(pages, (page) =>
        page.changes.some((change) => change.operation === "delete"),
    );
    expect({
        isShared,
        changes: [...inserted, ...deleted].flatMap((page) =>
            page.changes.map((change) => [change.operation, change.row["id"]]),
        ),
        isExpired: clock.now() >= expiresAt,
    }).toEqual({
        isShared: true,
        changes: [
            ["insert", shared],
            ["delete", shared],
        ],
        isExpired: true,
    });
});

/** Take pages until one satisfies a condition, keeping the stream open. */
async function take(pages: AsyncGenerator<Page>, isLast: (page: Page) => boolean): Promise<Page[]> {
    // read the pages one by one, leaving the stream open for a later take
    const taken: Page[] = [];
    for (let next = await pages.next(); next.done !== true; next = await pages.next()) {
        taken.push(next.value);
        if (isLast(next.value)) {
            break;
        }
    }

    return taken;
}
