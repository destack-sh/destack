import { defineObject, field, ObjectDescription, ObjectTypePool } from "../src/index.ts";
import { users } from "./fixture/user.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { TABLE } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";
import { DynamicObjectClient } from "../src/client/index.ts";
import { implementPackage, ObjectStore } from "../src/server/index.ts";
import { testEvents } from "../src/test/index.ts";
import { defineDatabase } from "@destack/db";
import { through } from "@destack/access";
import { Package } from "@destack/package";
import { definePackage } from "@destack/object";
import { testCallKey } from "@destack/object/test";
import { v7 } from "uuid";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook, notesPackage } from "./fixture/note.ts";
import objectPackage from "../src/package.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** A row's identifier. */
const Identified = schema.looseObject({ id: schema.string() });

/** Call the served notes as a user by the types their descriptions declare. */
async function connectNotes(
    endpoint: ReturnType<Awaited<ReturnType<typeof serveNotes>>["endpoint"]>,
) {
    // declare the types from their descriptions, as a manifest lists them
    const objects = ObjectTypePool.of([space]).declare([
        ObjectDescription.of(notebook),
        ObjectDescription.of(note),
    ]);

    return new DynamicObjectClient({
        clock: Clock.system,
        objects,
        scope: spaceId,
        endpoint,
    });
}

test("list the types descriptions refer to that neither the pool nor they declare", () => {
    // index the platform's space, then describe the note alone and beside its notebook
    const pool = ObjectTypePool.of([space]);

    // miss the notebook a note nests in, and nothing once the notebook is described too
    expect([
        pool.missing([ObjectDescription.of(note)]),
        pool.missing([ObjectDescription.of(notebook), ObjectDescription.of(note)]),
    ]).toEqual([[notebook.typeReference], []]);
});

test("call and list objects by their descriptions, refusing input their methods' schemas reject", async () => {
    const { endpoint } = await serveNotes();
    const client = await connectNotes(endpoint(users.alice));

    // create a notebook and a note by name, the note pushed as a mutation of one call
    const book = Identified.parse(await client.call("notebook", "create", { name: "Travel" }));
    const tickets = Identified.parse(
        await client.call("note", "create", { parentId: book.id, title: "Tickets" }),
    );
    await client.call("note", "create", { parentId: book.id, title: "Maps" });

    // list the notes a condition selects, in order, and read one by its get method
    const listed = await client.list("note", {
        where: { parentId: book.id },
        orderBy: { title: "asc" },
    });
    const read = await client.call("note", "get", { id: tickets.id });
    expect([
        listed.map((row) => row["title"]),
        schema.looseObject({ title: schema.string() }).parse(read).title,
    ]).toEqual([["Maps", "Tickets"], "Tickets"]);

    // refuse input the method's schema rejects, an unknown method and an unknown type
    expect(await refusal(client.call("note", "create", { title: 5 }))).toEqual([
        "BAD_REQUEST",
        "INVALID_INPUT",
        "invalid input: title: invalid input: expected string, received number",
    ]);
    expect(await refusal(client.call("note", "archive", {}))).toEqual([
        "BAD_REQUEST",
        "UNKNOWN_METHOD",
        "note has no method archive",
    ]);
    expect(await refusal(client.list("page"))).toEqual([
        "NOT_FOUND",
        "UNKNOWN_TYPE",
        "unknown object type page",
    ]);
});

test("watch the rows a query selects through a replica of their table, after every change", async () => {
    const { endpoint } = await serveNotes();
    const client = await connectNotes(endpoint(users.alice));
    const book = Identified.parse(await client.call("notebook", "create", { name: "Travel" }));
    await client.call("note", "create", { parentId: book.id, title: "Tickets" });

    // store the watched rows in a local database
    const local = await TestDatabase.create([], { dialect: "sqlite" });
    onTestFinished(() => local.close());

    // follow the starred and unstarred notes by title, reading the first rows the replica has
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const watched = client.watch(
        "note",
        { where: { parentId: book.id }, orderBy: { title: "asc" }, limit: 2 },
        local.database,
        controller.signal,
    );
    const titles = async () => {
        const next = await watched.next();

        return next.done === true ? undefined : next.value.map((row) => row["title"]);
    };
    expect(await titles()).toEqual(["Tickets"]);

    // read the rows again once another note arrives, within the limit
    await client.call("note", "create", { parentId: book.id, title: "Maps" });
    expect(await titles()).toEqual(["Maps", "Tickets"]);
    await client.call("note", "create", { parentId: book.id, title: "Packing" });
    expect(await titles()).toEqual(["Maps", "Packing"]);

    // stop on abort and drop the replica's rows
    controller.abort();
    expect(await watched.next()).toEqual({ done: true, value: undefined });
    expect(await local.database.select().from(client.object("note").table)).toEqual([]);
    expect(client.object("note").table[TABLE].sqlName).toBe(note.table[TABLE].sqlName);
});

