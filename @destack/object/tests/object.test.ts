import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { type DatabaseConnection, eq } from "@destack/db";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { schema, type JsonObject } from "@destack/schema";
import { v7 } from "uuid";
import { defineObject, field, Intrinsic } from "../src/index.ts";
import { contained, Manager, principal, relation, through, union } from "@destack/access";
import { RequestId } from "@destack/runtime/request";
import { Scope } from "@destack/sync";
import { ObjectStore, Stack } from "../src/server/index.ts";
import { comment, label, note, objectDatabase, task } from "./schema.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { request } from "./fixture/request.ts";
import { Clock } from "@destack/environment";

/** The space the declared objects live in. */
const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** Cards stacks declare by name, which their owners also write directly. */
const card = defineObject(
    {
        name: "card",
        plural: "cards",
        scope: space,
        declarable: { schema: schema.object({ title: schema.string() }) },
        fields: { owner: field.reference(principal.user).caller(), title: field.string() },
        permissions: { write: relation("owner") },
        adoptable: { by: "write" },
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

test.each(TEST_DIALECTS)(
    "read a declaration's release through its step's database while applying, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
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
                    resolve: async (_name, declared, context) => {
                        await context.release(manager.packageId);

                        return declared;
                    },
                    values: (_name, declared) => ({ title: declared.title }),
                },
            }),
        ];
        const applied = Stack.apply({
            now: Date.now(),
            database,
            objects,
            manager,
            as: spacePrincipal(spaceId),
            scope: spaceId,
            document: { notes: { one: { title: "first" } } },
            store: await serve(database),
            release: async (step) => {
                await step.read(note.table).findMany();
                throw new TypeError("no release in this test");
            },
        });

        // refuse with the release's failure instead of waiting behind the step's own transaction
        await expect(applied).rejects.toThrow(new TypeError("no release in this test"));
    },
);

test.each(TEST_DIALECTS)(
    "apply declared objects in dependency order, wait, leave detached and retire in reverse on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        const objects = [
            // list labels before notes
            label.implement({
                declaration: {
                    after: [note],
                    resolve: async (_name, declared, context) => {
                        if (declared.note === "later") {
                            context.wait("note later is not declared yet");
                        }

                        return {
                            noteId: schema
                                .identifier("note")
                                .parse(await context.require(note, declared.note)),
                            text: declared.text,
                        };
                    },
                    values: (_name, resolved) => resolved,
                },
            }),
            note.implement({
                declaration: {
                    values: (_name, declared) => ({ title: declared.title }),
                },
            }),
        ];
        await openSpace(database, spaceId);
        await database
            .insert(space.table)
            .values({ id: spaceId, scope: Scope.universe.id, createdAt: 0, updatedAt: 0 });
        const store = await serve(database);
        const apply = (document: JsonObject) =>
            Stack.apply({
                now: Date.now(),
                database,
                objects,
                manager,
                as: spacePrincipal(spaceId),
                scope: spaceId,
                document,
                store,
            });
        const titles = async () =>
            (await database.select().from(note.table).orderBy(note.table.managerName)).map(
                (row) => [row.managerName, row.title],
            );

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
        await expect(apply({ tags: {} })).rejects.toMatchObject({
            code: "UNSUPPORTED_DECLARATION",
            message: "this server cannot apply tags",
        });
        await expect(
            apply({ notes: {}, labels: { stray: { note: "missing", text: "stray" } } }),
        ).rejects.toMatchObject({ code: "INVALID_DECLARATION", message: "unknown note: missing" });

        // stop at a waiting declaration and retain the earlier writes
        expect(
            await apply({
                notes: { one: { title: "renamed" }, two: { title: "second" } },
                labels: { soon: { note: "later", text: "soon" } },
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
            deferred: "note later is not declared yet",
        });

        // detach one note and retire labels before notes, leaving it
        await database
            .update(note.table)
            .set({ detachedAt: 1 })
            .where(eq(note.table.managerName, "two"));
        expect(await apply({})).toEqual({
            steps: [
                { action: "delete", target: "label/urgent", risk: "destructive", detail: "retire" },
                { action: "delete", target: "note/one", risk: "destructive", detail: "retire" },
            ],
        });
        expect(await titles()).toEqual([["two", "second"]]);
    },
);

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
    expect(() => book(union(relation("owner")))).toThrow(
        new TypeError(
            "aggregate bookCount of book counts rows that readers of shelf may not list: list them through parent read",
        ),
    );
});

