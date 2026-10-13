import { Scope } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import {
    accessRelationship,
    anyone,
    none,
    principal,
    Relationship,
    relation,
} from "@destack/access";
import { AccessFixture } from "@destack/access/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Identifier, schema, Clock } from "@destack/schema";

import { RequestId } from "@destack/runtime/request";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";

/** A private space nobody reads. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** Rooms, a public scope type besides spaces. */
const room = defineObject({
    name: "room",
    plural: "rooms",
    scope: "universe",
    isScope: true,
    fields: {},
    relations: { reader: { subjects: [anyone.all()] } },
    permissions: { read: relation("reader") },
}).methods();

/** Desks in rooms. */
const desk = defineObject({
    name: "desk",
    plural: "desks",
    scope: room,
    fields: {},
    permissions: { read: none() },
}).methods();

/** Documents shared one at a time with their readers. */
const document = defineObject({
    name: "document",
    plural: "documents",
    scope: space,
    fields: { title: field.string(schema.string().min(1)) },
    relations: { reader: { subjects: [principal.user] } },
    permissions: { read: relation("reader"), write: none() },
}).methods((method) => ({
    create: method.create("write"),
    update: method.update("write"),
}));

test.each(TEST_DIALECTS)(
    "show a private space to readers of one object in it, forbid their writes, and hide it from strangers on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [...document.tables, ...callTables], {
            isMigrated: true,
        });
        onTestFinished(() => storage.close());
        const database = storage.database;
        await openSpace(database, spaceId, "private");

        // share one of two documents with alice
        const now = Date.now();
        const shared = Identifier.create("document");
        const other = Identifier.create("document");
        await database.insert(document.table).values(
            [
                { id: shared, title: "Plan" },
                { id: other, title: "Budget" },
            ].map((row) => ({ ...row, scope: spaceId, createdAt: now, updatedAt: now })),
        );
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                    object: document.reference(spaceId, shared),
                    relation: "reader",
                    subject: principal.user.reference("universe", "alice"),
                    createdAt: now,
                    expiresAt: null,
                },
                spaceId,
            ),
        );

        // serve the documents to the user each call refers to, through the delegates it lists
        const store = new ObjectStore({
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: [document],
            database,
            callKey: testCallKey,
            events: await testEvents(database),
            origin: {
                package: document.package,
            },
        });
        let context = userContext("alice", spaceId);
        const attempts = () =>
            Promise.all([
                outcome(
                    store
                        .query(context.caller(), document.call("get", { spaceId, id: shared }))
                        .then((row) => ({ id: row.id, title: row.title })),
                ),
                outcome(
                    store
                        .query(context.caller(), document.call("list", { spaceId }))
                        .then((page) => page.items.map((item) => item.title)),
                ),
                outcome(
                    store.query(context.caller(), document.call("get", { spaceId, id: other })),
                ),
                outcome(
                    store
                        .mutate(context.caller(), {
                            id: RequestId.create(Date.now()),
                            calls: [
                                document.call("update", { spaceId, id: shared, title: "Draft" }),
                            ],
                        })
                        .then(([written]) => written),
                ),
            ]);

        // let alice read and list her document, find nothing else, and forbid her writes to it
        const read = await attempts();
        expect(read).toEqual([{ id: shared, title: "Plan" }, ["Plan"], "NOT_FOUND", "FORBIDDEN"]);

        // find nothing at all for a stranger
        context = userContext("bob", spaceId);
        expect(await attempts()).toEqual(["NOT_FOUND", "NOT_FOUND", "NOT_FOUND", "NOT_FOUND"]);

        // show a delegate granted no reading what alice sees, asking her for the grant it lacks
        context = userContext("alice", spaceId, {
            delegates: [principal.installation.reference(spaceId, `installation-${v7()}`)],
            grant: {
                id: Identifier.create("grant"),
                scope: "alice",
                permissions: [{ ...document.permission("write"), scope: spaceId }],
            },
        });
        expect((await attempts()).slice(0, 3)).toEqual(["INSUFFICIENT_GRANT", [], "NOT_FOUND"]);
    },
);

test.each(TEST_DIALECTS)(
    "find nothing when a call names a scope its objects do not live in, though the caller sees it, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            [...document.tables, ...desk.tables, ...callTables],
            { isMigrated: true },
        );
        onTestFinished(() => storage.close());
        const database = storage.database;

        // insert a room anyone reads
        const roomId = schema.identifier("room").parse(`room-${v7()}`);
        const object = room.reference(Scope.universe.id, roomId);
        await new AccessFixture(database).copyScope(object);
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: schema.identifier("relationship").parse(`relationship-${v7()}`),
                    object,
                    relation: "reader",
                    subject: anyone.reference("*", "*"),
                    createdAt: 0,
                    expiresAt: null,
                },
                roomId,
            ),
        );

        // serve documents and desks to alice, unpinned to any scope
        const store = new ObjectStore({
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            objects: [document, desk],
            database,
            callKey: testCallKey,
            events: await testEvents(database),
            origin: {
                package: document.package,
            },
        });
        const context = userContext("alice", undefined);

        // list the room's desks and find no documents there
        const desks = await store.query(context.caller(), desk.call("list", { roomId }));
        await expect(
            store.query(context.caller(), document.call("list", { spaceId: roomId })),
        ).rejects.toMatchObject({ code: "NOT_FOUND" });
        expect(desks.items).toEqual([]);
    },
);

/** Settle a call as its result, or as the code of its failure. */
async function outcome(called: Promise<unknown>): Promise<unknown> {
    const refused = await refusal(called);

    return refused === "done" ? await called : refused[0];
}
