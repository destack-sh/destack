import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, Clock } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { chunk, CHUNK_CHARACTERS, defineObject, field, Sequence } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

/** The space keeping the documents. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d3");

/** Documents searched by their title and their text body. */
const document = defineObject({
    name: "document",
    plural: "documents",
    scope: space,
    search: { searchable: ["title", "body"] },
    fields: {
        owner: field.reference(principal.user).caller(),
        title: field.string(),
        body: field.text(),
    },
    permissions: { read: relation("owner"), write: relation("owner") },
}).methods((method) => ({
    create: method.create("write"),
    update: method.update("write"),
    list: method.list("read"),
}));

/** Build a new request identifier. */
function request(): string {
    return RequestId.create(Date.now());
}

test.for(TEST_DIALECTS)(
    "list the %s objects holding every word of a search in their searchable fields and text, across chunks",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({ name: "documents", tables: [...document.tables, ...callTables] }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: [document],
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: { package: document.package },
        });
        const context = userContext("user-1", spaceId);

        // write a long body whose first and last words land in different chunks, and a short one
        const [physics, notes] = await store.mutate(context.caller(), {
            id: request(),
            calls: [
                document.call("create", { spaceId, title: "Physics" }),
                document.call("create", { spaceId, title: "Quantum notes" }),
            ],
        });
        const filler = "wave ".repeat(CHUNK_CHARACTERS / 5);
        const long = `quantum ${filler}entanglement`;
        await store.mutate(context.caller(), {
            id: request(),
            calls: [
                document.call("edit", {
                    spaceId,
                    id: physics.id,
                    field: "body",
                    edits: new Sequence().change({ from: 0, to: 0, insert: long }, "user.1"),
                }),
                document.call("edit", {
                    spaceId,
                    id: notes.id,
                    field: "body",
                    edits: new Sequence().change(
                        { from: 0, to: 0, insert: "nothing new" },
                        "user.2",
                    ),
                }),
            ],
        });

        // search titles, bodies across chunks, both together, and words no document has
        const chunks = await storage.database.select().from(chunk);
        const titles = async (search: string) =>
            (
                await store.query(
                    context.caller(),
                    document.call("list", {
                        spaceId,
                        where: { SEARCH: search },
                        orderBy: { title: "asc" },
                    }),
                )
            ).items.map((item: { readonly title: string }) => item.title);
        expect({
            chunked: chunks.filter((row) => row.parentId === physics.id).length > 1,
            endsAtWords: chunks
                .filter((row) => row.parentId === physics.id)
                .toSorted((left, right) => (left.position < right.position ? -1 : 1))
                .slice(0, -1)
                .every((row) => row.text.endsWith(" ")),
            quantum: await titles("quantum"),
            acrossChunks: await titles("quantum entang"),
            titleAndBody: await titles("notes nothing"),
            none: await titles("quantum zebra"),
        }).toEqual({
            chunked: true,
            endsAtWords: true,
            quantum: ["Physics", "Quantum notes"],
            acrossChunks: ["Physics"],
            titleAndBody: ["Quantum notes"],
            none: [],
        });
    },
);
