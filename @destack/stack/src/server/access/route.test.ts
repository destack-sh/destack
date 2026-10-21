import { ObjectReference } from "@destack/sync";
import { asc } from "@destack/db";
import { TEST_DIALECTS } from "@destack/db/test";
import { ObjectError, Claim } from "@destack/object";
import { nth, found, present } from "@destack/schema";
import { CallError } from "@destack/runtime/error";
import { expect, test } from "@destack/test";
import type {} from "@destack/package/import-meta";
import { Route, route } from "../../object/index.ts";
import { RouteFixture } from "../../test/index.ts";

/** The package declaring the fixture's stacks. */
const PACKAGE_ID = import.meta.destack.package.id;

/** Serve each path of an account's hostnames once across the universe. */
test.for(TEST_DIALECTS)(
    "reconcile a space's routes on its account's hostnames, one per hostname path across the universe, on %s",
    async (dialect) => {
        await using fixture = await RouteFixture.open(dialect);
        const [first, second, rival] = fixture.spaceIds;
        const hostname = found(fixture.hostnames, fixture.accountId);
        const home = (space: string, on = hostname.name) => ({
            hostname: on,
            path: "/",
            match: "prefix",
            destination: {
                installation: { id: found(fixture.installations, space) },
                view: "notes",
            },
        });
        const old = {
            hostname: hostname.name,
            path: "/blog",
            match: "exact",
            destination: { redirect: "https://blog.example/", status: 308 },
        };

        // serve the space's installation and a redirect on the hostname
        expect(await fixture.apply(first, { home: home(first), old })).toEqual({
            steps: [
                { action: "create", target: "route/home", risk: "safe", detail: "declare" },
                { action: "create", target: "route/old", risk: "safe", detail: "declare" },
            ],
        });

        // refuse another space's installation and the same path in another space of the account
        await expect(fixture.apply(first, { home: home(second) })).rejects.toEqual(
            new ObjectError(
                "INVALID_DECLARATION",
                `unknown installation: ${fixture.installations.get(second)}`,
            ),
        );
        await expect(fixture.apply(second, { home: home(second) })).rejects.toEqual(
            new CallError("CONFLICT", { message: "path is taken" }),
        );

        // refuse another account's hostname and an unknown one and take the same path on the other account's own hostname
        const unknown = "missing.example.com";
        await expect(fixture.apply(rival, { home: home(rival) })).rejects.toEqual(
            new CallError("FORBIDDEN", {
                message: `hostname ${hostname.name} belongs to another account`,
            }),
        );
        await expect(fixture.apply(rival, { home: home(rival, unknown) })).rejects.toEqual(
            new CallError("NOT_FOUND", { message: `no hostname ${unknown}` }),
        );
        const own = found(fixture.hostnames, fixture.otherId);
        expect(await fixture.apply(rival, { home: home(rival, own.name) })).toEqual({
            steps: [{ action: "create", target: "route/home", risk: "safe", detail: "declare" }],
        });

        // keep one declared route per space and path managed by its space's stack
        const rows = await fixture.spaces
            .select()
            .from(route.table)
            .orderBy(asc(route.table.scope), asc(route.table.path));
        const declared = (index: number, scope: string, name: string) => ({
            id: nth(rows, index).id,
            createdAt: nth(rows, index).createdAt,
            updatedAt: nth(rows, index).createdAt,
            scope,
            hostname: hostname.id,
            conditions: {},
            tags: {},
            generation: 1,
            revision: 1,
            deletedAt: null,
            ...ObjectReference.columns("createdBy", undefined),
            ...ObjectReference.columns("updatedBy", undefined),
            ...ObjectReference.columns("deletedBy", undefined),
            detachedAt: null,
            managerInstallationId: fixture.stack(scope),
            managerPackageId: PACKAGE_ID,
            managerName: name,
        });
        const view = (scope: string) => ({
            path: "/",
            match: "prefix",
            installationId: fixture.installations.get(scope),
            view: "notes",
            adapter: null,
            redirect: null,
            redirectStatus: null,
        });
        expect(rows).toEqual([
            { ...declared(0, first, "home"), ...view(first) },
            {
                ...declared(1, first, "old"),
                path: "/blog",
                match: "exact",
                installationId: null,
                view: null,
                adapter: null,
                redirect: "https://blog.example/",
                redirectStatus: 308,
            },
            { ...declared(2, rival, "home"), ...view(rival), hostname: own.id },
        ]);

        // key each hostname path in the universe's index
        const owner = async (on: string) => {
            const { index, key } = Claim.key(route, "path", [on, "/", "prefix"]);

            return await fixture.lookup.owner(index, key);
        };
        expect([await owner(hostname.id), await owner(own.id)]).toEqual([
            { objectId: nth(rows, 0).id, owner: first },
            { objectId: nth(rows, 2).id, owner: rival },
        ]);
    },
);

/** Find a request's route as a front door does. */
test.for(TEST_DIALECTS)(
    "locate the route serving a host's path, the most specific first, on %s",
    async (dialect) => {
        await using fixture = await RouteFixture.open(dialect);
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
                destination: { installation, view: "notes" },
            },
            old: {
                hostname: notes,
                path: "/blog",
                match: "exact",
                destination: { redirect: "https://blog.example/", status: 308 },
            },
            api: {
                hostname: notes,
                path: "/api",
                match: "prefix",
                destination: { installation, adapter: "notes" },
            },
            names: {
                hostname: wildcard.name,
                path: "/",
                match: "prefix",
                destination: { installation, adapter: "notes" },
            },
        });

        // read the route and destination each host and path locate, none outside the hostnames
        const located = async (host: string, path: string) => {
            const served = await Route.locate(fixture.lookup, host, path);
            if (served === undefined) {
                return null;
            }
            const row = present(await Route.find(fixture.spaces, first, served.id), "the route");

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
    },
);
