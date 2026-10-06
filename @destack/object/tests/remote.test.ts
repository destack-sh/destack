import { expect, onTestFinished, test } from "@destack/test";
import { Table, TABLE } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { present, schema } from "@destack/schema";
import { RemoteClient } from "../src/client/index.ts";
import { describeObject } from "../src/inspect/index.ts";
import { serveNotes, spaceId } from "./fixture/device.ts";
import { note, notebook, notesDatabase, notesService } from "./fixture/note.ts";

/** A row's identifier. */
const Identified = schema.looseObject({ id: schema.string() });

/** Reach the served notes as a user by their descriptions and the database's declared tables. */
async function reachNotes(
    endpoint: ReturnType<Awaited<ReturnType<typeof serveNotes>>["endpoint"]>,
) {
    // build each type's table from the database's declared SQLite states, under the type's properties
    const states = notesDatabase.state().tables.sqlite;
    const objects = [notebook, note].map((type) => {
        const description = describeObject(type);
        const state = present(
            states.find((entry) => entry.table.name === description.table),
            `the state of ${description.table}`,
        );
        const properties = Object.fromEntries(
            Object.entries(description.columns).map(([property, column]) => [column, property]),
        );

        return { description, table: Table.describe(state, properties) };
    });

    // keep watched rows in a local database
    const local = await TestDatabase.create("sqlite", []);
    onTestFinished(() => local.close());

    return new RemoteClient({
        package: notesService.package,
        objects,
        scope: spaceId,
        endpoint,
        database: local.database,
    });
}

test.for(TEST_DIALECTS)(
    "call and list objects by their descriptions, refusing input their methods' schemas reject on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const remote = await reachNotes(endpoint("alice"));

        // create a notebook and a note by name, the note pushed as a mutation of one call
        const book = Identified.parse(await remote.call("notebook", "create", { name: "Travel" }));
        const tickets = Identified.parse(
            await remote.call("note", "create", { parentId: book.id, title: "Tickets" }),
        );
        await remote.call("note", "create", { parentId: book.id, title: "Maps" });

        // list the notes a condition selects, in order, and read one by its get method
        const listed = await remote.list("note", {
            where: { parentId: book.id },
            orderBy: { title: "asc" },
        });
        const read = await remote.call("note", "get", { id: tickets.id });
        expect([
            listed.map((row) => row["title"]),
            schema.looseObject({ title: schema.string() }).parse(read).title,
        ]).toEqual([["Maps", "Tickets"], "Tickets"]);

        // refuse input the method's schema rejects, an unknown method and an unknown type
        await expect(remote.call("note", "create", { title: 5 })).rejects.toMatchObject({
            code: "BAD_REQUEST",
            message: "Input validation failed",
        });
        await expect(remote.call("note", "archive", {})).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "note has no method archive",
        });
        await expect(remote.list("page")).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: "no object type page",
        });
    },
);

test.for(TEST_DIALECTS)(
    "watch the rows a query selects through a copy of their table, after every change on %s",
    async (dialect) => {
        const { endpoint } = await serveNotes(dialect);
        const remote = await reachNotes(endpoint("alice"));
        const book = Identified.parse(await remote.call("notebook", "create", { name: "Travel" }));
        await remote.call("note", "create", { parentId: book.id, title: "Tickets" });

        // follow the pinned and unpinned notes by title, reading the first rows the copy has
        const controller = new AbortController();
        onTestFinished(() => controller.abort());
        const watched = remote.watch(
            "note",
            { where: { parentId: book.id }, orderBy: { title: "asc" }, limit: 2 },
            controller.signal,
        );
        const titles = async () => {
            const next = await watched.next();

            return next.done === true ? undefined : next.value.map((row) => row["title"]);
        };
        expect(await titles()).toEqual(["Tickets"]);

        // read the rows again once another note arrives, within the limit
        await remote.call("note", "create", { parentId: book.id, title: "Maps" });
        expect(await titles()).toEqual(["Maps", "Tickets"]);
        await remote.call("note", "create", { parentId: book.id, title: "Packing" });
        expect(await titles()).toEqual(["Maps", "Packing"]);

        // stop on abort and drop the copy's rows
        controller.abort();
        expect(await watched.next()).toEqual({ done: true, value: undefined });
        expect(
            await remote.installation.database.select().from(remote.object("note").table),
        ).toEqual([]);
        expect(remote.object("note").table[TABLE].sqlName).toBe(note.table[TABLE].sqlName);
    },
);
