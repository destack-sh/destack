import type { CallableName } from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { none, principal, relation } from "@destack/access";

import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema, Clock } from "@destack/schema";

import { RequestId } from "@destack/runtime/request";
import { defineObject, field } from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";
import { userContext, users, userName } from "./fixture/user.ts";
import { reportError } from "@destack/runtime/server";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents, serversOf } from "../src/test/index.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { createRouterClient } from "@destack/runtime/server";

/** The space containing the shelves. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000011");

/** Books their owners shelve. */
const book = defineObject({
    name: "book",
    plural: "books",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        shelf: field.string(schema.string().min(1)),
        isTidy: field.boolean().default(false),
    },
    permissions: { read: relation("owner"), write: relation("owner") },
}).methods((method) => ({
    create: method.create("write", { fields: ["shelf"] }),
    update: method.update("write", { fields: ["isTidy"] }),
    tidy: method.updateMany("write", { fields: ["isTidy"], match: ["shelf"] }),
}));

/** Loan slips the space writes for a borrower, whom only the space sets. */
const slip = defineObject({
    name: "slip",
    plural: "slips",
    scope: space,
    fields: {
        borrower: field.reference(principal.user).caller(),
        shelf: field.string(),
        isReturned: field.boolean().default(false),
    },
    permissions: { read: relation("borrower"), issue: none() },
}).methods((method) => ({
    issue: method.create("issue", { isInternal: true }),
    close: method.update("issue", { isInternal: true }),
}));

/** Shelves their owners tidy, tidying their books with them. */
const shelf = defineObject({
    name: "shelf",
    plural: "shelves",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), name: field.string() },
    permissions: { read: relation("owner"), write: relation("owner") },
})
    .methods((method) => ({
        create: method.create("write", { fields: ["name"] }),
        tidy: method.mutation({
            permission: "write",
            output: schema.object({ count: schema.number() }),
        }),
        straighten: method.mutation({
            permission: "write",
            input: schema.object({ book: schema.string() }),
        }),
        lend: method.mutation({
            permission: "write",
            input: schema.object({ borrower: schema.identifier("user") }),
            output: schema.object({ slip: schema.string() }),
        }),
        misfile: method.mutation({ permission: "write" }),
        forge: method.mutation({
            permission: "write",
            input: schema.object({ borrower: schema.identifier("user") }),
        }),
    }))
    .implement({
        methods: {
            // tidy the shelf's books as the caller
            tidy: async (call) =>
                call
                    .mutate(call.caller, [
                        book.call("tidy", { where: { shelf: call.target.name }, isTidy: true }),
                    ])
                    .then(([tidied]) => tidied),
            // tidy one owned book as the caller
            straighten: async (call) => {
                await call.mutate(call.caller, [
                    book.call("update", { id: call.input.book, isTidy: true }),
                ]);

                return call.target;
            },
            // issue a slip to the borrower as the space and close it
            lend: async (call) => {
                const [slipped] = await call.mutate(call.self, [
                    slip.call("issue", {
                        borrowerId: call.input.borrower,
                        shelf: call.target.name,
                    }),
                ]);
                const issued = schema.looseObject({ id: schema.string() }).parse(slipped);
                await call.mutate(call.self, [
                    slip.call("close", { id: issued.id, isReturned: true }),
                ]);

                return { slip: issued.id };
            },
            // issue a slip as the caller, who has no permission to
            forge: async (call) => {
                await call.mutate(call.caller, [
                    slip.call("issue", {
                        borrowerId: call.input.borrower,
                        shelf: call.target.name,
                    }),
                ]);

                return call.target;
            },
            // shelve a book on an unnamed shelf, which the book's schema refuses
            misfile: async (call) => {
                await call.mutate(call.caller, [book.call("create", { shelf: "" })]);

                return call.target;
            },
        },
    });

/** The database with the shelves, the books, their access and the journal. */
const shelfDatabase = defineDatabase({
    name: "main",
    tables: [...callTables, ...book.tables, ...shelf.tables, ...slip.tables],
});

/** Read a call's refusal as the server reports it to its caller. */
function reported(pending: Promise<unknown>): Promise<"done" | readonly [string, string]> {
    return refusal(
        pending.catch((error: unknown) => {
            throw reportError(error);
        }),
    );
}

