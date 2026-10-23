import { defineObject, field, ObjectDescription } from "../src/index.ts";
import { users } from "./fixture/user.ts";
import { Package } from "@destack/package";
import { defineDatabase, type DatabaseConnection } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { none, principal, through } from "@destack/access";
import { schema, type Identifier } from "@destack/schema";
import { RequestId } from "@destack/sync";
import { subjectContext, testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { v7 } from "uuid";
import { ObjectStore, implementPackage } from "../src/server/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import { definePackage } from "@destack/object";
import { notebook, notesPackage } from "./fixture/note.ts";
import objectPackage from "../src/package.ts";
import { openSpace, space, unmoved, spacePrincipal } from "./fixture/space.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** An entry's natural key: the notebook's term, as a moniker identifies a symbol. */
const Term = schema.string().regex(/^term:[A-Za-z]+$/u);

/** An entry of a notebook's index, identified by its term and stored in files. */
const entry = defineObject({
    name: "entry",
    plural: "entries",
    scope: space,
    storage: "external",
    key: Term,
    nested: { in: notebook, receive: "read" },
    fields: { term: field.string() },
    permissions: { read: through("parent", "read") },
});

/** A lookup of an entry's term, which a mutation reads from the entries' files. */
const lookup = defineObject(
    {
        name: "lookup",
        plural: "lookups",
        scope: space,
        fields: {
            /** The term looked up. */
            term: field.string(),
        },
        permissions: { read: through("scope", "read"), write: through("scope", "read") },
    },
    (method) => ({
        create: method.create("write"),
        /** Read an entry's term from inside a mutation. */
        look: method.mutation({
            permission: "write",
            target: "scope",
            input: schema.object({ term: Term }),
            output: schema.object({ term: schema.string() }),
        }),
    }),
).implement({
    methods: {
        look: async (call) => {
            const spaceId = schema.identifier("space").parse(call.scope);
            const read = await call.query(
                call.caller,
                entry.call("get", { spaceId, id: call.input.term }),
            );

            return { term: read.term };
        },
    },
});

test("read external objects named by their key from the database their files open per scope, allowing callers by the durable database", async () => {
    // store a public space's notebook in the durable database
    const durable = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, notesPackage, { objects: { lookup }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => durable.close());
    const spaceId = schema.identifier("space").parse(`space-${v7()}`);
    await openSpace(durable.database, spaceId);
    const context = (user: Identifier<"user">) =>
        subjectContext(principal.user.reference("universe", user), spaceId);

    // open each scope's entries from files, here one database built per scope
    const files = await TestDatabase.create(
        defineDatabase({ name: "files", tables: entry.tables }),
        { isMigrated: true },
    );
    onTestFinished(() => files.close());
    const opened: string[] = [];
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([notebook, entry, lookup], {
            external: {
                objects: [entry],
                open: async (scope): Promise<DatabaseConnection> => {
                    opened.push(scope);

                    return files.database;
                },
            },
        }),
        database: durable.database,
        callKey: testCallKey,
        events: await testEvents(durable.database),
        origin: { package: entry.package },
    });
    const travel = await store
        .mutate(context(users.alice).access(), {
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
    const read = await store.query(
        context(users.alice).access(),
        entry.call("get", { spaceId, id }),
    );
    const listed = await store.query(
        context(users.alice).access(),
        entry.call("list", { spaceId }),
    );
    // read it from inside a mutation, outside its transaction as its rows never change
    const [looked] = await store.mutate(context(users.alice).access(), {
        id: RequestId.create(Date.now()),
        calls: [lookup.call("look", { spaceId, term: id })],
    });
    expect([read.term, listed.items.map((row) => row.term), looked, opened]).toEqual([
        "Vienna",
        ["Vienna"],
        { term: "Vienna" },
        [spaceId, spaceId, spaceId],
    ]);

    // follow the entries in a client's live query, through the external replica
    const notebooks = definePackage({ objects: { notebook, entry } });
    const served = TestObjectStore.serve(
        new ObjectStore({
            servers: [
                implementPackage(notebooks, {
                    external: { objects: [entry], open: async () => files.database },
                }),
            ],
            chain: async () => undefined,
            clock: Clock.system,
            origin: { package: Package.of(notebooks) },
            principal: spacePrincipal(spaceId),
            database: durable.database,
            callKey: testCallKey,
            events: await testEvents(durable.database),
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());
    const local = await TestDatabase.create(ObjectClient.tables({ notebook, entry }), {
        dialect: "sqlite",
    });
    onTestFinished(() => local.close());
    const client = await ObjectClient.open({
        clock: Clock.system,
        database: local.database,
        objects: { notebook, entry },
        scope: spaceId,
        caller: principal.user.reference("universe", users.alice),
        endpoint: served.as(users.alice).endpoint(notebooks),
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
    return defineObject(
        {
            name: "mark",
            plural: "marks",
            scope: space,
            storage: "external",
            fields: { term: field.string() },
            permissions: { receive: none() },
        },
        (method) => ({
            receive: method.mutation({ permission: "receive", target: "scope", isInternal }),
        }),
    );
}

test("refuse public writes on external objects, taking internal writes their store code runs", () => {
    // declare an external type with an internal write, and one with a public write
    expect(declareMark(true).storage).toBe("external");
    expect(() => ObjectDescription.of(declareMark(false))).toThrow(
        new TypeError("external object mark cannot write in method receive"),
    );
});
