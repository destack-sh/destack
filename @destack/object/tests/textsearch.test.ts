import type { Environment } from "@destack/environment";
import { testSimulation } from "../src/test/index.ts";
import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { chunk, CHUNK_CHARACTERS, defineObject, field, Sequence } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space with the documents. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d3");

/** Documents searched by their title and their text body. */
const document = defineObject(
    {
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
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write"),
        list: method.list("read"),
    }),
);

/** Build a new request identifier. */
function request(world: Environment): string {
    return RequestId.create(world);
}

test("list the objects with every word of a search in their searchable fields and text, across chunks", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        defineDatabase({
            name: "documents",
            ...tablesOf({
                packages: [objectPackage, { objects: { document }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: document.package },
    });
    const context = userContext(users.frank, spaceId, { environment: world });

    // write a long body whose first and last words land in different chunks, and a short one
    const [physics, notes] = await store.mutate(context.access(), {
        id: request(world),
        calls: [
            document.call("create", { spaceId, title: "Physics" }),
            document.call("create", { spaceId, title: "Quantum notes" }),
        ],
    });
    const filler = "wave ".repeat(CHUNK_CHARACTERS / 5);
    const long = `quantum ${filler}entanglement`;
    await store.mutate(context.access(), {
        id: request(world),
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
                edits: new Sequence().change({ from: 0, to: 0, insert: "nothing new" }, "user.2"),
            }),
        ],
    });

    // search titles, bodies across chunks, both together, and words no document has
    const chunks = await storage.database.select().from(chunk);
    const titles = async (search: string) =>
        (
            await store.query(
                context.access(),
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
});