test.each(TEST_DIALECTS)(
    "invoke another object's method in a call, as its caller, audited and refused alike, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, shelfDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([book, shelf, slip]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: {
                package: book.package,
            },
        });
        let context = userContext(users.frank, spaceId);
        const call = <Object extends typeof book | typeof shelf, Name extends CallableName<Object>>(
            object: Object,
            name: Name,
            input: NoInfer<RequestInput<Object, Name>>,
        ) =>
            request(
                store,
                object,
                name,
                spaceId,
                { requestId: RequestId.create(Date.now()), ...input },
                context,
            );

        // shelve two books of user-1 and one of user-2 on the same shelf name
        const home = await call(shelf, "create", { name: "home" });
        await call(book, "create", { shelf: "home" });
        await call(book, "create", { shelf: "home" });
        context = userContext(users.grace, spaceId);
        const foreign = await call(book, "create", { shelf: "home" });

        // tidy user-1's writable books on the shelf, auditing both calls
        context = userContext(users.frank, spaceId);
        const before = (await auditedActions(storage.database, "caller")).length;
        const tidied = await call(shelf, "tidy", { id: home.id });
        const auditedTidy = (await auditedActions(storage.database, "caller")).slice(before);

        // refuse straightening user-2's book through user-1's shelf, rolling back the whole call
        await expect(
            call(shelf, "straighten", { id: home.id, book: foreign.id }),
        ).rejects.toMatchObject({
            code: "NOT_FOUND",
        });
        const rows = await storage.database
            .select({ owner: book.table.ownerId, isTidy: book.table.isTidy })
            .from(book.table);
        const summary = rows
            .map((row) => `${userName(row.owner)} ${String(row.isTidy)}`)
            .toSorted();
        expect([tidied, auditedTidy, summary]).toEqual([
            { count: 2 },
            ["book.tidy", "shelf.tidy"],
            ["frank true", "frank true", "grace false"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "invoke methods as the space in a caller's call, writing objects the caller may not, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, shelfDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([
                book,
                shelf,
                // mark issued slips on the server when the base type is invoked
                slip.implement({
                    methods: {
                        issue: (call, next) =>
                            next(
                                call.with({
                                    input: { ...call.input, shelf: `${call.input["shelf"]}!` },
                                }),
                            ),
                    },
                }),
            ]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: {
                package: book.package,
            },
        });
        const context = userContext(users.frank, spaceId);
        const run = <Object extends typeof shelf | typeof slip, Name extends CallableName<Object>>(
            object: Object,
            name: Name,
            input: NoInfer<RequestInput<Object, Name>>,
        ) =>
            request(
                store,
                object,
                name,
                spaceId,
                { requestId: RequestId.create(Date.now()), ...input },
                context,
            );

        // lend a book to user-2, issuing and closing a slip user-1 may not write, and refuse user-1 issuing one
        const home = await run(shelf, "create", { name: "home" });
        const lent = await run(shelf, "lend", { id: home.id, borrower: users.grace });
        const forged = await refusal(run(shelf, "forge", { id: home.id, borrower: users.grace }));
        const rows = await storage.database
            .select({
                id: slip.table.id,
                borrower: slip.table.borrowerId,
                shelf: slip.table.shelf,
                isReturned: slip.table.isReturned,
            })
            .from(slip.table);

        // refuse a client pushing a method left out of its routes
        const mutationId = RequestId.create(Date.now());
        const pushed = createRouterClient(store.router(), { context: context }).replica.push({
            scope: spaceId,
            mutations: [
                {
                    id: mutationId,
                    calls: [
                        {
                            type: slip.typeReference,
                            method: "issue",
                            release: slip.package.version,
                            input: { borrower: users.frank, shelf: "home" },
                        },
                    ],
                },
            ],
        });
        // hide the space's methods from the routes and from pushes, routing only the slips' reads
        expect([rows, forged, Object.keys(slip.procedures), (await pushed).outcomes]).toEqual([
            [{ id: lent.slip, borrower: users.grace, shelf: "home!", isReturned: true }],
            ["NOT_FOUND", "slip not found"],
            ["get", "list"],
            [
                {
                    id: mutationId,
                    outcome: {
                        error: {
                            code: "BAD_REQUEST",
                            status: 400,
                            message: "no mutating method slip.issue",
                        },
                    },
                },
            ],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "refuse input a method's schema rejects, read directly or invoked in a call, as its procedure refuses it, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, shelfDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: serversOf([book, shelf, slip]),
            database: storage.database,
            callKey: testCallKey,
            events: await testEvents(storage.database),
            origin: { package: book.package },
        });
        const context = userContext(users.frank, spaceId);
        const home = await store
            .mutate(context.access(), {
                id: RequestId.create(Date.now()),
                calls: [shelf.call("create", { spaceId, name: "home" })],
            })
            .then(([written]) => written);

        // refuse a page of no books, and an invoked creation on an unnamed shelf, as callers see it
        expect([
            await reported(store.query(context.access(), book.call("list", { spaceId, limit: 0 }))),
            await reported(
                store
                    .mutate(context.access(), {
                        id: RequestId.create(Date.now()),
                        calls: [shelf.call("misfile", { spaceId, id: home.id })],
                    })
                    .then(([written]) => written),
            ),
        ]).toEqual([
            ["BAD_REQUEST", "invalid input: limit: too small: expected number to be >=1"],
            [
                "BAD_REQUEST",
                "invalid input: shelf: too small: expected string to have >=1 characters",
            ],
        ]);
    },
);
