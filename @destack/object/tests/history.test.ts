import { Subject } from "@destack/sync";
import { expect, onTestFinished, test } from "@destack/test";
import { principal, relation, through, union } from "@destack/access";
import { journal } from "@destack/audit/stack";
import { defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { schema } from "@destack/schema";

import { RequestId } from "@destack/service/request";
import {
    defineObject,
    field,
    Intrinsic,
    type CallableName,
    type ObjectType,
} from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { user } from "./schema.ts";
import { openSpace, space } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";
import { testCallKey } from "@destack/service/test";

/** The space containing the pages. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000007");

/** A minute in milliseconds, the step between the test's changes. */
const MINUTE = 60_000;

/** The sessions of changes to pages. */
const activity = defineObject(Intrinsic.activity(space));

/** The named points in pages' history. */
const checkpoint = defineObject(Intrinsic.checkpoint(space));

/** Pages with history that their owner and editors write. */
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
    methods: (method) => ({
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        update: method.update("edit"),
    }),
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
            subject: principal.user.reference("universe", "bob"),
        });
        await call(page, "update", { id: created.id, body: "outline" });

        // write it as bob and as alice again once her session ended
        as("bob");
        at(2 * MINUTE);
        await call(page, "update", { id: created.id, title: "Roadmap" });
        as("alice");
        at(30 * MINUTE);
        await call(page, "update", { id: created.id, body: "final" });

        // list one activity per session, with the fields each changed
        const activities = await call(activity, "list", {});
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
                caller: Subject.key(principal.user.reference("universe", "alice")),
                startedAt: 0,
                endedAt: 1 * MINUTE,
                fields: ["ownerId", "title", "body"],
                changes: 3,
            },
            {
                caller: Subject.key(principal.user.reference("universe", "bob")),
                startedAt: 2 * MINUTE,
                endedAt: 2 * MINUTE,
                fields: ["title"],
                changes: 1,
            },
            {
                caller: Subject.key(principal.user.reference("universe", "alice")),
                startedAt: 30 * MINUTE,
                endedAt: 30 * MINUTE,
                fields: ["body"],
                changes: 1,
            },
        ]);

        // read the page before each later session, and find nothing before it existed
        const read = async (position: object) => {
            const row = await call(page, "get", { id: created.id, at: position });

            return [row.title, row.body];
        };
        const [first, second, third] = sessions;
        if (first === undefined || second === undefined || third === undefined) {
            throw new TypeError("the page's history has fewer than three sessions");
        }
        expect([await read(second.from), await read(third.from)]).toEqual([
            ["Plan", "outline"],
            ["Roadmap", "outline"],
        ]);
        await expect(read(first.from)).rejects.toMatchObject({ code: "NOT_FOUND" });

        // list the pages at a position as each caller could read them then and now
        const listed = async (position: object) =>
            (await call(page, "list", { at: position })).items.map((item) => item.title);
        const aliceListed = await listed(beforeGrant);
        as("bob");
        const bobListed = await listed(beforeGrant);
        expect([aliceListed, bobListed]).toEqual([["Plan"], []]);

        // find nothing for bob before he was granted the page, nor for carol ever
        await expect(read(beforeGrant)).rejects.toMatchObject({ code: "NOT_FOUND" });
        expect(await read(third.from)).toEqual(["Roadmap", "outline"]);
        as("carol");
        await expect(read(third.from)).rejects.toMatchObject({ code: "NOT_FOUND" });

        // name a checkpoint and read the page at it
        as("alice");
        const named = await call(checkpoint, "create", { ...host, title: "Final" });
        expect([named.createdBy, await read(named.position)]).toEqual([
            Subject.key(principal.user.reference("universe", "alice")),
            ["Roadmap", "final"],
        ]);

        // revert to before bob's session
        const reverted = await call(page, "revert", { id: created.id, at: second.from });
        expect([reverted.title, reverted.body]).toEqual(["Plan", "outline"]);
        const latest = await call(activity, "list", {});
        expect(latest.items.find((item) => item.endedAt === 30 * MINUTE)).toMatchObject({
            changes: 2,
            fields: ["body", "title"],
        });
    },
);

test("refuse serving history that reads through an object keeping none", async () => {
    // read pages through an untracked folder
    const folder = defineObject({
        name: "folder",
        plural: "folders",
        scope: space,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner") },
        methods: (method) => ({ get: method.get("read") }),
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
        methods: (method) => ({ get: method.get("read") }),
    });

    // refuse the sheet's history, which the folder's lost changes decided
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({ name: "main", tables: [journal] }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    expect(
        () =>
            new ObjectServer({
                objects: { activity, folder, sheet },
                database: storage.database,
                callKey: testCallKey,
                origin: { package: activity.package, service: "test" },
            }),
    ).toThrow(
        new TypeError("object sheet keeps history but reads through folder, which keeps none"),
    );
});

/** Serve pages over a new database to one user at a set time. */
async function servePages(dialect: (typeof TEST_DIALECTS)[number]) {
    // keep the pages, their activities and checkpoints, and requests in one database
    const objects = { activity, checkpoint, page };
    const storage = await TestDatabase.create(
        dialect,
        defineDatabase({
            name: "main",
            tables: [journal, ...Object.values(objects).flatMap((object) => object.tables)],
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
        callKey: testCallKey,
        origin: {
            package: activity.package,
            service: "test",
        },
    });
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const sign = () =>
        userContext(current, spaceId, { signal: controller.signal, clock: () => now });
    let context = sign();

    return {
        storage,
        call: <Object extends ObjectType, Name extends CallableName<Object>>(
            object: Object,
            name: Name,
            input: object,
        ) =>
            server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            ),
        as: (caller: string) => {
            current = caller;
            context = sign();
        },
        at: (time: number) => {
            now = time;
            context = sign();
        },
    };
}

test("undo a revert by updating the reverted fields no one changed since back", () => {
    // restore the title and leave the later body change
    const input = { spaceId, id: "page-1", at: { epoch: "epoch", sequence: 3 } };
    const before = { title: "Roadmap", body: "final" };
    const after = { title: "Plan", body: "outline" };
    const { revert } = page.methods;
    if (revert.inverse === undefined) {
        throw new TypeError("the revert has no inverse");
    }
    expect(
        revert.inverse({
            object: page,
            name: "revert",
            input,
            before,
            after,
            current: { title: "Plan", body: "edited" },
        }),
    ).toEqual([
        {
            method: "page.update",
            release: page.package.version,
            input: { spaceId, id: "page-1", title: "Roadmap" },
        },
    ]);
});
