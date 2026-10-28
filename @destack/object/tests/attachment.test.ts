import { testSimulation } from "../src/test/index.ts";
import { present, schema, type Identifier } from "@destack/schema";
import { expect, refusal, onTestFinished, test } from "@destack/test";
import { InstallationId, intersection, principal, relation, through, union } from "@destack/access";
import { asc, eq, defineDatabase, Table } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { Package, PackageId } from "@destack/package";

import { ObjectReference, RequestId, Scope } from "@destack/sync";
import {
    defineObject,
    field,
    installation,
    ParentReference,
    type CallableName,
    type ObjectType,
} from "../src/index.ts";
import { user } from "./schema.ts";
import { openSpace, space, spacePrincipal } from "./fixture/space.ts";
import { userContext, users } from "./fixture/user.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents, serversOf } from "../src/test/index.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

/** The space containing the objects. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000003");

/** A package other than the one declaring the remarks. */
const OTHER_PACKAGE = PackageId.parse("package-019f5530-8000-7000-8000-0000000000b7");

/** The package of tags, which posters let its installations attach. */
const TAGS = Package.parse({
    id: "package-019f5530-8000-7000-8000-0000000000b8",
    name: "@example/tags",
    version: "2026.10.0",
});

/** Tags on any object that takes them, readable by whoever reads it. */
const tag = defineObject(
    {
        name: "tag",
        plural: "tags",
        scope: space,
        nested: { in: "any", receive: "tag" },
        fields: { text: field.string(schema.string().min(1)) },
        permissions: { read: through("parent", "read"), write: through("parent", "tag") },
    },
    (method) => ({ create: method.create("write") }),
    { package: TAGS },
);

/** Posters their owner reads, taking tags from readers and the tags package's installations. */
const poster = defineObject(
    {
        name: "poster",
        plural: "posters",
        scope: space,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner") },
        attachments: [tag.attach({ by: "read" })],
    },
    (method) => ({ create: method.create("read") }),
);

/** Remarks on any object that takes them, readable by whoever reads it. */
const remark = defineObject(
    {
        name: "remark",
        plural: "remarks",
        scope: space,
        nested: { in: "any", receive: "remark" },
        fields: { text: field.string(schema.string().min(1)) },
        permissions: { read: through("parent", "read"), write: through("parent", "remark") },
        aggregates: { remarkCount: { function: "count" } },
    },
    (method) => ({
        create: method.create("write"),
        delete: method.delete("write"),
        /** Count the remarks on a parent, checked as on a new remark there. */
        count: method.query({
            permission: "read",
            target: "scope",
            input: schema.object({ parent: ParentReference }),
            output: schema.object({ count: schema.int() }),
        }),
    }),
);

/** Markers that protect any hosting object from deletion. */
const marker = defineObject(
    {
        name: "marker",
        plural: "markers",
        scope: space,
        nested: { in: "any", delete: "restrict", receive: "mark" },
        fields: {},
        permissions: { read: through("parent", "read"), write: through("parent", "mark") },
    },
    (method) => ({ create: method.create("write"), delete: method.delete("write") }),
);

/** Articles taking remarks. */
const article = defineObject(
    {
        name: "article",
        plural: "articles",
        scope: space,
        fields: { owner: field.reference(principal.user).caller(), remarkCount: field.count() },
        permissions: { read: relation("owner"), edit: relation("owner") },
        attachments: [remark.attach({ by: "edit" })],
    },
    (method) => ({
        create: method.create("edit"),
        delete: method.delete("edit"),
    }),
);

/** Photos their owner remarks on and marks by reading them. */
const photo = defineObject(
    {
        name: "photo",
        plural: "photos",
        scope: space,
        fields: { owner: field.reference(principal.user).caller(), remarkCount: field.count() },
        permissions: { read: relation("owner") },
        attachments: [remark.attach({ by: "read" }), marker.attach({ by: "read" })],
    },
    (method) => ({
        create: method.create("read"),
        delete: method.delete("read"),
    }),
);

