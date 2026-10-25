import { expect, refusal, test } from "@destack/test";
import { Simulation } from "@destack/environment/simulation";
import { prerender } from "./prerender.ts";

test("refuse an uncanonical route and two routes writing one file before rendering", async () => {
    // plan each request against a server module that would fail to load, in a world
    await using simulation = new Simulation({ seed: 1, start: Date.UTC(2026, 9, 24) });
    const environment = simulation.environment("build");
    const refusals = await Promise.all(
        [{ routes: ["/notes/../about"] }, { routes: ["/about", "/about/"] }].map((options) =>
            refusal(prerender("/missing/server.js", "https://example.test", options, environment)),
        ),
    );

    // refuse each before the renderer runs
    expect(refusals).toEqual([
        ["BAD_REQUEST", "ROUTE_NOT_CANONICAL", "expected a canonical URL path: /notes/../about"],
        ["BAD_REQUEST", "DUPLICATE_PAGE", "prerender routes produce duplicate files"],
    ]);
});
