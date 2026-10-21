import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { startTelemetry } from "@destack/telemetry/bun";
import { createClient } from "../client/index.ts";
import { Health } from "./health.ts";
import { defineProcedure } from "../procedure/index.ts";
import { reportError } from "../error/report.ts";
import { fixturePackage } from "../test/fixture.ts";
import { implement, RequestHandler } from "./handler.ts";

test("capture a handler's unknown failure once where it happens, and never again where a client handles the server's answer", async ({
    onTestFinished,
}) => {
    // record the message of each captured exception
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
    const procedures = {
        read: defineProcedure({
            package: Package.of(fixturePackage),
            authentication: "public",
            permission: "anyone",
            audit: false,
        })
            .route({ method: "GET", path: "/read" })
            .output(schema.string()),
    };
    const implementation = implement(procedures);
    const router = implementation.router({
        read: implementation.read.handler(async () => {
            throw new Error("the disk is full");
        }),
    });
    const handler = new RequestHandler(router, {
        package: fixturePackage,
        health: new Health("read"),
    });
    const client = createClient(
        { router: procedures },
        {
            url: "https://test.local",
            fetch: async (request) => {
                const result = await handler.handle(request, { context: { request } });

                return result.matched ? result.response : new Response(null, { status: 404 });
            },
        },
    );

    // report the answer where the client handles it
    const answered = await client.read().then(
        () => undefined,
        (error: unknown) => error,
    );
    const reported = reportError(answered);

    // capture the failure in the server only
    expect([captured, reported.status, reported.message]).toEqual([
        ["the disk is full"],
        500,
        "internal server error",
    ]);
});
