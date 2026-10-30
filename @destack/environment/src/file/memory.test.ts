import { expect, test } from "@destack/test";
import { SimulatedClock } from "../clock/simulated.ts";
import { SimulatedRandom } from "../random/simulated.ts";
import { BunFileSystem } from "./bun.ts";
import { FileSystemError } from "./error.ts";
import type { FileEvent, FileSystem } from "./file.ts";
import { MemoryFileSystem } from "./memory.ts";

/** The start of the memory files' clock, in UTC epoch milliseconds. */
const START = Date.UTC(2026, 9, 1);

/** Play one script of operations under a base directory, reading each outcome or refusal. */
async function play(files: FileSystem, base: string): Promise<unknown[]> {
    // read an outcome, or a refusal by its reason and message with the base taken out
    const outcomes: unknown[] = [];
    const at = (path: string) => `${base}/${path}`;
    const run = async (operation: () => Promise<unknown>) => {
        try {
            outcomes.push(await operation());
        } catch (error) {
            const reason = FileSystemError.is(error) ? error.reason : "OTHER";
            outcomes.push([
                reason,
                error instanceof Error ? error.message.replaceAll(base, "~") : "",
            ]);
        }
    };
    const status = async (path: string, isFollowing = true) => {
        const read = isFollowing ? await files.stat(at(path)) : await files.lstat(at(path));

        return { kind: read.kind, size: read.size, mode: read.mode };
    };

    // create, write, append and read
    await run(() => files.mkdir(at("a/b"), { recursive: true }));
    await run(() => files.mkdir(at("a")));
    await run(() => files.writeTextFile(at("a/note.txt"), "hello"));
    await run(() => files.writeTextFile(at("a/note.txt"), " world", { append: true }));
    await run(() => files.writeTextFile(at("a/note.txt"), "again", { createNew: true }));
    await run(() => files.readTextFile(at("a/note.txt")));
    await run(() => files.readTextFile(at("a/missing.txt")));
    await run(() => files.readTextFile(at("a")));
    await run(() => files.writeTextFile(at("none/note.txt"), "x"));

    // list, link and read through links
    await run(() => files.symlink("note.txt", at("a/link")));
    await run(() => files.readLink(at("a/link")));
    await run(() => files.readTextFile(at("a/link")));
    await run(() => files.link(at("a/note.txt"), at("a/b/hard.txt")));
    await run(() => files.writeTextFile(at("a/b/hard.txt"), "shared"));
    await run(() => files.readTextFile(at("a/note.txt")));
    await run(() => files.readDirectory(at("a"), { recursive: true }));
    await run(() => status("a/link", false).then((read) => read.kind));
    await run(() => status("a/link"));

    // move, copy and remove
    await run(() => files.rename(at("a/b"), at("c")));
    await run(() => files.copy(at("a"), at("d")));
    await run(() => files.readDirectory(at("d"), { recursive: true }));
    await run(() => files.remove(at("d")));
    await run(() => files.remove(at("d"), { recursive: true }));
    await run(() => files.exists(at("d")));
    await run(() => files.remove(at("d"), { force: true }));

    // open, write at the cursor, seek, read and truncate
    await run(async () => {
        await using file = await files.open(at("c/data.bin"), {
            read: true,
            write: true,
            createNew: true,
        });
        await file
            .writable()
            .getWriter()
            .write(new Uint8Array([1, 2, 3, 4]));
        await file.seek(1);
        const buffer = new Uint8Array(2);
        const count = await file.read(buffer);
        await file.truncate(3);

        return [count, [...buffer], (await file.stat()).size];
    });
    await run(() =>
        files.open(at("c/data.bin"), { write: true, createNew: true }).then(() => "opened"),
    );
    await run(() => files.chmod(at("c/data.bin"), 0o600).then(() => status("c/data.bin")));

    // refuse copying a directory into itself, leave a rename between two hard links alone, and step up through a link
    await run(() => files.copy(at("a"), at("a/inside")));
    await run(() => files.link(at("c/data.bin"), at("c/twin.bin")));
    await run(() => files.rename(at("c/data.bin"), at("c/twin.bin")));
    await run(() => files.readDirectory(at("c")));
    await run(() => files.symlink(at("c"), at("a/to-c")));
    await run(() => files.readDirectory(at("a/to-c/..")));

    return outcomes;
}

