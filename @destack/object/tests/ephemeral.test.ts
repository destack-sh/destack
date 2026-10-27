import { testSimulation } from "../src/test/index.ts";
import { users } from "./fixture/user.ts";
import type { Page } from "@destack/sync";
import { present, schema, type Identifier } from "@destack/schema";
import { defineDatabase, Expression } from "@destack/db";
import { expect, onTestFinished, refusal, test } from "@destack/test";
import { intersection, principal, relation, through, union } from "@destack/access";
import { TestDatabase } from "@destack/db/test";
import * as sqlite from "@destack/db/bun";

import { subjectContext, testCallKey } from "@destack/object/test";
import { testEvents } from "../src/test/index.ts";
import { RequestId, Scope } from "@destack/sync";
import {
    type ObjectOf,
    defineObject,
    field,
    type CallableName,
    type ObjectType,
} from "../src/index.ts";
import { implementPackage, EphemeralStorage } from "../src/server/index.ts";
import { type LiveQuery, ObjectClient } from "../src/client/index.ts";
import type { ClientOptions } from "@destack/object/client";
import { definePackage } from "@destack/object";
import { user } from "./schema.ts";
import { openSpace, space, unmoved, spacePrincipal } from "./fixture/space.ts";
import { request, type RequestInput } from "./fixture/request.ts";
import { Environment } from "@destack/environment";
import { tablesOf } from "../src/index.ts";
import objectPackage from "../src/package.ts";
import { TestObjectStore } from "../src/test/index.ts";

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

test("share presence on a board across instances, guarded by access, gone after its writer", async () => {
    // run in a world of its own, moving its time while the work waits
    const simulation = testSimulation();
    const world = simulation.environment("test");
    await simulation.drive(
        (async () => {
            const { east, west } = await serveBoards(world);

            // let the owner share a board, and follow every durable type of the space beside presence
            const plans = await east.call(users.alice, board, "create", {
                requestId: RequestId.create(world),
            });
            const shared = await east.call(users.alice, board, "grant", {
                requestId: RequestId.create(world),
                id: plans.id,
                relation: "viewer",
                subject: principal.user.reference("universe", users.bob),
            });
            const durable = east.store.source.relayed(
                east.store.source.queriesPublication.subscription({
                    name: "objects",
                    scope: spaceId,
                    target: spaceId,
                    parameters: {},
                }),
                context(users.alice, world).context,
            );
            expect((await nextPage(durable)).complete).toBe(true);
            await durable.return(undefined);

            // be present on the board through the owner's client, once per board and writer
            const parent = {
                packageId: board.policy.definition.packageId,
                type: "board",
                id: plans.id,
            };
            const alice = await openClient(world, east, users.alice);
            await alice.client.query.presence.findMany().subscribe().ready;
            const created = alice.client
                .mutate(presence)
                .create({ parent, activity: "editing", cursor: 0 });
            const mine = await created.predicted;
            await created.confirmed;
            expect([
                mine.writerId === alice.client.writerId,
                await refusal(
                    alice.client.mutate(presence).create({ parent, activity: "viewing" }).confirmed,
                ),
            ]).toEqual([
                true,
                ["CONFLICT", "DUPLICATE", "another presence has the same parent and writerId"],
            ]);

            // show the owner's moving cursor to the viewer's client on the other instance
            const bob = await openClient(world, west, users.bob);
            const seen = bob.client.query.presence.findMany().subscribe();
            const shown = async () =>
                (await seen.read()).map((row) => [row["memberId"], row["activity"], row["cursor"]]);
            await until(seen, (rows) => rows.length === 1);
            const first = await shown();
            await alice.client.mutate(presence).update({ id: mine.id, cursor: 12 }).confirmed;
            await until(seen, (rows) => rows[0]?.["cursor"] === 12);
            expect([first, await shown()]).toEqual([
                [[users.alice, "editing", 0]],
                [[users.alice, "editing", 12]],
            ]);

            // find no board for a stranger's presence, and show the stranger nobody
            const carol = await openClient(world, west, users.carol);
            const stranger = carol.client.query.presence.findMany().subscribe();
            await stranger.ready;
            expect([
                await refusal(
                    carol.client.mutate(presence).create({ parent, activity: "viewing" }).confirmed,
                ),
                await stranger.read(),
            ]).toEqual([["NOT_FOUND", "NOT_FOUND", `board ${plans.id} not found`], []]);

            // remove the owner's presence after its linger, which another caller's stream under her writer never extends
            west.stream(users.bob, alice.client.writerId);
            alice.stop();
            await until(seen, (rows) => rows.length === 0);

            // retain a presence a call wrote on the other instance past its linger while its writer streams here
            const returning = await openClient(world, east, users.alice);
            await returning.client.query.presence.findMany().subscribe().ready;
            const viewing = await west.call(users.alice, presence, "create", {
                writerId: returning.client.writerId,
                parent,
                activity: "viewing",
            });
            await until(seen, (rows) => rows[0]?.["activity"] === "viewing");
            await world.clock.sleep(150);
            const held = await present(west.store.ephemeral, "the west ephemeral storage")
                .database.select({ id: presence.table.id })
                .from(presence.table);

            // hide the board's presence from the viewer once the owner revokes it
            await east.call(users.alice, board, "revoke", {
                requestId: RequestId.create(world),
                id: plans.id,
                relationshipId: shared.id,
            });
            await until(seen, (rows) => rows.length === 0);
            expect(held).toEqual([{ id: viewing.id }]);
        })(),
    );
});

