import { BuildCompiler } from "./compiler.ts";
import { BuildError } from "../error/index.ts";
import type { BuildRequest, BuildResponse } from "./message.ts";
import { readMessages, writeMessage } from "./message.ts";
import { Console } from "node:console";

/** Build sequential requests with one retained compiler. */
async function main(): Promise<void> {
    // reserve stdout for framed build results and send package logs to diagnostics
    globalThis.console = new Console({ stdout: process.stderr, stderr: process.stderr });
    const directory = Deno.args[0];
    await using compiler = new BuildCompiler(directory);
    for await (const message of readMessages(process.stdin)) {
        let response: BuildResponse;
        try {
            const request = message as BuildRequest;
            if (request.kind === "build") {
                const build = await compiler.build({ ...request.options, directory });
                response = {
                    result: { kind: "build", manifest: build.manifest, files: build.files },
                };
            } else if (request.kind === "inspect") {
                const inspection = await compiler.inspect({ ...request.options, directory });
                response = { result: { kind: "inspect", inspection } };
            } else {
                throw new BuildError("BUILD_FAILED", "unknown compiler request");
            }
        } catch (error) {
            response = {
                error: {
                    code: error instanceof BuildError ? error.code : "BUILD_FAILED",
                    message: error instanceof Error ? error.message : String(error),
                    cause: error instanceof Error ? error.cause : undefined,
                    stack: error instanceof Error ? error.stack : undefined,
                },
            };
        }
        await writeMessage(process.stdout, response);
    }
}

if (import.meta.main) {
    await main();
}
