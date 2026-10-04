import { BuildCompiler } from "./compiler.ts";
import { BuildError } from "../error/index.ts";
import { BuildRequest, type BuildResponse, readMessages, writeMessage } from "./message.ts";
import { Console } from "node:console";

/** Answer sequential requests on standard input with one retained compiler for the package directory the arguments name. */
export async function runCompiler(): Promise<void> {
    // reserve stdout for framed build results and send package logs to diagnostics
    Object.assign(
        globalThis.console,
        new Console({ stdout: process.stderr, stderr: process.stderr }),
        { write: writeDiagnostics },
    );
    const directory = process.argv[2];
    if (directory === undefined) {
        throw new BuildError("BUILD_FAILED", "the compiler needs a package directory");
    }
    await using compiler = new BuildCompiler(directory);
    for await (const request of readMessages(process.stdin, BuildRequest)) {
        await writeMessage(process.stdout, await respond(compiler, directory, request));
    }
}

/** Run one request, answering its result or its failure. */
async function respond(
    compiler: BuildCompiler,
    directory: string,
    request: BuildRequest,
): Promise<BuildResponse> {
    try {
        // plan a build's keys
        if (request.kind === "plan") {
            const keys = await compiler.plan({ ...request.options, directory });

            return { result: { kind: "plan", keys } };
        }
        // build the package, reusing the cached outputs
        else if (request.kind === "build") {
            const { build, outputs } = await compiler.build(
                { ...request.options, directory },
                request.reuse,
                request.destination,
            );

            return { result: { kind: "build", manifest: build.manifest, outputs } };
        }
        // inspect the package
        else {
            const inspection = await compiler.inspect({ ...request.options, directory });

            return { result: { kind: "inspect", inspection } };
        }
    } catch (error) {
        return { error: describeFailure(error) };
    }
}

/** Describe a failure for the host, keeping a build failure's code, its cause and its stack. */
function describeFailure(error: unknown): Extract<BuildResponse, { error: object }>["error"] {
    // describe a thrown value that is no error by its text
    if (!(error instanceof Error)) {
        return { code: "BUILD_FAILED", message: String(error) };
    }

    return {
        code: error instanceof BuildError ? error.code : "BUILD_FAILED",
        message: error.message,
        cause: cloneableCause(error.cause),
        ...(error.stack === undefined ? {} : { stack: error.stack }),
    };
}

/** Keep a failure's cause as the message carries it, its text when it cannot be cloned. */
function cloneableCause(cause: unknown): unknown {
    try {
        return structuredClone(cause);
    } catch (error) {
        if (!(error instanceof DOMException && error.name === "DataCloneError")) {
            throw error;
        }

        return cause instanceof Error ? cause.message : String(cause);
    }
}

/** Write console output to diagnostics, returning the bytes written. */
function writeDiagnostics(...messages: (string | ArrayBufferView | ArrayBuffer)[]): number {
    const bytes = Buffer.concat(messages.map((message) => toBuffer(message)));
    process.stderr.write(bytes);

    return bytes.length;
}

/** View console output as bytes. */
function toBuffer(message: string | ArrayBufferView | ArrayBuffer): Buffer {
    // encode text
    if (typeof message === "string") {
        return Buffer.from(message);
    }
    // view a typed array's bytes
    else if (ArrayBuffer.isView(message)) {
        return Buffer.from(message.buffer, message.byteOffset, message.byteLength);
    }
    // view a buffer's bytes
    else {
        return Buffer.from(message);
    }
}
