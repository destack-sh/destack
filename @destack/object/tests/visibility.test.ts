import { Scope } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import {
    accessRelationship,
    anyone,
    none,
    principal,
    Relationship,
    relation,
    type Delegate,
} from "@destack/access";
import { journal } from "@destack/audit";
import { copyScope } from "@destack/access/test";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier, schema } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";

import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { v7 } from "uuid";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";

/** A private space nobody reads. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

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
    methods: { list: method.list("read") },
});

/** Documents shared one at a time with their readers. */
const document = defineObject({
    name: "document",
    plural: "documents",
    scope: space,
    fields: { title: field.string(schema.string().min(1)) },
    relations: { reader: { subjects: [principal.user] } },
    permissions: { read: relation("reader"), write: none() },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write"),
    },
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
        const [shared, other] = [v7(), v7()].map((id) =>
            identifier("document").parse(`document-${id}`),
        );
        await database.insert(document.table).values(
            [
                { id: shared, title: "Plan" },
                { id: other, title: "Budget" },
            ].map((row) => ({ ...row, scope: spaceId, createdAt: now, updatedAt: now })),
        );
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: identifier("relationship").parse(`relationship-${v7()}`),
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
        let caller = { user: "alice", delegates: [] as Delegate[] };
        const server = new ObjectServer({
            objects: { document },
            database,
            context: () => ({
                subject: principal.user.reference("universe", caller.user),
                subjects: [principal.user.reference("universe", caller.user)],
                delegates: caller.delegates,
                now: Date.now(),
                attributes: {},
            }),
            callKey: testCallKey,
            origin: {
                package: document.package,
                service: "test",
            },
        });
        const context = {
            scope: spaceId,
            requireAuthentication: () => ({ id: caller.user }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const outcome = (called: Promise<unknown>) =>
            called.then(
                (result) => result,
                (error: { code: string }) => error.code,
            );
        const attempts = () =>
            Promise.all([
                outcome(server.query(document, "get", { spaceId, id: shared }, context)),
                outcome(
                    server
                        .query(document, "list", { spaceId }, context)
                        .then((page) =>
                            (page as { items: { title: string }[] }).items.map(
                                (item) => item.title,
                            ),
                        ),
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
        expect(read).toEqual([
            expect.objectContaining({ id: shared, title: "Plan" }),
            ["Plan"],
            "NOT_FOUND",
            "FORBIDDEN",
        ]);

        // find nothing at all for a stranger
        caller = { user: "bob", delegates: [] };
        expect(await attempts()).toEqual(["NOT_FOUND", "NOT_FOUND", "NOT_FOUND", "NOT_FOUND"]);

        // show a delegate lent nothing what alice sees, asking her for the grant it lacks
        caller = {
            user: "alice",
            delegates: [
                {
                    subject: principal.installation.reference(spaceId, `installation-${v7()}`),
                    authority: "lent",
                },
            ],
        };
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

        // hold a room anyone reads
        const roomId = identifier("room").parse(`room-${v7()}`);
        const object = room.reference(Scope.universe.id, roomId);
        await copyScope(database, object);
        await database.insert(accessRelationship).values(
            Relationship.encode(
                {
                    id: identifier("relationship").parse(`relationship-${v7()}`),
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
            context: () => ({
                subjects: [principal.user.reference("universe", "alice")],
                now: Date.now(),
                attributes: {},
            }),
            callKey: testCallKey,
            origin: {
                package: document.package,
                service: "test",
            },
        });
        const context = {
            requireAuthentication: () => ({ id: "alice" }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;

        // list the room's desks and find no documents there
        const desks = (await server.query(desk, "list", { roomId }, context)) as {
            items: unknown[];
        };
        await expect(
            server.query(document, "list", { spaceId: roomId }, context),
        ).rejects.toMatchObject({ code: "NOT_FOUND" });
        expect(desks.items).toEqual([]);
    },
);