test("refuse a containment grant on a type living in the universe alone, and allow it on a scope type there", () => {
    // declare the scope type, deciding by its own row's containment, and refuse the record
    expect(ledger(true).name).toBe("ledger");
    expect(() => ledger(false)).toThrow(
        new TypeError(
            "object ledger lives in the universe without principals: contained() in its read permission admits no one",
        ),
    );
});

test("refuse declarations that depend on themselves, also next to one depending on every other", async () => {
    // make notes and labels depend on each other, with a third type depending on every other
    const objects = [
        label.implement({ declaration: { after: [note], values: () => ({}) } }),
        note.implement({ declaration: { after: [label], values: () => ({}) } }),
        task.implement({ declaration: { after: "every", values: () => ({}) } }),
    ];

    // report the cycle instead of overflowing the stack
    const storage = await TestDatabase.create("sqlite", [], { isMigrated: true });
    onTestFinished(() => storage.close());
    await expect(
        Stack.apply({
            now: Date.now(),
            database: storage.database,
            objects,
            manager,
            as: spacePrincipal(spaceId),
            scope: spaceId,
            document: {},
            store: await serve(storage.database),
        }),
    ).rejects.toMatchObject({
        code: "CYCLIC_DECLARATION",
        message: "declarations of labels depend on themselves",
    });
});

test.each(TEST_DIALECTS)(
    "adopt a direct record into a declaration whose next apply rewrites it, refusing managed records and a second record per declaration on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [...card.tables, ...callTables], {
            isMigrated: true,
        });
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

        // write two cards directly as alice, and hand them over as her
        const context = userContext(users.alice, spaceId);
        const create = (title: string) =>
            store
                .mutate(context.access(), {
                    id: RequestId.create(Date.now()),
                    calls: [card.call("create", { spaceId, title })],
                })
                .then(([written]) => written);
        const welcome = { ...manager, name: "welcome" };
        const hand = (name: "adopt" | "detach", id: string, declaration = welcome) =>
            request(
                store,
                card,
                name,
                spaceId,
                {
                    id,
                    requestId: RequestId.create(Date.now()),
                    ...(name === "adopt" ? { manager: declaration } : {}),
                },
                context,
            );
        const [first, second] = [await create("Draft"), await create("Notes")];

        // adopt the first card, and rewrite it with the declared values on the next apply
        const adopted = await hand("adopt", first.id);
        const applied = await Stack.apply({
            now: Date.now(),
            database,
            objects: [
                card.implement({
                    declaration: { values: (_name, declared) => ({ title: declared.title }) },
                }),
            ],
            manager,
            as: spacePrincipal(spaceId),
            scope: spaceId,
            document: { cards: { welcome: { title: "Welcome" } } },
            store,
        });
        const [rewritten] = await database
            .select()
            .from(card.table)
            .where(eq(card.table.id, first.id));
        expect({
            adopted: [adopted.managerName, adopted.detachedAt, adopted.title],
            applied,
            rewritten: [rewritten?.managerName, rewritten?.title],
        }).toEqual({
            adopted: ["welcome", null, "Draft"],
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
            rewritten: ["welcome", "Welcome"],
        });

        // refuse adopting a managed card, and a second card into the declaration
        expect([
            await refusal(hand("adopt", first.id, { ...manager, name: "other" })),
            await refusal(hand("adopt", second.id)),
        ]).toEqual([
            ["CONFLICT", "card is managed by a declaration"],
            ["CONFLICT", "declaration welcome declares another card"],
        ]);

        // detach the first card, refuse it to another declaration, and adopt it back into its own
        const detached = await hand("detach", first.id);
        const elsewhere = await refusal(hand("adopt", first.id, { ...manager, name: "other" }));
        const readopted = await hand("adopt", first.id);
        expect({
            detached: [detached.managerName, detached.detachedAt === null],
            elsewhere,
            readopted: [readopted.managerName, readopted.detachedAt, readopted.title],
        }).toEqual({
            detached: ["welcome", false],
            elsewhere: ["CONFLICT", "card is detached from another declaration"],
            readopted: ["welcome", null, "Welcome"],
        });
    },
);

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
    ).toThrow(new TypeError("object memo methods update and retitle both answer PATCH /{id}"));
});

test("manage a declarable type over its own or a foreign table, and leave an undeclared type unmanaged", () => {
    // declare roles over access's role table, as a stack declares its roles
    const role = defineObject(
        {
            ...Intrinsic.role.definition(space),
            declarable: { schema: schema.object({ description: schema.string() }) },
        },
        Intrinsic.role.methods,
    );

    expect([card.isManaged(), role.isManaged(), comment.isManaged()]).toEqual([true, true, false]);
});
