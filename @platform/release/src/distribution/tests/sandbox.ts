import { Sandbox } from "@destack/sandbox";
import { BunEnvironment } from "@destack/environment/bun";
import { join } from "node:path";
import { text } from "node:stream/consumers";

/** The host the sandbox starts on. */
const environment = BunEnvironment.system;

/** Isolated files prepared by the executable verification. */
const [directory] = environment.process.arguments;
if (directory === undefined || directory === "") {
    throw new Error("missing sandbox directory");
}

/** Compiled sandbox process exercising the embedded Bun runtime. */
await using sandbox = await Sandbox.start(
    {
        executable: environment.process.executable,
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
        sockets: [],
        listening: "denied",
    },
    environment,
);
/** Captured sandbox result. */
const output = text(sandbox.stdout);
/** Captured sandbox diagnostics. */
const error = text(sandbox.stderr);
/** Exit status of the sandbox process. */
const exit = await sandbox.exited;
if (exit.code !== 0) {
    throw new Error(`sandbox failed: ${await error}`);
}
environment.process.stdout.writeSync(await output);