/** Tasks taking no remarks. */
const task = defineObject(
    {
        name: "task",
        plural: "tasks",
        scope: space,
        fields: { owner: field.reference(principal.user).caller() },
        permissions: { read: relation("owner") },
    },
    (method) => ({ create: method.create("read") }),
);

/** A reader's private favourite mark, one per object. */
const favourite = defineObject(
    {
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
            reader: {
                kind: "unique",
                on: ["parentPackageId", "parentType", "parentId", "readerId"],
            },
        },
    },
    (method) => ({
        create: method.create("write"),
        delete: method.delete("write"),
    }),
);

/** Boards shared with viewers who may mark them as a favourite. */
const board = defineObject(
    {
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
    },
    (method) => ({ create: method.create("manage"), delete: method.delete("manage") }),
);

test("map the types attached to served objects, which the database has beside them", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // serve photos alone over a database with their remarks and markers
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { photo, remark, marker }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf([photo]),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: { package: photo.package },
    });

    // map the remarks and markers whose permissions read through their photo
    expect(
        [remark, marker].map(
            (type) => store.authorizer.mapping(type.typeReference)?.table === type.table,
        ),
    ).toEqual([true, true]);
});

test("attach remarks to articles and photos, counting them and deleting them with their host", async () => {
    const { storage, store, context, call, host, as } = await serveObjects({
        remark,
        marker,
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
    const missing = `article-01996ab0-0000-7000-8000-00000000a007`;
    expect(
        await refusal(call(remark, "create", { ...host(task, task1.id), text: "Nope" })),
    ).toEqual(["BAD_REQUEST", "PARENT_TYPE_REFUSED", "task cannot have remarks"]);
    expect(
        await refusal(call(remark, "create", { ...host(article, missing), text: "Nope" })),
    ).toEqual(["NOT_FOUND", "NOT_FOUND", `article ${missing} not found`]);
    as(users.bob);
    expect(
        await refusal(call(remark, "create", { ...host(article, draft.id), text: "Nope" })),
    ).toEqual(["NOT_FOUND", "NOT_FOUND", `article ${draft.id} not found`]);
    as(users.alice);

    // include each article's remarks, and only those, sharing identifiers with no photo
    const pages = store.source.relayed(
        {
            ...store.source.queriesPublication.subscription({
                name: "objects",
                scope: spaceId,
                target: spaceId,
                parameters: {
                    queries: {
                        articles: {
                            object: "article",
                            with: { remarks: {} },
                        },
                    },
                },
            }),
            revision: { log: "follower", epoch: "follower", sequence: 1 },
        },
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

    // delete the article's remarks with it, leaving the marked photo with its remark
    const marked = await call(marker, "create", host(photo, snapshot.id));
    await call(article, "delete", { id: draft.id });
    expect(await refusal(call(photo, "delete", { id: snapshot.id }))).toEqual([
        "CONFLICT",
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
    await call(marker, "delete", { id: marked.id });
    await call(photo, "delete", { id: snapshot.id });
    expect([
        await storage.database
            .select()
            .from(photo.table)
            .where(eq(photo.table.id, schema.identifier("photo").parse(snapshot.id))),
        await storage.database.select().from(remark.table),
    ]).toEqual([[], []]);
});

test("let the attaching package's installation attach tags to a poster it cannot read, and refuse another package's", async () => {
    const { storage, environment, store, call, host } = await serveObjects({ poster, tag });

    // write alice's poster, and an installation of another package beside the tags' own
    const drawn = await call(poster, "create", {});
    const scope = space.reference(Scope.universe.id, spaceId);
    const own = await InstallationId.base(scope, TAGS.id, environment.crypto);
    const other = await InstallationId.base(scope, OTHER_PACKAGE, environment.crypto);
    const at = environment.clock.now();
    const row = Table.complete(installation.table, {
        id: other,
        scope: spaceId,
        origin: "base",
        packageId: OTHER_PACKAGE,
        createdAt: at,
        updatedAt: at,
        ...ObjectReference.columns("createdBy", spacePrincipal(spaceId)),
        ...ObjectReference.columns("updatedBy", spacePrincipal(spaceId)),
    });
    await storage.database.transaction(
        async (transaction) => await transaction.insert(installation.table).values(row),
    );

    // tag as each installation, which reads no poster
    const tagging = (id: Identifier<"installation">) =>
        TestObjectStore.serve(store)
            .as(principal.installation.reference(spaceId, id))
            .call(tag.call("create", { spaceId, ...host(poster, drawn.id), text: "Featured" }));
    const tagged = await tagging(own);
    expect([tagged.text, await refusal(tagging(other))]).toEqual([
        "Featured",
        ["NOT_FOUND", "NOT_FOUND", `poster ${drawn.id} not found`],
    ]);
});

test("store each reader's favourite of a shared board private and single", async () => {
    const { call, host, as } = await serveObjects({ favourite, board });

    // let the owner and a viewer each mark the board, once
    const plans = await call(board, "create", {});
    const shared = await call(board, "grant", {
        id: plans.id,
        relation: "viewer",
        subject: principal.user.reference("universe", users.bob),
    });
    const mine = await call(favourite, "create", host(board, plans.id));
    expect(await refusal(call(favourite, "create", host(board, plans.id)))).toEqual([
        "CONFLICT",
        "DUPLICATE",
        "another favourite has the same parent and reader",
    ]);
    as(users.bob);
    const theirs = await call(favourite, "create", host(board, plans.id));

    // show each reader only their mark, and refuse one to a reader of nothing
    const listed = async () => (await call(favourite, "list", {})).items.map((item) => item.id);
    expect(await listed()).toEqual([theirs.id]);
    as(users.alice);
    expect(await listed()).toEqual([mine.id]);
    as(users.carol);
    expect(await refusal(call(favourite, "create", host(board, plans.id)))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `board ${plans.id} not found`,
    ]);

    // hide a mark from a reader who lost the board, and delete the marks with the board
    as(users.alice);
    await call(board, "revoke", { id: plans.id, relationshipId: shared.id });
    as(users.bob);
    expect(await listed()).toEqual([]);
    as(users.alice);
    await call(board, "delete", { id: plans.id });
    expect(await listed()).toEqual([]);
});

test("authorize a scoped method of attachments on the parent its input refers to", async () => {
    const counted = remark.implement({
        methods: {
            count: async (call) => ({
                count: (
                    await call.read(remark).findMany({ where: { parentId: call.input.parent.id } })
                ).length,
            }),
        },
    });
    const { call, host, as } = await serveObjects({ remark: counted, article });

    // count the remarks on alice's article as alice, and refuse bob, who reads no article
    const draft = await call(article, "create", {});
    await call(counted, "create", { ...host(article, draft.id), text: "Tighten the intro" });
    const mine = await call(counted, "count", host(article, draft.id));
    as(users.bob);
    const theirs = await refusal(call(counted, "count", host(article, draft.id)));
    expect([mine, theirs]).toEqual([
        { count: 1 },
        ["FORBIDDEN", "PERMISSION_DENIED", "permission denied: read"],
    ]);
});

/** Serve object types over a new database to one user at a time. */
async function serveObjects(objects: Readonly<Record<string, ObjectType>>) {
    // run in a world of its own
    const world = testSimulation().environment("test");

    // store the objects and their requests in one database
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: objects, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);

    // decide every call as the current user
    const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
        chain: async () => undefined,
        environment: world,
        principal: spacePrincipal(spaceId),
        servers: serversOf(Object.values(objects)),
        database: storage.database,
        callKey: testCallKey,
        events: await testEvents(storage.database, world),
        origin: {
            package: present(Object.values(objects)[0], "a served object type").package,
        },
    });
    const controller = new AbortController();
    onTestFinished(() => controller.abort());
    const sign = (caller: Identifier<"user">) =>
        userContext(caller, spaceId, { environment: world, signal: controller.signal });
    let context = sign(users.alice);

    return {
        environment: world,
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
                { requestId: RequestId.create(world), ...input },
                context,
            ),
        host: (object: ObjectType, id: string) => ({
            parent: { packageId: object.policy.definition.packageId, type: object.name, id },
        }),
        as: (caller: Identifier<"user">) => {
            context = sign(caller);
        },
    };
}
