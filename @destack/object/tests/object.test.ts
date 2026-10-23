import { expect, onTestFinished, refusal, single, test } from "@destack/test";
import { TestDatabase } from "@destack/db/test";
import { type DatabaseConnection, eq } from "@destack/db";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { schema, type JsonObject } from "@destack/schema";
import { v7 } from "uuid";
import { defineObject, field, ObjectPolicy, type ObjectType } from "../src/index.ts";
import {
    accessRole,
    contained,
    Manager,
    principal,
    relation,
    through,
    none,
    union,
} from "@destack/access";
import { RequestId, Scope } from "@destack/sync";
import { ObjectStore, Stack } from "../src/server/index.ts";
import { defineResourceKind } from "@destack/resource";
import { ObjectDescription } from "../src/object/description.ts";
import { label, note, objectDatabase, task } from "./schema.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** The space the declared objects live in. */
const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** Cards stacks declare by name, which their owners also write directly. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: space,
        fields: { owner: field.reference(principal.user).caller(), title: field.string() },
        permissions: { write: relation("owner") },
    },
    (method) => ({ create: method.create("write") }),
);

/** Serve the declared objects over a database, applying their records. */
async function serve(database: DatabaseConnection): Promise<ObjectStore> {
    return new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([label, note, task]),
        policies: [space],
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: note.package },
    });
}

/** The installation and package declaring the objects. */
const manager = Manager.schema.omit({ name: true }).parse({
    installationId: "installation-01996ab0-0000-7000-8000-000000000001",
    packageId: "package-01996ab0-0000-7000-8000-000000000002",
});

test("read a declaration's release through its step's database while applying", async () => {
    const storage = await TestDatabase.create(objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);
    await database
        .insert(space.table)
        .values({ id: spaceId, scope: Scope.universe.id, createdAt: 0, updatedAt: 0 });

    // resolve each note through a release that reads the notes, then refuses
    const objects = [
        note.implement({
            declaration: {
                resolve: async (_name, declared, plan) => {
                    await plan.release(manager.packageId);

                    return declared;
                },
            },
        }),
    ];
    const applied = new Stack({
        database,
        objects,
        manager,
        as: spacePrincipal(spaceId),
        scope: spaceId,
        document: { notes: { one: { title: "first" } } },
        applied: {},
        store: await serve(database),
        release: async (step) => {
            await step.read(note.table).findMany();
            throw new TypeError("no release in this test");
        },
    }).apply(Date.now());

    // refuse with the release's failure instead of waiting behind the step's transaction
    await expect(applied).rejects.toThrow(new TypeError("no release in this test"));
});

