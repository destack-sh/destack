import { expect, test } from "@destack/test";
import { webOutput } from "./output.ts";

test("refuse a site that is no bare HTTP origin, and prerendered pages and site files without a site", () => {
    // expand requests with a non-HTTP site, a site with a path, and pages and files without one
    const base = { kind: "web", app: "src/app.tsx", ssr: { runtime: "bun" } };
    const refusals = [
        { ...base, site: "ftp://example.test" },
        { ...base, site: "https://example.test/blog" },
        { ...base, prerender: { routes: ["/"] } },
        { ...base, metadata: { robots: { rules: [] } } },
    ].map((request) => {
        try {
            webOutput.expand("site", request);

            return "expanded";
        } catch (error) {
            return error instanceof Error ? error.message : "unexpected failure";
        }
    });

    // refuse the sites as the schema reports them, then the missing ones
    const origin = JSON.stringify(
        [{ code: "custom", path: ["site"], message: "expected an HTTP origin" }],
        null,
        2,
    );
    const missing = "prerendered pages and site files require a site origin";
    expect(refusals).toEqual([origin, origin, missing, missing]);
});
