import { expect, onTestFinished, test, refusal } from "@destack/test";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
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
    expect(await refusal(readOptional(directory))).toEqual([
        "INTERNAL_SERVER_ERROR",
        "FILE_SYSTEM_FAILED",
        `cannot read ${directory}: EISDIR`,
    ]);
});
