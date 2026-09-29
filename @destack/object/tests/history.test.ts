import { AuditOutbox, auditOutboxTables } from "@destack/audit/outbox";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation, subjectKey, through, union } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import type { DatabaseConnection } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { defineObject, field, Intrinsic, method, type ObjectType } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { request, user } from "./schema.ts";
import { openSpace, space } from "./fixture/space.ts";

/** The space containing the pages. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000007");

/** A minute in milliseconds, the step between the test's changes. */
const MINUTE = 60_000;

/** The sessions of changes to pages. */
const activity = defineObject(Intrinsic.activity(space));

/** The named points in pages' history. */
const checkpoint = defineObject(Intrinsic.checkpoint(space));

/** Pages their owner and editors write, keeping their history. */
const page = defineObject({
    name: "page",
    plural: "pages",
    scope: space,
    fields: {
        owner: field.reference(principal.user).caller(),
        title: field.string(),
        body: field.string(),
    },
    relations: { editor: { subjects: [user] } },
    permissions: {
        read: union(relation("owner"), relation("editor")),
        edit: union(relation("owner"), relation("editor")),
        manage: relation("owner"),
    },
    shareable: { by: "manage" },
    attachments: [activity.attach({ by: "read" }), checkpoint.attach({ by: "edit" })],
    tracked: { by: "edit", activity },
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    },
});

test.for(TEST_DIALECTS)(
    "group each caller's changes into activities, and read and revert a page as it was, on %s",
    async (dialect) => {
        const { storage, call, as, at } = await servePages(dialect);

        // write a page as alice, twice within one session
        const created = await call(page, "create", { title: "Plan", body: "draft" });
        const host = {
            parent: { packageId: page.policy.definition.packageId, type: "page", id: created.id },
        };
        const beforeGrant = await storage.database.log.position();
        at(1 * MINUTE);
        await call(page, "grant", {
            id: created.id,
            relation: "editor",
            subject: principal.user.reference("global", "bob"),
        });
        await call(page, "update", { id: created.id, body: "outline" });

        // write it as bob, then as alice again once her session ended
        as("bob");
        at(2 * MINUTE);
        await call(page, "update", { id: created.id, title: "Roadmap" });
        as("alice");
        at(30 * MINUTE);
        await call(page, "update", { id: created.id, body: "final" });

        // list one activity per session, with the fields each changed
        const activities = (await call(activity, "list", {})) as unknown as {
            items: {
                caller: string;
                startedAt: number;
                endedAt: number;
                fields: string[];
                changes: number;
                from: { epoch: string; sequence: number };
            }[];
        };
        const sessions = activities.items.toSorted(
            (left, right) => left.startedAt - right.startedAt,
        );
        expect(
            sessions.map(({ caller, startedAt, endedAt, fields, changes }) => ({
                caller,
                startedAt,
                endedAt,
                fields,
                changes,
            })),
        ).toEqual([
            {
                caller: subjectKey(principal.user.reference("global", "alice")),
                startedAt: 0,
                endedAt: 1 * MINUTE,
                fields: ["owner", "title", "body"],
                changes: 3,
            },
            {
                caller: subjectKey(principal.user.reference("global", "bob")),
                startedAt: 2 * MINUTE,
                endedAt: 2 * MINUTE,
                fields: ["title"],
                changes: 1,
            },
            {
                caller: subjectKey(principal.user.reference("global", "alice")),
                startedAt: 30 * MINUTE,
                endedAt: 30 * MINUTE,
                fields: ["body"],
                changes: 1,
            },
        ]);

        // read the page before each later session, and find nothing before it existed
        const read = async (at: object) => {
            const row = (await call(page, "history", { id: created.id, at })) as unknown as {
                title: string;
                body: string;
            };

            return [row.title, row.body];
        };
        expect([await read(sessions[1]!.from), await read(sessions[2]!.from)]).toEqual([
            ["Plan", "outline"],
            ["Roadmap", "outline"],
        ]);
        await expect(read(sessions[0]!.from)).rejects.toMatchObject({ code: "NOT_FOUND" });

        // find nothing for bob before he was granted the page, nor for carol ever
        as("bob");
        await expect(read(beforeGrant)).rejects.toMatchObject({ code: "NOT_FOUND" });
        expect(await read(sessions[2]!.from)).toEqual(["Roadmap", "outline"]);
        as("carol");
        await expect(read(sessions[2]!.from)).rejects.toMatchObject({ code: "NOT_FOUND" });

        // name a checkpoint and read the page at it
        as("alice");
        const named = (await call(checkpoint, "create", {
            ...host,
            title: "Final",
        })) as unknown as { position: object; createdBy: string };
        expect([named.createdBy, await read(named.position)]).toEqual([
            subjectKey(principal.user.reference("global", "alice")),
            ["Roadmap", "final"],
        ]);

        // revert to before bob's session
        const reverted = (await call(page, "revert", {
            id: created.id,
            at: sessions[1]!.from,
        })) as unknown as { title: string; body: string };
        expect([reverted.title, reverted.body]).toEqual(["Plan", "outline"]);
        const latest = (await call(activity, "list", {})) as unknown as {
            items: { changes: number; fields: string[]; endedAt: number }[];
        };
        expect(latest.items.find((item) => item.endedAt === 30 * MINUTE)!).toMatchObject({
            changes: 2,
            fields: ["body", "title"],
        });
    },
);

