import { Package } from "@destack/package";
import { defineDatabase, type DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { none, principal, through } from "@destack/access";
import { schema, Clock } from "@destack/schema";
import { RequestId } from "@destack/runtime/request";
import { subjectContext, testCallKey } from "@destack/runtime/test";
import { testEvents } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectExtension, ObjectStore } from "../src/server/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import { definePackage } from "@destack/runtime";
import { serveObjects } from "./fixture/device.ts";
import { notebook, notesDatabase } from "./fixture/note.ts";
import { openSpace, space, unmoved, spacePrincipal } from "./fixture/space.ts";

/** An entry's natural key: the notebook's term, as a moniker identifies a symbol. */
const Term = schema.string().regex(/^term:[A-Za-z]+$/u);

/** An entry of a notebook's index, identified by its term and kept in files. */
const entry = defineObject({
    name: "entry",
    plural: "entries",
    scope: space,
    storage: "external",
    key: Term,
    nested: { in: notebook, receive: "read" },
    fields: { term: field.string() },
    permissions: { read: through("parent", "read") },
}).methods();

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
    const store = new ObjectStore({
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [notebook, entry],
        database: durable.database,
        external: [
            {
                objects: [entry],
                open: async (scope): Promise<DatabaseConnection> => {
                    opened.push(scope);

                    return files.database;
                },
            },
        ],
        callKey: testCallKey,
        events: await testEvents(durable.database),
        origin: { package: entry.package },
    });
    const travel = await store
        .mutate(context("alice").caller(), {
            id: RequestId.create(Date.now()),
            calls: [notebook.call("create", { spaceId, name: "Travel" })],
        })
        .then(([written]) => written);
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
    const read = await store.query(context("alice").caller(), entry.call("get", { spaceId, id }));
    const listed = await store.query(context("alice").caller(), entry.call("list", { spaceId }));
    expect([read.term, listed.items.map((row) => row.term), opened]).toEqual([
        "Vienna",
        ["Vienna"],
        [spaceId, spaceId],
    ]);

    // follow the entries in a client's live query, through the external copy
    const notebooks = definePackage({ objects: { notebook, entry } });
    const served = serveObjects(
        ObjectExtension.compose(
            [
                {
                    ...ObjectExtension.of(notebooks, []),
                    external: { objects: [entry], open: async () => files.database },
                },
            ],
            {
                clock: Clock.system,
                origin: { package: Package.of(notebooks) },
                principal: spacePrincipal(spaceId),
                database: durable.database,
                callKey: testCallKey,
                events: await testEvents(durable.database),
            },
        ).packages[0],
        spaceId,
    );
    const local = await TestDatabase.create("sqlite", ObjectClient.tables({ notebook, entry }));
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        clock: Clock.system,
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

/** Declare an external type whose one write is internal or public. */
function declareMark(isInternal: boolean) {
    return defineObject({
        name: "mark",
        plural: "marks",
        scope: space,
        storage: "external",
        fields: { term: field.string() },
        permissions: { receive: none() },
    }).methods((method) => ({
        receive: method.mutation({ permission: "receive", target: "none", isInternal }),
    }));
}

test("refuse public writes on external objects, taking internal writes their store code runs", () => {
    // declare an external type with an internal write, and one with a public write
    expect(declareMark(true).storage).toBe("external");
    expect(() => declareMark(false)).toThrow(
        new TypeError("external object mark writes in method receive"),
    );
});
