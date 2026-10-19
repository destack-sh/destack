import { users } from "./fixture/user.ts";
import type { Page } from "@destack/sync";
import { present, schema, type Identifier } from "@destack/schema";
import { type Dialect, defineDatabase, Expression } from "@destack/db";
import { expect, onTestFinished, refusal, test, vi } from "@destack/test";
import { intersection, principal, relation, through, union } from "@destack/access";
import { TEST_DIALECTS, TestDatabase } from "@destack/db/test";
import * as sqlite from "@destack/db/bun";

import { subjectContext, testCallKey } from "@destack/runtime/test";
import { callTables, testEvents } from "../src/test/index.ts";
import { RequestId } from "@destack/runtime/request";
import {
    type ObjectOf,
    defineObject,
    field,
    type CallableName,
    type ObjectType,
} from "../src/index.ts";
import { defineServer, EphemeralStorage, ObjectStore } from "../src/server/index.ts";
import { type LiveQuery, ObjectClient } from "../src/client/index.ts";
import type { ClientOptions } from "@destack/runtime/client";
import { definePackage } from "@destack/runtime/declare";
import { serveObjects } from "./fixture/device.ts";
import { user } from "./schema.ts";
import { openSpace, space, unmoved, spacePrincipal } from "./fixture/space.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { Clock } from "@destack/environment";

/** The space with the boards. */
const spaceId = schema.identifier("space").parse("space-01996ab0-0000-7000-8000-000000000009");

/** Who is on a board and where their cursor is. */
const presence = defineObject(
    {
        name: "presence",
        plural: "presences",
        scope: space,
        storage: "ephemeral",
        nested: { in: "any", receive: "present" },
        fields: {
            member: field.reference(principal.user).caller(),
            activity: field.string(schema.string().min(1).max(64)),
            cursor: field.integer().optional(),
            profile: field.reference((): ObjectOf<{ identity: "profile" }> => profile).optional(),
        },
        permissions: {
            read: through("parent", "present"),
            write: intersection(relation("member"), through("parent", "present")),
        },
        constraints: {
            writer: { kind: "unique", on: ["parentId", "writerId"] },
        },
    },
    (method) => ({
        create: method.create("write"),
        update: method.update("write"),
        delete: method.delete("write"),
        idle: method.updateMany("write", { fields: ["activity"], match: ["activity"] }),
    }),
);

/** Boards their owner shares with viewers, who are present on them. */
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
        attachments: [presence.attach({ by: "read" })],
    },
    (method) => ({ create: method.create("manage") }),
);

/** The profiles of a board's members. */
const profile = defineObject(
    {
        name: "profile",
        plural: "profiles",
        scope: space,
        nested: { in: board, receive: "manage" },
        fields: { name: field.string() },
        permissions: { read: through("parent", "read"), manage: through("parent", "manage") },
    },
    (method) => ({ create: method.create("manage") }),
);

/** The package serving the boards, profiles and presence. */
const boardsPackage = definePackage({ objects: { presence, board, profile } });

