import { testSimulation } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import { implementPackage } from "../src/server/index.ts";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { isNull } from "@destack/db";
import { TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../src/client/index.ts";
import { Package } from "@destack/package";
import { present, schema, type Identifier } from "@destack/schema";
import { createClient, type ClientOptions } from "@destack/object/client";
import { RequestId, ObjectReference, Scope } from "@destack/sync";
import { anyone, LinkSecret, principal, Role } from "@destack/access";
import { page, pageDatabase, pagesPackage } from "./fixture/page.ts";
import { openSpace, space, unmoved, spacePrincipal } from "./fixture/space.ts";
import { testCallKey } from "@destack/object/test";
import { testEvents } from "../src/test/index.ts";
import { Environment } from "@destack/environment";
import { TestObjectStore } from "../src/test/index.ts";

/** The space with the pages. */
const spaceId = schema.identifier("space").parse(`space-01996ab0-0000-7000-8000-00000000a00b`);

/** The rows an include adds, read for their titles. */
const TITLED_ROWS = schema.array(schema.looseObject({ title: schema.string() }));

/** Serve a space's pages to bearer-identified users and anonymous readers. */
async function servePages(world: Environment) {
    const storage = await TestDatabase.create(pageDatabase, {
        environment: world,
        isMigrated: true,
    });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, world, spaceId);

    // serve the space's pages
    const served = TestObjectStore.serve(
        await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            servers: [implementPackage(pagesPackage, {})],
            chain: async () => undefined,
            environment: world,
            origin: { package: Package.of(pagesPackage) },
            principal: spacePrincipal(spaceId),
            callKey: testCallKey,
            events: await testEvents(database, world),
            database,
        }),
    );
    onTestFinished(() => served[Symbol.asyncDispose]());
    const server = served.server(pagesPackage);

    // call the server with some headers
    const endpoint = (headers: Record<string, string>): ClientOptions => ({
        url: "https://page.test",
        headers,
        fetch: (request: Request) => server.fetch(request),
    });
    const connect = (headers: Record<string, string>) =>
        createClient(pagesPackage, endpoint(headers), world);

    return { as: (user: Identifier<"user">) => served.as(user), connect, endpoint, database };
}

/** Replicate a user's pages on a device. */
async function openDevice(world: Environment, user: Identifier<"user">, endpoint: ClientOptions) {
    // create the local database and replicate the space's pages in it
    const storage = await TestDatabase.create(ObjectClient.tables({ page }), {
        environment: world,
        dialect: "sqlite",
        storage: "file",
    });
    const client = await ObjectClient.open({
        environment: world,
        database: storage.database,
        objects: { page },
        scope: spaceId,
        caller: principal.user.reference("universe", user),
        endpoint,
        reconnect: unmoved,
    });

    // replicate every page of the space, pushing and following until the test finishes
    client.query.page.findMany({ deleted: "include" }).subscribe();
    const controller = new AbortController();
    const errors: unknown[] = [];
    const loops = [
        client.push(controller.signal, (error) => errors.push(error)),
        client.follow(controller.signal, (error) => errors.push(error)),
    ];
    onTestFinished(async () => {
        controller.abort();
        await Promise.all(loops);
        await storage.close();
    });

    // read the titles of the pages outside the trash under a parent, the roots by default
    const titles = async (parentId: string | null = null) => {
        const rows = await client.database
            .select({ title: page.table.title, parentId: page.table.parentId })
            .from(page.table)
            .where(isNull(page.table.deletedAt));

        return rows
            .filter((row) => row.parentId === parentId)
            .map((row) => row.title)
            .toSorted();
    };

    return { client, errors, titles };
}

