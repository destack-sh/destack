import { AuditOutbox, auditOutboxTables } from "@destack/audit/outbox";
import { schema } from "@destack/schema";
import { Condition } from "@destack/db/query";
import { expect, onTestFinished, test } from "@destack/test";
import { vi } from "vitest";
import {
    intersection,
    principal,
    relation,
    through,
    union,
    type ObjectReference,
} from "@destack/access";
import { AuditRecorder } from "@destack/audit";
import { unique, type DatabaseConnection, type Dialect, type JsonValue } from "@destack/db";
import { defineDatabase } from "@destack/db/declare";
import { relayHub, TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import { identifier } from "@destack/schema";
import { Bookmark } from "@destack/service/bookmark";
import { Journal } from "@destack/service/database";
import { RequestId } from "@destack/service/request";
import type { ServiceContext } from "@destack/service/server";
import type * as sync from "@destack/sync";
import type { TrackerMessage } from "@destack/sync";
import { defineObject, field, method, type ObjectType } from "../src/index.ts";
import { EphemeralStorage, ObjectServer } from "../src/server/index.ts";
import { ObjectClient } from "../src/client/index.ts";
import type { ReplicaProcedures } from "../src/replica/replica.ts";
import type { Client } from "@destack/service";
import { request, user } from "./schema.ts";
import { openSpace, space, unmoved } from "./fixture/space.ts";

/** How long a check may take to hold, in milliseconds: well within the 5 s test timeout. */
const UNTIL_MILLISECONDS = 2000;

/** The space holding the boards. */
const spaceId = identifier("space").parse("space-01996ab0-0000-7000-8000-000000000009");

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
        profile: field.reference((): ObjectType => profile).optional(),
    },
    constraints: (entry) => [unique("presence_client").on(entry.parentId, entry.client)],
    permissions: {
        read: through("parent", "present"),
        write: intersection(relation("member"), through("parent", "present")),
    },
    methods: {
        list: method.list("read"),
        create: method.create("write"),
        update: method.update("write"),
        delete: method.delete("write"),
        idle: method.updateMany("write", { fields: ["activity"], match: ["activity"] }),
    },
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
    methods: { list: method.list("read"), create: method.create("manage") },
});