test.for(TEST_DIALECTS)(
    "share presence on a board across instances, guarded by access, gone after its writer on %s",
    async (dialect) => {
        const { east, west } = await serveBoards(dialect);

        // let the owner share a board and be present on it
        const plans = await east.call(users.alice, board, "create", {
            requestId: RequestId.create(Date.now()),
        });
        const shared = await east.call(users.alice, board, "grant", {
            requestId: RequestId.create(Date.now()),
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("universe", users.bob),
        });
        // follow every durable type of the space next to the ephemeral one
        const durable = east.store.source.relayed(
            east.store.source.queriesPublication.subscription({
                name: "objects",
                scope: spaceId,
                target: spaceId,
                parameters: {},
            }),
            context(users.alice).context,
        );
        expect((await nextPage(durable)).complete).toBe(true);
        await durable.return(undefined);

        const alice = east.follow(users.alice, "writer-a");
        expect(await alice.rows()).toEqual([]);
        const mine = await east.call(users.alice, presence, "create", {
            writerId: "writer-a",
            parent: { packageId: board.policy.definition.packageId, type: "board", id: plans.id },
            activity: "editing",
            cursor: 0,
        });

        // record the writer on the row, once per board and writer
        expect(mine.writerId).toBe("writer-a");
        expect(
            await refusal(
                east.call(users.alice, presence, "create", {
                    writerId: "writer-a",
                    parent: {
                        packageId: board.policy.definition.packageId,
                        type: "board",
                        id: plans.id,
                    },
                    activity: "viewing",
                }),
            ),
        ).toEqual(["CONFLICT", "another presence has the same parent and writerId"]);

        // show the owner's moving cursor to the viewer on the other instance
        const bob = west.follow(users.bob, "writer-b");
        expect(await bob.rows()).toEqual([[users.alice, "editing", 0]]);
        await east.call(users.alice, presence, "update", {
            writerId: "writer-a",
            id: mine.id,
            cursor: 12,
        });
        expect(await bob.rows()).toEqual([[users.alice, "editing", 12]]);

        // find no board for a stranger's presence, and show the stranger nobody
        await expect(
            west.call(users.carol, presence, "create", {
                writerId: "writer-c",
                parent: {
                    packageId: board.policy.definition.packageId,
                    type: "board",
                    id: plans.id,
                },
                activity: "viewing",
            }),
        ).rejects.toMatchObject({ code: "NOT_FOUND", message: `no board ${plans.id}` });
        const carol = west.follow(users.carol, "writer-c");
        expect(await carol.rows()).toEqual([]);

        // remove the owner's presence everywhere after its linger
        expect(await west.follow(users.bob, "writer-a").rows()).toEqual([
            [users.alice, "editing", 12],
        ]);
        alice.close();
        expect(await bob.rows()).toEqual([]);

        // hide the board's presence from the viewer once the owner revokes it
        expect(await east.follow(users.alice, "writer-d").rows()).toEqual([]);
        vi.useFakeTimers({ toFake: ["setTimeout", "clearTimeout"], shouldAdvanceTime: true });
        onTestFinished(() => {
            vi.useRealTimers();
        });
        const viewing = await west.call(users.alice, presence, "create", {
            writerId: "writer-d",
            parent: { packageId: board.policy.definition.packageId, type: "board", id: plans.id },
            activity: "viewing",
        });
        expect(await bob.rows()).toEqual([[users.alice, "viewing", null]]);

        // hold the row past its linger while the writer streams on the other instance
        await vi.advanceTimersByTimeAsync(150);
        vi.useRealTimers();
        expect(
            await present(west.store.ephemeral, "the west ephemeral storage")
                .database.select({ id: presence.table.id })
                .from(presence.table),
        ).toEqual([{ id: viewing.id }]);
        await east.call(users.alice, board, "revoke", {
            requestId: RequestId.create(Date.now()),
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
        const plans = await east.call(users.alice, board, "create", {
            requestId: RequestId.create(Date.now()),
        });
        await east.call(users.alice, board, "grant", {
            requestId: RequestId.create(Date.now()),
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("universe", users.bob),
        });
        const parent = {
            packageId: board.policy.definition.packageId,
            type: "board",
            id: plans.id,
        };
        for (const [as, writerId] of [
            [users.alice, "writer-a"],
            [users.bob, "writer-b"],
        ] as const) {
            // stream each writer past the linger
            await east.follow(as, writerId).rows();
            await east.call(as, presence, "create", { writerId, parent, activity: "editing" });
        }

        // idle only the viewer's own editing presences
        const idled = await east.call(users.bob, presence, "idle", {
            writerId: "writer-b",
            where: { activity: "editing" },
            activity: "idle",
        });
        const rows = await present(east.store.ephemeral, "the east ephemeral storage")
            .database.select({
                writerId: presence.table.writerId,
                activity: presence.table.activity,
            })
            .from(presence.table);
        expect([
            idled,
            rows.toSorted((left, right) => left.writerId.localeCompare(right.writerId)),
        ]).toEqual([
            { count: 1 },
            [
                { writerId: "writer-a", activity: "editing" },
                { writerId: "writer-b", activity: "idle" },
            ],
        ]);
    },
);

test.for(TEST_DIALECTS)(
    "write and follow presence through object clients of both instances on %s",
    async (dialect) => {
        const { east, west } = await serveBoards(dialect);
        const plans = await east.call(users.alice, board, "create", {
            requestId: RequestId.create(Date.now()),
        });
        await east.call(users.alice, board, "grant", {
            requestId: RequestId.create(Date.now()),
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("universe", users.bob),
        });

        // follow the owner's presence from the other instance
        const alice = await openClient(east, users.alice);
        const bob = await openClient(west, users.bob);
        await alice.client.query.presence.findMany().subscribe().ready;
        const seen = bob.client.query.presence.findMany().subscribe();
        await alice.client.mutate(presence).create({
            parent: { packageId: board.policy.definition.packageId, type: "board", id: plans.id },
            activity: "editing",
            cursor: 3,
        }).confirmed;
        await reach(seen, (rows) => rows.length === 1);
        expect((await seen.read()).map((row) => [row["memberId"], row["cursor"]])).toEqual([
            [users.alice, 3],
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
        await reach(seen, (rows) => rows.length === 0);
    },
);

test.for(TEST_DIALECTS)(
    "deliver an object's events to a reader granted it while already following on %s",
    async (dialect) => {
        const { east, west } = await serveBoards(dialect);
        const plans = await east.call(users.alice, board, "create", {
            requestId: RequestId.create(Date.now()),
        });

        // listen for the board's events as bob before he may read it
        const alice = await openClient(east, users.alice);
        const bob = await openClient(west, users.bob);
        const boards = bob.client.query.board.findMany().subscribe();
        const listening = new AbortController();
        onTestFinished(() => listening.abort());
        const received = bob.client.broadcasts(board, plans.id, listening.signal).next();

        // grant him the board and deliver her reaction once his replica has it
        await east.call(users.alice, board, "grant", {
            requestId: RequestId.create(Date.now()),
            id: plans.id,
            relation: "viewer",
            subject: principal.user.reference("universe", users.bob),
        });
        await reach(boards, (rows) => rows.length === 1);
        await alice.client.broadcast(board, plans.id, { reaction: "👋" });
        expect((await received).value).toEqual({ reaction: "👋" });
    },
);

test.for(TEST_DIALECTS)(
    "include live presence with durable profiles, and count live viewers per board, across storages on %s",
    async (dialect) => {
        const { east, west } = await serveBoards(dialect);
        const plans = await east.call(users.alice, board, "create", {
            requestId: RequestId.create(Date.now()),
        });
        const notes = await east.call(users.alice, board, "create", {
            requestId: RequestId.create(Date.now()),
        });
        for (const shared of [plans, notes]) {
            await east.call(users.alice, board, "grant", {
                requestId: RequestId.create(Date.now()),
                id: shared.id,
                relation: "viewer",
                subject: principal.user.reference("universe", users.bob),
            });
        }
        const card = await east.call(users.alice, profile, "create", {
            requestId: RequestId.create(Date.now()),
            parentId: plans.id,
            name: "Alice",
        });

        // show the viewer each board with the presence on it and the profile each presence refers to
        const alice = await openClient(east, users.alice);
        const bob = await openClient(west, users.bob);
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
        await reach(boards, (rows) =>
            rows.some((row) => row.presences.some((entry) => entry.profile?.name === "Alice")),
        );
        expect(await shown()).toEqual([[["editing", "Alice"]], []]);

        // take only the presences the relation's own condition selects
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
        await reach(counts, (rows) => rows[0]?.viewers === 0);
        await reach(boards, (rows) => rows.every((row) => row.presences.length === 0));
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
            message: "object board cannot relate presences: it has another storage",
        });
    },
);

/** Serve boards from two instances over one durable database. */
async function serveBoards(dialect: Dialect) {
    // store the boards, their access and requests in one durable database
    const storage = await TestDatabase.create(
        dialect,
        defineDatabase({
            name: "main",
            tables: [...callTables, ...board.tables, ...profile.tables],
        }),
        { isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, spaceId);

    // serve them from two instances sharing presence over their database's channel
    const instance = async () => {
        const durable = await storage.connect(storage.database.tables);
        const ephemeral = present(
            await EphemeralStorage.open(
                [presence, board, profile],
                EphemeralStorage.channel(durable),
                (tables) => sqlite.connectBunSqlite(":memory:", tables),
                {
                    clock: Clock.system,
                    heartbeat: 1000,
                    linger: 50,
                    report: (error) => {
                        throw error;
                    },
                },
            ),
            "the ephemeral storage",
        );
        onTestFinished(() => ephemeral[Symbol.asyncDispose]());
        const store = new ObjectStore({
            chain: async () => undefined,
            clock: Clock.system,
            principal: spacePrincipal(spaceId),
            servers: [defineServer(boardsPackage, {})],
            database: durable,
            ephemeral,
            callKey: testCallKey,
            events: await testEvents(durable),
            origin: {
                package: presence.package,
            },
        });

        const served = serveObjects(store.package(boardsPackage), spaceId);

        return {
            store,
            /** Reach the instance over HTTP as a user. */
            endpoint: served.endpoint,
            /** Call a method as a user. */
            call: async <Object extends ObjectType, Name extends CallableName<Object>>(
                as: Identifier<"user">,
                object: Object,
                name: Name,
                input: NoInfer<RequestInput<Object, Name>>,
            ) => request(store, object, name, spaceId, input, context(as).context),
            /** Follow the space's presence as a user's client. */
            follow: (as: Identifier<"user">, writerId: string) => {
                // read the pages in the background into the rows received, waking whoever waits for the next
                const { context: followed, controller } = context(as);
                const pages = store.source.relayed(
                    store.source.ephemeralPublication.subscription({
                        name: "ephemeral",
                        scope: spaceId,
                        target: spaceId,
                        parameters: {
                            writerId,
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
                        // start over from a snapshot that resets and apply the page's changes
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
function context(as: Identifier<"user">) {
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
async function openClient(
    instance: { readonly endpoint: (user: Identifier<"user">) => ClientOptions },
    as: Identifier<"user">,
) {
    // store the boards and presence in a local database
    const storage = await TestDatabase.create(
        "sqlite",
        ObjectClient.tables({ board, profile, presence }),
        {
            storage: "file",
        },
    );

    const client = await ObjectClient.open({
        clock: Clock.system,
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

/** Wait until a live query's value passes a check, reading it again after every local commit. */
async function reach<Value>(
    query: LiveQuery<Value>,
    check: (value: Value) => boolean,
): Promise<void> {
    const stop = new AbortController();
    try {
        for await (const value of query.watch(stop.signal)) {
            if (check(value)) {
                return;
            }
        }
    } finally {
        stop.abort();
    }
    throw new Error("the query stopped before its value passed the check");
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
