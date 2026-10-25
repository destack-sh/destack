import { expect, test } from "@destack/test";
import {
    createFavicon,
    createFaviconAnimation,
    createFaviconBadge,
    createFaviconProgress,
    createFaviconScheme,
    makeFavicon,
    makeFaviconAnimation,
    makeFaviconBadge,
    makeFaviconProgress,
    makeFaviconScheme,
} from "./favicon.ts";
import { testHost } from "@destack/test/host";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { Simulation } from "@destack/environment/simulation";

/** The start of every test's world: 2026-10-24 at midnight UTC. */
const START = Date.UTC(2026, 9, 24);

/** The clock a server-side animation would run on. */
const clock = testHost().clock;

test("read plain favicon addresses on the server", async () => {
    // read each address on a display the server never asks
    await using simulation = new Simulation({ seed: 1, start: START });
    const { display } = new BrowserSimulation(simulation).environment("tab");
    expect([
        makeFavicon("/a.svg").href,
        createFavicon(() => "/b.svg")(),
        makeFaviconAnimation(["/1.png", "/2.png"], {}, clock).href,
        createFaviconAnimation(["/1.png"]).playing(),
        makeFaviconBadge("/base.png", 3).href,
        createFaviconBadge("/base.png", 3)(),
        makeFaviconProgress("/base.png", 50).href,
        createFaviconProgress("/base.png", 50)(),
        makeFaviconScheme({ light: "/light.svg", dark: "/dark.svg" }, {}, display).scheme,
        createFaviconScheme({ light: "/light.svg", dark: "/dark.svg" }).href(),
    ]).toEqual([
        "/a.svg",
        "/b.svg",
        "/1.png",
        false,
        "/base.png",
        "/base.png",
        "/base.png",
        "/base.png",
        "light",
        "/light.svg",
    ]);
});
