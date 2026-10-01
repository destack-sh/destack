import { expect, onTestFinished, test } from "@destack/test";
import { Authorization, principal, relation, type AccessContext } from "@destack/access";
import { AuditCall, AuditRecorder, defineAuditAction, Journal, journal } from "@destack/audit";
import { AuditHistory, auditTables } from "@destack/audit/history";
import { type DatabaseConnection } from "@destack/db";
import { Condition } from "@destack/db/query";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { PackageId } from "@destack/package";
import { identifier, schema } from "@destack/schema";
import type { QueryPage } from "@destack/sync";

import { subjectContext, testCallKey } from "@destack/service/test";
import { Bookmark } from "@destack/service/bookmark";
import type { ServiceContext } from "@destack/service/server";
import { v7 } from "uuid";
import { ObjectServer } from "../src/server/index.ts";
import { defineObject, field, Intrinsic, method } from "../src/index.ts";
import { openSpace, space } from "./fixture/space.ts";

/** The package declaring the test's space and action. */
const PACKAGE = {
    id: PackageId.parse("package-01996ab0-0000-7000-8000-000000000007"),
    name: "@example/space",
    version: "2026.9.0",
};

/** The space whose history the test follows. */
const SPACE_ID = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000001");

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
    let current = "owner";
    const server = new ObjectServer({
        objects: { call, target },
        policies: [space],
        database,
        context: (): AccessContext => ({
            subjects: [principal.user.reference("universe", current)],
            now: Date.now(),
            attributes: {},
        }),
        callKey: testCallKey,
        origin: { package: PACKAGE, service: "document" },
    });

    // create the space, owned by the first user, in the database holding it
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
    const context = (caller: string, signal = controller.signal) => {
        current = caller;

        return subjectContext(principal.user.reference("universe", caller), SPACE_ID, signal);
    };

    return { server, context, rename, history };
}

test.for(TEST_DIALECTS)(
    "follow a space's audited calls about one document as they arrive on %s",
    async (dialect) => {
        const { server, context, rename } = await serveHistory(dialect);
        await rename("draft");

        // follow the owner's calls about the plan, from a snapshot holding the earlier one
        const pages = server.source.sync(SPACE_ID, context("owner"), {
            queries: {
                plan: {
                    object: "call",
                    where: Condition.exists("targets", Condition.eq("objectId", "plan")),
                },
            },
        });
        const next = async () => {
            let page = (await pages.next()).value as QueryPage;
            while (page.changes.length === 0) {
                page = (await pages.next()).value as QueryPage;
            }

            // name each call by its identifier
            return page.changes.map((change) => [change.table, change.operation, change.row.id]);
        };
        const first = await rename("plan");
        expect(await next()).toEqual([["destack__audit__call", "insert", first]]);

        // hold later calls about the plan alone
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

        // hold the rename and the watch's own record after it ends
        const controller = new AbortController();
        const pages = server.source.sync(SPACE_ID, context("owner", controller.signal), {
            queries: {
                calls: { object: "call", order: [{ column: "recordedAt", direction: "asc" }] },
            },
        });
        const snapshot: QueryPage[] = [];
        while (!snapshot.at(-1)?.complete) {
            snapshot.push((await pages.next()).value as QueryPage);
        }
        const held = snapshot.flatMap((page) => page.changes.map((change) => change.row.id));

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
        expect(held).toEqual([renamed]);
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
    methods: {
        open: method({
            permission: "read",
            mutates: false,
            output: schema.object({ version: schema.number(), value: schema.string() }),
            audit: { details: schema.object({ version: schema.number() }) },
        }),
    },
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
            id: identifier("locker").parse(`locker-${v7()}`),
            scope: SPACE_ID,
            owner: "owner",
            version: 3,
            createdAt: 1,
            updatedAt: 1,
        })
        .returning();

    // record each audited read in the journal
    const handled = locker.handle({
        open: async (call) => ({ version: call.target!.version, value: "secret" }),
    });
    const server = new ObjectServer({
        objects: { locker: handled },
        database,
        context: (): AccessContext => ({
            subjects: [principal.user.reference("universe", "owner")],
            now: Date.now(),
            attributes: {},
        }),
        callKey: testCallKey,
        origin: {
            package: locker.package,
            service: "test",
        },
    });
    const context = {
        scope: SPACE_ID,
        bookmark: new Bookmark(),
        observed: new Bookmark(),
    } as unknown as ServiceContext;

    // name the version in the result, never the value
    expect(await server.call(handled, "open", { spaceId: SPACE_ID, id: row!.id }, context)).toEqual(
        {
            version: 3,
            value: "secret",
        },
    );
    const recorded = await new Journal(database, testCallKey).read();
    expect(
        recorded.map((read) => [
            read.method,
            read.execution.category,
            read.execution.targets,
            read.execution.details,
        ]),
    ).toEqual([
        ["locker.open", "access", { locker: { type: "locker", id: row!.id } }, { version: 3 }],
    ]);
});
