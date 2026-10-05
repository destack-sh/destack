import type { Page, TrackerMessage } from "@destack/sync";
import { present, schema } from "@destack/schema";
import { unique, type Dialect, defineDatabase, Expression } from "@destack/db";
import { expect, onTestFinished, test } from "@destack/test";
import { vi } from "vitest";
import { intersection, principal, relation, through, union } from "@destack/access";
import { journal } from "@destack/audit";
import { channelHub } from "@destack/db";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import * as sqlite from "@destack/db/bun";

import { subjectContext, testCallKey } from "@destack/service/test";
import { RequestId } from "@destack/service/request";
import { defineObject, field, type CallableName, type ObjectType } from "../src/index.ts";
import { EphemeralStorage, ObjectServer } from "../src/server/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import type { ClientOptions } from "@destack/service/client";
import { defineService } from "@destack/service/declare";
import { serveObjects } from "./fixture/device.ts";
import { user } from "./schema.ts";
import { openSpace, space, unmoved } from "./fixture/space.ts";

/** How long a check may take to pass, in milliseconds: well within the 5 s test timeout. */
const UNTIL_MILLISECONDS = 2000;

/** The space with the boards. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000009");

/** Who is on a board and where their cursor is. */
const presence = defineObject({
    name: "presence",
    plural: "presences",
    scope: space,
    storage: "ephemeral",
    linger: { milliseconds: 50 },
    nested: { in: "any", receive: "present" },
    fields: {
        member: field.reference(principal.user).caller(),
        activity: field.string(schema.string().min(1).max(64)),
        cursor: field.integer().optional(),
        profile: field.reference("profile", (): ObjectType => profile).optional(),
    },
    constraints: (entry) => [unique("presence_client").on(entry.parentId, entry.client)],
    permissions: {
        read: through("parent", "present"),
        write: intersection(relation("member"), through("parent", "present")),
    },
    methods: (method) => ({
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write"),
        delete: method.delete("write"),
        idle: method.updateMany("write", { fields: ["activity"], match: ["activity"] }),
    }),
});

/** Boards their owner shares with viewers, who are present on them. */
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
    attachments: [presence.attach({ by: "read" })],
    methods: (method) => ({ list: method.list("read"), create: method.create("manage") }),
});

/** The profiles of a board's members. */
const profile = defineObject({
    name: "profile",
    plural: "profiles",
    scope: space,
    nested: { in: board, receive: "manage" },
    fields: { name: field.string() },
    permissions: { read: through("parent", "read"), manage: through("parent", "manage") },
    methods: (method) => ({ list: method.list("read"), create: method.create("manage") }),
});

/** The service serving the boards, profiles and presence. */
const boardsService = defineService("boards", { objects: { presence, board, profile } });

