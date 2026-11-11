import { PackageId } from "@destack/package";
import { found } from "@destack/schema";
import { ROUTE_HEADER } from "@destack/object/workload";
import { route } from "@destack/stack/object";
import { TestRoute } from "@destack/stack/test";
import { expect, refusal, test } from "@destack/test";
import { type RouteAction, RouteRequest } from "./route.ts";

/** The package of the routed installation, which declares the notes mount. */
const NOTES_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000b1");

/** Read an action as what it sends where, and the response headers it sets. */
async function described(action: RouteAction | undefined) {
    // read a response by its status and location
    if (action === undefined) {
        return undefined;
    } else if (action.kind === "response") {
        return [action.response.status, action.response.headers.get("location")];
    }

    // read a forwarded request by its method, URL, marked header and cookie, and its response's header
    const marked = action.request.headers.get("x-route");
    const cookie = action.request.headers.get("cookie");
    const response = action.respond(new Response(null));
    const header = response.headers.get("cache-control");
    const forwarded = [
        action.kind,
        action.request.method,
        action.request.url,
        marked,
        cookie,
        header,
    ];

    return action.kind === "view" ? [...forwarded, action.installation, action.mount] : forwarded;
}

/** Act on a request the machine routed to the space by its route's rules. */
test("act on routed requests by their route's most specific rule: redirect, rewrite, change headers, and hand them to a mount, views or another site", async () => {
    await using fixture = await TestRoute.open();
    const [first] = fixture.spaceIds;
    const hostname = found(fixture.hostnames, fixture.accountId).name;
    const installation = { id: found(fixture.installations, first) };

    // declare an API whose writes and previews go to their own rules, a moved blog, a docs mirror and the home view
    await fixture.apply(first, {
        api: {
            hostname,
            path: "/api",
            match: "prefix",
            installation,
            rules: [
                {
                    entry: { kind: "mount", mount: "notes" },
                    filters: [{ kind: "response-headers", set: { "cache-control": "max-age=60" } }],
                },
                {
                    matches: { method: "POST" },
                    filters: [{ kind: "request-headers", set: { "x-route": "write" } }],
                    entry: { kind: "mount", mount: "notes" },
                },
                {
                    matches: {
                        method: "POST",
                        headers: [{ name: "x-preview", value: "1", kind: "exact" }],
                    },
                    filters: [{ kind: "rewrite", path: { kind: "prefix", value: "/preview" } }],
                    entry: { kind: "mount", mount: "notes" },
                },
            ],
        },
        old: {
            hostname,
            path: "/blog",
            match: "prefix",
            rules: [
                {
                    filters: [
                        {
                            kind: "redirect",
                            scheme: "https",
                            hostname: "blog.example",
                            path: { kind: "prefix", value: "/posts" },
                            status: 308,
                        },
                    ],
                },
            ],
        },
        docs: {
            hostname,
            path: "/docs",
            match: "prefix",
            rules: [
                {
                    matches: { query: [{ name: "v", value: "^[0-9]+$", kind: "regex" }] },
                    entry: { kind: "url", url: "https://docs.example/destack/" },
                },
            ],
        },
        home: {
            hostname,
            path: "/",
            match: "prefix",
            installation,
            rules: [{ entry: { kind: "view", view: "notes" } }],
        },
    });
    const rows = await fixture
        .database(first)
        .select({ id: route.table.id, managerName: route.table.managerName })
        .from(route.table);
    const declared = (name: string) =>
        found(new Map(rows.map((row) => [row.managerName, row.id])), name);

    // act on a request as routed to a route, the installation declaring the notes mount at /feed
    const mount = {
        installationId: installation.id,
        packageId: NOTES_PACKAGE,
        name: "notes",
        path: "/feed",
    };
    const act = async (
        name: string | undefined,
        path: string,
        init: RequestInit = {},
        mounts = [mount],
    ) => {
        const headers = new Headers(init.headers);
        if (name !== undefined) {
            headers.set(ROUTE_HEADER, declared(name));
        }
        const request = new Request(`https://eu.destack.cloud${path}`, { ...init, headers });

        return described(
            await RouteRequest.mount(fixture.database(first), first, request, () => mounts),
        );
    };

    // pick each request's most specific rule, then serve its entry below the route's path
    const feed = `https://eu.destack.cloud/spaces/${first}/installations/${installation.id}/${NOTES_PACKAGE}/feed`;
    expect([
        await act("api", "/api/notes?page=2"),
        await act("api", "/api/notes", { method: "POST" }),
        await act("api", "/api/notes", { method: "POST", headers: { "x-preview": "1" } }),
        await act("old", "/blog/2026/hello?ref=x"),
        await act("docs", "/docs/start?v=2", { headers: { cookie: "__Host-session=1" } }),
        await act("docs", "/docs/start?v=next"),
        await act("home", "/notes/1", { headers: { cookie: "__Host-session=1" } }),
        await act(undefined, "/api/notes"),
        await refusal(act("api", "/api/notes", {}, [])),
    ]).toEqual([
        ["mount", "GET", `${feed}/notes?page=2`, null, null, "max-age=60"],
        ["mount", "POST", `${feed}/notes`, "write", null, null],
        ["mount", "POST", `${feed}/preview/notes`, null, null, null],
        [308, "https://blog.example/posts/2026/hello?ref=x"],
        ["url", "GET", "https://docs.example/destack/start?v=2", null, null, null],
        [404, null],
        [
            "view",
            "GET",
            "https://eu.destack.cloud/notes/1",
            null,
            "__Host-session=1",
            null,
            installation.id,
            { base: "", view: "notes" },
        ],
        undefined,
        [
            "NOT_FOUND",
            "ROUTE_NOT_FOUND",
            `mount 'notes' of route '${declared("api")}' in installation '${installation.id}' not found`,
        ],
    ]);
});