test("refuse history without the activity among the attachments", () => {
    expect(() =>
        defineObject({
            name: "sheet",
            plural: "sheets",
            scope: space,
            fields: { owner: field.reference(principal.user).caller() },
            permissions: { read: relation("owner") },
            tracked: { by: "read", activity },
            methods: { get: method.get("read") },
        }),
    ).toThrow(new TypeError("object sheet keeps history but takes no activities"));
});

test("refuse serving history that reads through an object keeping none", () => {
    // read pages through an untracked folder
    const folder = defineObject({
        name: "folder",
        plural: "folders",
        scope: space,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner") },
        methods: { get: method.get("read") },
    });
    const sheet = defineObject({
        name: "sheet",
        plural: "sheets",
        scope: space,
        nested: { in: folder, receive: "read" },
        fields: { title: field.string() },
        permissions: { read: through("parent", "read") },
        attachments: [activity.attach({ by: "read" })],
        tracked: { by: "read", activity },
        methods: { get: method.get("read") },
    });

    // refuse the sheet's history, whose earlier readers the folder's lost changes decided
    expect(
        () =>
            new ObjectServer({
                objects: { activity, folder, sheet },
                database: {} as DatabaseConnection,
                context: () => ({ subjects: [], now: 0, attributes: {} }),
                journal: new Journal(request),
                audit: () => ({}) as AuditRecorder<DatabaseConnection>,
            }),
    ).toThrow(
        new TypeError("object sheet keeps history but reads through folder, which keeps none"),
    );
});

/** Serve pages over a new database to one user at a set time. */
async function servePages(dialect: (typeof TEST_DIALECTS)[number]) {
    // hold the pages, their activities and checkpoints, and requests in one database
    const objects = { activity, checkpoint, page };
    const storage = await TestDatabase.create(
        dialect,
        defineDatabase({
            name: "main",
            tables: [
                ...auditOutboxTables,
                request,
                ...Object.values(objects).flatMap((object) => object.tables),
            ],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // decide every call as the current user at the current time
    let current = "alice";
    let now = 0;
    const server = new ObjectServer({
        objects,
        database: storage.database,
        context: () => ({
            subjects: [principal.user.reference("global", current)],
            now,
            attributes: {},
        }),
        journal: new Journal(request),
        audit: AuditRecorder.service(new AuditOutbox(storage.database), {
            package: activity.package,
            service: "test",
        }),
    });
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const context = {
        scope: spaceId,
        requireCaller: () => ({ id: current }),
        bookmark: new Bookmark(),
        observed: new Bookmark(),
        signal: controller.signal,
        request: new Request("https://test.local", { signal: controller.signal }),
    } as unknown as ServiceContext;

    return {
        storage,
        call: async (object: ObjectType, name: string, input: object) =>
            (await server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            )) as { id: string },
        as: (user: string) => {
            current = user;
        },
        at: (time: number) => {
            now = time;
        },
    };
}

test("undo a revert by updating the reverted fields no one changed since back", () => {
    // restore the title, leaving the body changed since
    const input = { spaceId, id: "page-1", at: { epoch: "epoch", sequence: 3 } };
    const before = { title: "Roadmap", body: "final" };
    const after = { title: "Plan", body: "outline" };
    expect(
        page.methods.revert.inverse!({
            object: page,
            name: "revert",
            input,
            before,
            after,
            current: { title: "Plan", body: "edited" },
        }),
    ).toEqual([{ method: "page.update", input: { spaceId, id: "page-1", title: "Roadmap" } }]);
});
