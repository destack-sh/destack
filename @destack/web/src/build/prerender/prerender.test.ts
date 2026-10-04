import { expect, test } from "@destack/test";
import { prerender } from "./prerender.ts";

test("refuse a non-HTTP origin, an uncanonical route and two routes writing one file before rendering", async () => {
    // plan each request against a server module that would fail to load
    const refusals = await Promise.all(
        [
            { origin: "ftp://example.test", routes: ["/"] },
            { origin: "https://example.test", routes: ["/notes/../about"] },
            { origin: "https://example.test", routes: ["/about", "/about/"] },
        ].map(async (options) => {
            try {
                await prerender("/missing/server.js", options);

                return "rendered";
            } catch (error) {
                return error instanceof Error ? error.message : "unexpected failure";
            }
        }),
    );

    // refuse each before the renderer runs
    expect(refusals).toEqual([
        "expected an HTTP origin: ftp://example.test",
        "expected a canonical URL path: /notes/../about",
        "prerender routes produce duplicate files",
    ]);
});
