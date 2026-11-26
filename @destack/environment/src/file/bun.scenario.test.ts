import { join } from "node:path";
import { expect, test } from "@destack/test";
import type { FileEvent } from "./file.ts";
import { Simulation } from "../simulation/index.ts";
import { BunFileSystem } from "./bun.ts";
import { testWorldOptions } from "@destack/test/world";

/** How often the host watch test writes its warm-up file until the watcher reports it. */
const WARM_MILLISECONDS = 250;

/** The simulations' start: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

test("watch a host directory, reading a new file as created, a write as modified and a deletion as removed, until aborted", async () => {
    // watch a temporary directory of the host's files
    const files = new BunFileSystem();
    const directory = await files.makeTemporaryDirectory("watch-");
    const stopping = new AbortController();
    const events = (await files.watch(directory, { signal: stopping.signal }))[
        Symbol.asyncIterator
    ]();
    const next = async (): Promise<FileEvent | undefined> => {
        // read the note's next event, past any late one of the warm-up file
        for (let read = await events.next(); read.done !== true; read = await events.next()) {
            if (read.value.path === "note.txt") {
                return read.value;
            }
        }

        return undefined;
    };

    // write a warm-up file until the watcher reports it, as a host's watcher may miss writes just after it starts
    const warm = join(directory, "warm.txt");
    const first = events.next();
    for (let isReported = false; !isReported;) {
        await files.writeTextFile(warm, "warm");
        isReported = await Promise.race([
            first.then(() => true),
            Bun.sleep(WARM_MILLISECONDS).then(() => false),
        ]);
    }

    // create, write and remove one file, reading each event as it arrives
    const path = join(directory, "note.txt");
    await files.writeTextFile(path, "draft");
    const created = await next();
    await files.writeTextFile(path, "final", { append: true });
    const modified = await next();
    await files.remove(path);
    const removed = await next();

    // end the watch once aborted
    stopping.abort();
    const ended = await next();
    await files.remove(directory, { recursive: true });

    expect([created, modified, removed, ended]).toEqual([
        { kind: "create", path: "note.txt" },
        { kind: "modify", path: "note.txt" },
        { kind: "remove", path: "note.txt" },
        undefined,
    ]);
});

test("watch a world's directory, reading the world's own writes as created, modified and removed, until aborted", async () => {
    // watch a temporary directory of the world's host files
    await using simulation = new Simulation({
        world: testWorldOptions(),
        seed: 1,
        start: START,
    });
    const { files } = simulation.environment("host");
    const directory = await files.makeTemporaryDirectory("watch-");
    const stopping = new AbortController();
    const events = (await files.watch(directory, { signal: stopping.signal }))[
        Symbol.asyncIterator
    ]();
    const next = async (): Promise<FileEvent | undefined> => {
        const read = await simulation.wait(events.next());

        return read.done === true ? undefined : read.value;
    };

    // create, write and remove one file through the world, the first before any read
    const path = join(directory, "note.txt");
    await files.writeTextFile(path, "draft");
    const created = await next();
    await files.writeTextFile(path, "final", { append: true });
    const modified = await next();
    await files.remove(path);
    const removed = await next();

    // end the watch once aborted
    stopping.abort();
    const ended = await next();

    expect([created, modified, removed, ended]).toEqual([
        { kind: "create", path: "note.txt" },
        { kind: "modify", path: "note.txt" },
        { kind: "remove", path: "note.txt" },
        undefined,
    ]);
});
