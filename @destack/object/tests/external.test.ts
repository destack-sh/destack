import { defineDatabase, type DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { principal, through } from "@destack/access";
import { schema } from "@destack/schema";
import { RequestId } from "@destack/service/request";
import { subjectContext, testCallKey } from "@destack/service/test";
import { expect, onTestFinished, test } from "@destack/test";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import { defineService } from "@destack/service";
import { serveObjects } from "./fixture/device.ts";
import { notebook, notesDatabase } from "./fixture/note.ts";
import { openSpace, space, unmoved } from "./fixture/space.ts";

/** An entry's natural key: the notebook's term, as a moniker names a symbol. */
const Term = schema.string().regex(/^term:[A-Za-z]+$/u);

/** An entry of a notebook's index, named by its term and kept in files rather than the space's database. */
const entry = defineObject({
    name: "entry",
    plural: "entries",
    scope: space,
    storage: "external",
    key: Term,
    nested: { in: notebook, receive: "read" },
    fields: { term: field.string() },
    permissions: { read: through("parent", "read") },
    methods: (method) => ({ get: method.get("read"), list: method.list("read") }),
});

test("read external objects named by their key from the database their files open per scope, admitting callers by the durable database", async () => {
    // keep a public space's notebook in the durable database
    const durable = await TestDatabase.create("sqlite", notesDatabase, { isMigrated: true });
    onTestFinished(() => durable.close());
    const spaceId = schema.identifier("space").parse(`space-${v7()}`);
    await openSpace(durable.database, spaceId);
    const context = (user: string) =>
        subjectContext(principal.user.reference("universe", user), spaceId);

    // open each scope's entries from files, here one database built per scope
    const files = await TestDatabase.create(
        "sqlite",
        defineDatabase({ name: "files", tables: entry.tables }),
        { isMigrated: true },
    );
    onTestFinished(() => files.close());
    const opened: string[] = [];
    const server = new ObjectServer({
        objects: { notebook, entry },
        database: durable.database,
        external: {
            objects: [entry],
            open: async (scope): Promise<DatabaseConnection> => {
                opened.push(scope);

                return files.database;
            },
        },
        callKey: testCallKey,
        origin: { package: entry.package, service: "test" },
    });
    const travel = await server.call(
        notebook,
        "create",
        { spaceId, requestId: RequestId.create(), name: "Travel" },
        context("alice"),
    );
    const id = "term:Vienna";
    await files.database.insert(entry.table).values({
        id,
        scope: spaceId,
        parentId: travel.id,
        term: "Vienna",
        createdAt: 1,
        updatedAt: 1,
    });

    // read the entry through its notebook's read permission
    const read = await server.call(entry, "get", { spaceId, id }, context("alice"));
    const listed = await server.call(entry, "list", { spaceId }, context("alice"));
    expect([read.term, listed.items.map((row) => row.term), opened]).toEqual([
        "Vienna",
        ["Vienna"],
        [spaceId, spaceId],
    ]);

    // follow the entries in a client's live query, through the external copy
    const service = defineService("index", { objects: { notebook, entry } });
    const served = serveObjects(
        ObjectServer.serve(service, {
            database: durable.database,
            external: { objects: [entry], open: async () => files.database },
            callKey: testCallKey,
        }),
        spaceId,
    );
    const local = await TestDatabase.create("sqlite", ObjectClient.tables({ notebook, entry }));
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        database: local.database,
        objects: { notebook, entry },
        scope: spaceId,
        caller: principal.user.reference("universe", "alice"),
        endpoint: served.endpoint("alice"),
        reconnect: unmoved,
    });
    const stopping = new AbortController();
    const failures: unknown[] = [];
    const running = client.run(stopping.signal, (error) => failures.push(error));
    onTestFinished(async () => {
        stopping.abort();
        await running;
        await client.close();
    });
    const entries = client.query.entry.findMany().subscribe();
    await entries.ready;
    await expect
        .poll(async () => (await entries.read()).map((row) => row.term))
        .toEqual(["Vienna"]);
    expect(failures).toEqual([]);

    // refuse an identifier outside the key
    expect([entry.identifies(id), entry.identifies(`entry-${v7()}`)]).toEqual([true, false]);
});
