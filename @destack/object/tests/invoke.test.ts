import { AuditOutbox, auditOutboxTables } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier, schema } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { defineJournal, Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { defineObject, field, method } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { auditedActions } from "./fixture/audit.ts";

/** The space containing the shelves. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000011");

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
    methods: {
        create: method.create("write", { fields: ["shelf"] }),
        update: method.update("write", { fields: ["isTidy"] }),
        tidy: method.updateMany("write", { fields: ["isTidy"], match: ["shelf"] }),
    },
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
    methods: {
        issue: method.create(null, { isSystem: true }),
        close: method.update(null, { isSystem: true }),
    },
});

/** Shelves their owners tidy, tidying their books with them. */
const shelf = defineObject({
    name: "shelf",
    plural: "shelves",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), name: field.string() },
    permissions: { read: relation("owner"), write: relation("owner") },
    methods: {
        create: method.create("write", { fields: ["name"] }),
        tidy: method({ permission: "write", output: schema.object({ count: schema.number() }) }),
        straighten: method({
            permission: "write",
            input: schema.object({ book: schema.string() }),
        }),
        lend: method({
            permission: "write",
            input: schema.object({ borrower: schema.string() }),
            output: schema.object({ slip: schema.string() }),
        }),
    },
}).handle({
    // tidy the shelf's books as the caller
    tidy: async (call) =>
        call.invoke(book, "tidy", { where: { shelf: call.target!.name }, isTidy: true }),
    // tidy one owned book as the caller
    straighten: async (call) => {
        await call.invoke(book, "update", { id: call.input.book, isTidy: true });

        return call.target;
    },
    // issue a slip to the borrower as the system, then close it
    lend: async (call) => {
        const issued = (await call.invoke(slip, "issue", {
            borrower: call.input.borrower,
            shelf: call.target!.name,
        })) as { id: string };
        await call.invoke(slip, "close", { id: issued.id, isReturned: true });

        return { slip: issued.id };
    },
});

/** Replayable shelf requests. */
const journal = defineJournal("journal");

/** The database holding the shelves, the books, their access and the journal. */
const shelfDatabase = defineDatabase({
    name: "main",
    tables: [...auditOutboxTables, journal, ...book.tables, ...shelf.tables, ...slip.tables],
});

test.each(TEST_DIALECTS)(
    "invoke another object's method in a call, as its caller, audited and refused alike, on %s",
    async (dialect) => {
        const storage = await TestDatabase.create(dialect, shelfDatabase, { isMigrated: true });
        onTestFinished(() => storage.close());
        await openSpace(storage.database, spaceId);
        let current = "user-1";
        const server = new ObjectServer({
            objects: { book, shelf, slip },
            database: storage.database,
            context: () => ({
                subjects: [principal.user.reference("global", current)],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(journal),
            audit: AuditRecorder.service(new AuditOutbox(storage.database), {
                package: book.package,
                service: "test",
            }),
        });
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: current }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const call = async (object: typeof book | typeof shelf, name: string, input: object) =>
            (await server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            )) as {
                id: string;
            };

        // shelve two books of user-1 and one of user-2 on the same shelf name
        const home = await call(shelf, "create", { name: "home" });
        await call(book, "create", { shelf: "home" });
        await call(book, "create", { shelf: "home" });
        current = "user-2";
        const foreign = await call(book, "create", { shelf: "home" });

        // tidy user-1's writable books on the shelf, auditing both calls
        current = "user-1";
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
            .select({ owner: book.table.owner, isTidy: book.table.isTidy })
            .from(book.table);
        const summary = rows.map((row) => `${String(row.owner)} ${String(row.isTidy)}`).sort();
        expect([tidied, auditedTidy, summary]).toEqual([
            { count: 2 },
            ["Book.tidy", "Shelf.tidy"],
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
                // mark issued slips on the server, which invoking the base type still runs
                slip: slip.handle({
                    issue: (call, next) =>
                        next(
                            call.with({
                                input: { ...call.input, shelf: `${String(call.input.shelf)}!` },
                            }),
                        ),
                }),
            },
            database: storage.database,
            context: () => ({
                subjects: [principal.user.reference("global", "user-1")],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(journal),
            audit: AuditRecorder.service(new AuditOutbox(storage.database), {
                package: book.package,
                service: "test",
            }),
        });
        const context = {
            scope: spaceId,
            requireCaller: () => ({ id: "user-1" }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
        } as unknown as ServiceContext;
        const call = async (object: typeof shelf | typeof slip, name: string, input: object) =>
            (await server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            )) as { id: string; slip: string };

        // lend a book to user-2, issuing and closing a slip user-1 may not write
        const home = await call(shelf, "create", { name: "home" });
        const lent = await call(shelf, "lend", { id: home.id, borrower: "user-2" });
        const rows = await storage.database
            .select({
                id: slip.table.id,
                borrower: slip.table.borrower,
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
                    calls: [{ method: "slip.issue", input: { borrower: "user-1", shelf: "home" } }],
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