test("apply declared objects in dependency order, wait, leave detached and retire in reverse", async () => {
    const storage = await TestDatabase.create(objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    const objects = [
        // list labels before notes
        label.implement({
            declaration: {
                // wait on a label whose text is not ready yet
                resolve: async (_name, declared, plan) => {
                    if (declared.text === "later") {
                        plan.wait("label soon is not ready yet");
                    }

                    return declared;
                },
            },
        }),
        note,
    ];
    await openSpace(database, spaceId);
    await database
        .insert(space.table)
        .values({ id: spaceId, scope: Scope.universe.id, createdAt: 0, updatedAt: 0 });
    const store = await serve(database);
    let applied: JsonObject = {};
    const apply = async (document: JsonObject) => {
        // apply the document against the one applied last
        const plan = await new Stack({
            database,
            objects,
            manager,
            as: spacePrincipal(spaceId),
            scope: spaceId,
            document,
            applied,
            store,
        }).apply(Date.now());
        applied = document;

        return plan;
    };
    const titles = async () =>
        (await database.select().from(note.table).orderBy(note.table.managerName)).map((row) => [
            row.managerName,
            row.title,
        ]);

    // create notes before the labels referencing them
    expect(
        await apply({
            notes: { one: { title: "first" }, two: { title: "second" } },
            labels: { urgent: { note: "one", text: "urgent" } },
        }),
    ).toEqual({
        steps: [
            { action: "create", target: "note/one", risk: "safe", detail: "declare" },
            { action: "create", target: "note/two", risk: "safe", detail: "declare" },
            { action: "create", target: "label/urgent", risk: "safe", detail: "declare" },
        ],
    });

    // reject collections no object reads and references to undeclared objects
    expect(await refusal(apply({ tags: {} }))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "UNSUPPORTED_DECLARATION",
        "this server cannot apply tags",
    ]);
    expect(
        await refusal(apply({ notes: {}, labels: { stray: { note: "missing", text: "stray" } } })),
    ).toEqual(["INTERNAL_SERVER_ERROR", "INVALID_DECLARATION", "unknown note: missing"]);

    // stop at a waiting declaration and retain the earlier writes
    expect(
        await apply({
            notes: { one: { title: "renamed" }, two: { title: "second" } },
            labels: { soon: { note: "one", text: "later" } },
        }),
    ).toEqual({
        steps: [
            {
                action: "update",
                target: "note/one",
                risk: "safe",
                detail: "change declared fields",
                fields: { title: { before: "first", after: "renamed" } },
            },
        ],
        deferred: "label soon is not ready yet",
    });

    // release one note and retire labels before notes, keeping it
    expect(await apply({ released: ["notes/two"] })).toEqual({
        steps: [
            { action: "delete", target: "label/urgent", risk: "destructive", detail: "retire" },
            { action: "delete", target: "note/one", risk: "destructive", detail: "retire" },
        ],
    });
    expect(await titles()).toEqual([["two", "second"]]);
});

test("plan nothing when a document is applied again unchanged, tags and references included", async () => {
    const storage = await TestDatabase.create(objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);
    await database
        .insert(space.table)
        .values({ id: spaceId, scope: Scope.universe.id, createdAt: 0, updatedAt: 0 });
    const store = await serve(database);
    const document = {
        notes: { one: { title: "first", tags: { team: "design" } } },
        labels: { urgent: { note: "one", text: "urgent" } },
    };
    const apply = (applied: JsonObject) =>
        new Stack({
            database,
            objects: [label, note],
            manager,
            as: spacePrincipal(spaceId),
            scope: spaceId,
            document,
            applied,
            store,
        }).apply(Date.now());

    // create once, then plan nothing for the same document
    await apply({});
    expect(await apply(document)).toEqual({ steps: [] });
});

/** Vaults of a space, a resource kind only an installed package provisions. */
const vault = defineObject({
    name: "vault",
    plural: "vaults",
    scope: space,
    provisioned: { kind: defineResourceKind("vault", { spec: schema.object({}) }) },
    fields: {},
});

test("leave a collection no served type collects to the installed type of its plural, refusing it without one", async () => {
    const storage = await TestDatabase.create(objectDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);
    await database
        .insert(space.table)
        .values({ id: spaceId, scope: Scope.universe.id, createdAt: 0, updatedAt: 0 });

    // declare a note the store serves and a vault only an installed package does
    const apply = async (installing?: () => Promise<readonly ObjectType[]>) =>
        new Stack({
            database,
            objects: [note],
            manager,
            as: spacePrincipal(spaceId),
            scope: spaceId,
            document: { notes: { draft: { title: "Draft" } }, vaults: { secrets: {} } },
            applied: {},
            store: await serve(database),
            ...(installing === undefined ? {} : { installing }),
        }).apply(Date.now());

    // refuse the vaults without installed types, then plan them under the installed vault type
    const refused = await refusal(apply());
    const planned = await apply(async () => [vault]);
    expect([refused, planned]).toEqual([
        ["INTERNAL_SERVER_ERROR", "UNSUPPORTED_DECLARATION", "this server cannot apply vaults"],
        {
            steps: [
                { action: "create", target: "note/draft", risk: "safe", detail: "declare" },
                { action: "create", target: "vault/secrets", risk: "safe", detail: "declare" },
            ],
            deferred: "vaults wait for the store to serve their types",
        },
    ]);
});

