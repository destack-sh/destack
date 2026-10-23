import { expect, refusal, test } from "@destack/test";
import { webOutput } from "./output.ts";

/** A web request rendering on bun, without a site. */
const BASE = { kind: "web", app: "src/app.tsx", ssr: { runtime: "bun" } };

/** Defer expanding a request into a site's outputs. */
const expand = (request: Parameters<typeof webOutput.expand>[1]) => () =>
    webOutput.expand("site", request);

test("refuse a site that is no bare HTTP origin, and prerendered pages and site files without a site", async () => {
    // refuse a non-HTTP site and a site with a path
    const origin = [
        "BAD_REQUEST",
        "INVALID_OPTIONS",
        "invalid web output site: site: expected an HTTP origin",
    ];
    expect([
        await refusal(expand({ ...BASE, site: "ftp://example.test" })),
        await refusal(expand({ ...BASE, site: "https://example.test/blog" })),
    ]).toEqual([origin, origin]);

    // refuse pages and files without a site
    const missing = [
        "BAD_REQUEST",
        "SITE_REQUIRED",
        "prerendered pages and site files require a site origin",
    ];
    expect([
        await refusal(expand({ ...BASE, prerender: { routes: ["/"] } })),
        await refusal(expand({ ...BASE, metadata: { robots: { rules: [] } } })),
    ]).toEqual([missing, missing]);
});
