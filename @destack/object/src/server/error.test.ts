import { testSimulation } from "../test/world.ts";
import { Simulation } from "@destack/environment/simulation";
import { Package } from "@destack/package";
import { schema } from "@destack/schema";
import { expect, refusal, test } from "@destack/test";
import { FileSystemError } from "@destack/environment";
import { AccessError } from "@destack/access";
import { startTelemetry } from "@destack/telemetry/bun";
import { createClient } from "../client/index.ts";
import { Health } from "./health.ts";
import { defineProcedure } from "../procedure/index.ts";
import { reportError } from "../error/report.ts";
import { fixturePackage } from "../test/fixture.ts";
import { implement, RequestHandler } from "./handler.ts";

/** Declare a public read procedure of the fixture package at a path. */
function declared(path: `/${string}`) {
    return defineProcedure({
        package: Package.of(fixturePackage),
        authentication: "public",
        permission: "anyone",
        audit: false,
    })
        .route({ method: "GET", path })
        .output(schema.string());
}

test("capture a handler's unknown failure once where it happens, and never again where a client handles the server's answer", async ({
    onTestFinished,
}) => {
    const world = testSimulation().environment("test");
    // record the message of each captured exception, in a world
    await using simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 24) });
    const captured: unknown[] = [];
    const telemetry = await startTelemetry(
        {
            name: "notes",
            version: "2026.9.0",
            report: (error) => {
                throw error;
            },
            metrics: {},
            signals: () => ({
                traces: {},
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
            }),
        },
        simulation.environment("server"),
    );
    onTestFinished(() => telemetry.shutdown());
    const environment = { ...world, telemetry };

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
        environment,
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
        environment,
    );

    // report the answer where the client handles it
    const answered = await client.read().then(
        () => undefined,
        (error: unknown) => error,
    );
    const reported = reportError(answered, telemetry);

    // capture the failure in the server only
    expect([captured, reported.status, reported.message]).toEqual([
        ["the disk is full"],
        500,
        "internal server error",
    ]);
});

test("answer a host family's failure as an internal error without its message, capturing it, and pass a caller family's on", async ({
    onTestFinished,
}) => {
    // record the message of each captured exception, in a world
    const world = testSimulation().environment("test");
    await using simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 24) });
    const captured: unknown[] = [];
    const telemetry = await startTelemetry(
        {
            name: "notes",
            version: "2026.9.0",
            report: (error) => {
                throw error;
            },
            metrics: {},
            signals: () => ({
                traces: {},
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
            }),
        },
        simulation.environment("server"),
    );
    onTestFinished(() => telemetry.shutdown());
    const environment = { ...world, telemetry };

    // fail one handler with a file system failure naming a host path, and another with another package's refusal for callers
    const procedures = { foreign: declared("/foreign"), own: declared("/own") };
    const implementation = implement(procedures);
    const router = implementation.router({
        foreign: implementation.foreign.handler(async () => {
            throw FileSystemError.of("NOT_FOUND", "cannot open /machine/files/secret");
        }),
        own: implementation.own.handler(async () => {
            throw AccessError.of("ROLE_NOT_FOUND", "role not found");
        }),
    });
    const handler = new RequestHandler(router, {
        package: fixturePackage,
        environment,
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
        environment,
    );

    // answer the file system failure as an internal error, captured, and the caller family's refusal as it is
    expect({
        foreign: await refusal(client.foreign()),
        own: await refusal(client.own()),
        captured,
    }).toEqual({
        foreign: ["INTERNAL_SERVER_ERROR", "INTERNAL_SERVER_ERROR", "internal server error"],
        own: ["NOT_FOUND", "ROLE_NOT_FOUND", "role not found"],
        captured: ["cannot open /machine/files/secret"],
    });
});
