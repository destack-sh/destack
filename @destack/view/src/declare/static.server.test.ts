import { expect, test } from "@destack/test";
import { Package, PackageId } from "@destack/package";
import { describeView } from "../inspect/index.ts";
import { defineView } from "./view.ts";

/** The declaring package. */
const site = Package.parse({
    id: PackageId.parse("package-019f7480-0000-7000-8000-000000000002"),
    name: "@example/site",
    version: "2026.9.0",
});

test("describe a static view by its origin and hydration, never loading its component, shell or pages", () => {
    // declare a static site with its own shell and pages module, none of which load
    const view = defineView(
        {
            name: "site",
            component: () => Promise.reject(new Error("loaded")),
            render: {
                mode: "static",
                origin: "https://example.test",
                document: () => Promise.reject(new Error("loaded")),
                pages: () => Promise.reject(new Error("loaded")),
            },
        },
        { package: site },
    );

    // describe the site by what its build reads before running it
    expect(describeView(view)).toEqual({
        name: "site",
        presents: [],
        categories: [],
        packages: [],
        render: { mode: "static", origin: "https://example.test", hydrate: "page" },
    });
});
