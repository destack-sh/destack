import { ObjectServer } from "../src/server/index.ts";
import { expect, onTestFinished, test } from "@destack/test";
import { isNull, type Dialect } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { ObjectClient } from "../src/client/index.ts";
import { PackageId } from "@destack/package";
import { ResourceContext } from "@destack/resource/context";
import { present, schema } from "@destack/schema";
import { Authentication } from "@destack/service/authentication";
import { createClient, type ClientOptions } from "@destack/service/client";
import { Health } from "@destack/service/health";
import { RequestId } from "@destack/service/request";
import { Server } from "@destack/service/server";
import { v7 } from "uuid";
import { anyone, LinkSecret, principal } from "@destack/access";
import { Subject } from "@destack/sync";
import { page, pageDatabase, pagesService } from "./fixture/page.ts";
import { openSpace, unmoved } from "./fixture/space.ts";
import { testCallKey } from "@destack/service/test";

/** The space with the pages. */
const spaceId = schema.identifier("space").parse(`space-${v7()}`);

/** The package serving the pages. */
const audience = PackageId.parse("package-01a0d5eb-fbad-732f-bbb4-a58aebbeb908");

/** The rows an include adds, read for their titles. */
const TITLED_ROWS = schema.array(schema.looseObject({ title: schema.string() }));

/** Serve a space's pages to bearer-named users and anonymous readers. */
async function servePages(dialect: Dialect) {
    const storage = await TestDatabase.create(dialect, pageDatabase, { isMigrated: true });
    onTestFinished(() => storage.close());
    const database = storage.database;
    await openSpace(database, spaceId);

    // serve the space's pages
    const server = Server.start({
        ...ObjectServer.serve(pagesService, {
            callKey: testCallKey,
            database,
        }),
        audience,
        scope: spaceId,
        resources: new ResourceContext(),
        health: new Health("pages"),
        drainTimeout: 1000,
        authorizeHost: async () => {},
        authenticate: async (request) => {
            const bearer = request.headers.get("authorization");
            if (bearer === null) {
                return null;
            }
            const id = bearer.slice("Bearer ".length);
            const subject = principal.user.reference("universe", id);
            const now = Date.now();

            return new Authentication({
                subject,
                subjects: [subject],
                credential: { kind: "user", id },
                audience,
                scope: spaceId,
                verifiedAt: now,
                expiresAt: now + 60_000,
            });
        },
    });
    onTestFinished(() => server.close());

    // call the server with some headers
    const endpoint = (headers: Record<string, string>): ClientOptions => ({
        url: "https://page.test",
        headers,
        fetch: (request: Request) => server.fetch(request),
    });
    const connect = (headers: Record<string, string>) =>
        createClient(pagesService, endpoint(headers));

    return { connect, endpoint };
}

/** Keep a user's pages on a device. */
async function openDevice(user: string, endpoint: ClientOptions) {
    // create the local database and keep the space's pages in it
    const storage = await TestDatabase.create("sqlite", ObjectClient.tables({ page }), {
        storage: "file",
    });
    const client = await ObjectClient.open({
        database: storage.database,
        objects: { page },
        scope: spaceId,
        caller: principal.user.reference("universe", user),
        endpoint,
        reconnect: unmoved,
    });

    // keep every page of the space, pushing and following until the test finishes
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
            .where(isNull(page.table.deletionRequestedAt));

        return rows
            .filter((row) => row.parentId === parentId)
            .map((row) => row.title)
            .toSorted();
    };

    return { client, errors, titles };
}

