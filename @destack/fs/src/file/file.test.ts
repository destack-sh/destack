import { expect, onTestFinished, test } from "@destack/test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { FileSystemError } from "../error/index.ts";
import { readOptional } from "./file.ts";

test("read a file's text, nothing for a missing file, and fail on any other error", async () => {
    // hold one file in a temporary directory
    const directory = await mkdtemp(join(tmpdir(), "destack-read-"));
    onTestFinished(() => rm(directory, { recursive: true }));
    await writeFile(join(directory, "present.json"), "{}");

    // read the file, answer nothing for a missing one, and refuse reading a directory as a file
    expect([
        await readOptional(join(directory, "present.json")),
        await readOptional(join(directory, "missing.json")),
    ]).toEqual(["{}", undefined]);
    const failure = await readOptional(directory).catch((error: unknown) => error);
    expect(failure).toBeInstanceOf(FileSystemError);
    expect(failure).toMatchObject({
        operation: "read",
        path: directory,
        code: "EISDIR",
        message: `cannot read ${directory}: EISDIR`,
        cause: { code: "EISDIR" },
    });
});
