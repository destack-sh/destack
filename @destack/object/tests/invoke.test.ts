import type { CallableName } from "../src/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import { defineObject, field } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";
import { userContext } from "./fixture/user.ts";
import { reportError } from "@destack/service/server";
import { testCallKey } from "@destack/service/test";

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
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("write", { fields: ["shelf"] }),
        update: method.update("write", { fields: ["isTidy"] }),
        tidy: method.updateMany("write", { fields: ["isTidy"], match: ["shelf"] }),
    }),
});

/** Loan slips the system writes for a borrower, whom only the system names. */
const slip = defineObject({
    name: "slip",
    plural: "slips",
    scope: space,
    fields: {
        borrower: field.reference(principal.user).caller(),
        shelf: field.string(),
        isReturned: field.boolean().default(false),
    },
    permissions: { read: relation("borrower") },
    methods: (method) => ({
        issue: method.create(null, { isSystem: true }),
        close: method.update(null, { isSystem: true }),
    }),
});

/** Shelves their owners tidy, tidying their books with them. */
const shelf = defineObject({
    name: "shelf",
    plural: "shelves",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), name: field.string() },
    permissions: { read: relation("owner"), write: relation("owner") },
    methods: (method) => ({
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
            input: schema.object({ borrower: schema.string() }),
            output: schema.object({ slip: schema.string() }),
        }),
        misfile: method.mutation({ permission: "write" }),
    }),
}).handle({
    // tidy the shelf's books as the caller
    tidy: async (call) =>
        call.invoke(book).tidy({ where: { shelf: call.target.name }, isTidy: true }),
    // tidy one owned book as the caller
    straighten: async (call) => {
        await call.invoke(book).update({ id: call.input.book, isTidy: true });

        return call.target;
    },
    // issue a slip to the borrower as the system and close it
    lend: async (call) => {
        const issued = schema.looseObject({ id: schema.string() }).parse(
            await call.invoke(slip).issue({
                borrowerId: call.input.borrower,
                shelf: call.target.name,
            }),
        );
        await call.invoke(slip).close({ id: issued.id, isReturned: true });

        return { slip: issued.id };
    },
    // shelve a book on an unnamed shelf, which the book's schema refuses
    misfile: async (call) => {
        await call.invoke(book).create({ shelf: "" });

        return call.target;
    },
});

/** The database with the shelves, the books, their access and the journal. */
const shelfDatabase = defineDatabase({
    name: "main",
    tables: [journal, ...book.tables, ...shelf.tables, ...slip.tables],
});

/** Read a call's refusal as the service reports it to its caller. */
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
        const server = new ObjectServer({
            objects: { book, shelf, slip },
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: book.package,
                service: "test",
            },
        });
        let context = userContext("user-1", spaceId);
        const call = <Object extends typeof book | typeof shelf, Name extends CallableName<Object>>(
            object: Object,
            name: Name,
            input: object,
        ) =>
            server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            );

        // shelve two books of user-1 and one of user-2 on the same shelf name
        const home = await call(shelf, "create", { name: "home" });
        await call(book, "create", { shelf: "home" });
        await call(book, "create", { shelf: "home" });
        context = userContext("user-2", spaceId);
        const foreign = await call(book, "create", { shelf: "home" });

        // tidy user-1's writable books on the shelf, auditing both calls
        context = userContext("user-1", spaceId);
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
        const summary = rows.map((row) => `${row.owner} ${String(row.isTidy)}`).toSorted();
        expect([tidied, auditedTidy, summary]).toEqual([
            { count: 2 },
            ["book.tidy", "shelf.tidy"],
            ["user-1 true", "user-1 true", "user-2 false"],
        ]);
    },
);

test.each(TEST_DIALECTS)(
    "invoke system methods in a caller's call, writing objects the caller may not, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, shelfDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        const server = new ObjectServer({
            objects: {
                book,
                shelf,
                // mark issued slips on the server when the base type is invoked
                slip: slip.handle({
                    issue: (call, next) =>
                        next(
                            call.with({
                                input: { ...call.input, shelf: `${call.input["shelf"]}!` },
                            }),
                        ),
                }),
            },
            database: storage.database,
            callKey: testCallKey,
            origin: {
                package: book.package,
                service: "test",
            },
        });
        const context = userContext("user-1", spaceId);
        const call = <Object extends typeof shelf | typeof slip, Name extends CallableName<Object>>(
            object: Object,
            name: Name,
            input: object,
        ) =>
            server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            );

        // lend a book to user-2, issuing and closing a slip user-1 may not write
        const home = await call(shelf, "create", { name: "home" });
        const lent = await call(shelf, "lend", { id: home.id, borrower: "user-2" });
        const rows = await storage.database
            .select({
                id: slip.table.id,
                borrower: slip.table.borrowerId,
                shelf: slip.table.shelf,
                isReturned: slip.table.isReturned,
            })
            .from(slip.table);

        // refuse a client pushing a system method
        const pushed = server.push(
            spaceId,
            [
                {
                    id: "mutation-1",
                    calls: [
                        {
                            method: "slip.issue",
                            release: slip.package.version,
                            input: { borrower: "user-1", shelf: "home" },
                        },
                    ],
                },
            ],
            context,
        );
        // hide system methods from the routes and from pushes
        expect([rows, Object.keys(slip.procedures), (await pushed).outcomes]).toEqual([
            [{ id: lent.slip, borrower: "user-2", shelf: "home!", isReturned: true }],
            [],
            [
                {
                    id: "mutation-1",
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
        const server = new ObjectServer({
            objects: { book, shelf, slip },
            database: storage.database,
            callKey: testCallKey,
            origin: { package: book.package, service: "test" },
        });
        const context = userContext("user-1", spaceId);
        const home = await server.call(
            shelf,
            "create",
            { spaceId, requestId: RequestId.create(), name: "home" },
            context,
        );

        // refuse a page of no books, and an invoked creation on an unnamed shelf, as callers see it
        expect([
            await reported(server.call(book, "list", { spaceId, limit: 0 }, context)),
            await reported(
                server.call(
                    shelf,
                    "misfile",
                    { spaceId, requestId: RequestId.create(), id: home.id },
                    context,
                ),
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