test.each(TEST_DIALECTS)(
    "share page trees with members and links, and move pages within them on %s",
    async (dialect) => {
        const { connect } = await servePages(dialect);
        const alice = connect({ authorization: "Bearer alice" });
        const bob = connect({ authorization: "Bearer bob" });
        const carol = connect({ authorization: "Bearer carol" });

        // build a tree of pages private to its owner
        const create = (title: string, parentId?: string) =>
            alice.page.create({
                spaceId,
                requestId: RequestId.create(),
                title,
                ...(parentId === undefined
                    ? {}
                    : { parentId: schema.identifier("page").parse(parentId) }),
            });
        const handbook = await create("Handbook");
        const onboarding = await create("Onboarding", handbook.id);
        const week = await create("First week", onboarding.id);
        await expect(bob.page.get({ spaceId, id: week.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no page ${week.id}`,
        });
        await expect(
            bob.page.create({
                spaceId,
                requestId: RequestId.create(),
                parentId: week.id,
                title: "Spam",
            }),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no page ${week.id}` });

        // share the tree's root: editors edit every subpage but may not delete or move them
        await alice.page.grant({
            spaceId,
            id: handbook.id,
            requestId: RequestId.create(),
            relation: "editor",
            subject: principal.user.reference("universe", "bob"),
        });
        expect((await bob.page.get({ spaceId, id: week.id })).title).toBe("First week");
        const tools = await bob.page.create({
            spaceId,
            requestId: RequestId.create(),
            parentId: onboarding.id,
            title: "Tools",
        });
        expect(Subject.read(tools.owner)).toEqual(principal.user.reference("universe", "bob"));
        await expect(
            bob.page.move({
                spaceId,
                id: week.id,
                requestId: RequestId.create(),
                revision: week.revision,
                parentId: null,
            }),
        ).rejects.toMatchObject({ code: "FORBIDDEN", message: "permission denied: manage" });

        // move a subtree out of the shared tree, and refuse moving a page into its own subtree
        await alice.page.move({
            spaceId,
            id: week.id,
            requestId: RequestId.create(),
            revision: week.revision,
            parentId: null,
        });
        await expect(bob.page.get({ spaceId, id: week.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no page ${week.id}`,
        });
        await expect(
            alice.page.move({
                spaceId,
                id: handbook.id,
                requestId: RequestId.create(),
                revision: handbook.revision,
                parentId: onboarding.id,
            }),
        ).rejects.toMatchObject({
            code: "CONFLICT",
            message: "page cannot move into its own subtree",
        });

        // publish the tree through a link anyone presenting its secret may read, signed in or not
        const link = await LinkSecret.create();
        await alice.page.grant({
            spaceId,
            id: handbook.id,
            requestId: RequestId.create(),
            relation: "public",
            subject: anyone.reference("*", "*"),
            conditions: { linkSecret: link.digest },
        });
        const visitor = connect({ "destack-link-secret": link.secret });
        expect((await visitor.page.get({ spaceId, id: tools.id })).title).toBe("Tools");
        expect(
            (await visitor.page.list({ spaceId })).items.map((item) => item.title).toSorted(),
        ).toEqual(["Handbook", "Onboarding", "Tools"]);
        await expect(connect({}).page.get({ spaceId, id: handbook.id })).rejects.toMatchObject({
            code: "NOT_FOUND",
            message: `no page ${handbook.id}`,
        });
        await expect(
            visitor.page.update({
                spaceId,
                id: tools.id,
                requestId: RequestId.create(),
                revision: tools.revision,
                title: "Defaced",
            }),
        ).rejects.toMatchObject({ code: "UNAUTHORIZED", message: "missing caller credential" });

        // explain to the owner why a reader may not edit
        const why = await alice.page.explain({
            spaceId,
            id: onboarding.id,
            permission: "edit",
            subject: principal.user.reference("universe", "carol"),
        });
        expect([
            why.isAllowed,
            present(why.authorities[0], "the first authority").grants.map((grant) => [
                grant.path.join(" / "),
                grant.failure,
            ]),
        ]).toEqual([
            false,
            [
                ["page edit / page edit-direct / field owner holding relation owner", "subject"],
                [
                    `page edit / through parent to page ${handbook.id} / page edit-direct / field owner holding relation owner`,
                    "subject",
                ],
                [
                    `page edit / through parent to page ${handbook.id} / page edit-direct / relation editor`,
                    "subject",
                ],
            ],
        ]);

        // let a reader ask for editing, and edit once the owner accepts
        const request = await carol.page.propose({
            spaceId,
            id: handbook.id,
            requestId: RequestId.create(),
            relationship: {
                relation: "editor",
                subject: principal.user.reference("universe", "carol"),
            },
        });
        await alice.page.accept({
            spaceId,
            id: handbook.id,
            requestId: RequestId.create(),
            proposalId: request.id,
        });
        const renamed = await carol.page.update({
            spaceId,
            id: onboarding.id,
            requestId: RequestId.create(),
            revision: onboarding.revision,
            title: "Welcome",
        });
        expect(renamed.title).toBe("Welcome");
        expect(
            (
                await alice.page.explain({
                    spaceId,
                    id: onboarding.id,
                    permission: "edit",
                    subject: principal.user.reference("universe", "carol"),
                })
            ).isAllowed,
        ).toBe(true);
    },
);

test.each(TEST_DIALECTS)(
    "build page trees offline in one mutation, and move and trash pages predictively on %s",
    async (dialect) => {
        const { connect, endpoint } = await servePages(dialect);
        const alice = connect({ authorization: "Bearer alice" });
        const device = await openDevice("alice", endpoint({ authorization: "Bearer alice" }));

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
        expect(
            (await alice.page.list({ spaceId })).items.map((item) => item.title).toSorted(),
        ).toEqual(["First week", "Handbook", "Onboarding"]);

        // move a page to the root, and refuse a move into its own subtree before queueing it
        const moved = device.client.mutate(page).move({ id: week.id, parentId: null });
        await moved.predicted;
        expect(await device.titles()).toEqual(["First week", "Handbook"]);
        await moved.confirmed;
        const cycle = device.client.mutate(page).move({ id: handbook.id, parentId: onboarding.id });
        await expect(cycle.predicted).rejects.toMatchObject({
            code: "CONFLICT",
            message: "page cannot move into its own subtree",
        });
        await expect(cycle.confirmed).rejects.toMatchObject({
            code: "CONFLICT",
            message: "page cannot move into its own subtree",
        });
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
    },
);

test.each(TEST_DIALECTS)(
    "follow a shared tree on a device, dropping pages moved out of it and taking them back on %s",
    async (dialect) => {
        const { connect, endpoint } = await servePages(dialect);
        const alice = connect({ authorization: "Bearer alice" });

        // share a tree of four pages with bob as a viewer
        const create = (title: string, parentId?: string) =>
            alice.page.create({
                spaceId,
                requestId: RequestId.create(),
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
            requestId: RequestId.create(),
            relation: "viewer",
            subject: principal.user.reference("universe", "bob"),
        });

        // keep the shared tree on bob's device
        const device = await openDevice("bob", endpoint({ authorization: "Bearer bob" }));
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
            requestId: RequestId.create(),
            parentId: null,
        });
        await expect.poll(titles).toEqual(["Handbook", "Onboarding"]);
        await alice.page.move({
            spaceId,
            id: week.id,
            requestId: RequestId.create(),
            parentId: onboarding.id,
        });
        await expect.poll(titles).toEqual(["Day one", "First week", "Handbook", "Onboarding"]);
        expect(device.errors).toEqual([]);
    },
);

test.for(TEST_DIALECTS)("list the pages below and above each page on %s", async (dialect) => {
    const { connect } = await servePages(dialect);
    const alice = connect({ authorization: "Bearer alice" });

    // nest a guide under a handbook, and a chapter under the guide
    const create = async (title: string, parentId?: string) =>
        alice.page.create({
            spaceId,
            requestId: RequestId.create(),
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

test.for(TEST_DIALECTS)(
    "follow the pages below a page on a device as the tree grows on %s",
    async (dialect) => {
        const { connect, endpoint } = await servePages(dialect);
        const alice = connect({ authorization: "Bearer alice" });
        const { client: device } = await openDevice(
            "alice",
            endpoint({ authorization: "Bearer alice" }),
        );
        const handbook = await alice.page.create({
            spaceId,
            requestId: RequestId.create(),
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
    },
);

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