test("share page trees with members and links, and move pages within them", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { as, connect } = await servePages(world);
    const alice = as(users.alice).client(pagesPackage);
    const bob = as(users.bob).client(pagesPackage);
    const carol = as(users.carol).client(pagesPackage);

    // build a tree of pages private to its owner
    const create = (title: string, parentId?: string) =>
        alice.page.create({
            spaceId,
            requestId: RequestId.create(world),
            title,
            ...(parentId === undefined
                ? {}
                : { parentId: schema.identifier("page").parse(parentId) }),
        });
    const handbook = await create("Handbook");
    const onboarding = await create("Onboarding", handbook.id);
    const week = await create("First week", onboarding.id);
    expect(await refusal(bob.page.get({ spaceId, id: week.id }))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `page ${week.id} not found`,
    ]);
    expect(
        await refusal(
            bob.page.create({
                spaceId,
                requestId: RequestId.create(world),
                parentId: week.id,
                title: "Spam",
            }),
        ),
    ).toEqual(["NOT_FOUND", "NOT_FOUND", `page ${week.id} not found`]);

    // share the tree's root: editors edit every subpage but may not delete or move them
    await alice.page.grant({
        spaceId,
        id: handbook.id,
        requestId: RequestId.create(world),
        relation: "editor",
        subject: principal.user.reference("universe", users.bob),
    });
    expect((await bob.page.get({ spaceId, id: week.id })).title).toBe("First week");
    const tools = await bob.page.create({
        spaceId,
        requestId: RequestId.create(world),
        parentId: onboarding.id,
        title: "Tools",
    });
    expect(ObjectReference.at("owner", tools)).toEqual(
        principal.user.reference("universe", users.bob),
    );
    expect(
        await refusal(
            bob.page.move({
                spaceId,
                id: week.id,
                requestId: RequestId.create(world),
                revision: week.revision,
                parentId: null,
            }),
        ),
    ).toEqual(["FORBIDDEN", "PERMISSION_DENIED", "permission denied: manage"]);

    // move a subtree out of the shared tree, and refuse moving a page into its subtree
    await alice.page.move({
        spaceId,
        id: week.id,
        requestId: RequestId.create(world),
        revision: week.revision,
        parentId: null,
    });
    expect(await refusal(bob.page.get({ spaceId, id: week.id }))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `page ${week.id} not found`,
    ]);
    expect(
        await refusal(
            alice.page.move({
                spaceId,
                id: handbook.id,
                requestId: RequestId.create(world),
                revision: handbook.revision,
                parentId: onboarding.id,
            }),
        ),
    ).toEqual(["CONFLICT", "MOVE_INTO_SUBTREE", "page cannot move into its subtree"]);

    // publish the tree through a link anyone presenting its secret may read, signed in or not
    const link = await LinkSecret.create(world);
    await alice.page.grant({
        spaceId,
        id: handbook.id,
        requestId: RequestId.create(world),
        relation: "public",
        subject: anyone.reference("*", "*"),
        linkSecret: link.digest,
    });
    const visitor = connect({ "Destack-Link-Secret": link.secret });
    expect((await visitor.page.get({ spaceId, id: tools.id })).title).toBe("Tools");
    expect(
        (await visitor.page.list({ spaceId })).items.map((item) => item.title).toSorted(),
    ).toEqual(["Handbook", "Onboarding", "Tools"]);
    expect(await refusal(connect({}).page.get({ spaceId, id: handbook.id }))).toEqual([
        "NOT_FOUND",
        "NOT_FOUND",
        `page ${handbook.id} not found`,
    ]);
    expect(
        await refusal(
            visitor.page.update({
                spaceId,
                id: tools.id,
                requestId: RequestId.create(world),
                revision: tools.revision,
                title: "Defaced",
            }),
        ),
    ).toEqual(["UNAUTHORIZED", "CREDENTIAL_MISSING", "missing caller credential"]);

    // explain to the owner why a reader may not write, beside the pages' installations
    const why = await alice.page.explain({
        spaceId,
        id: onboarding.id,
        permission: "write",
        subject: principal.user.reference("universe", users.carol),
    });
    const parent = `page write / through parent to page ${handbook.id} / page write-direct`;
    const installed = (prefix: string) => [
        [`${prefix} / set installation of package ${pagesPackage.id} in ${spaceId}`, "subject"],
        [`${prefix} / set installation of package ${pagesPackage.id} in universe`, "subject"],
    ];
    expect([
        why.isAllowed,
        why.tuples.map((tuple) => [tuple.path.join(" / "), tuple.failure]),
    ]).toEqual([
        false,
        [
            ["page write / page write-direct / field ownerId holding relation owner", "subject"],
            ...installed("page write / page write-direct"),
            [
                `page write / page write-direct / role ${Role.OWNER} bound on space ${spaceId}`,
                "subject",
            ],
            [`${parent} / field ownerId holding relation owner`, "subject"],
            [`${parent} / relation editor`, "subject"],
            ...installed(parent),
            [`${parent} / role ${Role.OWNER} bound on space ${spaceId}`, "subject"],
            ...installed("page write"),
            [`page write / role ${Role.OWNER} bound on space ${spaceId}`, "subject"],
        ],
    ]);

    // let a reader ask for editing, and edit once the owner accepts
    const request = await carol.page.invite({
        spaceId,
        id: handbook.id,
        requestId: RequestId.create(world),
        relationship: {
            relation: "editor",
            subject: principal.user.reference("universe", users.carol),
        },
    });
    await alice.page.accept({
        spaceId,
        id: handbook.id,
        requestId: RequestId.create(world),
        invitationId: request.id,
    });
    const renamed = await carol.page.update({
        spaceId,
        id: onboarding.id,
        requestId: RequestId.create(world),
        revision: onboarding.revision,
        title: "Welcome",
    });
    expect(renamed.title).toBe("Welcome");
    expect(
        (
            await alice.page.explain({
                spaceId,
                id: onboarding.id,
                permission: "write",
                subject: principal.user.reference("universe", users.carol),
            })
        ).isAllowed,
    ).toBe(true);
});