test("describe the served types with the types they refer to, declare them from those descriptions alone, and refuse a type the store does not know", async () => {
    const { endpoint } = await serveNotes();
    const client = await connectNotes(endpoint(users.alice));

    // describe every served type through the store, then declare each from the descriptions
    const declared = ObjectTypePool.of([]).declare(await client.describe());
    const unknown = { packageId: note.package.id, type: "page" };

    expect(declared.map((type) => [type.name, type.table[TABLE].sqlName])).toEqual([
        ["note", note.table[TABLE].sqlName],
        ["notebook", notebook.table[TABLE].sqlName],
        ["space", space.table[TABLE].sqlName],
    ]);
    expect(await refusal(client.describe([unknown]))).toEqual([
        "NOT_FOUND",
        "UNKNOWN_TYPE",
        `unknown object type page of ${note.package.id}`,
    ]);
});

/** A shelf of a space: a separate scope, which bookmarks live in. */
const shelf = defineObject({
    name: "shelf",
    plural: "shelves",
    scope: space,
    isScope: true,
    fields: {
        /** The shelf's name. */
        name: field.string(),
    },
    permissions: { read: through("scope", "read"), write: through("scope", "read") },
});

/** A label another package stores, which bookmarks refer to though the store serves no labels. */
const label = defineObject({
    name: "label",
    plural: "labels",
    scope: space,
    fields: {
        /** The label's text. */
        text: field.string(),
    },
    permissions: { read: through("scope", "read") },
});

/** A bookmark stored on a shelf. */
const bookmark = defineObject({
    name: "bookmark",
    plural: "bookmarks",
    scope: shelf,
    fields: {
        /** The bookmarked address. */
        url: field.string(),
        /** The label of the bookmark, absent for none. */
        label: field.reference(() => label).optional(),
    },
    permissions: { read: through("scope", "read"), write: through("scope", "read") },
});

test("read objects of a scope inside the space a client is bound to, refusing a scope outside it, and describe every type they refer to", async () => {
    // serve shelves and their bookmarks over HTTP, bound to the space
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [
                    objectPackage,
                    notesPackage,
                    { objects: { shelf, bookmark }, tables: [] },
                ],
                follows: [],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);
    const shelves = definePackage({ objects: { shelf, bookmark } });
    const served = TestObjectStore.serve(
        new ObjectStore({
            servers: [implementPackage(shelves, {})],
            chain: async () => undefined,
            origin: { package: Package.of(shelves) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(storage.database),
            database: storage.database,
            clock: Clock.system,
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());

    // create a shelf, store a bookmark on it, and read the bookmark through a client of the space
    const client = new DynamicObjectClient({
        clock: Clock.system,
        objects: [shelf, bookmark],
        scope: spaceId,
        endpoint: served.as(users.alice).endpoint(shelves),
        homes: () => served.as(users.alice).endpoint(shelves),
    });
    const created = Identified.parse(await client.call("shelf", "create", { name: "Reading" }));
    const marked = Identified.parse(
        await client.call("bookmark", "create", {
            shelfId: created.id,
            url: "https://destack.com",
        }),
    );
    const read = await client.call("bookmark", "get", { shelfId: created.id, id: marked.id });

    // refuse a shelf of another space
    const elsewhere = `shelf-${v7()}`;
    const refused = await client.call("bookmark", "list", { shelfId: elsewhere }).then(
        () => "read",
        (error: { code?: string }) => error.code,
    );

    // describe the bookmark with every type it refers to, the label the store serves none of among them
    const described = await client.describe([bookmark.typeReference]);

    expect([
        schema.looseObject({ url: schema.string() }).parse(read).url,
        refused,
        described.map((type) => type.name).toSorted(),
    ]).toEqual(["https://destack.com", "NOT_FOUND", ["bookmark", "label", "shelf", "space"]]);
});
