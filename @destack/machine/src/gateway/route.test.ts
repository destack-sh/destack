import { PackageId } from "@destack/package";
import { found } from "@destack/schema";
import { ROUTE_HEADER } from "@destack/object/workload";
import { route } from "@destack/stack/object";
import { TestRoute } from "@destack/stack/test";
import { expect, refusal, test } from "@destack/test";
import { RouteRequest } from "./route.ts";

/** The package of the routed installation, which declares the notes adapter. */
const NOTES_PACKAGE = PackageId.parse("package-01996ab0-0000-7000-8000-0000000000b1");

/** Address a request the machine routed to the space. */
test("address a routed request to its installation's adapter path, redirect or view by the route it matches, refusing an undeclared adapter", async () => {
    await using fixture = await TestRoute.open();
    const [first] = fixture.spaceIds;
    const hostname = found(fixture.hostnames, fixture.accountId).name;
    const installation = found(fixture.installations, first);

    // declare an adapter, a redirect and a view on the hostname
    await fixture.apply(first, {
        api: {
            hostname,
            path: "/api",
            match: "prefix",
            installation: { id: installation },
            adapter: "notes",
        },
        old: {
            hostname,
            path: "/blog",
            match: "exact",
            redirect: "https://blog.example/",
            redirectStatus: 308,
        },
        home: {
            hostname,
            path: "/",
            match: "prefix",
            installation: { id: installation },
            view: "notes",
        },
    });
    const rows = await fixture
        .database(first)
        .select({ id: route.table.id, managerName: route.table.managerName })
        .from(route.table);
    const declared = (name: string) =>
        found(new Map(rows.map((row) => [row.managerName, row.id])), name);

    // mount each routed request at the machine's own origin, the installation declaring the notes adapter at /feed
    const adapter = {
        installationId: installation,
        packageId: NOTES_PACKAGE,
        name: "notes",
        path: "/feed",
    };
    const mount = async (name?: string, adapters = [adapter]) => {
        const headers = name === undefined ? {} : { [ROUTE_HEADER]: declared(name) };
        const mounted = await RouteRequest.mount(
            fixture.database(first),
            first,
            new Request("https://eu.destack.cloud/api/notes?page=2", { headers }),
            () => adapters,
        );

        // read a redirect by its status and location, and an adapter's request by its address
        if (mounted?.kind === "redirect") {
            return [mounted.response.status, mounted.response.headers.get("location")];
        } else if (mounted?.kind === "adapter") {
            return mounted.request.url;
        }

        return mounted;
    };

    // address the adapter below its package's mount, the path below the route's after its own
    expect([
        await mount("api"),
        await mount("old"),
        await mount(),
        await mount("home"),
        await refusal(mount("api", [])),
    ]).toEqual([
        `https://eu.destack.cloud/spaces/${first}/installations/${installation}/${NOTES_PACKAGE}/feed/notes?page=2`,
        [308, "https://blog.example/"],
        undefined,
        { kind: "view", installation, mount: { base: "", view: "notes" } },
        [
            "NOT_FOUND",
            "ROUTE_NOT_FOUND",
            `route ${declared("api")} leads to adapter notes, which installation ${installation} does not declare`,
        ],
    ]);
});