test("refuse aggregates that measure rows some readers of their holder may not list", () => {
    // maintain a count of books on shelves everyone related reads
    const shelf = defineObject({
        name: "shelf",
        plural: "shelves",
        scope: space,
        fields: { bookCount: field.count() },
        relations: { reader: { subjects: [principal.user] } },
        permissions: { read: relation("reader") },
    });
    const book = (read: ReturnType<typeof union>) =>
        defineObject({
            name: "book",
            plural: "books",
            scope: space,
            nested: { in: shelf, receive: "read" },
            fields: { title: field.string().default("") },
            aggregates: { bookCount: { function: "count" } },
            relations: { owner: { subjects: [principal.user] } },
            permissions: { read },
        });

    // count books every shelf reader lists, and refuse books only their owners list
    expect(book(union(relation("owner"), through("parent", "read"))).name).toBe("book");
    expect(() => ObjectDescription.of(book(union(relation("owner"))))).toThrow(
        new TypeError(
            "aggregate bookCount of book counts rows that readers of shelf may not list: list them through parent read",
        ),
    );
});

test("refuse a containment grant on a type living in the universe alone, and allow it on a scope type there", () => {
    // declare the scope type, deciding by its row's containment, and refuse the record
    expect(ledger(true).name).toBe("ledger");
    expect(() => ObjectDescription.of(ledger(false))).toThrow(
        new TypeError(
            "object ledger is in the universe, which has no principals: contained() in its read permission allows no one",
        ),
    );
});

