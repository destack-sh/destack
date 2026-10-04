import { expect, onTestFinished, test } from "@destack/test";
import { Authorization, principal, relation } from "@destack/access";
import { AuditCall, AuditRecorder, defineAuditAction, Journal, journal } from "@destack/audit";
import { AuditHistory, auditTables } from "@destack/audit/history";
import { type DatabaseConnection, defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { present, schema } from "@destack/schema";
import type { Page } from "@destack/sync";

import { testCallKey } from "@destack/service/test";
import { v7 } from "uuid";
import { ObjectServer } from "../src/server/index.ts";
import { defineObject, field, Intrinsic } from "../src/index.ts";
import { openSpace, space } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";

/** The package declaring the test's space and action. */
const PACKAGE = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000007"),
    name: "@example/space",
    version: "2026.9.0",
};

/** The space whose history the test follows. */
const SPACE_ID = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

/** A space's audited calls. */
const call = defineObject(Intrinsic.auditCall(space));
/** The objects a space's audited calls name. */
const target = defineObject(Intrinsic.auditTarget(space, call));

/** Rename a document as the call's target. */
const renameDocument = defineAuditAction(
    {
        name: "document.rename",
        targets: schema.object({
            document: schema.object({ type: schema.literal("document"), id: schema.string() }),
        }),
        details: schema.object({}),
    },
    { package: PACKAGE },
);

