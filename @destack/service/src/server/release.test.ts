import { expect, test } from "@destack/test";
import { schema } from "@destack/schema";
import { Expression } from "@destack/db/query";
import { defineService } from "../declare/index.ts";
import { Health } from "../health/index.ts";
import { defineProcedure } from "../service/index.ts";
import { implement } from "./handler.ts";
import { Server } from "./server.ts";
import { hosting } from "./tests/fixture.ts";
import { VERSION_HEADER } from "../request/index.ts";

/** A search procedure whose input renamed text to query and gained a limit in 2026.9.0. */
const search = defineProcedure({
    authentication: "public",
    permission: null,
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

/** The search service, serving callers of 2026.8.0 and later. */
const service = defineService("search", { search, since: "2026.8.0" });

test("convert inputs of earlier releases, dropping the fields this release no longer declares, and refuse releases the service does not serve", async () => {
    // serve the search, echoing the input it receives
    const implementation = implement(service.router);
    await using server = Server.start({
        ...hosting,
        service,
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
        const body = await (await server.fetch(request)).json();

        return body.code === undefined ? body : `${body.code}: ${body.message}`;
    };

    // convert an earlier input, pass a current one, and refuse the rest
    expect([
        await call("2026.8.5", { text: "notes" }),
        await call(service.package.version, { query: "tasks", limit: 5 }),
        await call("2026.10.0", { query: "tasks", limit: 5 }),
        await call("2026.7.0", { text: "notes" }),
        await call(undefined, { query: "tasks", limit: 5 }),
        await call("next", { query: "tasks", limit: 5 }),
    ]).toEqual([
        { query: "notes", limit: 50 },
        { query: "tasks", limit: 5 },
        `CONFLICT: service search serves ${service.package.version}, the caller speaks 2026.10.0`,
        "CONFLICT: service search no longer serves releases before 2026.8.0",
        "BAD_REQUEST: service search requires Destack-Version",
        "BAD_REQUEST: invalid Destack-Version: next",
    ]);
});
