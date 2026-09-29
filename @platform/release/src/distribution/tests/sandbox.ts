import { Sandbox } from "@destack/sandbox";
import { join } from "node:path";
import { text } from "node:stream/consumers";

/** Isolated files prepared by the executable verification. */
const directory = process.argv[2];
if (!directory) {
    throw new Error("missing sandbox directory");
}

/** Compiled sandbox process exercising the embedded Bun runtime. */
await using sandbox = await Sandbox.start({
    executable: process.execPath,
    arguments: [
        "--no-env-file",
        "-e",
        `
        import { readFile } from "node:fs/promises";
        let denied;
        try { await readFile(${JSON.stringify(join(directory, "private.txt"))}); }
        catch (error) { denied = error.code; }
        console.log(JSON.stringify({allowed: await readFile(${JSON.stringify(join(directory, "readable.txt"))}, "utf8"), denied}));
    `,
    ],
    directory,
    environment: { BUN_BE_BUN: "1" },
    read: [join(directory, "readable.txt")],
    write: [],
    network: [],
});
/** Captured sandbox result. */
const output = text(sandbox.stdout);
/** Captured sandbox diagnostics. */
const error = text(sandbox.stderr);
/** Exit status of the sandbox process. */
const exit = await sandbox.exited;
if (exit.code !== 0) {
    throw new Error(`sandbox failed: ${await error}`);
}
process.stdout.write(await output);