test("build page trees offline in one mutation, and move and trash pages predictively", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { as } = await servePages(world);
    const alice = as(users.alice).client(pagesPackage);
    const device = await openDevice(world, users.alice, as(users.alice).endpoint(pagesPackage));

    // create a tree of three pages in one mutation, shown before the server executes it
    const tree = device.client.mutation(async (mutation) => {
        const pages = mutation.call(page);
        const handbook = await pages.create({ title: "Handbook" });
        const onboarding = await pages.create({ title: "Onboarding", parentId: handbook.id });
        const week = await pages.create({ title: "First week", parentId: onboarding.id });

        return { handbook, onboarding, week };
    });
    const { handbook, onboarding, week } = await tree.predicted;
    expect([await device.titles(), await device.titles(onboarding.id)]).toEqual([
        ["Handbook"],
        ["First week"],
    ]);
    await tree.confirmed;
    expect((await alice.page.list({ spaceId })).items.map((item) => item.title).toSorted()).toEqual(
        ["First week", "Handbook", "Onboarding"],
    );

    // move a page to the root, and refuse a move into its subtree before queueing it
    const moved = device.client.mutate(page).move({ id: week.id, parentId: null });
    await moved.predicted;
    expect(await device.titles()).toEqual(["First week", "Handbook"]);
    await moved.confirmed;
    const cycle = device.client.mutate(page).move({ id: handbook.id, parentId: onboarding.id });
    expect(await refusal(cycle.predicted)).toEqual([
        "CONFLICT",
        "MOVE_INTO_SUBTREE",
        "page cannot move into its subtree",
    ]);
    expect(await refusal(cycle.confirmed)).toEqual([
        "CONFLICT",
        "MOVE_INTO_SUBTREE",
        "page cannot move into its subtree",
    ]);
    expect(await device.client.prediction.pending(device.client.database)).toEqual([]);

    // trash and restore a page, shown at once and confirmed by the server
    const trashed = device.client.mutate(page).delete({ id: week.id });
    await trashed.predicted;
    expect(await device.titles()).toEqual(["Handbook"]);
    await trashed.confirmed;
    expect(
        (await alice.page.list({ spaceId, deleted: "only" })).items.map((item) => item.title),
    ).toEqual(["First week"]);
    const restored = device.client.mutate(page).restore({ id: week.id });
    await restored.predicted;
    expect(await device.titles()).toEqual(["First week", "Handbook"]);
    await restored.confirmed;
    expect(device.errors).toEqual([]);
});

test("follow a shared tree on a device, dropping pages moved out of it and taking them back", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { as } = await servePages(world);
    const alice = as(users.alice).client(pagesPackage);

    // share a tree of four pages with bob as a viewer
    const create = (title: string, parentId?: string) =>
        alice.page.create({
            spaceId,
            requestId: RequestId.create(world),
            title,
            ...(parentId === undefined
                ? {}
                : { parentId: schema.identifier("page").parse(parentId) }),
        });
    const handbook = await create("Handbook");
    const onboarding = await create("Onboarding", handbook.id);
    const week = await create("First week", onboarding.id);
    await create("Day one", week.id);
    await alice.page.grant({
        spaceId,
        id: handbook.id,
        requestId: RequestId.create(world),
        relation: "viewer",
        subject: principal.user.reference("universe", users.bob),
    });

    // replicate the shared tree on bob's device
    const device = await openDevice(world, users.bob, as(users.bob).endpoint(pagesPackage));
    const titles = async () => {
        const rows = await device.client.database
            .select({ title: page.table.title })
            .from(page.table);

        return rows.map((row) => row.title).toSorted();
    };
    await expect.poll(titles).toEqual(["Day one", "First week", "Handbook", "Onboarding"]);

    // drop and regain a subtree when it moves out of and into the shared tree
    await alice.page.move({
        spaceId,
        id: week.id,
        requestId: RequestId.create(world),
        parentId: null,
    });
    await expect.poll(titles).toEqual(["Handbook", "Onboarding"]);
    await alice.page.move({
        spaceId,
        id: week.id,
        requestId: RequestId.create(world),
        parentId: onboarding.id,
    });
    await expect.poll(titles).toEqual(["Day one", "First week", "Handbook", "Onboarding"]);
    expect(device.errors).toEqual([]);
});

