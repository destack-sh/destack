import { asc } from "@destack/db";
import { Claim } from "@destack/object";
import { found, present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Route, route } from "../../object/index.ts";
import { TestRoute } from "../../test/index.ts";

/** Serve each path of an account's hostnames once across the universe. */
test("reconcile a space's routes on its account's hostnames, one per hostname path across the universe", async () => {
    await using fixture = await TestRoute.open();
    const [first, second, rival] = fixture.spaceIds;
    const hostname = found(fixture.hostnames, fixture.accountId);
    const home = (spaceId: string, on = hostname.name) =>
        ({
            hostname: on,
            path: "/",
            match: "prefix",
            installation: { id: found(fixture.installations, spaceId) },
            view: "notes",
        }) as const;
    const old = {
        hostname: hostname.name,
        path: "/blog",
        match: "exact",
        redirect: "https://blog.example/",
        redirectStatus: 308,
    } as const;

    // serve the space's installation and a redirect on the hostname
    await fixture.apply(first, { home: home(first), old });

    // refuse another space's installation and the same path in another space of the account
    const refused = {
        installation: await fixture.submit(first, { home: home(second) }),
        path: await fixture.submit(second, { home: home(second) }),
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
                    redirect: route.table.redirect,
                })
                .from(route.table)
                .orderBy(asc(route.table.path))
        ).map((row) => Object.values(row));
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
            installation: ["Blocked", `unknown installation: ${fixture.installations.get(second)}`],
            path: ["ApplyFailed", "path is taken"],
        },
        foreign: {
            hostname: ["ApplyFailed", `hostname ${hostname.name} not found`],
            unknown: ["ApplyFailed", `hostname ${unknown} not found`],
        },
        routes: [
            [
                [first, "home", hostname.name, "/", found(fixture.installations, first), null],
                [first, "old", hostname.name, "/blog", null, "https://blog.example/"],
            ],
            [],
            [[rival, "home", own.name, "/", found(fixture.installations, rival), null]],
        ],
        owners: [first, rival],
    });
});

test("refuse a route on a machine the account's hostname requests do not reach", async () => {
    // serve the spaces on a machine the account's hostnames do not reach, as a cloud machine before the edge provisions them
    await using fixture = await TestRoute.open({ isServingHostnames: false });
    const [first] = fixture.spaceIds;
    const hostname = found(fixture.hostnames, fixture.accountId);
    const home = {
        hostname: hostname.name,
        path: "/",
        match: "prefix",
        installation: { id: found(fixture.installations, first) },
        view: "notes",
    } as const;

    // apply a route there, which no request would reach
    expect(await fixture.submit(first, { home })).toEqual([
        "ApplyFailed",
        `requests for ${hostname.name} do not reach this space's machine`,
    ]);
});

/** Find a request's route as a front door does. */
test("locate the route serving a host's path, the most specific first", async () => {
    await using fixture = await TestRoute.open();
    const [first] = fixture.spaceIds;
    const notes = found(fixture.hostnames, fixture.accountId).name;
    const wildcard = await fixture.serve(fixture.accountId, "*.example.com");
    const installation = { id: found(fixture.installations, first) };

    // declare a view, a redirect and an adapter on the hostname, and an adapter on its wildcard
    await fixture.apply(first, {
        home: {
            hostname: notes,
            path: "/",
            match: "prefix",
            installation,
            view: "notes",
        },
        old: {
            hostname: notes,
            path: "/blog",
            match: "exact",
            redirect: "https://blog.example/",
            redirectStatus: 308,
        },
        api: {
            hostname: notes,
            path: "/api",
            match: "prefix",
            installation,
            adapter: "notes",
        },
        names: {
            hostname: wildcard.name,
            path: "/",
            match: "prefix",
            installation,
            adapter: "notes",
        },
    });

    // read the route and destination each host and path locate, none outside the hostnames
    const located = async (host: string, path: string) => {
        const served = await Route.locate(fixture.lookup, host, path);
        if (served === undefined) {
            return null;
        }
        const row = present(
            await Route.find(fixture.database(first), first, served.id),
            "the route",
        );

        return [served.scope, row.managerName, Route.destination(row)];
    };
    const view = { kind: "view", installationId: installation.id, view: "notes" };
    const adapter = { kind: "adapter", installationId: installation.id, adapter: "notes" };
    const redirect = { kind: "redirect", location: "https://blog.example/", status: 308 };
    expect([
        await located(notes, "/blog"),
        await located(notes, "/blog/2026"),
        await located(notes, "/api/v1/notes"),
        await located("blog.example.com", "/anything"),
        await located("example.org", "/"),
    ]).toEqual([
        [first, "old", redirect],
        [first, "home", view],
        [first, "api", adapter],
        [first, "names", adapter],
        null,
    ]);
});
