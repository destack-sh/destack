import { testSimulation } from "../test/world.ts";
import { PackageVersion } from "@destack/package";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Expression } from "@destack/db";
import { Package } from "../declare/index.ts";
import { Health } from "./health.ts";
import { Procedure, VERSION_HEADER } from "../procedure/index.ts";
import { implement } from "./handler.ts";
import { Server } from "./http.ts";
import { fixturePackage, hosting, authenticateByName } from "../test/fixture.ts";

/** A search procedure whose input renamed text to query and gained a limit in 2026.9.0. */
const search = Procedure.of({
    package: PackageVersion.of(fixturePackage),
    authentication: "public",
    permission: "anyone",
    audit: false,
    convert: {
        "2026.9.0": {
            query: Expression.column("text"),
            limit: Expression.coalesce(Expression.column("limit"), Expression.literal(50)),
        },
    },
})
    .route({ method: "POST", path: "/search" })
    .input(schema.object({ query: schema.string(), limit: schema.number().int() }))
    .output(schema.object({ query: schema.string(), limit: schema.number().int() }));

/** The body of a refused call. */
const Failure = schema
    .object({ code: schema.string(), reason: schema.string(), message: schema.string() })
    .loose();

/** The fixture package, serving callers of 2026.8.0 and later. */
const served = Package.of(fixturePackage, { since: "2026.8.0" });

test("convert inputs of earlier releases, dropping the fields this release no longer declares, and refuse releases the package does not serve", async () => {
    const world = testSimulation().environment("test");
    // serve a search that echoes its input
    const implementation = implement({ search });
    await using server = Server.start({
        environment: world,
        ...hosting,
        authenticate: authenticateByName(world.clock),
        package: served,
        router: implementation.router({
            search: implementation.search.handler(({ input }) => ({
                query: input.query,
                limit: input.limit,
            })),
        }),
        health: new Health("search"),
        drainTimeout: 1000,
    });
    const call = async (release: string | undefined, input: unknown) => {
        const request = new Request("https://test.local/search", {
            method: "POST",
            headers: {
                "content-type": "application/json",
                authorization: "alice",
                ...(release === undefined ? {} : { [VERSION_HEADER]: release }),
            },
            body: JSON.stringify(input),
        });
        const body: unknown = await (await server.fetch(request)).json();
        const failure = Failure.safeParse(body);

        return failure.success
            ? [failure.data.code, failure.data.reason, failure.data.message]
            : body;
    };

    // convert an earlier input
    expect([
        await call("2026.8.5", { text: "notes" }),
        await call(served.version, { query: "tasks", limit: 5 }),
        await call("2026.10.0", { query: "tasks", limit: 5 }),
        await call("2026.7.0", { text: "notes" }),
        await call(undefined, { query: "tasks", limit: 5 }),
        await call("next", { query: "tasks", limit: 5 }),
    ]).toEqual([
        { query: "notes", limit: 50 },
        { query: "tasks", limit: 5 },
        [
            "PRECONDITION_FAILED",
            "VERSION_BLOCKED",
            `cannot call package '@destack/fixture': it serves '${served.version}', but the caller uses '2026.10.0'`,
        ],
        [
            "PRECONDITION_FAILED",
            "VERSION_BLOCKED",
            "cannot call package '@destack/fixture': it no longer serves releases before '2026.8.0'",
        ],
        [
            "BAD_REQUEST",
            "VERSION_REQUIRED",
            "'Destack-Version' for package '@destack/fixture' is required",
        ],
        [
            "BAD_REQUEST",
            "VERSION_INVALID",
            "Destack-Version 'next' is invalid: it must be a version",
        ],
    ]);
});

test("speak the release of the package defining a procedure, which the served package's retirement leaves alone", async () => {
    const world = testSimulation().environment("test");
    // serve another package's echo procedure from the fixture package, at a later release than the fixture's
    const protocol = PackageVersion.parse({
        id: "package-019f7480-0000-7000-8000-000000000002",
        name: "@destack/protocol",
        version: "2026.11.0",
    });
    const echo = Procedure.of({
        package: protocol,
        authentication: "public",
        permission: "anyone",
        audit: false,
    })
        .route({ method: "POST", path: "/echo" })
        .input(schema.object({ text: schema.string() }))
        .output(schema.object({ text: schema.string() }));
    const implementation = implement({ echo });
    await using server = Server.start({
        environment: world,
        ...hosting,
        authenticate: authenticateByName(world.clock),
        package: served,
        router: implementation.router({
            echo: implementation.echo.handler(({ input }) => input),
        }),
        health: new Health("echo"),
        drainTimeout: 1000,
    });
    const call = async (release: string) => {
        const request = new Request("https://test.local/echo", {
            method: "POST",
            headers: {
                "content-type": "application/json",
                authorization: "alice",
                [VERSION_HEADER]: release,
            },
            body: JSON.stringify({ text: "hi" }),
        });
        const body: unknown = await (await server.fetch(request)).json();
        const failure = Failure.safeParse(body);

        return failure.success
            ? [failure.data.code, failure.data.reason, failure.data.message]
            : body;
    };

    // answer the protocol's releases, an old one beside the fixture's retirement, and refuse a newer one
    expect([await call("2026.11.0"), await call("2026.7.0"), await call("2026.12.0")]).toEqual([
        { text: "hi" },
        { text: "hi" },
        [
            "PRECONDITION_FAILED",
            "VERSION_BLOCKED",
            "cannot call package '@destack/protocol': it serves '2026.11.0', but the caller uses '2026.12.0'",
        ],
    ]);
});
