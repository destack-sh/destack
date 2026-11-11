import { asc } from "@destack/db";
import { Claim } from "@destack/object";
import { found, present } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { expect, test } from "@destack/test";
import { Route, route } from "../../object/index.ts";
import type { StackInput } from "../../declare/stack.ts";
import { TestRoute } from "../../test/index.ts";

/** A route as a stack declares it. */
type RouteInput = NonNullable<StackInput["routes"]>[string];

/** Serve each path of an account's hostnames once across the universe. */
test("reconcile a space's routes on its account's hostnames, one per hostname path across the universe", async () => {
    await using fixture = await TestRoute.open();
    const [first, second, rival] = fixture.spaceIds;
    const hostname = found(fixture.hostnames, fixture.accountId);
    const home = (spaceId: string, on = hostname.name): RouteInput => ({
        hostname: on,
        path: "/",
        match: "prefix",
        installation: { id: found(fixture.installations, spaceId) },
        rules: [{ entry: { kind: "view", view: "notes" } }],
    });
    const moved = {
        kind: "redirect",
        scheme: "https",
        hostname: "blog.example",
        path: { kind: "full", value: "/" },
        status: 308,
    } as const;
    const old: RouteInput = {
        hostname: hostname.name,
        path: "/blog",
        match: "exact",
        rules: [{ filters: [moved] }],
    };

    // serve the space's installation and a redirect on the hostname
    await fixture.apply(first, { home: home(first), old });

    // refuse another space's installation, the same path in another space of the account, and rules that cannot serve
    const invalid = (rules: RouteInput["rules"]): RouteInput => ({
        ...old,
        path: "/invalid",
        rules,
    });
    const refused = {
        installation: await fixture.submit(first, { home: home(second) }),
        path: await fixture.submit(second, { home: home(second) }),
        both: await fixture.submit(first, {
            both: invalid([{ filters: [moved], entry: { kind: "origin" } }]),
        }),
        uninstalled: await fixture.submit(first, {
            uninstalled: invalid([{ entry: { kind: "view", view: "notes" } }]),
        }),
        pattern: await fixture.submit(first, {
            pattern: invalid([
                {
                    matches: { query: [{ name: "v", value: "(", kind: "regex" }] },
                    entry: { kind: "url", url: "https://example.org/" },
                },
            ]),
        }),
    };

    // conceal another account's hostname as an unknown one, and take the same path on the other account's own hostname
    const unknown = "missing.example.com";
    const foreign = {
        hostname: await fixture.submit(rival, { home: home(rival) }),
        unknown: await fixture.submit(rival, { home: home(rival, unknown) }),
    };
    const own = found(fixture.hostnames, fixture.otherId);
    await fixture.apply(rival, { home: home(rival, own.name) });

    // allow one declared route per space and path, managed by its space's stack
    const declared = async (spaceId: string) =>
        (
            await fixture
                .database(spaceId)
                .select({
                    scope: route.table.scope,
                    name: route.table.managerName,
                    hostname: route.table.hostname,
                    path: route.table.path,
                    installationId: route.table.installationId,
                    rules: route.table.rules,
                })
                .from(route.table)
                .orderBy(asc(route.table.path))
        ).map(({ rules, ...row }) => [...Object.values(row), rules.length]);
    const routes = [await declared(first), await declared(second), await declared(rival)];

    // key each hostname path in the universe's index
    const owner = async (on: string) => {
        const { index, key } = Claim.key(route, "path", [on, "/", "prefix"]);

        return (await fixture.lookup.resolve({ index, keys: [key] })).get(key)?.owner;
    };
    expect({
        refused,
        foreign,
        routes,
        owners: [await owner(hostname.name), await owner(own.name)],
    }).toEqual({
        refused: {
            installation: [
                "failed",
                `@destack/object DECLARATION_INVALID: installation '${fixture.installations.get(second)}' is invalid: it is unknown`,
            ],
            path: ["failed", "@destack/object NAME_EXISTS: 'path' already exists"],
            both: [
                "failed",
                "@destack/stack ROUTE_INVALID: route rule is invalid: it must either redirect or have an entry",
            ],
            uninstalled: [
                "failed",
                "@destack/stack ROUTE_INVALID: route is invalid: a route serving an installation's view needs its installation",
            ],
            pattern: [
                "failed",
                "@destack/stack ROUTE_INVALID: pattern '(' for 'v' is invalid: it must be a URL pattern",
            ],
        },
        foreign: {
            hostname: [
                "failed",
                `@destack/stack HOSTNAME_NOT_FOUND: hostname '${hostname.name}' not found`,
            ],
            unknown: [
                "failed",
                `@destack/stack HOSTNAME_NOT_FOUND: hostname '${unknown}' not found`,
            ],
        },
        routes: [
            [
                [first, "home", hostname.name, "/", found(fixture.installations, first), 1],
                [first, "old", hostname.name, "/blog", null, 1],
            ],
            [],
            [[rival, "home", own.name, "/", found(fixture.installations, rival), 1]],
        ],
        owners: [first, rival],
    });
});

