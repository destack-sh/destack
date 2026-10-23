import { expect, refusal, test } from "@destack/test";
import { prerender } from "./prerender.ts";

test("refuse an uncanonical route and two routes writing one file before rendering", async () => {
    // plan each request against a server module that would fail to load
    const refusals = await Promise.all(
        [{ routes: ["/notes/../about"] }, { routes: ["/about", "/about/"] }].map((options) =>
            refusal(prerender("/missing/server.js", "https://example.test", options)),
        ),
    );

    // refuse each before the renderer runs
    expect(refusals).toEqual([
        ["BAD_REQUEST", "ROUTE_NOT_CANONICAL", "expected a canonical URL path: /notes/../about"],
        ["BAD_REQUEST", "DUPLICATE_PAGE", "prerender routes produce duplicate files"],
    ]);
});