/** The profiles of a board's members. */
const profile = defineObject({
    name: "profile",
    plural: "profiles",
    scope: space,
    nested: { in: board, receive: "manage" },
    fields: { name: field.string() },
    permissions: { read: through("parent", "read"), manage: through("parent", "manage") },
    methods: { list: method.list("read"), create: method.create("manage") },
});

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
            subject: principal.user.reference("global", "bob"),
        });
        // follow every durable type of the space next to the ephemeral one
        const durable = east.server.sync(spaceId, context("alice").context);
        expect((await durable.next()).value?.complete).toBe(true);
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

        // show the viewer on the other instance the owner's cursor as it moves
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
            await west.server
                .ephemeral!.database.select({ id: presence.table.id })
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
            subject: principal.user.reference("global", "bob"),
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
            // keep each client streaming, so its presence stays past the linger
            await east.follow(as, client).rows();
            await east.call(as, presence, "create", { client, parent, activity: "editing" });
        }

        // idle only the viewer's own editing presences
        const idled = await east.call("bob", presence, "idle", {
            client: "client-b",
            where: { activity: "editing" },
            activity: "idle",
        });
        const rows = await east.server
            .ephemeral!.database.select({
                client: presence.table.client,
                activity: presence.table.activity,
            })
            .from(presence.table);
        expect([
            idled,
            rows.sort((left, right) => left.client.localeCompare(right.client)),
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
            subject: principal.user.reference("global", "bob"),
        });

        // follow the owner's presence from the other instance
        const alice = await clientOf(east.server, "alice");
        const bob = await clientOf(west.server, "bob");
        await alice.client.subscribe(presence).ready;
        const seen = bob.client.subscribe(presence);
        await alice.client.mutate(presence).create({
            parent: { packageId: board.policy.definition.packageId, type: "board", id: plans.id },
            activity: "editing",
            cursor: 3,
        }).confirmed;
        await until(async () => (await seen.read()).length === 1);
        expect((await seen.read()).map((row) => [row.member, row.cursor])).toEqual([["alice", 3]]);

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
        const alice = await clientOf(east.server, "alice");
        const bob = await clientOf(west.server, "bob");
        const boards = bob.client.subscribe(board);
        const listening = new AbortController();
        onTestFinished(() => listening.abort());
        const received = bob.client.broadcasts(board, plans.id, listening.signal).next();

        // grant him the board, then deliver her reaction once his copy holds it
        await east.call("alice", board, "grant", {
            requestId: RequestId.create(),
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("global", "bob"),
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
                subject: principal.user.reference("global", "bob"),
            });
        }
        const card = await east.call("alice", profile, "create", {
            requestId: RequestId.create(),
            parentId: plans.id,
            name: "Alice",
        });

        // show the viewer each board with the presence on it and the profile each presence names
        const alice = await clientOf(east.server, "alice");
        const bob = await clientOf(west.server, "bob");
        await alice.client.subscribe(presence).ready;
        await alice.client.mutate(presence).create({
            parent: { packageId: board.policy.definition.packageId, type: "board", id: plans.id },
            activity: "editing",
            profile: card.id,
        }).confirmed;
        const boards = bob.client.subscribe(board, {
            order: [{ column: "createdAt", direction: "asc" }],
            include: { presences: { include: { profile: {} } } },
        });
        await boards.ready;
        const present = async () =>
            (await boards.read()).map((row) =>
                (row.presences as Record<string, unknown>[]).map((entry) => [
                    entry.activity,
                    (entry.profile as Record<string, unknown> | null)?.name,
                ]),
            );
        await until(async () => (await present()).flat().some(([, name]) => name === "Alice"));
        expect(await present()).toEqual([[["editing", "Alice"]], []]);

        // count each board's live viewers locally
        const counts = bob.client.subscribe(board, {
            order: [{ column: "createdAt", direction: "asc" }],
            include: {
                viewers: {
                    via: "presences",
                    aggregate: { values: { count: { function: "count" } } },
                },
            },
        });
        await counts.ready;
        const viewers = async () =>
            (await counts.read()).map((row) => row.viewers as { count?: number });
        expect(await viewers()).toEqual([{ count: 1 }, { count: 0 }]);

        // count none once the viewer's presence goes, and forget her profile
        alice.stop();
        await until(async () => (await viewers())[0]?.count === 0);
        await until(async () => (await present()).flat().length === 0);
        expect(await present()).toEqual([[], []]);

        // refuse conditions relating the other storage's objects
        let refused: unknown;
        try {
            bob.client.subscribe(board, { where: Condition.exists("presences") });
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
            new EphemeralStorage(memory.database, [board], relayHub<TrackerMessage>()(), {
                report: (error) => {
                    throw error;
                },
            }),
    ).toThrow(new TypeError("object board is durable, not ephemeral"));
});

