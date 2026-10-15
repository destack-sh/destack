import { present, schema, Clock } from "@destack/schema";
import { expect, refusal, onTestFinished, test } from "@destack/test";
import { intersection, principal, relation, through, union } from "@destack/access";
import { asc, eq, type Dialect, defineDatabase } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { v7 } from "uuid";

import { RequestId } from "@destack/runtime/request";
import {
    defineObject,
    field,
    ParentReference,
    type CallableName,
    type ObjectType,
} from "../src/index.ts";
import { ObjectStore } from "../src/server/index.ts";
import { user } from "./schema.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext } from "./fixture/user.ts";
import { testCallKey } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";
import { request, type RequestInput } from "./fixture/request.ts";

/** The space containing the objects. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** Remarks on any object that takes them, readable by whoever reads it. */
const remark = defineObject({
    name: "remark",
    plural: "remarks",
    scope: space,
    nested: { in: "any", receive: "remark" },
    fields: { text: field.string(schema.string().min(1)) },
    permissions: { read: through("parent", "read"), write: through("parent", "remark") },
    aggregates: { remarkCount: { function: "count" } },
}).methods((method) => ({
    create: method.create("write"),
    delete: method.delete("write"),
    /** Count the remarks on a parent, checked as on a new remark there. */
    count: method.query({
        permission: "read",
        target: "scope",
        input: schema.object({ parent: ParentReference }),
        output: schema.object({ count: schema.int() }),
    }),
}));

/** Pins that keep any hosting object from deletion. */
const pin = defineObject({
    name: "pin",
    plural: "pins",
    scope: space,
    nested: { in: "any", delete: "restrict", receive: "pin" },
    fields: {},
    permissions: { read: through("parent", "read"), write: through("parent", "pin") },
}).methods((method) => ({ create: method.create("write"), delete: method.delete("write") }));

/** Articles taking remarks. */
const article = defineObject({
    name: "article",
    plural: "articles",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), remarkCount: field.count() },
    permissions: { read: relation("owner"), edit: relation("owner") },
    attachments: [remark.attach({ by: "edit" })],
}).methods((method) => ({
    create: method.create("edit"),
    delete: method.delete("edit"),
}));

/** Photos their owner remarks on and pins by reading them. */
const photo = defineObject({
    name: "photo",
    plural: "photos",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), remarkCount: field.count() },
    permissions: { read: relation("owner") },
    attachments: [remark.attach({ by: "read" }), pin.attach({ by: "read" })],
}).methods((method) => ({
    create: method.create("read"),
    delete: method.delete("read"),
}));

/** Tasks taking no remarks. */
const task = defineObject({
    name: "task",
    plural: "tasks",
    scope: space,
    fields: { owner: field.reference(principal.user).caller() },
    permissions: { read: relation("owner") },
}).methods((method) => ({ create: method.create("read") }));

/** A reader's private favourite mark, one per object. */
const favourite = defineObject({
    name: "favourite",
    plural: "favourites",
    scope: space,
    nested: { in: "any", receive: "favourite" },
    fields: { reader: field.reference(principal.user).caller() },
    permissions: {
        read: intersection(relation("reader"), through("parent", "favourite")),
        write: intersection(relation("reader"), through("parent", "favourite")),
    },
    constraints: {
        reader: { kind: "unique", on: ["parentPackageId", "parentType", "parentId", "readerId"] },
    },
}).methods((method) => ({
    create: method.create("write"),
    delete: method.delete("write"),
}));

/** Boards shared with viewers who may mark them as a favourite. */
const board = defineObject({
    name: "board",
    plural: "boards",
    scope: space,
    fields: { owner: field.reference(principal.user).caller() },
    relations: { viewer: { subjects: [user] } },
    permissions: {
        read: union(relation("owner"), relation("viewer")),
        manage: relation("owner"),
    },
    shareable: { by: "manage" },
    attachments: [favourite.attach({ by: "read" })],
}).methods((method) => ({ create: method.create("manage"), delete: method.delete("manage") }));

