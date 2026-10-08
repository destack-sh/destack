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

test("read plain favicon addresses on the server", () => {
    expect([
        makeFavicon("/a.svg").href,
        createFavicon(() => "/b.svg")(),
        makeFaviconAnimation(["/1.png", "/2.png"]).href,
        createFaviconAnimation(["/1.png"]).playing(),
        makeFaviconBadge("/base.png", 3).href,
        createFaviconBadge("/base.png", 3)(),
        makeFaviconProgress("/base.png", 50).href,
        createFaviconProgress("/base.png", 50)(),
        makeFaviconScheme({ light: "/light.svg", dark: "/dark.svg" }).scheme,
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
