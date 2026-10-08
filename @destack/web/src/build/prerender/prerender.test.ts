import { expect, test } from "@destack/test";
import { prerender } from "./prerender.ts";

test("refuse an uncanonical route and two routes writing one file before rendering", async () => {
    // plan each request against a server module that would fail to load
    const refusals = await Promise.all(
        [{ routes: ["/notes/../about"] }, { routes: ["/about", "/about/"] }].map(
            async (options) => {
                try {
                    await prerender("/missing/server.js", "https://example.test", options);

                    return "rendered";
                } catch (error) {
                    return error instanceof Error ? error.message : "unexpected failure";
                }
            },
        ),
    );

    // refuse each before the renderer runs
    expect(refusals).toEqual([
        "expected a canonical URL path: /notes/../about",
        "prerender routes produce duplicate files",
    ]);
});
