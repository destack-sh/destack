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
import { journal } from "@destack/audit/stack";
import { AccessFixture } from "@destack/access/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { Identifier, schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";
import { testCallKey } from "@destack/service/test";

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
});

/** Desks in rooms. */
const desk = defineObject({
    name: "desk",
    plural: "desks",
    scope: room,
    fields: {},
    permissions: { read: none() },
    methods: (method) => ({ list: method.list("read") }),
});

/** Documents shared one at a time with their readers. */
const document = defineObject({
    name: "document",
    plural: "documents",
    scope: space,
    fields: { title: field.string(schema.string().min(1)) },
    relations: { reader: { subjects: [principal.user] } },
    permissions: { read: relation("reader"), write: none() },
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write"),
    }),
});

test.each(TEST_DIALECTS)(
    "show a private space to readers of one object in it, forbid their writes, and hide it from strangers on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, [...document.tables, journal], {
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

        // serve the documents to the user each call names, through the delegates it names
        const server = new ObjectServer({
            objects: { document },
            database,
            callKey: testCallKey,
            origin: {
                package: document.package,
                service: "test",
            },
        });
        let context = userContext("alice", spaceId);
        const attempts = () =>
            Promise.all([
                outcome(
                    server
                        .call(document, "get", { spaceId, id: shared }, context)
                        .then((row) => ({ id: row.id, title: row.title })),
                ),
                outcome(
                    server
                        .call(document, "list", { spaceId }, context)
                        .then((page) => page.items.map((item) => item.title)),
                ),
                outcome(server.query(document, "get", { spaceId, id: other }, context)),
                outcome(
                    server.call(
                        document,
                        "update",
                        { spaceId, id: shared, requestId: RequestId.create(), title: "Draft" },
                        context,
                    ),
                ),
            ]);

        // let alice read and list her document, find nothing else, and forbid her writes to it
        const read = await attempts();
        expect(read).toEqual([{ id: shared, title: "Plan" }, ["Plan"], "NOT_FOUND", "FORBIDDEN"]);

        // find nothing at all for a stranger
        context = userContext("bob", spaceId);
        expect(await attempts()).toEqual(["NOT_FOUND", "NOT_FOUND", "NOT_FOUND", "NOT_FOUND"]);

        // show a delegate lent nothing what alice sees, asking her for the grant it lacks
        context = userContext("alice", spaceId, {
            delegates: [
                {
                    subject: principal.installation.reference(spaceId, `installation-${v7()}`),
                    authority: "lent",
                },
            ],
        });
        expect((await attempts()).slice(0, 3)).toEqual(["INSUFFICIENT_GRANT", [], "NOT_FOUND"]);
    },
);

test.each(TEST_DIALECTS)(
    "find nothing when a call names a scope its objects do not live in, though the caller sees it, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(
            dialect,
            [...document.tables, ...desk.tables, journal],
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
        const server = new ObjectServer({
            objects: { document, desk },
            database,
            callKey: testCallKey,
            origin: {
                package: document.package,
                service: "test",
            },
        });
        const context = userContext("alice", undefined);

        // list the room's desks and find no documents there
        const desks = await server.call(desk, "list", { roomId }, context);
        await expect(
            server.query(document, "list", { spaceId: roomId }, context),
        ).rejects.toMatchObject({ code: "NOT_FOUND" });
        expect(desks.items).toEqual([]);
    },
);

/** Settle a call as its result, or as the code of its failure. */
async function outcome(called: Promise<unknown>): Promise<unknown> {
    const refused = await refusal(called);

    return refused === "done" ? await called : refused[0];
}
