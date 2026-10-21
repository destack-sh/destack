import { Package } from "@destack/package";
import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Expression } from "@destack/db";
import { PackageHandle } from "../declare/index.ts";
import { Health } from "./health.ts";
import { defineProcedure, VERSION_HEADER } from "../procedure/index.ts";
import { implement } from "./handler.ts";
import { Server } from "./http.ts";
import { fixturePackage, hosting } from "../test/fixture.ts";
import { Clock } from "@destack/environment";

/** A search procedure whose input renamed text to query and gained a limit in 2026.9.0. */
const search = defineProcedure({
    package: Package.of(fixturePackage),
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
const Failure = schema.object({ code: schema.string(), message: schema.string() }).loose();

/** The fixture package, serving callers of 2026.8.0 and later. */
const served = PackageHandle.of(fixturePackage, { since: "2026.8.0" });

test("convert inputs of earlier releases, dropping the fields this release no longer declares, and refuse releases the package does not serve", async () => {
    // serve a search that echoes its input
    const implementation = implement({ search });
    await using server = Server.start({
        clock: Clock.system,
        ...hosting,
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

        return failure.success ? `${failure.data.code}: ${failure.data.message}` : body;
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
        `CONFLICT: package @destack/fixture serves ${served.version}, but the caller uses 2026.10.0`,
        "CONFLICT: package @destack/fixture no longer serves releases before 2026.8.0",
        "BAD_REQUEST: package @destack/fixture requires Destack-Version",
        "BAD_REQUEST: invalid Destack-Version: next",
    ]);
});

test("speak the release of the package defining a procedure, which the served package's retirement leaves alone", async () => {
    // serve another package's echo procedure from the fixture package, at a later release than the fixture's
    const protocol = Package.parse({
        id: "package-019f7480-0000-7000-8000-000000000002",
        name: "@destack/protocol",
        version: "2026.11.0",
    });
    const echo = defineProcedure({
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
        clock: Clock.system,
        ...hosting,
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

        return failure.success ? `${failure.data.code}: ${failure.data.message}` : body;
    };

    // answer the protocol's releases, an old one beside the fixture's retirement, and refuse a newer one
    expect([await call("2026.11.0"), await call("2026.7.0"), await call("2026.12.0")]).toEqual([
        { text: "hi" },
        { text: "hi" },
        "CONFLICT: package @destack/protocol serves 2026.11.0, but the caller uses 2026.12.0",
    ]);
});