test.for(TEST_DIALECTS)(
    "share presence on a board across instances, guarded by access, gone after its client on %s",
    async (dialect) => {
        const { east, west } = await serveBoards(dialect);

        // let the owner share a board and be present on it
        const plans = await east.call("alice", board, "create", {
            requestId: RequestId.create(),
        });
        const shared = await east.call("alice", board, "grant", {
            requestId: RequestId.create(),
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("universe", "bob"),
        });
        // follow every durable type of the space next to the ephemeral one
        const durable = east.server.source.relayed(
            east.server.source.queriesShape.subscription({
                name: "objects",
                scope: spaceId,
                below: spaceId,
                parameters: {},
            }),
            context("alice").context,
        );
        expect((await nextPage(durable)).complete).toBe(true);
        await durable.return(undefined);

        const alice = east.follow("alice", "client-a");
        expect(await alice.rows()).toEqual([]);
        const mine = await east.call("alice", presence, "create", {
            client: "client-a",
            parent: { packageId: board.policy.definition.packageId, type: "board", id: plans.id },
            activity: "editing",
            cursor: 0,
        });

        // name the writing client on the row, once per board and client
        expect(mine.client).toBe("client-a");
        await expect(
            east.call("alice", presence, "create", {
                client: "client-a",
                parent: {
                    packageId: board.policy.definition.packageId,
                    type: "board",
                    id: plans.id,
                },
                activity: "viewing",
            }),
        ).rejects.toMatchObject({
            code: "DUPLICATE",
            message: "a record with the same unique key exists",
        });

        // show the owner's moving cursor to the viewer on the other instance
        const bob = west.follow("bob", "client-b");
        expect(await bob.rows()).toEqual([["alice", "editing", 0]]);
        await east.call("alice", presence, "update", {
            client: "client-a",
            id: mine.id,
            cursor: 12,
        });
        expect(await bob.rows()).toEqual([["alice", "editing", 12]]);

        // find no board for a stranger's presence, and show the stranger nobody
        await expect(
            west.call("carol", presence, "create", {
                client: "client-c",
                parent: {
                    packageId: board.policy.definition.packageId,
                    type: "board",
                    id: plans.id,
                },
                activity: "viewing",
            }),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no board ${plans.id}` });
        const carol = west.follow("carol", "client-c");
        expect(await carol.rows()).toEqual([]);

        // remove the owner's presence everywhere after its linger
        expect(await west.follow("bob", "client-a").rows()).toEqual([["alice", "editing", 12]]);
        alice.close();
        expect(await bob.rows()).toEqual([]);

        // hide the board's presence from the viewer once the owner revokes it
        expect(await east.follow("alice", "client-d").rows()).toEqual([]);
        vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"], shouldAdvanceTime: true });
        onTestFinished(() => {
            vi.useRealTimers();
        });
        const viewing = await west.call("alice", presence, "create", {
            client: "client-d",
            parent: { packageId: board.policy.definition.packageId, type: "board", id: plans.id },
            activity: "viewing",
        });
        expect(await bob.rows()).toEqual([["alice", "viewing", null]]);

        // keep the row past its linger while the client streams on the other instance
        await vi.advanceTimersByTimeAsync(150);
        vi.useRealTimers();
        expect(
            await present(west.server.ephemeral, "the west ephemeral storage")
                .database.select({ id: presence.table.id })
                .from(presence.table),
        ).toEqual([{ id: viewing.id }]);
        await east.call("alice", board, "revoke", {
            requestId: RequestId.create(),
            id: plans.id,
            relationshipId: shared.id,
        });
        expect(await bob.rows()).toEqual([]);
    },
);

test.for(TEST_DIALECTS)(
    "change many presences at once, only those the caller may write, on %s",
    async (dialect) => {
        const { east } = await serveBoards(dialect);

        // let the owner share a board, both users editing on it
        const plans = await east.call("alice", board, "create", { requestId: RequestId.create() });
        await east.call("alice", board, "grant", {
            requestId: RequestId.create(),
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("universe", "bob"),
        });
        const parent = {
            packageId: board.policy.definition.packageId,
            type: "board",
            id: plans.id,
        };
        for (const [as, client] of [
            ["alice", "client-a"],
            ["bob", "client-b"],
        ] as const) {
            // keep each client streaming past the linger
            await east.follow(as, client).rows();
            await east.call(as, presence, "create", { client, parent, activity: "editing" });
        }

        // idle only the viewer's own editing presences
        const idled = await east.call("bob", presence, "idle", {
            client: "client-b",
            where: { activity: "editing" },
            activity: "idle",
        });
        const rows = await present(east.server.ephemeral, "the east ephemeral storage")
            .database.select({
                client: presence.table.client,
                activity: presence.table.activity,
            })
            .from(presence.table);
        expect([
            idled,
            rows.toSorted((left, right) => left.client.localeCompare(right.client)),
        ]).toEqual([
            { count: 1 },
            [
                { client: "client-a", activity: "editing" },
                { client: "client-b", activity: "idle" },
            ],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "write and follow presence through object clients of both instances on %s",
    async (dialect) => {
        const { east, west } = await serveBoards(dialect);
        const plans = await east.call("alice", board, "create", { requestId: RequestId.create() });
        await east.call("alice", board, "grant", {
            requestId: RequestId.create(),
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("universe", "bob"),
        });

        // follow the owner's presence from the other instance
        const alice = await clientOf(east, "alice");
        const bob = await clientOf(west, "bob");
        await alice.client.query.presence.findMany().subscribe().ready;
        const seen = bob.client.query.presence.findMany().subscribe();
        await alice.client.mutate(presence).create({
            parent: { packageId: board.policy.definition.packageId, type: "board", id: plans.id },
            activity: "editing",
            cursor: 3,
        }).confirmed;
        await until(async () => (await seen.read()).length === 1);
        expect((await seen.read()).map((row) => [row["memberId"], row["cursor"]])).toEqual([
            ["alice", 3],
        ]);

        // deliver her reaction to the board's readers on the other instance
        const listening = new AbortController();
        onTestFinished(() => listening.abort());
        const reactions = bob.client.broadcasts(board, plans.id, listening.signal);
        const received = reactions.next();
        await alice.client.broadcast(board, plans.id, { reaction: "👍" });
        expect((await received).value).toEqual({ reaction: "👍" });

        // let her presence go once her client stops following
        alice.stop();
        await until(async () => (await seen.read()).length === 0);
    },
);

test.for(TEST_DIALECTS)(
    "deliver an object's events to a reader granted it while already following on %s",
    async (dialect) => {
        const { east, west } = await serveBoards(dialect);
        const plans = await east.call("alice", board, "create", { requestId: RequestId.create() });

        // listen for the board's events as bob before he may read it
        const alice = await clientOf(east, "alice");
        const bob = await clientOf(west, "bob");
        const boards = bob.client.query.board.findMany().subscribe();
        const listening = new AbortController();
        onTestFinished(() => listening.abort());
        const received = bob.client.broadcasts(board, plans.id, listening.signal).next();

        // grant him the board, then deliver her reaction once his copy has it
        await east.call("alice", board, "grant", {
            requestId: RequestId.create(),
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("universe", "bob"),
        });
        await until(async () => (await boards.read()).length === 1);
        await alice.client.broadcast(board, plans.id, { reaction: "👋" });
        expect((await received).value).toEqual({ reaction: "👋" });
    },
);

test.for(TEST_DIALECTS)(
    "include live presence with durable profiles, and count live viewers per board, across storages on %s",
    async (dialect) => {
        const { east, west } = await serveBoards(dialect);
        const plans = await east.call("alice", board, "create", { requestId: RequestId.create() });
        const notes = await east.call("alice", board, "create", { requestId: RequestId.create() });
        for (const shared of [plans, notes]) {
            await east.call("alice", board, "grant", {
                requestId: RequestId.create(),
                id: shared.id,
                relation: "viewer",
                subject: principal.user.reference("universe", "bob"),
            });
        }
        const card = await east.call("alice", profile, "create", {
            requestId: RequestId.create(),
            parentId: plans.id,
            name: "Alice",
        });

        // show the viewer each board with the presence on it and the profile each presence names
        const alice = await clientOf(east, "alice");
        const bob = await clientOf(west, "bob");
        await alice.client.query.presence.findMany().subscribe().ready;
        await alice.client.mutate(presence).create({
            parent: { packageId: board.policy.definition.packageId, type: "board", id: plans.id },
            activity: "editing",
            profileId: card.id,
        }).confirmed;
        const boards = bob.client.query.board
            .findMany({
                orderBy: { createdAt: "asc" },
                with: { presences: { with: { profile: true } } },
            })
            .subscribe();
        await boards.ready;
        const shown = async () =>
            (await boards.read()).map((row) =>
                row.presences.map((entry) => [entry.activity, entry.profile?.name]),
            );
        await until(async () => (await shown()).flat().some(([, name]) => name === "Alice"));
        expect(await shown()).toEqual([[["editing", "Alice"]], []]);

        // keep only the presences the relation's own condition selects
        const filtered = bob.client.query.board
            .findMany({
                orderBy: { createdAt: "asc" },
                with: { presences: { where: { activity: "idle" } } },
            })
            .subscribe();
        await filtered.ready;
        expect((await filtered.read()).map((row) => row.presences.length)).toEqual([0, 0]);
        await filtered.close();

        // count each board's live viewers locally
        const counts = bob.client.query.board
            .findMany({
                orderBy: { createdAt: "asc" },
                extras: { viewers: Expression.rollup("count", "presences") },
            })
            .subscribe();
        await counts.ready;
        const viewers = async () => (await counts.read()).map((row) => ({ count: row.viewers }));
        expect(await viewers()).toEqual([{ count: 1 }, { count: 0 }]);

        // count none once the viewer's presence goes, and forget her profile
        alice.stop();
        await until(async () => (await viewers())[0]?.count === 0);
        await until(async () => (await shown()).flat().length === 0);
        expect(await shown()).toEqual([[], []]);

        // refuse conditions relating the other storage's objects
        let refused: unknown;
        try {
            bob.client.query.board.findMany({ where: { presences: {} } }).subscribe();
        } catch (error) {
            refused = error;
        }
        expect(refused).toMatchObject({
            code: "BAD_REQUEST",
            message: "object board relates no presences of another storage",
        });
    },
);

test("refuse durability on ephemeral objects and ephemeral objects outside their store", async () => {
    // refuse lingering durable objects and recoverable ephemeral ones
    expect(() =>
        defineObject({
            name: "note",
            plural: "notes",
            scope: space,
            linger: { seconds: 1 },
            fields: {},
            permissions: [],
        }),
    ).toThrow(new TypeError("object note lingers without being ephemeral"));
    expect(() =>
        defineObject({
            name: "cursor",
            plural: "cursors",
            scope: space,
            storage: "ephemeral",
            recoverable: { within: { days: 1 }, by: "write" },
            fields: {},
            permissions: ["write"],
        }),
    ).toThrow(new TypeError("ephemeral object cursor takes no recoverable"));
    expect(() =>
        defineObject({
            name: "cursor",
            plural: "cursors",
            scope: space,
            storage: "ephemeral",
            linger: { seconds: -1 },
            fields: {},
            permissions: [],
        }),
    ).toThrow(new TypeError('linger of cursor is no duration: {"seconds":-1}'));

    // refuse a durable object in a memory store
    const memory = await TestDatabase.create("sqlite", board.tables, { isMigrated: true });
    onTestFinished(() => memory.close());
    expect(
        () =>
            new EphemeralStorage(memory.database, [board], channelHub<TrackerMessage>()(), {
                report: (error) => {
                    throw error;
                },
            }),
    ).toThrow(new TypeError("object board is durable, not ephemeral"));
});

/** Serve boards from two instances over one durable database. */
async function serveBoards(dialect: Dialect) {
    // keep the boards, their access and requests in one durable database
    const storage = await TestDatabase.create(
        dialect,
        defineDatabase({
            name: "main",
            tables: [journal, ...board.tables, ...profile.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // serve them from two instances sharing presence over their database's channel
    const instance = async () => {
        const durable = await storage.connect(storage.database.tables);
        const store = present(
            await EphemeralStorage.open(
                [presence, board, profile],
                EphemeralStorage.channel(durable),
                (tables) => sqlite.connect(":memory:", tables),
                {
                    heartbeat: 1000,
                    report: (error) => {
                        throw error;
                    },
                },
            ),
            "the ephemeral storage",
        );
        onTestFinished(() => store[Symbol.asyncDispose]());
        const server = new ObjectServer({
            objects: { presence, board, profile },
            database: durable,
            ephemeral: store,
            callKey: testCallKey,
            origin: {
                package: presence.package,
                service: "test",
            },
        });

        const served = serveObjects(server.implement(boardsService), spaceId);

        return {
            server,
            /** Reach the instance over HTTP as a user. */
            endpoint: served.endpoint,
            /** Call a method as a user. */
            call: async <Object extends ObjectType, Name extends CallableName<Object>>(
                as: string,
                object: Object,
                name: Name,
                input: object,
            ) => server.call(object, name, { spaceId, ...input }, context(as).context),
            /** Follow the space's presence as a user's client. */
            follow: (as: string, client: string) => {
                // read the pages in the background into the rows received, waking whoever waits for the next
                const { context: followed, controller } = context(as);
                const pages = server.source.relayed(
                    server.source.ephemeralShape.subscription({
                        name: "ephemeral",
                        scope: spaceId,
                        below: spaceId,
                        parameters: {
                            client,
                            queries: { presences: { object: "presence" } },
                        },
                    }),
                    followed,
                );
                const rows = new Map<string, Record<string, unknown>>();
                let arrived = 0;
                let wake = idle;
                const reading = (async () => {
                    for await (const page of pages) {
                        // start over from a snapshot that resets, then apply the page's changes
                        if (page.reset) {
                            rows.clear();
                        }
                        for (const change of page.changes) {
                            const id = schema.string().parse(change.row["id"]);
                            if (change.operation === "delete") {
                                rows.delete(id);
                            } else {
                                rows.set(id, change.row);
                            }
                        }
                        arrived += 1;
                        wake();
                    }
                })();
                onTestFinished(async () => {
                    // end the stream before its databases close
                    controller.abort();
                    await reading;
                });
                let read = 0;

                return {
                    /** Read the rows once the next page arrives. */
                    rows: async () => {
                        read += 1;
                        for (let reached = arrived; reached < read; reached = arrived) {
                            const { promise, resolve } = Promise.withResolvers<void>();
                            wake = resolve;
                            await promise;
                        }

                        return [...rows.values()].map((row) => [
                            row["memberId"],
                            row["activity"],
                            row["cursor"],
                        ]);
                    },
                    /** Leave, closing the stream. */
                    close: () => controller.abort(),
                };
            },
        };
    };

    return { east: await instance(), west: await instance() };
}

/** Build a user's request context with its own cancellation. */
function context(as: string) {
    const controller = new AbortController();
    onTestFinished(() => controller.abort());

    return {
        controller,
        context: subjectContext(principal.user.reference("universe", as), spaceId, {
            signal: controller.signal,
        }),
    };
}

/** Open a user's client of the boards over one instance. */
async function clientOf(instance: { endpoint(user: string): ClientOptions }, as: string) {
    // keep the boards and presence in a local database
    const storage = await TestDatabase.create(
        "sqlite",
        ObjectClient.tables({ board, profile, presence }),
        {
            storage: "file",
        },
    );

    const client = await ObjectClient.open({
        database: storage.database,
        objects: { board, profile, presence },
        scope: spaceId,
        caller: principal.user.reference("universe", as),
        endpoint: instance.endpoint(as),
        reconnect: unmoved,
    });

    // run until stopped, failing on any reported failure
    const controller = new AbortController();
    const errors: unknown[] = [];
    const running = client.run(controller.signal, (error) => errors.push(error));
    onTestFinished(async () => {
        controller.abort();
        await running;
        await client.close();
        await storage.close();
        expect(errors).toEqual([]);
    });

    return { client, stop: () => controller.abort() };
}

/** Wait until a check passes, giving timers and messages a turn between checks. */
async function until(check: () => Promise<boolean>): Promise<void> {
    const deadline = Date.now() + UNTIL_MILLISECONDS;
    while (!(await check())) {
        // fail a check that never passes, well within the test's own timeout
        if (Date.now() > deadline) {
            throw new Error(`the check did not pass within ${UNTIL_MILLISECONDS} ms`);
        }
        await new Promise((resolve) => {
            setTimeout(resolve, 2);
        });
    }
}

/** Wait for nothing, until a reader waiting for the next page replaces it. */
function idle(): void {}

/** Read the next page of a sync stream, refusing an ended stream. */
async function nextPage(pages: AsyncGenerator<Page>): Promise<Page> {
    const read = await pages.next();
    if (read.done === true) {
        throw new TypeError("the sync stream ended");
    }

    return read.value;
}
