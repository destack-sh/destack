import { principal, relation } from "@destack/access";
import { Expression } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { argument, plural, t } from "@destack/locale";
import { schema } from "@destack/schema";
import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { testSimulation } from "../src/test/index.ts";
import { ObjectClient, ObjectRemote } from "../src/client/index.ts";
import { defineObject, field, RICH_TEXT, searchable } from "../src/index.ts";
import { space } from "../src/test/space.ts";

/** Notes searched by their title. */
const note = defineObject(
    {
        name: "note",
        plural: "notes",
        title: t`${plural(argument("count"), { one: "Note", other: "Notes" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(schema.string()),
        },
        access: { permissions: { read: relation("owner"), write: relation("owner") } },
        traits: [searchable({ text: ["title"] })],
    },
    (method) => ({
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write"),
        delete: method.delete("write"),
    }),
);

/** Tasks searched by their title. */
const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        title: t`${plural(argument("count"), { one: "Task", other: "Tasks" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(schema.string()),
        },
        access: { permissions: { read: relation("owner"), write: relation("owner") } },
        traits: [searchable({ text: ["title"] })],
    },
    (method) => ({ list: method.list("read"), create: method.create("write") }),
);

/** Pages searched by their title and their rich text. */
const page = defineObject(
    {
        name: "page",
        plural: "pages",
        title: t`${plural(argument("count"), { one: "Page", other: "Pages" })}`,
        scope: space,
        fields: {
            owner: field.reference(principal.user).caller(),
            title: field.string(schema.string()),
            body: field.text(RICH_TEXT),
        },
        access: { permissions: { read: relation("owner"), write: relation("owner") } },
        traits: [searchable({ text: ["title", "body"] })],
    },
    (method) => ({
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write"),
    }),
);

test("search a client's rows live, as they are added and renamed", async () => {
    // open a client whose server never answers
    const environment = testSimulation().environment("view");
    const storage = await TestDatabase.create(ObjectClient.tables({ note, task }), { environment });
    const client = await ObjectClient.open({
        environment,
        database: storage.database,
        objects: { note, task },
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d3"),
        caller: principal.user.reference(
            Scope.universe.id,
            "user-01996ab0-0000-7000-8000-000000000003",
        ),
        remote: ObjectRemote.unanswered(),
    });
    onTestFinished(async () => {
        await client.close();
        await storage.close();
    });

    // search for a word's start, add a note with it, and rename the first
    const { id } = await client.mutate(note).create({ title: "Order form" }).predicted;
    const live = client
        .queryType(note)
        .findMany({ where: { SEARCH: "ord" }, orderBy: { title: "asc" } })
        .subscribe();
    const read = async () => (await live.read()).map((row) => row.title);
    const first = await read();
    await client.mutate(note).create({ title: "Reorder the order book" }).predicted;
    const added = await read();
    await client.mutate(note).update({ id, title: "Margins" }).predicted;
    const renamed = await read();
    await live.close();

    // order the notes by how relevant their titles are, as they are added
    for (const title of ["The order book", "Order", "Border order form"]) {
        await client.mutate(note).create({ title }).predicted;
    }
    const ordered = client
        .queryType(note)
        .findMany({
            where: { SEARCH: "order" },
            extras: { relevance: Expression.relevance(["title"], "order") },
            orderBy: { relevance: "desc", title: "asc" },
            limit: 3,
        })
        .subscribe();
    const best = (await ordered.read()).map((row) => row.title);
    await client.mutate(note).create({ title: "Order form" }).predicted;
    const bestAfter = (await ordered.read()).map((row) => row.title);
    await ordered.close();

    // search both types, as they are added
    const found = client.search("ord", { types: [note, task], limit: 3 }).subscribe();
    const hits = async () => (await found.read()).map(({ type, row }) => [type.name, row["title"]]);
    await client.mutate(task).create({ title: "Ord" }).predicted;
    const both = await hits();
    await client.mutate(note).create({ title: "ORD" }).predicted;
    const bothAfter = await hits();
    await found.close();
    const blank = client.search(" ", { types: [task], limit: 3 }).subscribe();
    const every = (await blank.read()).map(({ reference }) => reference.type);
    await blank.close();

    expect({ both, bothAfter, every }).toEqual({
        both: [
            ["task", "Ord"],
            ["note", "Order"],
            ["note", "Order form"],
        ],
        bothAfter: [
            ["note", "ORD"],
            ["task", "Ord"],
            ["note", "Order"],
        ],
        every: ["task"],
    });
    expect({ first, added, renamed, best, bestAfter }).toEqual({
        first: ["Order form"],
        added: ["Order form", "Reorder the order book"],
        renamed: ["Reorder the order book"],
        best: ["Order", "Border order form", "Reorder the order book"],
        bestAfter: ["Order", "Order form", "Border order form"],
    });
});

test("search a client's rich text by what a person reads", async () => {
    // open a client whose server never answers, with a page of a heading, a split word and a mention
    const environment = testSimulation().environment("view");
    const storage = await TestDatabase.create(ObjectClient.tables({ page }), { environment });
    const client = await ObjectClient.open({
        environment,
        database: storage.database,
        objects: { page },
        scope: schema.identifier("space").parse("space-01996ab0-0000-7000-8000-0000000000d3"),
        caller: principal.user.reference(
            Scope.universe.id,
            "user-01996ab0-0000-7000-8000-000000000003",
        ),
        remote: ObjectRemote.unanswered(),
    });
    onTestFinished(async () => {
        await client.close();
        await storage.close();
    });
    const { id } = await client.mutate(page).create({ title: "Plans" }).predicted;
    await client.call(page, "edit", {
        id,
        field: "body",
        edits: [
            { insert: { type: "heading", value: { level: 1 } }, run: "ada.1", start: 0 },
            { insert: "Harb", run: "ada.1", start: 1, after: { run: "ada.1", offset: 0 } },
            { insert: "our", run: "bob.1", start: 0, after: { run: "ada.1", offset: 4 } },
            {
                insert: { type: "paragraph", value: null },
                run: "bob.1",
                start: 3,
                after: { run: "bob.1", offset: 2 },
            },
            {
                insert: "The margins 東京都庁",
                run: "bob.1",
                start: 4,
                after: { run: "bob.1", offset: 3 },
            },
        ],
    }).predicted;
    const found = async (text: string) => {
        const live = client.search(text, { types: [page], limit: 5 }).subscribe();
        const hits = (await live.read()).map(({ row, excerpt }) => [row["title"], excerpt]);
        await live.close();

        return hits;
    };

    expect({
        heading: await found("harbour"),
        acrossBlocks: await found("harbour margins"),
        withTitle: await found("plans margins"),
        joined: await found("harbourthe"),
        han: await found("都庁"),
        title: await found("plans"),
        none: await found("zzz"),
    }).toEqual({
        heading: [["Plans", "Harbour"]],
        acrossBlocks: [["Plans", "Harbour"]],
        withTitle: [["Plans", "The margins 東京都庁"]],
        joined: [],
        han: [["Plans", "The margins 東京都庁"]],
        title: [["Plans", undefined]],
        none: [],
    });
});
