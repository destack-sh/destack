import { found } from "@destack/schema";
import { ROUTE_PATH } from "@destack/object/workload";
import { route } from "@destack/stack/object";
import { TestRoute } from "@destack/stack/test";
import { expect, test } from "@destack/test";
import { ROUTE_HEADER } from "../space/route.ts";
import { RouteRequest } from "./route.ts";

/** Address a request the machine routed to the space. */
test("address a routed request to its installation's adapter, redirect or view by the route it matches", async () => {
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

    // mount each routed request at the machine's own origin
    const mount = async (name?: string) => {
        const headers = name === undefined ? {} : { [ROUTE_HEADER]: declared(name) };
        const mounted = await RouteRequest.mount(
            fixture.database(first),
            first,
            new Request("https://eu.destack.cloud/api/notes", { headers }),
        );

        // read a redirect by its status and location, and an adapter's request by its address
        if (mounted?.kind === "redirect") {
            return [mounted.response.status, mounted.response.headers.get("location")];
        } else if (mounted?.kind === "adapter") {
            return mounted.request.url;
        }

        return mounted;
    };

    expect([await mount("api"), await mount("old"), await mount(), await mount("home")]).toEqual([
        `https://eu.destack.cloud/spaces/${first}/installations/${installation}${ROUTE_PATH}/notes`,
        [308, "https://blog.example/"],
        undefined,
        { kind: "view", installation, mount: { base: "", view: "notes" } },
    ]);
});