test("change many presences at once, only those the caller may write", async () => {
    // run in a world of its own
    const world = testSimulation().environment("test");

    const { east } = await serveBoards(world);

    // let the owner share a board, both users editing on it through their clients
    const plans = await east.call(users.alice, board, "create", {
        requestId: RequestId.create(world),
    });
    await east.call(users.alice, board, "grant", {
        requestId: RequestId.create(world),
        id: plans.id,
        relation: "viewer",
        subject: principal.user.reference("universe", users.bob),
    });
    const parent = { packageId: board.policy.definition.packageId, type: "board", id: plans.id };
    const clients = [
        await openClient(world, east, users.alice),
        await openClient(world, east, users.bob),
    ];
    for (const { client } of clients) {
        // stream each writer past the linger, then be present
        await client.query.presence.findMany().subscribe().ready;
        await client.mutate(presence).create({ parent, activity: "editing" }).confirmed;
    }

    // idle only the viewer's editing presences
    const [, bob] = clients;
    await present(bob, "bob's client")
        .client.mutate(presence)
        .idle({
            where: { activity: "editing" },
            activity: "idle",
        }).confirmed;
    const rows = await present(east.store.ephemeral, "the east ephemeral storage")
        .database.select({ memberId: presence.table.memberId, activity: presence.table.activity })
        .from(presence.table);
    expect(rows.toSorted(byMember)).toEqual(
        [
            { memberId: users.alice, activity: "editing" },
            { memberId: users.bob, activity: "idle" },
        ].toSorted(byMember),
    );
});

test("write and follow presence through object clients of both instances", async () => {
    // run in a world of its own, moving its time while the work waits
    const simulation = testSimulation();
    const world = simulation.environment("test");
    await simulation.drive(
        (async () => {
            const { east, west } = await serveBoards(world);
            const plans = await east.call(users.alice, board, "create", {
                requestId: RequestId.create(world),
            });
            await east.call(users.alice, board, "grant", {
                requestId: RequestId.create(world),
                id: plans.id,
                relation: "viewer",
                subject: principal.user.reference("universe", users.bob),
            });

            // follow the owner's presence from the other instance
            const alice = await openClient(world, east, users.alice);
            const bob = await openClient(world, west, users.bob);
            await alice.client.query.presence.findMany().subscribe().ready;
            const seen = bob.client.query.presence.findMany().subscribe();
            await alice.client.mutate(presence).create({
                parent: {
                    packageId: board.policy.definition.packageId,
                    type: "board",
                    id: plans.id,
                },
                activity: "editing",
                cursor: 3,
            }).confirmed;
            await until(seen, (rows) => rows.length === 1);
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
            await until(seen, (rows) => rows.length === 0);
        })(),
    );
});

