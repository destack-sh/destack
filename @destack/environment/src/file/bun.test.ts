import { join } from "node:path";
import { expect, refusal, test } from "@destack/test";
import type { DirectoryEntry, FileEvent } from "./file.ts";
import { Simulation } from "../simulation/index.ts";
import { BunFileSystem } from "./bun.ts";

/** The simulations' start: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** The process's user, which every test system has. */
const USER = process.getuid?.() ?? 0;

/** Sort directory entries by name. */
function sorted(entries: readonly DirectoryEntry[]): DirectoryEntry[] {
    return entries.toSorted((left, right) => (left.name < right.name ? -1 : 1));
}

test("refuse a missing file as not found, a taken path as already existing, and a directory read as a file", async () => {
    // write a file in a temporary directory of a world
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    const directory = await files.makeTemporaryDirectory("files-");
    await files.writeTextFile(join(directory, "present.json"), "{}");

    expect({
        missing: await refusal(files.readTextFile(join(directory, "missing.json"))),
        taken: await refusal(
            files.writeTextFile(join(directory, "present.json"), "{}", { createNew: true }),
        ),
        directory: await refusal(files.readTextFile(directory)),
    }).toEqual({
        missing: [
            "NOT_FOUND",
            "NOT_FOUND",
            `cannot read ${join(directory, "missing.json")}: ENOENT`,
        ],
        taken: [
            "CONFLICT",
            "ALREADY_EXISTS",
            `cannot write ${join(directory, "present.json")}: EEXIST`,
        ],
        directory: [
            "INTERNAL_SERVER_ERROR",
            "FILE_SYSTEM_FAILED",
            `cannot read ${directory}: EISDIR`,
        ],
    });
});

test("create a user's private runtime directory once, refusing another user's, one others may open, and a link", async () => {
    // create the user's directory twice, and prepare another's, an opened one and a link
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    const parent = await files.makeTemporaryDirectory("runtime-");
    const own = join(parent, "own");
    const created = [await BunFileSystem.trust(own, USER), await BunFileSystem.trust(own, USER)];
    const opened = join(parent, "opened");
    await files.mkdir(opened, { mode: 0o700 });
    await files.chmod(opened, 0o755);
    const linked = join(parent, "linked");
    await files.symlink(own, linked);

    expect({
        created,
        mode: (await files.stat(own)).mode & 0o777,
        other: await refusal(BunFileSystem.trust(join(parent, "other"), USER + 1)),
        opened: await refusal(BunFileSystem.trust(opened, USER)),
        linked: await refusal(BunFileSystem.trust(linked, USER)),
    }).toEqual({
        created: [own, own],
        mode: 0o700,
        other: [
            "INTERNAL_SERVER_ERROR",
            "FILE_SYSTEM_FAILED",
            `cannot trust runtime directory ${join(parent, "other")}: EACCES`,
        ],
        opened: [
            "INTERNAL_SERVER_ERROR",
            "FILE_SYSTEM_FAILED",
            `cannot trust runtime directory ${opened}: EACCES`,
        ],
        linked: [
            "INTERNAL_SERVER_ERROR",
            "FILE_SYSTEM_FAILED",
            `cannot trust runtime directory ${linked}: EACCES`,
        ],
    });
});

test("remove an empty directory without recursion and refuse a full one, and list entries in walk order", async () => {
    // make an empty and a full directory, with names that sort apart from their walk order
    const files = new BunFileSystem();
    const directory = await files.makeTemporaryDirectory("remove-");
    await files.mkdir(join(directory, "empty"));
    await files.mkdir(join(directory, "full/inner"), { recursive: true });
    await files.writeTextFile(join(directory, "full-note.txt"), "x");
    const listed = (await files.readDirectory(directory, { recursive: true })).map(
        (entry) => entry.name,
    );

    // remove the empty one, refuse the full one, then remove it recursively
    await files.remove(join(directory, "empty"));
    const refused = await refusal(files.remove(join(directory, "full")));
    await files.remove(directory, { recursive: true });

    expect({ listed, refused }).toEqual({
        listed: ["empty", "full", "full/inner", "full-note.txt"],
        refused: [
            "CONFLICT",
            "ALREADY_EXISTS",
            `cannot remove ${join(directory, "full")}: ENOTEMPTY`,
        ],
    });
});