/** Serve boards from two instances over one durable database. */
async function serveBoards(dialect: Dialect) {
    // hold the boards, their access and requests in one durable database
    const storage = await TestDatabase.create(
        dialect,
        defineDatabase({
            name: "main",
            tables: [...auditOutboxTables, request, ...board.tables, ...profile.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // serve them from two instances sharing presence over a relay
    const join = relayHub<TrackerMessage>();
    const instance = async () => {
        const memory = await TestDatabase.create("sqlite", presence.tables, { isMigrated: true });
        const store = new EphemeralStorage(memory.database, [presence], join(), {
            heartbeat: 1000,
            report: (error) => {
                throw error;
            },
        });
        onTestFinished(async () => {
            store.close();
            await memory.close();
        });
        const server = new ObjectServer({
            objects: { presence, board, profile },
            database: storage.database,
            ephemeral: store,
            context: (context) => ({
                subjects: [principal.user.reference("global", context.requireCaller().id)],
                now: Date.now(),
                attributes: {},
            }),
            journal: new Journal(request),
            audit: AuditRecorder.service(new AuditOutbox(storage.database), {
                package: presence.package,
                service: "test",
            }),
        });

        return {
            server,
            /** Call a method as a user. */
            call: async (as: string, object: ObjectType, name: string, input: object) => {
                return (await server.call(
                    object,
                    name,
                    { spaceId, ...input },
                    context(as).context,
                )) as {
                    id: string;
                    client?: string;
                };
            },
            /** Follow the space's presence as a user's client. */
            follow: (as: string, client: string) => {
                // read the pages in the background into the rows held, waking whoever waits for the next
                const { context: followed, controller } = context(as);
                const pages = server.sync(spaceId, followed, {
                    client,
                    queries: { presences: { object: "presence" } },
                });
                const held = new Map<string, Record<string, unknown>>();
                let arrived = 0;
                let wake = () => {};
                const reading = (async () => {
                    for await (const page of pages) {
                        // start over from a snapshot that resets, then apply the page's changes
                        if (page.reset) {
                            held.clear();
                        }
                        for (const change of page.changes) {
                            if (change.operation === "delete") {
                                held.delete(change.row.id as string);
                            } else {
                                held.set(change.row.id as string, change.row);
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
                        while (arrived < read) {
                            await new Promise<void>((resolve) => (wake = resolve));
                        }

                        return [...held.values()].map((row) => [
                            row.member,
                            row.activity,
                            row.cursor,
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
        context: {
            scope: spaceId,
            caller: { id: as },
            requireCaller: () => ({ id: as }),
            bookmark: new Bookmark(),
            observed: new Bookmark(),
            signal: controller.signal,
            request: new Request("https://test.local", { signal: controller.signal }),
        } as unknown as ServiceContext,
    };
}

/** Open a user's client of the boards over one instance. */
async function clientOf(server: Pick<ObjectServer, "push" | "sync" | "broadcast">, as: string) {
    // hold the boards and presence in a local database
    const storage = await TestDatabase.create(
        "sqlite",
        ObjectClient.tables([board, profile, presence]),
        {
            storage: "file",
        },
    );

    // reach the instance's replica procedures directly as the user
    const procedures = {
        push: async (input: { scope: string; mutations: sync.Mutation[]; client?: string }) =>
            server.push(input.scope, input.mutations, context(as).context, input.client),
        broadcast: async (input: {
            scope: string;
            object: Omit<ObjectReference, "scope">;
            event: JsonValue;
        }) => {
            await server.broadcast(input.scope, input.object, input.event, context(as).context);

            return {};
        },
        sync: async (
            input: {
                scope: string;
                queries?: Record<string, never>;
                previous?: Record<string, never>;
                after?: never;
                client?: string;
            },
            options: { signal: AbortSignal },
        ) => {
            const { context: followed, controller } = context(as);
            options.signal.addEventListener("abort", () => controller.abort());

            const pages = server.sync(input.scope, followed, {
                ...(input.after === undefined ? {} : { after: input.after }),
                ...(input.queries === undefined ? {} : { queries: input.queries }),
                ...(input.previous === undefined ? {} : { previous: input.previous }),
                ...(input.client === undefined ? {} : { client: input.client }),
            });

            // fail an aborted stream as a transport does, rather than ending it
            return (async function* () {
                yield* pages;
                options.signal.throwIfAborted();
            })();
        },
    } as unknown as Client<ReplicaProcedures>;
    const client = await ObjectClient.open({
        database: storage.database,
        objects: [board, profile, presence],
        scope: spaceId,
        caller: principal.user.reference("global", as),
        service: procedures,
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

/** Wait until a check holds, giving timers and messages a turn between checks. */
async function until(check: () => Promise<boolean>): Promise<void> {
    const deadline = Date.now() + UNTIL_MILLISECONDS;
    while (!(await check())) {
        // fail a check that never holds, well within the test's own timeout
        if (Date.now() > deadline) {
            throw new Error(`the check did not hold within ${UNTIL_MILLISECONDS} ms`);
        }
        await new Promise((resolve) => setTimeout(resolve, 2));
    }
}
