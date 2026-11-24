import { asc } from "@destack/db";
import { Claim } from "@destack/object";
import { found, present } from "@destack/schema";
import { ObjectReference } from "@destack/sync";
import { expect, test } from "@destack/test";
import { Route, route } from "@destack/stack/object";
import type { StackDocument } from "@destack/stack/declare";
import { TestRoute } from "../test/index.ts";

/** A route as a stack declares it. */
type RouteInput = NonNullable<StackDocument["routes"]>[string];

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

    // refuse another space's installation and rules that cannot serve, and declare the same path in another space of the account
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

    // read why the second space's route on the taken path is not ready
    await fixture.simulation.idle();
    const [taken] = await fixture
        .database(second)
        .select({ conditions: route.table.conditions })
        .from(route.table);
    const ready = taken?.conditions["Ready"];

    // key each hostname path in the universe's index
    const owner = async (on: string) => {
        const { index, key } = Claim.key(route, "path", [on, "/", "prefix"]);

        return (await fixture.lookup.resolve({ index, keys: [key] })).get(key)?.owner;
    };
    expect({
        refused,
        foreign,
        routes,
        taken: [ready?.status, ready?.reason, ready?.message],
        owners: [await owner(hostname.name), await owner(own.name)],
    }).toEqual({
        refused: {
            installation: [
                "BAD_REQUEST",
                "DECLARATION_INVALID",
                `installation '${fixture.installations.get(second)}' is invalid: it is unknown`,
            ],
            // declare the same path in another space, whose route waits for the name
            path: undefined,
            both: [
                "BAD_REQUEST",
                "ROUTE_INVALID",
                "route rule is invalid: it must either redirect or have an entry",
            ],
            uninstalled: [
                "BAD_REQUEST",
                "ROUTE_INVALID",
                "route is invalid: a route serving an installation's view needs its installation",
            ],
            pattern: [
                "BAD_REQUEST",
                "ROUTE_INVALID",
                "pattern '(' for 'v' is invalid: it must be a URL pattern",
            ],
        },
        foreign: {
            hostname: ["NOT_FOUND", "HOSTNAME_NOT_FOUND", `hostname '${hostname.name}' not found`],
            unknown: ["NOT_FOUND", "HOSTNAME_NOT_FOUND", `hostname '${unknown}' not found`],
        },
        routes: [
            [
                [first, "home", hostname.name, "/", found(fixture.installations, first), 1],
                [first, "old", hostname.name, "/blog", null, 1],
            ],
            [[second, "home", hostname.name, "/", found(fixture.installations, second), 1]],
            [[rival, "home", own.name, "/", found(fixture.installations, rival), 1]],
        ],
        taken: ["false", "NameTaken", `'${hostname.name} / prefix' is already taken`],
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
        "PRECONDITION_FAILED",
        "HOSTNAME_BLOCKED",
        `cannot route '${hostname.name}': its requests do not reach this space's machine`,
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

    // read the route each host and path locate among the served spaces, none outside the hostnames
    const locate = async (host: string, path: string, served: readonly string[] = [first]) => {
        const serves = async (scope: string) => served.includes(scope);
        const located = await Route.locate(fixture.lookup, host, path, serves);
        if (located === undefined) {
            return null;
        }
        const row = present(
            await Route.find(fixture.database(first), first, located.id),
            "the route",
        );

        return [located.scope, row.managerName];
    };
    expect([
        await locate(notes, "/roadmap"),
        await locate(notes, "/docs/guide"),
        await locate(notes, "/blog"),
        await locate(notes, "/blog/2026"),
        await locate(notes, "/api/v1/notes"),
        await locate("blog.example.com", "/anything"),
        await locate("example.org", "/"),
        await locate(notes, "/roadmap", []),
    ]).toEqual([
        [first, "roadmap"],
        [first, "docs"],
        [first, "old"],
        [first, "home"],
        [first, "api"],
        [first, "names"],
        null,
        null,
    ]);
});
