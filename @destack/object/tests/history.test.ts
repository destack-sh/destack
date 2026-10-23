import { ObjectReference } from "@destack/sync";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { principal, relation, through, union } from "@destack/access";
import { defineDatabase, type LogPosition } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { schema, type Identifier } from "@destack/schema";

import {
    activity,
    type CallableName,
    checkpoint,
    defineObject,
    field,
    type ObjectType,
} from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { user } from "./schema.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { Clock } from "@destack/environment";
import { VirtualClock } from "@destack/environment/simulation";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";

/** The space containing the pages. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000007");

/** A minute in milliseconds, the step between the test's changes. */
const MINUTE = 60_000;

/** Pages with history that their owner and editors write. */
const page = defineObject(
    {
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
    },
    (method) => ({
        create: method.create("edit"),
        update: method.update("edit"),
    }),
);

test("group each caller's changes into activities, and read and revert a page as it was", async () => {
    const { storage, call, as, at } = await servePages();

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
        subject: principal.user.reference("universe", users.bob),
    });
    await call(page, "update", { id: created.id, body: "outline" });

    // write it as bob and as alice again once her session ended
    as(users.bob);
    at(2 * MINUTE);
    await call(page, "update", { id: created.id, title: "Roadmap" });
    as(users.alice);
    at(30 * MINUTE);
    await call(page, "update", { id: created.id, body: "final" });

    // list one activity per session, with the fields each changed
    const activities = await call(activity, "list", {});
    const sessions = activities.items.toSorted((left, right) => left.startedAt - right.startedAt);
    expect(
        sessions.map((session) => ({
            caller: ObjectReference.at("caller", session),
            startedAt: session.startedAt,
            endedAt: session.endedAt,
            fields: session.fields,
            changes: session.changes,
        })),
    ).toEqual([
        {
            caller: principal.user.reference("universe", users.alice),
            startedAt: 0,
            endedAt: 1 * MINUTE,
            fields: ["ownerId", "title", "body"],
            changes: 3,
        },
        {
            caller: principal.user.reference("universe", users.bob),
            startedAt: 2 * MINUTE,
            endedAt: 2 * MINUTE,
            fields: ["title"],
            changes: 1,
        },
        {
            caller: principal.user.reference("universe", users.alice),
            startedAt: 30 * MINUTE,
            endedAt: 30 * MINUTE,
            fields: ["body"],
            changes: 1,
        },
    ]);

    // read the page before each later session, and find nothing before it existed
    const read = async (position: LogPosition) => {
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
    expect(await refusal(read(first.from))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `page ${created.id} not found`,
    ]);

    // list the pages at a position as each caller could read them then and now
    const listed = async (position: LogPosition) =>
        (await call(page, "list", { at: position })).items.map((item) => item.title);
    const aliceListed = await listed(beforeGrant);
    as(users.bob);
    const bobListed = await listed(beforeGrant);
    expect([aliceListed, bobListed]).toEqual([["Plan"], []]);

    // find nothing for bob before he was granted the page, nor for carol ever
    expect(await refusal(read(beforeGrant))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `page ${created.id} not found`,
    ]);
    expect(await read(third.from)).toEqual(["Roadmap", "outline"]);
    as(users.carol);
    expect(await refusal(read(third.from))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `page ${created.id} not found`,
    ]);

    // create a named checkpoint and read the page at it
    as(users.alice);
    const named = await call(checkpoint, "create", { ...host, title: "Final" });
    expect([ObjectReference.at("createdBy", named), await read(named.position)]).toEqual([
        principal.user.reference("universe", users.alice),
        ["Roadmap", "final"],
    ]);

    // revert to before bob's session
    const reverted = await call(page, "revert", { id: created.id, at: second.from });
    expect([reverted.title, reverted.body]).toEqual(["Plan", "outline"]);
    const latest = await call(activity, "list", {});
    expect(
        latest.items
            .filter((item) => item.endedAt === 30 * MINUTE)
            .map(({ changes, fields }) => ({ changes, fields })),
    ).toEqual([{ changes: 2, fields: ["body", "title"] }]);
});

test("refuse serving history that reads through an object recording none", async () => {
    // read pages through an untracked folder
    const folder = defineObject({
        name: "folder",
        plural: "folders",
        scope: space,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner") },
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
    });

    // refuse the sheet's history, which the folder's lost changes decided
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({ packages: [objectPackage], follows: [], tables: [] }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const events = await testEvents(storage.database);
    expect(
        () =>
            new ObjectStore({
                chain: async () => undefined,
                clock: Clock.system,
                principal: spacePrincipal(spaceId),
                servers: serversOf([activity, folder, sheet]),
                database: storage.database,
                callKey: testCallKey,
                events,
                origin: { package: activity.package },
            }),
    ).toThrow(
        new TypeError(
            "object sheet records history but reads through folder, which has no history",
        ),
    );
});

/** Serve pages over a new database to one user at a set time. */
async function servePages() {
    // store the pages, their activities and checkpoints, and requests in one database
    const objects = [activity, checkpoint, page];
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [
                    objectPackage,
                    {
                        objects: Object.fromEntries(objects.map((object) => [object.name, object])),
                        tables: [],
                    },
                ],
                follows: [],
                tables: [],
            }),
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // decide every call as the current user at the current time
    let current = users.alice;
    const clock = new VirtualClock(0);
    const store = new ObjectStore({
        chain: async () => undefined,
        principal: spacePrincipal(spaceId),
        servers: serversOf(objects),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: {
            package: activity.package,
        },
        clock,
    });
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const sign = () => userContext(current, spaceId, { signal: controller.signal, clock: clock });
    let context = sign();

    return {
        storage,
        call: <Object extends ObjectType, Name extends CallableName<Object>>(
            object: Object,
            name: Name,
            input: NoInfer<RequestInput<Object, Name>>,
        ) => request(store, object, name, spaceId, input, context),
        as: (caller: Identifier<"user">) => {
            current = caller;
            context = sign();
        },
        at: (time: number) => {
            clock.set(time);
            context = sign();
        },
    };
}

test("undo a revert by updating the reverted fields no one changed since back", () => {
    // restore the title and leave the later body change
    const input = { spaceId, id: "page-1", at: { log: "log", epoch: "epoch", sequence: 3 } };
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
            type: page.typeReference,
            method: "update",
            release: page.package.version,
            input: { spaceId, id: "page-1", title: "Roadmap" },
        },
    ]);
});
