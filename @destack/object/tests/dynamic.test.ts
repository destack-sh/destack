import { Package } from "@destack/package";
import { expect, onTestFinished, test } from "@destack/test";
import { TABLE } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, Clock } from "@destack/schema";
import { DynamicObjectClient } from "../src/client/index.ts";
import { ObjectType } from "../src/index.ts";
import { space } from "./fixture/space.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook, notesPackage } from "./fixture/note.ts";

/** A row's identifier. */
const Identified = schema.looseObject({ id: schema.string() });

/** Reach the served notes as a user by the types their descriptions declare. */
async function reachNotes(
    endpoint: ReturnType<Awaited<ReturnType<typeof serveNotes>>["endpoint"]>,
) {
    // declare the types from their descriptions, as a manifest lists them
    const objects = ObjectType.ofAll([notebook.describe(), note.describe()], [space]);

    return new DynamicObjectClient({
        clock: Clock.system,
        package: Package.of(notesPackage),
        objects,
        scope: spaceId,
        endpoint,
    });
}

test.for(TEST_DIALECTS)(
    "call and list objects by their descriptions, refusing input their methods' schemas reject on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const client = await reachNotes(endpoint("alice"));

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
        await expect(client.call("note", "create", { title: 5 })).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "invalid input: title: invalid input: expected string, received number",
        });
        await expect(client.call("note", "archive", {})).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "note has no method archive",
        });
        await expect(client.list("page")).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "no object type page",
        });
    },
);

test.for(TEST_DIALECTS)(
    "watch the rows a query selects through a replica of their table, after every change on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const client = await reachNotes(endpoint("alice"));
        const book = Identified.parse(await client.call("notebook", "create", { name: "Travel" }));
        await client.call("note", "create", { parentId: book.id, title: "Tickets" });

        // keep the watched rows in a local database
        const local = await TestDatabase.create("sqlite", []);
        onTestFinished(() => local.close());

        // follow the pinned and unpinned notes by title, reading the first rows the replica has
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
    },
);

test.for(TEST_DIALECTS)(
    "describe the served types with the types they refer to, declare them from those descriptions alone, and refuse a type the store does not know on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const client = await reachNotes(endpoint("alice"));

        // describe every served type through the store, then declare each from the descriptions
        const declared = ObjectType.ofAll(await client.describe(), []);
        const unknown = { packageId: note.package.id, type: "page" };

        expect(declared.map((type) => [type.name, type.table[TABLE].sqlName])).toEqual([
            ["note", note.table[TABLE].sqlName],
            ["notebook", notebook.table[TABLE].sqlName],
            ["space", space.table[TABLE].sqlName],
        ]);
        await expect(client.describe([unknown])).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no object type page of ${note.package.id} is known here`,
        });
    },
);
