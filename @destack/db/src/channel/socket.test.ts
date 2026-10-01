import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { expect, onTestFinished, test } from "@destack/test";
import { socketChannel } from "./socket.ts";

/** A message between parties. */
type Note = { readonly kind: string; readonly from: string };

/** Listen on a channel, resolving once joined, and collect what arrives. */
async function party(path: string) {
    const channel = socketChannel<Note>(path);
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

/** Wait until a condition holds, failing after a second. */
async function until(check: () => boolean): Promise<void> {
    const deadline = Date.now() + 1000;
    while (!check()) {
        if (Date.now() > deadline) {
            throw new Error("the condition never held");
        }
        await new Promise((resolve) => setTimeout(resolve, 5));
    }
}

test("send messages between the parties of one path, and keep the channel once its serving party leaves", async () => {
    // join two parties: the first serves the socket, the second connects to it
    const directory = await mkdtemp(join(tmpdir(), "destack-socket-"));
    onTestFinished(() => rm(directory, { recursive: true, force: true }));
    const path = join(directory, "database.db");
    const first = await party(path);
    const second = await party(path);
    onTestFinished(second.stop);

    // deliver in both directions
    second.channel.notify({ kind: "commit", from: "second" });
    first.channel.notify({ kind: "commit", from: "first" });
    await until(() => first.received.length === 1 && second.received.length === 1);

    // let the second serve once the first leaves, resuming its listener, and reach a third
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
