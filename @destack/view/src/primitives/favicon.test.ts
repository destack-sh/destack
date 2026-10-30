import { testWorld } from "../test/browser.ts";
import { afterAll, afterEach, beforeAll, expect, refusal, test, vi } from "@destack/test";
import { createSignal, flush } from "solid-js";
import { createPageRoot } from "../test/dom.ts";
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
import { testBrowser } from "../test/browser.ts";

/** The drawing operations of each canvas. */
const drawings = new WeakMap<HTMLCanvasElement, string[]>();

/** The drawings each data address numbers, in order. */
const drawn: string[][] = [];

/** The originals of the replaced members, restored after the tests. */
const originals: [object, string, PropertyDescriptor | undefined][] = [];

/** Replace a member of a prototype for the tests. */
function replace(target: object, name: string, descriptor: PropertyDescriptor): void {
    originals.push([target, name, Object.getOwnPropertyDescriptor(target, name)]);
    Object.defineProperty(target, name, { configurable: true, ...descriptor });
}

/** The media query of a device preferring a dark scheme. */
const DARK = "(prefers-color-scheme: dark)";

beforeAll(() => {
    // load images on the next microtask, failing those whose address starts with error:
    const source = Object.getOwnPropertyDescriptor(HTMLImageElement.prototype, "src");
    replace(HTMLImageElement.prototype, "src", {
        get(this: HTMLImageElement): unknown {
            return source?.get?.call(this);
        },
        set(this: HTMLImageElement, value: string) {
            source?.set?.call(this, value);
            void Promise.resolve().then(() =>
                this.dispatchEvent(new Event(value.startsWith("error:") ? "error" : "load")),
            );
        },
    });

    // record drawing operations, and read them back as the data address
    replace(HTMLCanvasElement.prototype, "getContext", {
        value: function (this: HTMLCanvasElement) {
            const operations: string[] = [];
            drawings.set(this, operations);
            const context = {
                fillStyle: "",
                strokeStyle: "",
                font: "",
                clearRect: () => {},
                drawImage: (image: HTMLImageElement) =>
                    operations.push(`image ${image.getAttribute("src") ?? ""}`),
                beginPath: () => {},
                arc: (x: number, y: number, _radius: number, _start: number, end: number) =>
                    operations.push(`arc ${x} ${y} ${end.toFixed(2)}`),
                fill: () => operations.push(`fill ${context.fillStyle}`),
                stroke: () => operations.push(`stroke ${context.strokeStyle}`),
                fillText: (text: string) => operations.push(`text ${text} ${context.fillStyle}`),
            };

            return context;
        },
    });
    replace(HTMLCanvasElement.prototype, "toDataURL", {
        value: function (this: HTMLCanvasElement) {
            drawn.push(drawings.get(this) ?? []);

            return `data:drawn,${drawn.length - 1}`;
        },
    });
});

afterEach(() => {
    for (const link of document.head.querySelectorAll("link")) {
        link.remove();
    }
});

afterAll(() => {
    for (const [target, name, descriptor] of originals.toReversed()) {
        if (descriptor === undefined) {
            Reflect.deleteProperty(target, name);
        } else {
            Object.defineProperty(target, name, descriptor);
        }
    }
});

/** Read the icon link's address attribute, null without a link. */
function iconHref(rel = "icon"): string | null {
    return document.head.querySelector(`link[rel="${rel}"]`)?.getAttribute("href") ?? null;
}

/** Read the overlay a favicon address draws. */
function readOverlay(href: string | null | undefined): string {
    const number = /^data:drawn,(\d+)$/u.exec(href ?? "")?.[1];

    return number === undefined ? "plain" : (drawn[Number(number)] ?? []).slice(1).join(";");
}

/** Let the image loads and drawings settle. */
async function settle(): Promise<void> {
    for (let round = 0; round < 10; round++) {
        await Promise.resolve();
    }
    flush();
}

test("make the favicon link when there is none, removing it on dispose", () => {
    const favicon = makeFavicon("/a.svg");
    const made = iconHref();
    favicon.setHref("/b.svg");
    const changed = iconHref();
    favicon.dispose();

    expect([made, changed, iconHref()]).toEqual(["/a.svg", "/b.svg", null]);
});

test("take over an existing link, putting back its address or its missing one, in nested order", () => {
    // take over a link with an address, twice, then one without
    const link = document.createElement("link");
    link.rel = "icon";
    link.setAttribute("href", "/original.svg");
    document.head.append(link);
    const outer = makeFavicon("/outer.svg");
    const inner = makeFavicon("/inner.svg");
    inner.dispose();
    const afterInner = iconHref();
    outer.dispose();
    const afterOuter = iconHref();
    link.removeAttribute("href");
    makeFavicon("/taken.svg").dispose();

    expect([afterInner, afterOuter, link.hasAttribute("href")]).toEqual([
        "/outer.svg",
        "/original.svg",
        false,
    ]);
});