/** Serve a space's history to its owner and to a user without a role in it. */
async function serveHistory(dialect: (typeof TEST_DIALECTS)[number]) {
    const storage = await TestDatabase.create(
        dialect,
        defineDatabase({
            name: "main",
            tables: [...auditTables, ...call.tables, space.table, journal],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const database = storage.database;

    // deliver every call the space records straight into its history
    const history = new AuditHistory(database);
    const recorder = new AuditRecorder<DatabaseConnection>(
        {
            caller: { type: "system", name: "document" },
            package: PACKAGE,
            service: "document",
            scope: SPACE_ID,
        },
        {
            record: async (recorded) => {
                await history.ingest({ calls: [recorded] });
            },
        },
    );
    const rename = async (id: string): Promise<string> =>
        (
            await recorder.record(database, renameDocument, {
                targets: { document: { type: "document", id } },
                details: {},
                outcome: { kind: "success" },
            })
        ).execution.id;

    // serve the space's calls as the calling user
    const server = new ObjectServer({
        objects: { call, target },
        policies: [space],
        database,
        callKey: testCallKey,
        origin: { package: PACKAGE, service: "document" },
    });

    // create the space in the database with its owner
    const owner = principal.user.reference("universe", "owner");
    await database
        .insert(space.table)
        .values({ id: SPACE_ID, scope: "universe", createdAt: 1, updatedAt: 1 });
    await new Authorization(server.authorizer, database, () => ({
        subjects: [owner],
        now: Date.now(),
        attributes: {},
    })).create(space.reference("universe", SPACE_ID), { owner });
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const context = (caller: string, signal = controller.signal) =>
        userContext(caller, SPACE_ID, { signal });

    return { server, context, rename, history };
}

test.for(TEST_DIALECTS)(
    "follow a space's audited calls about one document as they arrive on %s",
    async (dialect) => {
        const { server, context, rename } = await serveHistory(dialect);
        await rename("draft");

        // follow the owner's calls about the plan, from a snapshot with the earlier one
        const pages = server.source.relayed(
            server.source.queriesShape.subscription({
                name: "objects",
                scope: SPACE_ID,
                below: SPACE_ID,
                parameters: {
                    queries: {
                        plan: {
                            object: "call",
                            where: { targets: { objectId: "plan" } },
                        },
                    },
                },
            }),
            context("owner"),
        );
        const next = async () => {
            // skip the pages without changes
            let page = await nextPage(pages);
            while (page.changes.length === 0) {
                page = await nextPage(pages);
            }

            // name each call by its identifier
            return page.changes.map((change) => [change.table, change.operation, change.row["id"]]);
        };
        const first = await rename("plan");
        expect(await next()).toEqual([["destack__audit__call", "insert", first]]);

        // keep later calls about the plan alone
        await rename("draft");
        const second = await rename("plan");
        expect(await next()).toEqual([["destack__audit__call", "insert", second]]);
    },
);

test.for(TEST_DIALECTS)(
    "record each read of a space's history in the history itself on %s",
    async (dialect) => {
        const { server, context, rename, history } = await serveHistory(dialect);
        const renamed = await rename("plan");

        // keep the rename and the watch's own record after it ends
        const controller = new AbortController();
        const pages = server.source.relayed(
            server.source.queriesShape.subscription({
                name: "objects",
                scope: SPACE_ID,
                below: SPACE_ID,
                parameters: {
                    queries: {
                        calls: { object: "call", orderBy: { recordedAt: "asc" } as const },
                    },
                },
            }),
            context("owner", controller.signal),
        );
        const snapshot: Page[] = [];
        while (snapshot.at(-1)?.complete !== true) {
            snapshot.push(await nextPage(pages));
        }
        const sent = snapshot.flatMap((page) => page.changes.map((change) => change.row["id"]));

        // record the watch end and a listing without a stranger's refused read
        controller.abort();
        await pages.return(undefined);
        await server.query(call, "list", { spaceId: SPACE_ID }, context("owner"));
        await expect(
            server.query(call, "get", { spaceId: SPACE_ID, id: renamed }, context("stranger")),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no scope ${SPACE_ID}` });
        await server.journal.deliver(history);
        const recorded = await history.list({
            scope: SPACE_ID,
            limit: 100,
        });
        expect(sent).toEqual([renamed]);
        expect(
            recorded.items.map(({ call: read }: { call: AuditCall }) => [
                read.method,
                read.execution.category,
                read.execution.outcome,
            ]),
        ).toEqual([
            ["document.rename", "activity", { kind: "success" }],
            [
                "call.watch",
                "access",
                {
                    kind: "cancelled",
                    error: { code: "CANCELLED", status: 499, message: "cancelled" },
                },
            ],
            ["call.list", "access", { kind: "success" }],
        ]);
    },
);

/** Lockers with audited reads that record the disclosed version. */
const locker = defineObject({
    name: "locker",
    plural: "lockers",
    scope: space,
    audited: { reads: true },
    fields: { owner: field.reference(principal.user).caller(), version: field.integer() },
    permissions: { read: relation("owner") },
    methods: (method) => ({
        open: method.query({
            permission: "read",
            output: schema.object({ version: schema.number(), value: schema.string() }),
            audit: { details: schema.object({ version: schema.number() }) },
        }),
    }),
});

test("name the version a read disclosed in its recorded call, apart from its value", async () => {
    const storage = await TestDatabase.create("sqlite", [...locker.tables, journal], {
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, SPACE_ID);
    const [row] = await database
        .insert(locker.table)
        .values({
            id: schema.identifier("locker").parse(`locker-${v7()}`),
            scope: SPACE_ID,
            ownerId: "owner",
            version: 3,
            createdAt: 1,
            updatedAt: 1,
        })
        .returning();

    // record each audited read in the journal
    const handled = locker.handle({
        open: async (opening) => ({ version: opening.target.version, value: "secret" }),
    });
    const server = new ObjectServer({
        objects: { locker: handled },
        database,
        callKey: testCallKey,
        origin: {
            package: locker.package,
            service: "test",
        },
    });
    const context = userContext("owner", SPACE_ID);
    const { id } = present(row, "the inserted locker");

    // name the version in the result, never the value
    expect(await server.call(handled, "open", { spaceId: SPACE_ID, id }, context)).toEqual({
        version: 3,
        value: "secret",
    });
    const recorded = await new Journal(database, testCallKey).read();
    expect(
        recorded.map((read) => [
            read.method,
            read.execution.category,
            read.execution.targets,
            read.execution.details,
        ]),
    ).toEqual([["locker.open", "access", { locker: { type: "locker", id } }, { version: 3 }]]);
});

/** Read the next page of a sync stream, refusing an ended stream. */
async function nextPage(pages: AsyncGenerator<Page>): Promise<Page> {
    const read = await pages.next();
    if (read.done === true) {
        throw new TypeError("the sync stream ended");
    }

    return read.value;
}
