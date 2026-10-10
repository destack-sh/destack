import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { startTelemetry } from "@destack/telemetry/bun";
import { createClient } from "../client/index.ts";
import { defineService } from "../declare/index.ts";
import { Health } from "../health/index.ts";
import { defineProcedure } from "../service/index.ts";
import { reportError } from "./error.ts";
import { implement, ServiceHandler } from "./handler.ts";

/** The service the handler serves, whose release the client speaks. */
const fixture = defineService("fixture", {});

test("capture a handler's unknown failure once where it happens, and never again where a client handles the service's answer", async ({
    onTestFinished,
}) => {
    // keep the message of each captured exception
    const captured: unknown[] = [];
    const telemetry = await startTelemetry({
        name: "notes",
        version: "2026.9.0",
        report: (error) => {
            throw error;
        },
        traces: {},
        metrics: {},
        logs: {
            processors: [
                {
                    onEmit: (record) => {
                        captured.push(record.attributes["exception.message"]);
                    },
                    forceFlush: async () => {},
                    shutdown: async () => {},
                },
            ],
        },
    });
    onTestFinished(() => telemetry.shutdown());

    // fail a handler
    const service = {
        read: defineProcedure({ authentication: "public", permission: "anyone", audit: false })
            .route({ method: "GET", path: "/read" })
            .output(schema.string()),
    };
    const implementation = implement(service);
    const router = implementation.router({
        read: implementation.read.handler(async () => {
            throw new Error("the disk is full");
        }),
    });
    const handler = new ServiceHandler(router, { service: fixture, health: new Health("read") });
    const client = createClient(defineService("fixture", service), {
        url: "https://test.local",
        fetch: async (request) => {
            const result = await handler.handle(request, { context: { request } });

            return result.matched ? result.response : new Response(null, { status: 404 });
        },
    });

    // report the answer where the client handles it
    const answered = await client.read().then(
        () => undefined,
        (error: unknown) => error,
    );
    const reported = reportError(answered);

    // capture the failure in the service only
    expect([captured, reported.status, reported.message]).toEqual([
        ["the disk is full"],
        500,
        "internal server error",
    ]);
});
