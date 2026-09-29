import { schema } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { intersection, principal, relation, through, union } from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { AuditOutbox, auditOutboxTables } from "@destack/audit/outbox";
import { asc, eq, unique, type DatabaseConnection, type Dialect } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { v7 } from "uuid";
import type { QueryPage } from "@destack/sync";
import { Bookmark } from "@destack/service/bookmark";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import { defineObject, field, method, type ObjectType } from "../src/index.ts";
import { ObjectServer } from "../src/server/index.ts";
import { request, user } from "./schema.ts";
import { openSpace, space } from "./fixture/space.ts";

/** The space containing the objects. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** Remarks on any object that takes them, readable by whoever reads it. */
const remark = defineObject({
    name: "remark",
    plural: "remarks",
    scope: space,
    nested: { in: "any", receive: "remark" },
    fields: { text: field.string(schema.string().min(1)) },
    permissions: { read: through("parent", "read"), write: through("parent", "remark") },
    aggregates: { remarkCount: { function: "count" } },
    methods: {
        list: method.list("read"),
        create: method.create("write"),
        delete: method.delete("write"),
    },
});

/** Pins that keep any hosting object from deletion. */
const pin = defineObject({
    name: "pin",
    plural: "pins",
    scope: space,
    nested: { in: "any", delete: "restrict", receive: "pin" },
    fields: {},
    permissions: { read: through("parent", "read"), write: through("parent", "pin") },
    methods: { create: method.create("write"), delete: method.delete("write") },
});

/** Articles taking remarks. */
const article = defineObject({
    name: "article",
    plural: "articles",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), remarkCount: field.count() },
    permissions: { read: relation("owner"), edit: relation("owner") },
    attachments: [remark.attach({ by: "edit" })],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("edit"),
        delete: method.delete("edit"),
    },
});

/** Photos their owner remarks on and pins by reading them. */
const photo = defineObject({
    name: "photo",
    plural: "photos",
    scope: space,
    fields: { owner: field.reference(principal.user).caller(), remarkCount: field.count() },
    permissions: { read: relation("owner") },
    attachments: [remark.attach({ by: "read" }), pin.attach({ by: "read" })],
    methods: {
        get: method.get("read"),
        list: method.list("read"),
        create: method.create("read"),
        delete: method.delete("read"),
    },
});

/** Tasks taking no remarks. */
const task = defineObject({
    name: "task",
    plural: "tasks",
    scope: space,
    fields: { owner: field.reference(principal.user).caller() },
    permissions: { read: relation("owner") },
    methods: { create: method.create("read") },
});

/** A reader's private favourite mark, one per object. */
const favourite = defineObject({
    name: "favourite",
    plural: "favourites",
    scope: space,
    nested: { in: "any", receive: "favourite" },
    fields: { reader: field.reference(principal.user).caller() },
    constraints: (entry) => [
        unique("favourite_reader").on(
            entry.parentPackageId,
            entry.parentType,
            entry.parentId,
            entry.reader,
        ),
    ],
    permissions: {
        read: intersection(relation("reader"), through("parent", "favourite")),
        write: intersection(relation("reader"), through("parent", "favourite")),
    },
    methods: {
        list: method.list("read"),
        create: method.create("write"),
        delete: method.delete("write"),
    },
});

/** Boards their owner shares with viewers, each of whom may mark them as a favourite. */
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
    methods: { create: method.create("manage"), delete: method.delete("manage") },
});