test("refuse a route on a machine the account's hostname requests do not reach", async () => {
    // serve the spaces on a machine the account's hostnames do not reach, as a cloud machine before the edge provisions them
    await using fixture = await TestRoute.open({ isServingHostnames: false });
    const [first] = fixture.spaceIds;
    const hostname = found(fixture.hostnames, fixture.accountId);
    const home: RouteInput = {
        hostname: hostname.name,
        path: "/",
        match: "prefix",
        installation: { id: found(fixture.installations, first) },
        rules: [{ entry: { kind: "view", view: "notes" } }],
    } as const;

    // apply a route there, which no request would reach
    expect(await fixture.submit(first, { home })).toEqual([
        "failed",
        `@destack/stack HOSTNAME_BLOCKED: cannot route '${hostname.name}': its requests do not reach this space's machine`,
    ]);
});

/** Find a request's route as a front door does. */
test("locate the route serving a host's path, the most specific first", async () => {
    await using fixture = await TestRoute.open();
    const [first] = fixture.spaceIds;
    const notes = found(fixture.hostnames, fixture.accountId).name;
    const wildcard = await fixture.serve(fixture.accountId, "*.example.com");
    const installation = { id: found(fixture.installations, first) };

    // declare a view, a view of an object, a whole origin, a redirect and a mount on the hostname, and one on its wildcard
    const roadmap = ObjectReference.parse({
        packageId: "package-01996ab0-0000-7000-8000-0000000000c1",
        type: "note",
        scope: first,
        id: "note-01996ab0-0000-7000-8000-0000000000aa",
    });
    await fixture.apply(first, {
        home: {
            hostname: notes,
            path: "/",
            match: "prefix",
            installation,
            rules: [{ entry: { kind: "view", view: "notes" } }],
        },
        roadmap: {
            hostname: notes,
            path: "/roadmap",
            match: "prefix",
            installation,
            rules: [{ entry: { kind: "view", view: "notes", object: roadmap } }],
        },
        docs: {
            hostname: notes,
            path: "/docs",
            match: "prefix",
            installation,
            rules: [{ entry: { kind: "origin" } }],
        },
        old: {
            hostname: notes,
            path: "/blog",
            match: "exact",
            rules: [
                {
                    filters: [{ kind: "redirect", hostname: "blog.example", status: 308 }],
                },
            ],
        },
        api: {
            hostname: notes,
            path: "/api",
            match: "prefix",
            installation,
            rules: [{ entry: { kind: "mount", mount: "notes" } }],
        },
        names: {
            hostname: wildcard.name,
            path: "/",
            match: "prefix",
            installation,
            rules: [{ entry: { kind: "mount", mount: "notes" } }],
        },
    });

    // read the route each host and path locate, none outside the hostnames
    const located = async (host: string, path: string) => {
        const served = await Route.locate(fixture.lookup, host, path);
        if (served === undefined) {
            return null;
        }
        const row = present(
            await Route.find(fixture.database(first), first, served.id),
            "the route",
        );

        return [served.scope, row.managerName];
    };
    expect([
        await located(notes, "/roadmap"),
        await located(notes, "/docs/guide"),
        await located(notes, "/blog"),
        await located(notes, "/blog/2026"),
        await located(notes, "/api/v1/notes"),
        await located("blog.example.com", "/anything"),
        await located("example.org", "/"),
    ]).toEqual([
        [first, "roadmap"],
        [first, "docs"],
        [first, "old"],
        [first, "home"],
        [first, "api"],
        [first, "names"],
        null,
    ]);
});