test("behave as the host's files do across writes, links, moves, copies, removals and handles", async () => {
    // play the script on the host's files in a temporary directory
    const host = new BunFileSystem();
    const directory = await host.makeTemporaryDirectory("destack-memory-");
    const real = await host.realPath(directory);
    let expected: unknown[];
    try {
        expected = await play(host, real);
    } finally {
        await host.remove(directory, { recursive: true });
    }

    // play it on memory files
    const clock = new SimulatedClock(START);
    const memory = MemoryFileSystem.create({
        clock,
        random: new SimulatedRandom(1),
        workingDirectory: "/base",
    });
    const played = await play(memory, "/base");

    expect(played).toEqual(expected);
});

test("resolve relative paths from the working directory, and stop a watch its reader leaves", async () => {
    // write through a relative path, and read it at its absolute one
    const memory = MemoryFileSystem.create({
        clock: new SimulatedClock(START),
        random: new SimulatedRandom(1),
        workingDirectory: "/world",
    });
    await memory.writeTextFile("note.txt", "relative");
    const read = await memory.readTextFile("/world/note.txt");

    // read a watch's first change, then leave it, which ends it
    const watched = memory.watch("/world")[Symbol.asyncIterator]();
    const first = watched.next();
    await memory.writeTextFile("later.txt", "x");
    const firstRead = await first;
    const seen: FileEvent | undefined = firstRead.done === true ? undefined : firstRead.value;
    await watched.return?.();
    const after = await watched.next();

    expect({ read, seen, isEnded: after.done }).toEqual({
        read: "relative",
        seen: { kind: "create", path: "later.txt" },
        isEnded: true,
    });
});

test("lock a file shared or exclusive, waking a waiting lock once the holder releases", async () => {
    // hold the file shared twice, and refuse it exclusive meanwhile
    const memory = MemoryFileSystem.create({
        clock: new SimulatedClock(START),
        random: new SimulatedRandom(1),
        workingDirectory: "/",
    });
    const first = await memory.lock("/lock", "shared");
    const second = await memory.tryLock("/lock", "shared");
    const refused = await memory.tryLock("/lock", "exclusive");

    // wait for it exclusive until both release
    let isTaken = false;
    const waiting = memory.lock("/lock", "exclusive").then((lock) => {
        isTaken = true;

        return lock;
    });
    await first.close();
    const afterFirst = isTaken;
    await second?.close();
    const taken = await waiting;

    expect({ refused, afterFirst, mode: taken.mode }).toEqual({
        refused: undefined,
        afterFirst: false,
        mode: "exclusive",
    });
});

test("watch a directory's changes relative to it, below it only when recursive", async () => {
    // watch a directory flat and recursively, then change a file in it and one below it
    const memory = MemoryFileSystem.create({
        clock: new SimulatedClock(START),
        random: new SimulatedRandom(1),
        workingDirectory: "/",
        directories: ["/watched/below"],
    });
    const stop = new AbortController();
    const read = async (isRecursive: boolean) => {
        const events: unknown[] = [];
        for await (const event of memory.watch("/watched", {
            recursive: isRecursive,
            signal: stop.signal,
        })) {
            events.push(event);
        }

        return events;
    };
    const [flat, deep] = [read(false), read(true)];
    await memory.writeTextFile("/watched/top.txt", "a");
    await memory.writeTextFile("/watched/below/deep.txt", "b");
    await memory.remove("/watched/top.txt");
    stop.abort();

    expect({ flat: await flat, deep: await deep }).toEqual({
        flat: [
            { kind: "create", path: "top.txt" },
            { kind: "remove", path: "top.txt" },
        ],
        deep: [
            { kind: "create", path: "top.txt" },
            { kind: "create", path: "below/deep.txt" },
            { kind: "remove", path: "top.txt" },
        ],
    });
});