test("refuse declarations that depend on themselves, also next to one depending on every other", async () => {
    // declare notes and labels depending on each other, with a third type depending on every other
    const objects = [
        label,
        note.implement({ declaration: { after: [label] } }),
        task.implement({ declaration: { after: "every" } }),
    ];

    // report the cycle instead of overflowing the stack
    const storage = await TestDatabase.create(
        tablesOf({ packages: [], follows: [], tables: [] }).tables,
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const store = await serve(storage.database);
    expect(
        await refusal(
            () =>
                new Stack({
                    database: storage.database,
                    objects,
                    manager,
                    as: spacePrincipal(spaceId),
                    scope: spaceId,
                    document: {
                        notes: { first: { title: "First" } },
                        labels: { urgent: { note: "first", text: "Urgent" } },
                    },
                    applied: {},
                    store,
                }),
        ),
    ).toEqual([
        "INTERNAL_SERVER_ERROR",
        "CYCLIC_DECLARATION",
        "declarations of labels depend on themselves",
    ]);
});

test("adopt a direct record into a declaration once, rewriting it, release it once, and refuse records another declaration manages, undeclared adoptions and declared releases", async () => {
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { card }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([card]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: card.package },
    });

    // write two cards directly as alice
    const context = userContext(users.alice, spaceId);
    const create = (title: string) =>
        store
            .mutate(context.access(), {
                id: RequestId.create(Date.now()),
                calls: [card.call("create", { spaceId, title })],
            })
            .then(([written]) => written);
    const [first, second] = [await create("Draft"), await create("Notes")];
    let last: JsonObject = {};
    const apply = async (document: JsonObject, declaring = manager) => {
        // apply the document against the one applied last
        const plan = await new Stack({
            database,
            objects: [card],
            manager: declaring,
            as: spacePrincipal(spaceId),
            scope: spaceId,
            document,
            applied: last,
            store,
        }).apply(Date.now());
        last = document;

        return plan;
    };
    const read = async (id: typeof first.id) =>
        single(await database.select().from(card.table).where(eq(card.table.id, id)));

    // adopt the first card under its declaration's name, rewriting it with the declared values
    const welcome = { cards: { welcome: { title: "Welcome" } } };
    const applied = await apply({ ...welcome, adopted: { "cards/welcome": first.id } });
    const adopted = await read(first.id);

    // apply the same document again without writing the adopted card
    await apply({ ...welcome, adopted: { "cards/welcome": first.id } });
    const again = await read(first.id);

    // refuse adopting a second card into the declaration, and the first into another stack's
    const other = {
        ...manager,
        installationId: schema
            .identifier("installation")
            .parse("installation-01996ab0-0000-7000-8000-000000000009"),
    };
    const refusals = [
        await refusal(apply({ ...welcome, adopted: { "cards/welcome": second.id } })),
        await refusal(
            apply(
                { cards: { mine: { title: "Mine" } }, adopted: { "cards/mine": first.id } },
                other,
            ),
        ),
        await refusal(apply({ ...welcome, adopted: { "cards/other": second.id } })),
        await refusal(apply({ ...welcome, released: ["cards/welcome"] })),
    ];

    // release the first card, keeping it, then adopt it back into its declaration
    await apply({ released: ["cards/welcome"] });
    const released = await read(first.id);
    await apply({ released: ["cards/welcome"] });
    const releasedAgain = await read(first.id);
    await apply({ ...welcome, adopted: { "cards/welcome": first.id } });
    const readopted = await read(first.id);
    expect({
        applied,
        adopted: [adopted.managerName, adopted.detachedAt, adopted.title],
        again: again.revision - adopted.revision,
        refusals,
        released: [released.managerName, released.detachedAt === null, released.title],
        releasedRevisions: [
            released.revision - again.revision,
            releasedAgain.revision - released.revision,
        ],
        readopted: [readopted.managerName, readopted.detachedAt],
    }).toEqual({
        applied: {
            steps: [
                {
                    action: "update",
                    target: "card/welcome",
                    risk: "safe",
                    detail: "change declared fields",
                    fields: { title: { before: "Draft", after: "Welcome" } },
                },
            ],
        },
        adopted: ["welcome", null, "Welcome"],
        again: 0,
        refusals: [
            [
                "INTERNAL_SERVER_ERROR",
                "INVALID_DECLARATION",
                "card welcome already declares another record",
            ],
            [
                "INTERNAL_SERVER_ERROR",
                "INVALID_DECLARATION",
                `card ${first.id} is managed by another declaration`,
            ],
            [
                "INTERNAL_SERVER_ERROR",
                "INVALID_DECLARATION",
                `card other adopts ${second.id} but does not declare other`,
            ],
            [
                "INTERNAL_SERVER_ERROR",
                "INVALID_DECLARATION",
                "card welcome is both declared and released",
            ],
        ],
        released: ["welcome", false, "Welcome"],
        releasedRevisions: [1, 0],
        readopted: ["welcome", null],
    });
});

/** Boards stacks declare, trashed on retiring and replaced when their size changes. */
const board = defineObject(
    {
        name: "board",
        plural: "boards",
        scope: space,
        fields: { title: field.string(), size: field.integer() },
        recoverable: { within: { days: 1 }, by: "write" },
        permissions: ["write"],
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write", { fields: ["title"] }),
    }),
);

/** Badges stacks declare, archived inactive on retiring. */
const badge = defineObject(
    {
        name: "badge",
        plural: "badges",
        scope: space,
        fields: { title: field.string(), active: field.boolean().default(true) },
        permissions: ["write"],
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write", { fields: ["title", "active"] }),
    }),
);

