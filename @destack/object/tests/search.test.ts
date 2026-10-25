import { testSimulation } from "../src/test/index.ts";
import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { RequestId, Scope } from "@destack/sync";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { defineObject, field } from "../src/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space with the notes. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d2");

/** Notes searched by their title and status. */
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        scope: space,
        search: { searchable: ["title", "status"], sortable: ["title"] },
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(),
            status: field.enum(["draft", "published"]),
        },
        permissions: { read: relation("owner"), write: relation("owner") },
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write"),
        list: method.list("read"),
    }),
);

test("list the objects whose searchable fields have every word of a search, beside other conditions", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const storage = await TestDatabase.create(
        defineDatabase({
            name: "notes",
            ...tablesOf({
                packages: [objectPackage, { objects: { note }, tables: [] }],
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
        servers: serversOf([note]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: note.package },
    });
    const context = userContext(users.frank, spaceId, { environment: world });

    // write three notes, then rename one
    const written = await store.mutate(context.access(), {
        id: RequestId.create(world),
        calls: [
            note.call("create", { spaceId, title: "Ada Lovelace", status: "draft" }),
            note.call("create", { spaceId, title: "Analytical Engine", status: "published" }),
            note.call("create", { spaceId, title: "Café notes", status: "published" }),
        ],
    });
    await store.mutate(context.access(), {
        id: RequestId.create(world),
        calls: [note.call("update", { spaceId, id: written[1].id, title: "Engine of Ada" })],
    });

    // list only by a search, beside a condition, and by a search matching nothing
    const titles = async (where: object) =>
        (
            await store.query(
                context.access(),
                note.call("list", { spaceId, where, orderBy: { title: "asc" } }),
            )
        ).items.map((item: { readonly title: string }) => item.title);
    expect({
        ada: await titles({ SEARCH: "ada" }),
        publishedAda: await titles({ SEARCH: "ada pub" }),
        draftAda: await titles({ AND: [{ SEARCH: "ada" }, { status: "draft" }] }),
        cafe: await titles({ SEARCH: "CAFÉ" }),
        none: await titles({ SEARCH: "zzz" }),
    }).toEqual({
        ada: ["Ada Lovelace", "Engine of Ada"],
        publishedAda: ["Engine of Ada"],
        draftAda: ["Ada Lovelace"],
        cafe: ["Café notes"],
        none: [],
    });
});