test("list the pages below and above each page", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { as } = await servePages(world);
    const alice = as(users.alice).client(pagesPackage);

    // nest a guide under a handbook, and a chapter under the guide
    const create = async (title: string, parentId?: string) =>
        alice.page.create({
            spaceId,
            requestId: RequestId.create(world),
            title,
            ...(parentId === undefined
                ? {}
                : { parentId: schema.identifier("page").parse(parentId) }),
        });
    const handbook = await create("Handbook");
    const guide = await create("Guide", handbook.id);
    const chapter = await create("Chapter", guide.id);

    // list the top page with every page below it, and the deepest with every page above it
    const tops = await alice.page.list({
        spaceId,
        where: { parentId: { isNull: true } },
        with: { descendants: { orderBy: { title: "asc" } as const } },
    });
    const deepest = await alice.page.list({
        spaceId,
        where: { title: "Chapter" },
        with: { ancestors: {} },
    });
    expect([
        tops.items.map((item) => item.title),
        includedTitles(tops.included, "descendants", handbook.id),
        includedTitles(deepest.included, "ancestors", chapter.id).toSorted(),
    ]).toEqual([["Handbook"], ["Chapter", "Guide"], ["Guide", "Handbook"]]);
});

test("follow the pages below a page on a device as the tree grows", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { as } = await servePages(world);
    const alice = as(users.alice).client(pagesPackage);
    const { client: device } = await openDevice(
        world,
        users.alice,
        as(users.alice).endpoint(pagesPackage),
    );
    const handbook = await alice.page.create({
        spaceId,
        requestId: RequestId.create(world),
        title: "Handbook",
    });

    // follow the top pages with every page below them, read once and live
    const tops = device.query.page
        .findMany({
            where: { parentId: { isNull: true } },
            with: { descendants: { orderBy: { title: "asc" } } },
        })
        .subscribe();
    await tops.ready;
    const controller = new AbortController();
    const watched = tops.watch(controller.signal);
    const next = async () => {
        const read = await watched.next();
        if (read.done === true) {
            throw new TypeError("the watched pages ended");
        }

        return shelf(read.value);
    };
    expect(await next()).toEqual([["Handbook", []]]);

    // grow the tree two levels, watching until both show
    const guide = await device
        .mutate(page)
        .create({ parentId: schema.identifier("page").parse(handbook.id), title: "Guide" })
        .predicted;
    await watched.next();
    await device.mutate(page).create({ parentId: guide.id, title: "Chapter" }).predicted;
    let shown = await next();
    while (shown[0]?.[1].length !== 2) {
        shown = await next();
    }
    expect([shown, shelf(await tops.read())]).toEqual([
        [["Handbook", ["Chapter", "Guide"]]],
        [["Handbook", ["Chapter", "Guide"]]],
    ]);
    controller.abort();
});

/** Read the titles of the rows a listed row includes under a name. */
function includedTitles(
    included: Readonly<Record<string, Readonly<Record<string, unknown>>>> | undefined,
    name: string,
    id: string,
): string[] {
    const rows = present(present(included, "the listed rows' includes")[name], `include ${name}`)[
        id
    ];

    return TITLED_ROWS.parse(rows).map((row) => row.title);
}

/** Read the top pages' titles, each with the titles of the pages below it. */
function shelf(
    rows: readonly {
        readonly title: string;
        readonly descendants: readonly { readonly title: string }[];
    }[],
): [string, string[]][] {
    return rows.map((row) => [row.title, row.descendants.map((below) => below.title)]);
}