test("drive the link of another relation", () => {
    const favicon = makeFavicon("/touch.png", { rel: "apple-touch-icon" });
    const shown = iconHref("apple-touch-icon");
    favicon.dispose();

    expect([shown, iconHref()]).toEqual(["/touch.png", null]);
});

test("follow a reactive favicon address, putting back the link on cleanup", async () => {
    const { tab } = testWorld();
    const [href, setHref] = createSignal("/first.svg", { ownedWrite: true });
    const observed = createPageRoot((disposeRoot) => {
        const current = createFavicon(href);
        const first = iconHref();
        setHref("/second.svg");
        flush();
        const second = [iconHref(), current().endsWith("/second.svg")];
        disposeRoot();

        return [first, second];
    }, tab);

    expect([observed, iconHref()]).toEqual([["/first.svg", ["/second.svg", true]], null]);
});

test("cycle favicon frames every interval, pausing, resuming and stopping after the last unless looping", async () => {
    const { simulation, tab } = testWorld();
    const looping = makeFaviconAnimation(
        ["/1.png", "/2.png", "/3.png"],
        { interval: 100 },
        tab.clock,
    );
    const frames = [looping.frame];
    for (let step = 0; step < 3; step++) {
        await simulation.clock.sleep(100);
        await simulation.idle();
        frames.push(looping.frame);
    }
    looping.pause();
    await simulation.clock.sleep(300);
    await simulation.idle();
    const paused = [looping.frame, looping.playing];
    looping.dispose();

    // cycle once without looping, and never with one frame or without autoplay
    const once = makeFaviconAnimation(
        ["/1.png", "/2.png"],
        { interval: 100, loop: false },
        tab.clock,
    );
    await simulation.clock.sleep(500);
    await simulation.idle();
    const ended = [once.frame, once.playing];
    once.dispose();
    const single = makeFaviconAnimation(["/1.png"], {}, tab.clock);
    const manual = makeFaviconAnimation(["/1.png", "/2.png"], { autoplay: false }, tab.clock);

    expect({ frames, paused, ended, single: single.playing, manual: manual.playing }).toEqual({
        frames: [0, 1, 2, 0],
        paused: [0, false],
        ended: [1, false],
        single: false,
        manual: false,
    });
});

test("cycle reactive frames, starting over when they change and pausing while the page is hidden", async () => {
    const { simulation, tab } = testWorld();
    const [frames, setFrames] = createSignal<readonly string[]>(["/1.png", "/2.png"], {
        ownedWrite: true,
    });
    let isHidden = false;
    Object.defineProperty(document, "hidden", { configurable: true, get: () => isHidden });
    const { animation, dispose } = createPageRoot(
        (disposeRoot) => ({
            animation: createFaviconAnimation(frames, { interval: 100 }),
            dispose: disposeRoot,
        }),
        tab,
    );
    await simulation.clock.sleep(100);
    await simulation.idle();
    flush();
    const advanced = [animation.frame(), iconHref()];

    // shrink to one frame, then grow again
    setFrames(["/only.png"]);
    flush();
    const shrunk = [animation.frame(), animation.playing()];
    setFrames(["/a.png", "/b.png"]);
    flush();
    const grown = animation.playing();

    // hide and show the page
    isHidden = true;
    document.dispatchEvent(new Event("visibilitychange"));
    flush();
    const whileHidden = animation.playing();
    isHidden = false;
    document.dispatchEvent(new Event("visibilitychange"));
    flush();
    const shownAgain = animation.playing();
    dispose();
    Reflect.deleteProperty(document, "hidden");

    expect({ advanced, shrunk, grown, whileHidden, shownAgain }).toEqual({
        advanced: [1, "/2.png"],
        shrunk: [0, false],
        grown: true,
        whileHidden: false,
        shownAgain: true,
    });
});

test("draw a count, a capped count, a text or a dot as a badge, and nothing for falsy values", async () => {
    // draw each kind of value
    const values = [3, 150, "new", true, 0, false, undefined, ""] as const;
    const hrefs: string[] = [];
    for (const value of values) {
        const badge = makeFaviconBadge("/base.png", value, { position: "top-left" });
        await settle();
        hrefs.push(readOverlay(iconHref()));
        badge.dispose();
    }

    expect(hrefs).toEqual([
        "arc 9.6 9.6 6.28;fill #e11d48;text 3 #ffffff",
        "arc 9.6 9.6 6.28;fill #e11d48;text 99+ #ffffff",
        "arc 9.6 9.6 6.28;fill #e11d48;text new #ffffff",
        "arc 9.6 9.6 6.28;fill #e11d48",
        "plain",
        "plain",
        "plain",
        "plain",
    ]);
});