test("map the types attached to served objects, which the database keeps beside them", async () => {
    // serve photos alone over a database keeping their remarks and pins
    const storage = await TestDatabase.create(
        "sqlite",
        defineDatabase({
            name: "main",
            tables: [...callTables, ...photo.tables, ...remark.tables, ...pin.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: [photo],
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: { package: photo.package },
    });

    // map the remarks and pins whose permissions read through their photo
    expect(
        [remark, pin].map(
            (type) => store.authorizer.mapping(type.typeReference)?.table === type.table,
        ),
    ).toEqual([true, true]);
});

test.for(TEST_DIALECTS)(
    "attach remarks to articles and photos, counting them and deleting them with their host, on %s",
    async (dialect) => {
        const { storage, store, context, call, host, as } = await serveObjects(dialect, {
            remark,
            pin,
            article,
            photo,
            task,
        });

        // remark on alice's article and photo, and count the remarks on each
        const draft = await call(article, "create", {});
        const snapshot = await call(photo, "create", {});
        const task1 = await call(task, "create", {});
        await call(remark, "create", { ...host(article, draft.id), text: "Tighten the intro" });
        await call(remark, "create", { ...host(article, draft.id), text: "Cite the source" });
        const caption = await call(remark, "create", { ...host(photo, snapshot.id), text: "Crop" });
        const counts = async () => [
            ...(await storage.database.select().from(article.table)).map((row) => row.remarkCount),
            ...(await storage.database.select().from(photo.table)).map((row) => row.remarkCount),
        ];
        expect(await counts()).toEqual([2, 1]);

        // refuse invalid, missing and unreadable hosts
        const missing = `article-${v7()}`;
        expect(
            await refusal(call(remark, "create", { ...host(task, task1.id), text: "Nope" })),
        ).toEqual(["BAD_REQUEST", "task cannot have remarks"]);
        expect(
            await refusal(call(remark, "create", { ...host(article, missing), text: "Nope" })),
        ).toEqual(["NOT_FOUND", `no article ${missing}`]);
        as("bob");
        expect(
            await refusal(call(remark, "create", { ...host(article, draft.id), text: "Nope" })),
        ).toEqual(["NOT_FOUND", `no article ${draft.id}`]);
        as("alice");

        // include each article's remarks, and only its own, sharing identifiers with no photo
        const pages = store.source.relayed(
            store.source.queriesShape.subscription({
                name: "objects",
                scope: spaceId,
                below: spaceId,
                parameters: {
                    queries: {
                        articles: {
                            object: "article",
                            with: { remarks: {} },
                        },
                    },
                },
            }),
            context(),
        );
        const read = await pages.next();
        if (read.done === true) {
            throw new TypeError("the sync stream ended before its first page");
        }
        expect(
            read.value.changes
                .filter((change) => change.table === "destack__object__remark")
                .map((change) => schema.string().parse(change.row["text"]))
                .toSorted((left, right) => left.localeCompare(right)),
        ).toEqual(["Cite the source", "Tighten the intro"]);

        // delete the article's remarks with it, and keep the pinned photo with its remark
        const pinned = await call(pin, "create", host(photo, snapshot.id));
        await call(article, "delete", { id: draft.id });
        expect(await refusal(call(photo, "delete", { id: snapshot.id }))).toEqual([
            "BROKEN_REFERENCE",
            "the change would leave a reference to a missing record",
        ]);
        const remaining = await storage.database
            .select({ id: remark.table.id })
            .from(remark.table)
            .orderBy(asc(remark.table.id));
        expect(remaining.map((row) => row.id)).toEqual([caption.id]);
        expect(await counts()).toEqual([1]);

        // delete the photo with its remark once unpinned
        await call(pin, "delete", { id: pinned.id });
        await call(photo, "delete", { id: snapshot.id });
        expect([
            await storage.database
                .select()
                .from(photo.table)
                .where(eq(photo.table.id, schema.identifier("photo").parse(snapshot.id))),
            await storage.database.select().from(remark.table),
        ]).toEqual([[], []]);
    },
);

test.for(TEST_DIALECTS)(
    "keep each reader's favourite of a shared board private and single on %s",
    async (dialect) => {
        const { call, host, as } = await serveObjects(dialect, { favourite, board });

        // let the owner and a viewer each mark the board, once
        const plans = await call(board, "create", {});
        const shared = await call(board, "grant", {
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("universe", "bob"),
        });
        const mine = await call(favourite, "create", host(board, plans.id));
        expect(await refusal(call(favourite, "create", host(board, plans.id)))).toEqual([
            "CONFLICT",
            "another favourite has the same parent and reader",
        ]);
        as("bob");
        const theirs = await call(favourite, "create", host(board, plans.id));

        // show each reader only their own mark, and refuse one to a reader of nothing
        const listed = async () => (await call(favourite, "list", {})).items.map((item) => item.id);
        expect(await listed()).toEqual([theirs.id]);
        as("alice");
        expect(await listed()).toEqual([mine.id]);
        as("carol");
        expect(await refusal(call(favourite, "create", host(board, plans.id)))).toEqual([
            "NOT_FOUND",
            `no board ${plans.id}`,
        ]);

        // hide a mark from a reader who lost the board, and delete the marks with the board
        as("alice");
        await call(board, "revoke", { id: plans.id, relationshipId: shared.id });
        as("bob");
        expect(await listed()).toEqual([]);
        as("alice");
        await call(board, "delete", { id: plans.id });
        expect(await listed()).toEqual([]);
    },
);

test.for(TEST_DIALECTS)(
    "authorize a scoped method of attachments on the parent its input refers to, on %s",
    async (dialect) => {
        const counted = remark.implement({
            methods: {
                count: async (call) => ({
                    count: (
                        await call
                            .read(remark)
                            .findMany({ where: { parentId: call.input.parent.id } })
                    ).length,
                }),
            },
        });
        const { call, host, as } = await serveObjects(dialect, { remark: counted, article });

        // count the remarks on alice's article as alice, and refuse bob, who reads no article
        const draft = await call(article, "create", {});
        await call(counted, "create", { ...host(article, draft.id), text: "Tighten the intro" });
        const mine = await call(counted, "count", host(article, draft.id));
        as("bob");
        const theirs = await refusal(call(counted, "count", host(article, draft.id)));
        expect([mine, theirs]).toEqual([{ count: 1 }, ["FORBIDDEN", "permission denied: read"]]);
    },
);

/** Serve object types over a new database to one user at a time. */
async function serveObjects(dialect: Dialect, objects: Readonly<Record<string, ObjectType>>) {
    // keep the objects and their requests in one database
    const storage = await TestDatabase.create(
        dialect,
        defineDatabase({
            name: "main",
            tables: [...callTables, ...Object.values(objects).flatMap((object) => object.tables)],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // decide every call as the current user
    const store = new ObjectStore({
        chain: async () => undefined,
        clock: Clock.system,
        principal: spacePrincipal(spaceId),
        objects: Object.values(objects),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database),
        origin: {
            package: present(Object.values(objects)[0], "a served object type").package,
        },
    });
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const sign = (caller: string) => userContext(caller, spaceId, { signal: controller.signal });
    let context = sign("alice");

    return {
        storage,
        store,
        /** Read the current caller's request context. */
        context: () => context,
        call: <Object extends ObjectType, Name extends CallableName<Object>>(
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
            ),
        host: (object: ObjectType, id: string) => ({
            parent: { packageId: object.policy.definition.packageId, type: object.name, id },
        }),
        as: (caller: string) => {
            context = sign(caller);
        },
    };
}
