import { Scope, RequestId } from "@destack/sync";
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
import { TestDatabase } from "@destack/db/test";
import { Identifier, schema } from "@destack/schema";

import { v7 } from "uuid";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { Clock } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

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
});

/** Documents shared one at a time with their readers. */
const document = defineObject(
    {
        name: "document",
        plural: "documents",
        scope: space,
        fields: { title: field.string(schema.string().min(1)) },
        relations: { reader: { subjects: [principal.user] } },
        permissions: { read: relation("reader"), write: none() },
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write"),
    }),
);

test("show a private space to readers of one object in it, forbid their writes, and hide it from strangers", async () => {
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        {
            isMigrated: true,
        },
    );
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
        Relationship.encode({
            id: schema.identifier("relationship").parse(`relationship-${v7()}`),
            object: document.reference(spaceId, shared),
            relation: "reader",
            subject: principal.user.reference("universe", users.alice),
            createdAt: now,
            expiresAt: null,
        }),
    );

    // serve the documents to the user each call refers to, through the delegates it lists
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: {
            package: document.package,
        },
    });
    let context = userContext(users.alice, spaceId);
    const attempts = () =>
        Promise.all([
            outcome(
                store
                    .query(context.access(), document.call("get", { spaceId, id: shared }))
                    .then((row) => ({ id: row.id, title: row.title })),
            ),
            outcome(
                store
                    .query(context.access(), document.call("list", { spaceId }))
                    .then((page) => page.items.map((item) => item.title)),
            ),
            outcome(store.query(context.access(), document.call("get", { spaceId, id: other }))),
            outcome(
                store
                    .mutate(context.access(), {
                        id: RequestId.create(Date.now()),
                        calls: [document.call("update", { spaceId, id: shared, title: "Draft" })],
                    })
                    .then(([written]) => written),
            ),
        ]);

    // let alice read and list her document, find nothing else, and forbid her writes to it
    const read = await attempts();
    expect(read).toEqual([{ id: shared, title: "Plan" }, ["Plan"], "NOT_FOUND", "FORBIDDEN"]);

    // find nothing at all for a stranger: no document, and no scope to list or write in
    context = userContext(users.bob, spaceId);
    expect(await attempts()).toEqual(["NOT_FOUND", "NOT_FOUND", "NOT_FOUND", "NOT_FOUND"]);

    // answer a stranger's listing of the space exactly as a listing of a space nobody created
    const missing = `space-${v7()}`;
    const list = (id: string) =>
        refusal(store.query(context.access(), document.call("list", { spaceId: id })));
    expect([await list(spaceId), await list(missing)]).toEqual([
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${spaceId} not found`],
        ["NOT_FOUND", "SCOPE_NOT_FOUND", `scope ${missing} not found`],
    ]);

    // show a delegate granted no reading what alice sees, asking her for the grant it lacks
    const delegate = principal.installation.reference(spaceId, `installation-${v7()}`);
    const grantId = await new AccessFixture(storage.database).replicateGrant(
        principal.user.reference("universe", users.alice),
        delegate,
        [{ ...document.permission("write"), scope: spaceId }],
    );
    context = userContext(users.alice, spaceId, {
        actors: [delegate],
        grants: [{ id: grantId, scope: users.alice }],
    });
    expect((await attempts()).slice(0, 3)).toEqual(["FORBIDDEN", [], "NOT_FOUND"]);
});

test("find nothing when a call asks for a scope its objects do not live in, though the caller sees it", async () => {
    const storage = await TestDatabase.create(
        tablesOf({
            packages: [objectPackage, { objects: { document, desk }, tables: [] }],
            follows: [],
            tables: [],
        }).tables,
        {
            isMigrated: true,
        },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;

    // insert a room anyone reads
    const roomId = schema.identifier("room").parse(`room-${v7()}`);
    const object = room.reference(Scope.universe.id, roomId);
    await new AccessFixture(database).replicateScope(object, room.policy);
    await database.insert(accessRelationship).values(
        Relationship.encode({
            id: schema.identifier("relationship").parse(`relationship-${v7()}`),
            object,
            relation: "reader",
            subject: anyone.reference("*", "*"),
            createdAt: 0,
            expiresAt: null,
        }),
    );

    // serve documents and desks to alice, unpinned to any scope
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        servers: serversOf([document, desk]),
        database,
        callKey: testCallKey,
        events: await testEvents(database),
        origin: {
            package: document.package,
        },
    });
    const context = userContext(users.alice, undefined);

    // list the room's desks and find no documents there
    const desks = await store.query(context.access(), desk.call("list", { roomId }));
    expect(
        await refusal(store.query(context.access(), document.call("list", { spaceId: roomId }))),
    ).toEqual(["NOT_FOUND", "TYPE_NOT_IN_SCOPE", `no documents in scope ${roomId}`]);
    expect(desks.items).toEqual([]);
});

/** Settle a call as its result, or as the code of its failure. */
async function outcome(called: Promise<unknown>): Promise<unknown> {
    const refused = await refusal(called);

    return refused === "done" ? await called : refused[0];
}
