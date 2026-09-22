import { BuildCompiler } from "./compiler.ts";
import { BuildError } from "../error/index.ts";
import type { BuildRequest, BuildResponse } from "./message.ts";
import { readMessages, writeMessage } from "./message.ts";
import { Console } from "node:console";

/** Build sequential requests with one retained compiler. */
async function main(): Promise<void> {
    // reserve stdout for framed build results and send package logs to diagnostics
    Object.assign(
        globalThis.console,
        new Console({ stdout: process.stderr, stderr: process.stderr }),
    );
    console.write = (...messages) => {
        const bytes = Buffer.concat(
            messages.map((message) =>
                typeof message === "string"
                    ? Buffer.from(message)
                    : ArrayBuffer.isView(message)
                      ? Buffer.from(message.buffer, message.byteOffset, message.byteLength)
                      : Buffer.from(message),
            ),
        );
        process.stderr.write(bytes);

        return bytes.length;
    };
    const directory = process.argv[2];
    await using compiler = new BuildCompiler(directory);
    for await (const message of readMessages(process.stdin)) {
        let response: BuildResponse;
        try {
            const request = message as BuildRequest;
            if (request.kind === "build") {
                const build = await compiler.build(
                    { ...request.options, directory },
                    request.destination,
                );
                response = {
                    result: { kind: "build", manifest: build.manifest },
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
