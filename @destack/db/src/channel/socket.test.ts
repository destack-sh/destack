import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { schema } from "@destack/schema";
import { typedChannel } from "./channel.ts";
import { channelFiles, socketChannel } from "./socket.ts";
import { testHost } from "@destack/test/host";

/** The host the parties meet on, as their socket is a real one. */
const HOST = testHost();

/** The longest wait for a condition, in milliseconds. */
const UNTIL_MILLISECONDS = 1000;

/** The wait between checks of a condition, in milliseconds. */
const CHECK_MILLISECONDS = 5;

/** A message between parties. */
const Note = schema.object({ kind: schema.string(), from: schema.string() });
/** A message between parties. */
type Note = schema.Infer<typeof Note>;

/** Listen on a channel, resolving once joined, and collect what arrives. */
async function party(path: string) {
    const channel = typedChannel(socketChannel(path, HOST), Note);
    const received: Note[] = [];
    const joined = Promise.withResolvers<void>();
    let joins = 0;
    const stop = channel.listen(
        (message) => received.push(message),
        () => {
            joins++;
            joined.resolve();
        },
    );
    await joined.promise;

    return { channel, received, stop, joins: () => joins };
}

/** Wait until a condition is true, failing after a second. */
async function until(check: () => boolean): Promise<void> {
    const deadline = HOST.clock.now() + UNTIL_MILLISECONDS;
    while (!check()) {
        if (HOST.clock.now() > deadline) {
            throw new Error("the condition never became true");
        }
        await HOST.clock.sleep(CHECK_MILLISECONDS);
    }
}

test("send messages between the parties of one path, and continue the channel once its serving party leaves", async () => {
    // join two parties: the first serves the socket, the second connects to it
    const directory = await HOST.files.makeTemporaryDirectory("destack-socket-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const path = join(directory, "database.db");
    const first = await party(path);
    const second = await party(path);
    onTestFinished(second.stop);

    // deliver in both directions
    second.channel.notify({ kind: "commit", from: "second" });
    first.channel.notify({ kind: "commit", from: "first" });
    await until(() => first.received.length === 1 && second.received.length === 1);

    // let the second serve once the first leaves, then send to a third
    first.stop();
    await until(() => second.joins() === 2);
    const third = await party(path);
    onTestFinished(third.stop);
    third.channel.notify({ kind: "commit", from: "third" });
    await until(() => second.received.length === 2);
    expect([first.received, second.received, third.received]).toEqual([
        [{ kind: "commit", from: "second" }],
        [
            { kind: "commit", from: "first" },
            { kind: "commit", from: "third" },
        ],
        [],
    ]);
});

test("release the lock a party that only notified took to serve, once it closes the channel", async () => {
    // notify on a channel no party listens on, which joins it as its serving party
    const directory = await HOST.files.makeTemporaryDirectory("destack-socket-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const path = join(directory, "database.db");
    const files = channelFiles(path, await HOST.files.runtimeDirectory());
    const channel = socketChannel(path, HOST);
    channel.notify({ kind: "commit", from: "writer" });
    const deadline = HOST.clock.now() + UNTIL_MILLISECONDS;
    while (!(await HOST.files.exists(files.socket))) {
        if (HOST.clock.now() > deadline) {
            throw new Error("the party never served the socket");
        }
        await HOST.clock.sleep(CHECK_MILLISECONDS);
    }

    // refuse the lock while it serves, and grant it once the party closes the channel
    const held = await HOST.files.tryLock(files.lock, "exclusive");
    await channel.close();
    const released = await HOST.files.tryLock(files.lock, "exclusive");
    await released?.close();

    expect([held, released === undefined]).toEqual([undefined, false]);
});

test("keep hearing the channel when a listener starts as the last one stops", async () => {
    // stop the serving party's only listener and start another in the same turn
    const directory = await HOST.files.makeTemporaryDirectory("destack-socket-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const path = join(directory, "database.db");
    const first = await party(path);
    first.stop();
    const received: Note[] = [];
    const stop = first.channel.listen((message) => received.push(message));
    onTestFinished(stop);

    // deliver another party's message to the new listener
    const other = await party(path);
    onTestFinished(other.stop);
    other.channel.notify({ kind: "commit", from: "other" });
    await until(() => received.length === 1);

    expect(received).toEqual([{ kind: "commit", from: "other" }]);
});

test("refuse messages and listeners on a channel once it is closed", async () => {
    // close a channel that never joined, then use it
    const directory = await HOST.files.makeTemporaryDirectory("destack-socket-");
    onTestFinished(() => HOST.files.remove(directory, { recursive: true }));
    const path = join(directory, "database.db");
    const channel = socketChannel(path, HOST);
    await channel.close();
    const refusals = [
        () => channel.notify({ kind: "commit", from: "closed" }),
        () => channel.listen(() => {}),
    ].map((use) => {
        try {
            use();

            return undefined;
        } catch (error) {
            return error;
        }
    });

    expect(refusals).toEqual([
        new TypeError(`the channel of ${path} is closed`),
        new TypeError(`the channel of ${path} is closed`),
    ]);
});
