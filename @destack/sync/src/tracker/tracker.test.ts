import { expect, onTestFinished, test } from "@destack/test";
import { defineTable, eq, integer, text, type DatabaseConnection } from "@destack/db";
import type { Relay } from "@destack/db/relay";
import { relayHub, TestDatabase } from "@destack/db/test";
import { Tracker, type TrackerMessage } from "./tracker.ts";

/** Cursors of sessions on a board, held in memory. */
const cursor = defineTable(
    "cursor",
    {
        /** The cursor identity. */
        id: text("id").primaryKey(),
        /** The board's scope. */
        scope: text("scope").notNull(),
        /** The position in the board's text. */
        position: integer("position").notNull(),
    },
    { log: {} },
);

test("replicate rows between instances, and remove them with their owner", async () => {
    const join = relayHub<TrackerMessage>();
    const east = await open(join());
    const west = await open(join());

    // write, move and remove a cursor and see each on the other instance
    await write(east, "alice", (database) =>
        database.insert(cursor).values({ id: "a", scope: "space", position: 0 }),
    );
    expect(await read(west)).toEqual([{ id: "a", scope: "space", position: 0 }]);
    await write(east, "alice", (database) =>
        database.update(cursor).set({ position: 12 }).where(eq(cursor.id, "a")),
    );
    expect(await read(west)).toEqual([{ id: "a", scope: "space", position: 12 }]);
    await write(east, "alice", (database) => database.delete(cursor).where(eq(cursor.id, "a")));
    expect([await read(east), await read(west)]).toEqual([[], []]);

    // remove an ended owner's rows everywhere
    await write(west, "bob", (database) =>
        database.insert(cursor).values([
            { id: "b", scope: "space", position: 1 },
            { id: "c", scope: "space", position: 2 },
        ]),
    );
    await write(east, "carol", (database) =>
        database.insert(cursor).values({ id: "d", scope: "space", position: 3 }),
    );
    await west.end("bob");
    await east.settled();
    expect([await read(east), await read(west)]).toEqual([
        [{ id: "d", scope: "space", position: 3 }],
        [{ id: "d", scope: "space", position: 3 }],
    ]);

    // deliver a broadcast on every instance
    const events: unknown[] = [];
    west.listen("board", (event) => events.push(event));
    east.broadcast("board", { reaction: "👍" });
    await until(() => events.length === 1);
    expect(events).toEqual([{ reaction: "👍" }]);
});

test("hand a late instance the rows, and drop those of stopped and silent instances", async () => {
    const join = relayHub<TrackerMessage>();
    const east = await open(join());
    await write(east, "alice", (database) =>
        database.insert(cursor).values({ id: "a", scope: "space", position: 0 }),
    );

    // answer a late instance with the earlier rows
    const west = await open(join());
    await until(async () => (await read(west)).length === 1);

    // drop a stopped instance's rows at once
    east.close();
    await until(async () => (await read(west)).length === 0);

    // drop a silent instance's rows after three missed heartbeats
    let isCrashed = false;
    const relay = join();
    const crashing = await open({
        post: (message) => !isCrashed && relay.post(message),
        listen: (receive, resume) => relay.listen(receive, resume),
    });
    await write(crashing, "bob", (database) =>
        database.insert(cursor).values({ id: "b", scope: "space", position: 1 }),
    );
    await until(async () => (await read(west)).length === 1);
    isCrashed = true;
    await until(async () => (await read(west)).length === 0);
});

test("replicate which instance holds an owner, released everywhere once its instance stops", async () => {
    const join = relayHub<TrackerMessage>();
    const east = await open(join());
    const west = await open(join());
    const changes: [string, boolean][] = [];
    west.watchHolds((owner, isHeld) => changes.push([owner, isHeld]));

    // see a hold on the other instance until released
    east.hold("alice");
    await until(() => west.isHeld("alice"));
    east.release("alice");
    await until(() => !west.isHeld("alice"));

    // release an instance's holds when it stops
    east.hold("bob");
    await until(() => west.isHeld("bob"));
    east.close();
    await until(() => !west.isHeld("bob"));
    expect(changes).toEqual([
        ["alice", true],
        ["alice", false],
        ["bob", true],
        ["bob", false],
    ]);
});

/** Track the cursors of an in-memory database on a relay. */
async function open(relay: Relay<TrackerMessage>): Promise<Tracker> {
    const storage = await TestDatabase.create("sqlite", [cursor], { isMigrated: true });
    const tracker = new Tracker(storage.database, [cursor], relay, { heartbeat: 5 });
    onTestFinished(async () => {
        tracker.close();
        await storage.close();
    });

    return tracker;
}

/** Write in an owner's transaction and publish it. */
async function write(
    tracker: Tracker,
    owner: string,
    run: (database: DatabaseConnection) => PromiseLike<unknown>,
): Promise<void> {
    const rows = await tracker.database.transaction(async (transaction) => {
        await run(transaction);

        return tracker.record(transaction, () => owner);
    });
    tracker.publish(rows);

    // let the relay deliver the queued message
    await new Promise<void>((resolve) => queueMicrotask(resolve));
}

/** Read an instance's cursors after it applies its messages. */
async function read(tracker: Tracker) {
    await tracker.settled();

    return tracker.database.select().from(cursor).orderBy(cursor.id);
}

/** Wait until a check holds. */
async function until(check: () => boolean | Promise<boolean>): Promise<void> {
    while (!(await check())) {
        await new Promise((resolve) => setTimeout(resolve, 1));
    }
}