test("keep the plain favicon, reporting why, when its image fails to load", async () => {
    const reported: unknown[] = [];
    vi.stubGlobal("reportError", (error: unknown) => reported.push(error));
    const badge = makeFaviconBadge("error:missing.png", 3);
    await settle();
    const shown = iconHref();
    badge.dispose();
    vi.unstubAllGlobals();

    // report the failed load as the call error it converts to
    const failures = await Promise.all(reported.map((error) => refusal(Promise.reject(error))));
    expect([shown, failures]).toEqual([
        "error:missing.png",
        [["BAD_GATEWAY", "FAVICON_LOAD_FAILED", "favicon image failed to load: error:missing.png"]],
    ]);
});

test("leave the restored favicon alone when a drawing settles after disposal", async () => {
    const link = document.createElement("link");
    link.rel = "icon";
    link.setAttribute("href", "/original.svg");
    document.head.append(link);
    makeFaviconBadge("/base.png", 3).dispose();
    makeFaviconProgress("/base.png", 50).dispose();
    await settle();

    expect(iconHref()).toBe("/original.svg");
});

test("draw a progress ring's track, and its arc from the top for a positive share, clamped", async () => {
    const hrefs: string[] = [];
    for (const progress of [undefined, 0, 50, 150] as const) {
        const ring = makeFaviconProgress("/base.png", progress, {
            color: "blue",
            trackColor: "gray",
        });
        await settle();
        hrefs.push(readOverlay(iconHref()));
        ring.dispose();
    }

    expect(hrefs).toEqual([
        "plain",
        "arc 16 16 6.28;stroke gray",
        "arc 16 16 6.28;stroke gray;arc 16 16 1.57;stroke blue",
        "arc 16 16 6.28;stroke gray;arc 16 16 4.71;stroke blue",
    ]);
});

test("redraw a reactive badge and ring, applying only the latest drawing", async () => {
    const { tab } = testWorld();
    const [count, setCount] = createSignal<number>(1, { ownedWrite: true });
    const [progress, setProgress] = createSignal<number | undefined>(10, { ownedWrite: true });
    const observed = await createPageRoot(async (disposeRoot) => {
        const badge = createFaviconBadge("/base.png", count);
        const ring = createFaviconProgress("/ring.png", progress, { rel: "apple-touch-icon" });
        flush();
        setCount(2);
        flush();
        setCount(3);
        flush();
        setProgress(undefined);
        flush();
        await settle();
        const read = [
            readOverlay(iconHref()).split(";").at(-1),
            badge().startsWith("data:"),
            ring(),
        ];
        disposeRoot();

        return read;
    }, tab);

    expect(observed).toEqual(["text 3 #ffffff", true, new URL("/ring.png", document.baseURI).href]);
});

test("show the favicon of the device's scheme, following its changes and new icons", async () => {
    const { tab } = testWorld();
    // follow the scheme without an owner
    tab.display.emulate({ [DARK]: true });
    const fixed = makeFaviconScheme({ light: "/light.svg", dark: "/dark.svg" }, {}, tab.display);
    const startedDark = [fixed.scheme, iconHref()];
    tab.display.emulate({ [DARK]: false });
    const turnedLight = [fixed.scheme, iconHref()];
    fixed.dispose();

    // follow the scheme and reactive icons in a root
    const [icons, setIcons] = createSignal(
        { light: "/light.svg", dark: "/dark.svg" },
        { ownedWrite: true },
    );
    const reactive = createPageRoot((disposeRoot) => {
        const favicon = createFaviconScheme(icons);
        tab.display.emulate({ [DARK]: true });
        flush();
        const dark = [favicon.scheme(), iconHref()];
        setIcons({ light: "/light-2.svg", dark: "/dark-2.svg" });
        flush();
        const replaced = iconHref();
        disposeRoot();

        return [dark, replaced];
    }, tab);

    expect({ startedDark, turnedLight, reactive, after: iconHref() }).toEqual({
        startedDark: ["dark", "/dark.svg"],
        turnedLight: ["light", "/light.svg"],
        reactive: [["dark", "/dark.svg"], "/dark-2.svg"],
        after: null,
    });
});

test("refuse an animation without frames, which would point the favicon at the page", () => {
    expect(() => makeFaviconAnimation([], {}, testBrowser().clock)).toThrow(
        new RangeError("the favicon animation has no frame 0"),
    );
});