test("remove a missing path when forced, and refuse it as not found otherwise", async () => {
    // remove a path that never existed, forced and then not
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    const missing = join(await files.makeTemporaryDirectory("remove-"), "missing");

    expect({
        forced: await files.remove(missing, { force: true }),
        refused: await refusal(files.remove(missing)),
    }).toEqual({
        forced: undefined,
        refused: ["NOT_FOUND", "NOT_FOUND", `cannot remove ${missing}: ENOENT`],
    });
});

test("list a directory's entries by name, and every entry below it by relative path when recursive", async () => {
    // write a file beside a subdirectory with a file inside
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    const directory = await files.makeTemporaryDirectory("list-");
    await files.mkdir(join(directory, "nested"));
    await files.writeTextFile(join(directory, "top.txt"), "top");
    await files.writeTextFile(join(directory, "nested", "inner.txt"), "inner");

    expect({
        flat: sorted(await files.readDirectory(directory)),
        recursive: sorted(await files.readDirectory(directory, { recursive: true })),
    }).toEqual({
        flat: [
            { name: "nested", kind: "directory" },
            { name: "top.txt", kind: "file" },
        ],
        recursive: [
            { name: "nested", kind: "directory" },
            { name: "nested/inner.txt", kind: "file" },
            { name: "top.txt", kind: "file" },
        ],
    });
});

test("read and write a file at its cursor after a seek, append at its end, and flush a directory", async () => {
    // write a file, then overwrite one byte at an offset and append after it
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    const directory = await files.makeTemporaryDirectory("cursor-");
    const path = join(directory, "note.txt");
    await files.writeTextFile(path, "hello world");
    {
        await using file = await files.open(path, { read: true, write: true });
        await file.seek(5);
        await file.writable().getWriter().write(new TextEncoder().encode("!"));
    }
    {
        await using file = await files.open(path, { append: true });
        await file.writable().getWriter().write(new TextEncoder().encode(" again"));
    }

    // read from an offset into a buffer and as a stream, and flush the directory's entries
    await using reader = await files.open(path, { read: true });
    const offset = await reader.seek(6);
    const buffer = new Uint8Array(5);
    const counted = await reader.read(buffer);
    const tail = await new Response(reader.readable()).text();
    const end = await reader.read(buffer);
    await using entries = await files.open(directory, { read: true });
    await entries.sync();

    expect({
        whole: await files.readTextFile(path),
        offset,
        counted,
        word: new TextDecoder().decode(buffer),
        tail,
        end,
    }).toEqual({
        whole: "hello!world again",
        offset: 6,
        counted: 5,
        word: "world",
        tail: " again",
        end: null,
    });
});

test("create a temporary directory inside a given directory", async () => {
    // create one directory inside another temporary one
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    const parent = await files.makeTemporaryDirectory("parent-");
    const child = await files.makeTemporaryDirectory("child-", { directory: parent });

    expect(await files.readDirectory(parent)).toEqual([
        { name: child.slice(parent.length + 1), kind: "directory" },
    ]);
});

test("refuse reading below a directory without permissions as denied", async () => {
    // write a file into a directory, then take every permission from the directory
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    const directory = await files.makeTemporaryDirectory("denied-");
    const locked = join(directory, "locked");
    const file = join(locked, "file.txt");
    await files.mkdir(locked);
    await files.writeTextFile(file, "private");
    await files.chmod(locked, 0);

    // refuse the status read and the exists check, then give the permissions back
    try {
        expect([await refusal(files.stat(file)), await refusal(files.exists(file))]).toEqual([
            ["FORBIDDEN", "PERMISSION_DENIED", `cannot read ${file}: EACCES`],
            ["FORBIDDEN", "PERMISSION_DENIED", `cannot read ${file}: EACCES`],
        ]);
    } finally {
        await files.chmod(locked, 0o700);
    }
});

test("watch a directory from the call on, reading a new file as created, a write as modified and a deletion as removed, until aborted", async () => {
    // watch a temporary directory of the host's files
    await using simulation = new Simulation({ seed: 1, start: START });
    const { files } = simulation.environment("host");
    const directory = await files.makeTemporaryDirectory("watch-");
    const stopping = new AbortController();
    const events = files.watch(directory, { signal: stopping.signal })[Symbol.asyncIterator]();
    const next = async (): Promise<FileEvent | undefined> => {
        const read = await events.next();

        return read.done === true ? undefined : read.value;
    };

    // create, write and remove one file, the first before any read, reading each event as it arrives
    const path = join(directory, "note.txt");
    await new BunFileSystem().writeTextFile(path, "draft");
    const created = await next();
    await new BunFileSystem().writeTextFile(path, "final", { append: true });
    const modified = await next();
    await new BunFileSystem().remove(path);
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