test("deliver an object's events to a reader granted it while already following", async () => {
    // run in a world of its own, moving its time while the work waits
    const simulation = testSimulation();
    const world = simulation.environment("test");
    await simulation.drive(
        (async () => {
            const { east, west } = await serveBoards(world);
            const plans = await east.call(users.alice, board, "create", {
                requestId: RequestId.create(world),
            });

            // listen for the board's events as bob before he may read it
            const alice = await openClient(world, east, users.alice);
            const bob = await openClient(world, west, users.bob);
            const boards = bob.client.query.board.findMany().subscribe();
            const listening = new AbortController();
            onTestFinished(() => listening.abort());
            const received = bob.client.broadcasts(board, plans.id, listening.signal).next();

            // grant him the board and deliver her reaction once his replicas have it and follow its events
            await east.call(users.alice, board, "grant", {
                requestId: RequestId.create(world),
                id: plans.id,
                relation: "viewer",
                subject: principal.user.reference("universe", users.bob),
            });
            await until(boards, (rows) => rows.length === 1);
            await bob.client.query.presence.findMany().subscribe().ready;
            await alice.client.broadcast(board, plans.id, { reaction: "👋" });
            expect((await received).value).toEqual({ reaction: "👋" });
        })(),
    );
});

test("include live presence with durable profiles, and count live viewers per board, across storages", async () => {
    // run in a world of its own, moving its time while the work waits
    const simulation = testSimulation();
    const world = simulation.environment("test");
    await simulation.drive(
        (async () => {
            const { east, west } = await serveBoards(world);
            const plans = await east.call(users.alice, board, "create", {
                requestId: RequestId.create(world),
            });
            simulation.clock.advance(1);
            const notes = await east.call(users.alice, board, "create", {
                requestId: RequestId.create(world),
            });
            for (const shared of [plans, notes]) {
                await east.call(users.alice, board, "grant", {
                    requestId: RequestId.create(world),
                    id: shared.id,
                    relation: "viewer",
                    subject: principal.user.reference("universe", users.bob),
                });
            }
            const card = await east.call(users.alice, profile, "create", {
                requestId: RequestId.create(world),
                parentId: plans.id,
                name: "Alice",
            });

            // show the viewer each board with the presence on it and the profile each presence refers to
            const alice = await openClient(world, east, users.alice);
            const bob = await openClient(world, west, users.bob);
            await alice.client.query.presence.findMany().subscribe().ready;
            await alice.client.mutate(presence).create({
                parent: {
                    packageId: board.policy.definition.packageId,
                    type: "board",
                    id: plans.id,
                },
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
            await until(boards, (rows) =>
                rows.some((row) => row.presences.some((entry) => entry.profile?.name === "Alice")),
            );
            expect(await shown()).toEqual([[["editing", "Alice"]], []]);

            // take only the presences the relation's condition selects
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
            const viewers = async () =>
                (await counts.read()).map((row) => ({ count: row.viewers }));
            expect(await viewers()).toEqual([{ count: 1 }, { count: 0 }]);

            // count none once the viewer's presence goes, and forget her profile
            alice.stop();
            await until(counts, (rows) => rows[0]?.viewers === 0);
            await until(boards, (rows) => rows.every((row) => row.presences.length === 0));
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
        })(),
    );
});

/** Serve boards from two instances over one durable database. */
async function serveBoards(world: Environment) {
    // store the boards, their access and requests in one durable database
    const storage = await TestDatabase.create(
        defineDatabase({
            name: "main",
            ...tablesOf({
                packages: [objectPackage, { objects: { board, profile }, tables: [] }],
                follows: [],
                tables: [],
            }),
        }),
        { environment: world, isMigrated: true },
    );
    onTestFinished(() => storage.close());
    await openSpace(storage.database, world, spaceId);

    // serve them from two instances sharing presence over their database's tracker
    const instance = async () => {
        const durable = await storage.connect(storage.database.tables);
        const ephemeral = present(
            await EphemeralStorage.open(
                [presence, board, profile],
                durable.tracker,
                (tables) => sqlite.connectBunSqlite(":memory:", tables, { environment: world }),
                {
                    linger: 50,
                    report: (error) => {
                        throw error;
                    },
                },
            ),
            "the ephemeral storage",
        );
        onTestFinished(() => ephemeral[Symbol.asyncDispose]());
        const store = await TestObjectStore.receive(space.reference(Scope.universe.id, spaceId), {
            chain: async () => undefined,
            environment: world,
            principal: spacePrincipal(spaceId),
            servers: [implementPackage(boardsPackage, {})],
            database: durable,
            ephemeral,
            callKey: testCallKey,
            events: await testEvents(durable, world),
            origin: {
                package: presence.package,
            },
        });

        const served = TestObjectStore.serve(store);

        onTestFinished(() => served[Symbol.asyncDispose]());

        return {
            store,
            /** Call the instance over HTTP as a user. */
            endpoint: (as: Identifier<"user">) => served.as(as).endpoint(boardsPackage),
            /** Call a method as a user. */
            call: async <Object extends ObjectType, Name extends CallableName<Object>>(
                as: Identifier<"user">,
                object: Object,
                name: Name,
                input: NoInfer<RequestInput<Object, Name>>,
            ) => request(store, object, name, spaceId, input, context(as, world).context),
            /** Stream the space's presence as a user under a writer, discarding every page. */
            stream: (as: Identifier<"user">, writerId: string) => {
                // read the pages in the background until closed, ending before the databases close
                const { context: streamed, controller } = context(as, world);
                const pages = store.source.relayed(
                    store.source.ephemeralPublication.subscription({
                        name: "ephemeral",
                        scope: spaceId,
                        target: spaceId,
                        parameters: { writerId, queries: { presences: { object: "presence" } } },
                    }),
                    streamed,
                );
                const reading = (async () => {
                    for await (const page of pages) {
                        // discard the page
                        void page;
                    }
                })();
                onTestFinished(async () => {
                    controller.abort();
                    await reading;
                });

                return { close: () => controller.abort() };
            },
        };
    };

    return { east: await instance(), west: await instance() };
}

/** Build a user's request context with a separate cancellation. */
function context(as: Identifier<"user">, world: Environment) {
    const controller = new AbortController();
    onTestFinished(() => controller.abort());

    return {
        controller,
        context: subjectContext(principal.user.reference("universe", as), spaceId, {
            environment: world,
            signal: controller.signal,
        }),
    };
}

/** Open a user's client of the boards over one instance. */
async function openClient(
    world: Environment,
    instance: { readonly endpoint: (user: Identifier<"user">) => ClientOptions },
    as: Identifier<"user">,
) {
    // store the boards and presence in a local database
    const storage = await TestDatabase.create(ObjectClient.tables({ board, profile, presence }), {
        environment: world,
        dialect: "sqlite",
        storage: "file",
    });

    const client = await ObjectClient.open({
        environment: world,
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
async function until<Value>(
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

/** Order presences by their member. */
function byMember(left: { memberId: string }, right: { memberId: string }): number {
    return left.memberId.localeCompare(right.memberId);
}

/** Read the next page of a sync stream, refusing an ended stream. */
async function nextPage(pages: AsyncGenerator<Page>): Promise<Page> {
    const read = await pages.next();
    if (read.done === true) {
        throw new TypeError("the sync stream ended");
    }

    return read.value;
}