test.for(TEST_DIALECTS)(
    "attach remarks to articles and photos, counting them and deleting them with their host, on %s",
    async (dialect) => {
        const { storage, server, context, call, host, as } = await serveObjects(dialect, {
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
        await expect(
            call(remark, "create", { ...host(task, task1.id), text: "Nope" }),
        ).rejects.toMatchObject({ code: "BAD_REQUEST", message: "task takes no remarks" });
        await expect(
            call(remark, "create", { ...host(article, missing), text: "Nope" }),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no article ${missing}` });
        as("bob");
        await expect(
            call(remark, "create", { ...host(article, draft.id), text: "Nope" }),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no article ${draft.id}` });
        as("alice");

        // include each article's remarks, and only its own, sharing identifiers with no photo
        const pages = server.sync(spaceId, context, {
            queries: {
                articles: {
                    object: "article",
                    include: { remarks: {} },
                },
            },
        });
        const page = (await pages.next()).value as QueryPage;
        expect(
            page.changes
                .filter((change) => change.table === "destack__object__remark")
                .map((change) => change.row.text as string)
                .toSorted((left, right) => left.localeCompare(right)),
        ).toEqual(["Cite the source", "Tighten the intro"]);

        // delete the article's remarks with it, and keep the pinned photo with its remark
        const pinned = await call(pin, "create", host(photo, snapshot.id));
        await call(article, "delete", { id: draft.id });
        await expect(call(photo, "delete", { id: snapshot.id })).rejects.toMatchObject({
            code: "BROKEN_REFERENCE",
            message: "the change would leave a reference to a missing record",
        });
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
                .where(eq(photo.table.id, identifier("photo").parse(snapshot.id))),
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
            subject: principal.user.reference("global", "bob"),
        });
        const mine = await call(favourite, "create", host(board, plans.id));
        await expect(call(favourite, "create", host(board, plans.id))).rejects.toMatchObject({
            code: "DUPLICATE",
            message: "a record with the same unique key exists",
        });
        as("bob");
        const theirs = await call(favourite, "create", host(board, plans.id));

        // show each reader only their own mark, and refuse one to a reader of nothing
        const listed = async () =>
            (
                (await call(favourite, "list", {})) as unknown as { items: { id: string }[] }
            ).items.map((item) => item.id);
        expect(await listed()).toEqual([theirs.id]);
        as("alice");
        expect(await listed()).toEqual([mine.id]);
        as("carol");
        await expect(call(favourite, "create", host(board, plans.id))).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no board ${plans.id}`,
        });

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

test("receive attachments by the host's own permission of their name, or derive it from the attaching one", () => {
    // keep the host's own remark permission when it attaches remarks by it
    const own = relation("owner");
    const notebook = defineObject({
        name: "notebook",
        plural: "notebooks",
        scope: space,
        fields: { owner: field.reference(principal.user).caller(), remarkCount: field.count() },
        permissions: { read: own, remark: own },
        methods: { get: method.get("read") },
        attachments: [remark.attach({ by: "remark" })],
    });
    expect(notebook.policy.definition.permissions.remark).toEqual(own);

    // refuse attaching by a permission other than the declared receiving one
    expect(() =>
        defineObject({
            name: "journal",
            plural: "journals",
            scope: space,
            fields: { owner: field.reference(principal.user).caller(), remarkCount: field.count() },
            permissions: { read: own, remark: own },
            methods: { get: method.get("read") },
            attachments: [remark.attach({ by: "read" })],
        }),
    ).toThrow(
        new TypeError(
            "object journal declares permission remark, so it attaches remarks by remark",
        ),
    );
});

test("refuse attaching by a permission the host lacks, and aggregates the host does not hold", () => {
    // refuse a missing attaching permission
    expect(() =>
        defineObject({
            name: "page",
            plural: "pages",
            scope: space,
            fields: { remarkCount: field.count() },
            permissions: { read: relation("owner") },
            attachments: [remark.attach({ by: "edit" })],
        }),
    ).toThrow(new TypeError("object page attaches remarks by missing permission edit"));

    // refuse a host without the attachment's count
    expect(() =>
        defineObject({
            name: "sheet",
            plural: "sheets",
            scope: space,
            fields: { owner: field.reference(principal.user).caller() },
            permissions: { read: relation("owner") },
            methods: { get: method.get("read") },
            attachments: [remark.attach({ by: "read" })],
        }),
    ).toThrow(new TypeError("aggregate remarkCount of remark fills no count field of sheet"));
});

/** Serve object types over a new database to one user at a time. */
async function serveObjects(dialect: Dialect, objects: Readonly<Record<string, ObjectType>>) {
    // hold the objects and their requests in one database
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

    // decide every call as the current user
    let current = "alice";
    const server = new ObjectServer({
        objects,
        database: storage.database,
        context: () => ({
            subjects: [principal.user.reference("global", current)],
            now: Date.now(),
            attributes: {},
        }),
        journal: new Journal(request),
        audit: AuditRecorder.service(new AuditOutbox(storage.database), {
            package: Object.values(objects)[0]!.package,
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
        server,
        context,
        call: async (object: ObjectType, name: string, input: object) =>
            (await server.call(
                object,
                name,
                { spaceId, requestId: RequestId.create(), ...input },
                context,
            )) as { id: string },
        host: (object: ObjectType, id: string) => ({
            parent: { packageId: object.policy.definition.packageId, type: object.name, id },
        }),
        as: (user: string) => {
            current = user;
        },
    };
}
