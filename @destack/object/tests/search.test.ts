import { principal, relation } from "@destack/access";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, Clock } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

/** The space keeping the notes. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d2");

/** Notes searched by their title and status. */
const note = defineObject({
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
}).methods((method) => ({
    create: method.create("write"),
    update: method.update("write"),
    list: method.list("read"),
}));

test.for(TEST_DIALECTS)(
    "list the %s objects whose searchable fields hold every word of a search, beside other conditions",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            defineDatabase({ name: "notes", tables: [...note.tables, ...callTables] }),
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const store = new ObjectStore({
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: [note],
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: { package: note.package },
        });
        const context = userContext("user-1", spaceId);

        // write three notes, then rename one
        const written = await store.mutate(context.caller(), {
            id: RequestId.create(Date.now()),
            calls: [
                note.call("create", { spaceId, title: "Ada Lovelace", status: "draft" }),
                note.call("create", { spaceId, title: "Analytical Engine", status: "published" }),
                note.call("create", { spaceId, title: "Café notes", status: "published" }),
            ],
        });
        await store.mutate(context.caller(), {
            id: RequestId.create(Date.now()),
            calls: [note.call("update", { spaceId, id: written[1].id, title: "Engine of Ada" })],
        });

        // list by a search alone, beside a condition, and by a search matching nothing
        const titles = async (where: object) =>
            (
                await store.query(
                    context.caller(),
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
    },
);