test("restore a trashed declared record, replace one whose changed field its type cannot update, and activate an archived one declared again", async () => {
    const boards = board.implement({ declaration: {} });
    const badges = badge.implement({ declaration: { retire: "archived" } });
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { board, badge }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const { database } = storage;
    await openSpace(database, spaceId);
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([boards, badges]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: { package: board.package },
    });
    let last: JsonObject = {};
    const apply = async (document: JsonObject) => {
        // apply the document against the one applied last, listing each step's action and fields
        const plan = await new Stack({
            database,
            objects: [boards, badges],
            manager,
            as: spacePrincipal(spaceId),
            scope: spaceId,
            document,
            applied: last,
            store,
        }).apply(Date.now());
        last = document;

        return plan.steps.map((step) => [
            step.action,
            step.target,
            step.risk,
            Object.keys(step.fields ?? {}),
        ]);
    };
    const rows = async () => ({
        boards: (await database.select().from(board.table)).map((row) => [
            row.size,
            row.deletedAt === null,
        ]),
        badges: (await database.select().from(badge.table)).map((row) => [row.title, row.active]),
    });

    // declare a board and a badge, then retire the board to the trash
    const declared = {
        boards: { main: { title: "Main", size: 3 } },
        badges: { gold: { title: "Gold" } },
    };
    await apply(declared);
    const retired = await apply({ badges: declared.badges });

    // restore the board on the next apply, then replace it for a new size
    const restored = await apply(declared);
    const replaced = await apply({ ...declared, boards: { main: { title: "Main", size: 4 } } });
    const afterReplace = await rows();

    // archive the badge once undeclared, then activate it once declared again
    const archived = await apply({ boards: { main: { title: "Main", size: 4 } } });
    const archivedRows = await rows();
    const activated = await apply({ ...declared, boards: { main: { title: "Main", size: 4 } } });
    expect({
        retired,
        restored,
        replaced,
        afterReplace,
        archived,
        archivedRows,
        activated,
        final: await rows(),
    }).toEqual({
        retired: [["delete", "board/main", "destructive", []]],
        restored: [["restore", "board/main", "safe", ["deletedAt"]]],
        replaced: [["replace", "board/main", "destructive", ["size"]]],
        afterReplace: {
            boards: [
                [3, false],
                [4, true],
            ],
            badges: [["Gold", true]],
        },
        archived: [["update", "badge/gold", "safe", []]],
        archivedRows: {
            boards: [
                [3, false],
                [4, true],
            ],
            badges: [["Gold", false]],
        },
        activated: [["update", "badge/gold", "safe", ["active"]]],
        final: {
            boards: [
                [3, false],
                [4, true],
            ],
            badges: [["Gold", true]],
        },
    });
});

/** Declare a universe ledger, a scope or a record, readable by contained spaces. */
function ledger(isScope: boolean) {
    return defineObject({
        name: "ledger",
        plural: "ledgers",
        scope: "universe",
        ...(isScope ? { isScope } : {}),
        fields: { title: field.string().default("") },
        permissions: { read: union(relation("owner"), contained(principal.space)) },
        relations: { owner: { subjects: [principal.user] } },
    });
}

test("refuse two methods answering one route, which a router would send to one of them", () => {
    // declare a second update beside the first, both at the object's path
    expect(() =>
        ObjectDescription.of(
            defineObject(
                {
                    name: "memo",
                    plural: "memos",
                    scope: space,
                    fields: { title: field.string() },
                    permissions: ["read", "write"],
                },
                (method) => ({
                    update: method.update("write", { fields: ["title"] }),
                    retitle: method.update("write", { fields: ["title"] }),
                }),
            ),
        ),
    ).toThrow(new TypeError("object memo methods update and retitle both handle PATCH /{id}"));
});

test("manage every durable type with a create verb, and no type without one", () => {
    // read roles over access's role table without verbs
    const readRole = defineObject({
        name: "role",
        plural: "roles",
        mapping: ObjectPolicy.readMapping(accessRole),
    });

    expect([card.isDeclarable(), note.isDeclarable(), readRole.isDeclarable()]).toEqual([
        true,
        true,
        false,
    ]);
});

test("let roles grant every permission of a type but its reserved ones", () => {
    const vaulted = defineObject({
        name: "vaulted",
        plural: "vaulteds",
        scope: "universe",
        fields: {},
        permissions: { read: none(), write: none(), consume: none() },
        reserved: ["consume"],
    });

    expect([vaulted.permissions, vaulted.grantable]).toEqual([
        ["read", "write", "consume"],
        ["read", "write"],
    ]);
});
