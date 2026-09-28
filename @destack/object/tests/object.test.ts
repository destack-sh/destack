import { expect, onTestFinished, test } from "@destack/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { eq } from "@destack/db";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import { defineObject, field, Manager, method } from "../src/index.ts";
import { principal, relation, through, union } from "@destack/access";
import { defineReconciler, Reconciliation } from "../src/server/index.ts";
import { label, note, objectDatabase, team } from "./schema.ts";
import { space } from "./fixture/space.ts";

/** The space the declared objects live in. */
const spaceId = identifier("space").parse(`space-${v7()}`);

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
        const reconcilers = [
            // list labels before notes
            defineReconciler(label, {
                after: [note],
                resolve: async (_name, declared, context) => {
                    if (declared.note === "later") {
                        context.wait("note later is not declared yet");
                    }

                    return {
                        note: identifier("note").parse(await context.require(note, declared.note)),
                        text: declared.text,
                    };
                },
                values: (_name, resolved) => resolved,
            }),
            defineReconciler(note, {
                values: (_name, declared) => ({ title: declared.title }),
            }),
        ];
        const apply = (document: Readonly<Record<string, unknown>>) =>
            Reconciliation.apply({ database, reconcilers, manager, scope: spaceId, document });
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
            changes: {
                notes: [
                    { action: "create", name: "one" },
                    { action: "create", name: "two" },
                ],
                labels: [{ action: "create", name: "urgent" }],
            },
        });

        // reject collections no object reads and references to undeclared objects
        await expect(apply({ tags: {} })).rejects.toMatchObject({
            code: "UNSUPPORTED_DECLARATION",
            message: "this host cannot apply tags",
        });
        await expect(
            apply({ notes: {}, labels: { stray: { note: "missing", text: "stray" } } }),
        ).rejects.toMatchObject({ code: "INVALID_DECLARATION", message: "unknown note: missing" });

        // stop at a declaration that waits, keeping what was written before it
        expect(
            await apply({
                notes: { one: { title: "renamed" }, two: { title: "second" } },
                labels: { soon: { note: "later", text: "soon" } },
            }),
        ).toEqual({
            changes: {
                notes: [
                    {
                        action: "update",
                        name: "one",
                        fields: { title: { before: "first", after: "renamed" } },
                    },
                ],
            },
            waiting: "note later is not declared yet",
        });

        // detach one note, then retire labels before notes and keep it
        await database
            .update(note.table)
            .set({ detachedAt: 1 })
            .where(eq(note.table.managerName, "two"));
        expect(await apply({})).toEqual({
            changes: {
                notes: [{ action: "delete", name: "one" }],
                labels: [{ action: "delete", name: "urgent" }],
            },
        });
        expect(await titles()).toEqual([["two", "second"]]);
    },
);

test("refuse aggregates that measure rows some readers of their holder may not list", () => {
    // hold a count of books on shelves everyone related reads
    const shelf = defineObject({
        name: "shelf",
        plural: "shelves",
        scope: space,
        fields: { bookCount: field.count() },
        relations: { reader: { subjects: [principal.user] } },
        permissions: { read: relation("reader") },
        methods: { get: method.get("read"), list: method.list("read") },
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
            methods: { list: method.list("read") },
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
    const reconcilers = [
        defineReconciler(label, { after: [note], values: () => ({}) }),
        defineReconciler(note, { after: [label], values: () => ({}) }),
        defineReconciler(team, { after: "every", values: () => ({}) }),
    ];

    // report the cycle instead of overflowing the stack
    await expect(
        Reconciliation.apply({
            database: undefined as never,
            reconcilers,
            manager,
            scope: spaceId,
            document: {},
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
            methods: { delete: method.delete("write") },
        }),
    ).toThrow(new TypeError("object draft declares method delete, which a trait derives"));
});
