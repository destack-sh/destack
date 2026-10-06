import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { expect, test } from "@destack/test";

/** The package directory, whose preload a child process runs with. */
const root = fileURLToPath(new URL("../..", import.meta.url));

/** A process writing each logs export to its output that fails as its environment's failure asks. */
const failing = `
import { startTelemetry } from ${JSON.stringify(fileURLToPath(new URL("index.ts", import.meta.url)))};
import { OtlpExporter } from ${JSON.stringify(fileURLToPath(new URL("../otlp/index.ts", import.meta.url)))};

const exporter = new OtlpExporter(async (signal, body) => {
    if (signal === "logs") {
        process.stdout.write(new TextDecoder().decode(body) + "\\n");
    }
}, () => {});
await startTelemetry(exporter.options({ name: "@example/notes", version: "2026.9.0" }));
if (process.env.FAILURE === "rejection") {
    void Promise.reject(new TypeError("note sync rejected"));
} else {
    setTimeout(() => {
        throw new RangeError("note index out of range");
    });
}
`;

/** Run the failing process to its end. */
async function fail(failure: "exception" | "rejection") {
    const child = spawn(process.execPath, ["-e", failing], {
        cwd: root,
        env: { ...process.env, FAILURE: failure },
    });
    let output = "";
    let errors = "";
    child.stdout.on("data", (chunk: Buffer) => (output += chunk.toString()));
    child.stderr.on("data", (chunk: Buffer) => (errors += chunk.toString()));
    const code = await new Promise<number | null>((resolve) => {
        child.on("close", resolve);
    });

    return {
        code,
        exports: output
            .split("\n")
            .filter((line) => line !== "")
            .map((line): unknown => JSON.parse(line)),
        isReported: errors.includes(failure === "exception" ? "note index" : "note sync"),
    };
}

/** The resource of the release. */
const resource: unknown = expect.objectContaining({
    attributes: [
        { key: "service.name", value: { stringValue: "@example/notes" } },
        { key: "service.version", value: { stringValue: "2026.9.0" } },
    ],
});

/** The logs export of one captured failure of the release. */
function captured(type: string, message: string) {
    const attributes: unknown = expect.arrayContaining([
        { key: "exception.type", value: { stringValue: type } },
        { key: "exception.message", value: { stringValue: message } },
    ]);

    return {
        resourceLogs: [
            {
                resource,
                scopeLogs: [
                    {
                        scope: { name: "@example/notes", version: "2026.9.0" },
                        logRecords: [
                            expect.objectContaining({
                                severityNumber: 17,
                                severityText: "ERROR",
                                eventName: "exception",
                                attributes,
                            }),
                        ],
                    },
                ],
            },
        ],
    };
}

test("export a process's uncaught exception and unhandled rejection with its release, then end it as it would have ended", async () => {
    expect([await fail("exception"), await fail("rejection")]).toEqual([
        {
            code: 1,
            exports: [captured("RangeError", "note index out of range")],
            isReported: true,
        },
        { code: 1, exports: [captured("TypeError", "note sync rejected")], isReported: true },
    ]);
}, 20_000);
