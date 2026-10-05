import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { type DatabaseConnection, eq } from "@destack/db";
import { testCallKey } from "@destack/service/test";
import { schema, type JsonObject } from "@destack/schema";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { Manager, principal, relation, through, union } from "@destack/access";
import { Scope } from "@destack/sync";
import { AccessFixture } from "@destack/access/test";
import { ObjectServer, Stack } from "../src/server/index.ts";
import { label, note, objectDatabase, task } from "./schema.ts";
import { space } from "./fixture/space.ts";

/** The space the declared objects live in. */
const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** Serve the declared objects over a database, applying their records. */
function serve(database: DatabaseConnection): ObjectServer {
    return new ObjectServer({
        objects: { label, note, task },
        policies: [space],
        database,
        callKey: testCallKey,
        origin: { package: note.package, service: "test" },
    });
}

/** The installation and package declaring the objects. */
const manager = Manager.schema.omit({ name: true }).parse({
    installationId: "installation-01996ab0-0000-7000-8000-000000000001",
    packageId: "package-01996ab0-0000-7000-8000-000000000002",
});

test.each(TEST_DIALECTS)(
    "apply declared objects in dependency order, wait, keep detached and retire in reverse on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, objectDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        const database = storage.database;
        const objects = [
            // list labels before notes
            label.declare({
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
            }),
            note.declare({
                values: (_name, declared) => ({ title: declared.title }),
            }),
        ];
        await new AccessFixture(database).copyScope(space.reference(Scope.universe.id, spaceId));
        const server = serve(database);
        const apply = (document: JsonObject) =>
            Stack.apply({ database, objects, manager, scope: spaceId, document, server });
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
            message: "this host cannot apply tags",
        });
        await expect(
            apply({ notes: {}, labels: { stray: { note: "missing", text: "stray" } } }),
        ).rejects.toMatchObject({ code: "INVALID_DECLARATION", message: "unknown note: missing" });

        // stop at a waiting declaration and keep the earlier writes
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

        // detach one note and retire labels before notes, keeping it
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
    // keep a count of books on shelves everyone related reads
    const shelf = defineObject({
        name: "shelf",
        plural: "shelves",
        scope: space,
        fields: { bookCount: field.count() },
        relations: { reader: { subjects: [principal.user] } },
        permissions: { read: relation("reader") },
        methods: (method) => ({ get: method.get("read"), list: method.list("read") }),
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
            methods: (method) => ({ list: method.list("read") }),
        });

    // count books every shelf reader lists, and refuse books only their owners list
    expect(book(union(relation("owner"), through("parent", "read"))).name).toBe("book");
    expect(() => book(union(relation("owner")))).toThrow(
        new TypeError(
            "aggregate bookCount of book measures rows that readers of shelf may not list: list them through parent read",
        ),
    );
});

test("refuse declarations that depend on themselves, also next to one depending on every other", async () => {
    // make notes and labels depend on each other, with a third type depending on every other
    const objects = [
        label.declare({ after: [note], values: () => ({}) }),
        note.declare({ after: [label], values: () => ({}) }),
        task.declare({ after: "every", values: () => ({}) }),
    ];

    // report the cycle instead of overflowing the stack
    const storage = await TestDatabase.create("sqlite", [], { isMigrated: true });
    onTestFinished(() => storage.close());
    await expect(
        Stack.apply({
            database: storage.database,
            objects,
            manager,
            scope: spaceId,
            document: {},
            server: serve(storage.database),
        }),
    ).rejects.toMatchObject({
        code: "CYCLIC_DECLARATION",
        message: "declarations of labels depend on themselves",
    });
});

test("refuse a declared method a trait derives", () => {
    expect(() =>
        defineObject({
            name: "draft",
            plural: "drafts",
            scope: space,
            recoverable: { within: { days: 1 }, by: "write" },
            fields: {},
            permissions: ["write"],
            methods: (method) => ({ delete: method.delete("write") }),
        }),
    ).toThrow(new TypeError("object draft declares method delete, which a trait derives"));
});
