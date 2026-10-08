import { afterAll, afterEach, beforeAll, beforeEach, expect, test, vi } from "@destack/test";
import { createRoot, createSignal, flush } from "solid-js";
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

/** Whether the device prefers a dark scheme. */
let isDark = false;

/** The scheme lists made, to tell of changes. */
const schemeLists: EventTarget[] = [];

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

    // answer the scheme query from the test's flag
    replace(window, "matchMedia", {
        value: (media: string) => {
            const list = Object.assign(new EventTarget(), {
                media,
                onchange: null,
                addListener: () => {},
                removeListener: () => {},
            });
            Object.defineProperty(list, "matches", { get: () => isDark });
            schemeLists.push(list);

            return list;
        },
    });
});

beforeEach(() => {
    isDark = false;
});

afterEach(() => {
    vi.useRealTimers();
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

/** Read the overlay a favicon address draws, the drawing's operations after its base image, or plain for an undrawn one. */
function overlayOf(href: string | null | undefined): string {
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

/** Tell every scheme list that the scheme changed. */
function changeScheme(dark: boolean): void {
    isDark = dark;
    for (const list of schemeLists) {
        list.dispatchEvent(Object.assign(new Event("change"), { matches: dark }));
    }
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

test("follow a reactive favicon address, putting back the link on cleanup", () => {
    const [href, setHref] = createSignal("/first.svg", { ownedWrite: true });
    const observed = createRoot((disposeRoot) => {
        const current = createFavicon(href);
        const first = iconHref();
        setHref("/second.svg");
        flush();
        const second = [iconHref(), current().endsWith("/second.svg")];
        disposeRoot();

        return [first, second];
    });

    expect([observed, iconHref()]).toEqual([["/first.svg", ["/second.svg", true]], null]);
});

test("cycle favicon frames every interval, pausing, resuming and stopping after the last unless looping", () => {
    vi.useFakeTimers();
    const looping = makeFaviconAnimation(["/1.png", "/2.png", "/3.png"], { interval: 100 });
    const frames = [looping.frame];
    for (let step = 0; step < 3; step++) {
        vi.advanceTimersByTime(100);
        frames.push(looping.frame);
    }
    looping.pause();
    vi.advanceTimersByTime(300);
    const paused = [looping.frame, looping.playing];
    looping.dispose();

    // cycle once without looping, and never with one frame or without autoplay
    const once = makeFaviconAnimation(["/1.png", "/2.png"], { interval: 100, loop: false });
    vi.advanceTimersByTime(500);
    const ended = [once.frame, once.playing];
    once.dispose();
    const single = makeFaviconAnimation(["/1.png"]);
    const manual = makeFaviconAnimation(["/1.png", "/2.png"], { autoplay: false });

    expect({ frames, paused, ended, single: single.playing, manual: manual.playing }).toEqual({
        frames: [0, 1, 2, 0],
        paused: [0, false],
        ended: [1, false],
        single: false,
        manual: false,
    });
});

test("cycle reactive frames, starting over when they change and pausing while the page is hidden", () => {
    vi.useFakeTimers();
    const [frames, setFrames] = createSignal<readonly string[]>(["/1.png", "/2.png"], {
        ownedWrite: true,
    });
    let isHidden = false;
    Object.defineProperty(document, "hidden", { configurable: true, get: () => isHidden });
    const observed = createRoot((disposeRoot) => {
        const animation = createFaviconAnimation(frames, { interval: 100 });
        vi.advanceTimersByTime(100);
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
        disposeRoot();

        return { advanced, shrunk, grown, whileHidden, shownAgain };
    });
    Reflect.deleteProperty(document, "hidden");

    expect(observed).toEqual({
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
        hrefs.push(overlayOf(iconHref()));
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

    expect([shown, reported]).toEqual([
        "error:missing.png",
        [new Error("favicon image failed to load: error:missing.png")],
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
        hrefs.push(overlayOf(iconHref()));
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
    const [count, setCount] = createSignal<number>(1, { ownedWrite: true });
    const [progress, setProgress] = createSignal<number | undefined>(10, { ownedWrite: true });
    const observed = await createRoot(async (disposeRoot) => {
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
        const read = [overlayOf(iconHref()).split(";").at(-1), badge().startsWith("data:"), ring()];
        disposeRoot();

        return read;
    });

    expect(observed).toEqual(["text 3 #ffffff", true, `${location.origin}/ring.png`]);
});

test("show the favicon of the device's scheme, following its changes and new icons", () => {
    // follow the scheme without an owner
    isDark = true;
    const fixed = makeFaviconScheme({ light: "/light.svg", dark: "/dark.svg" });
    const startedDark = [fixed.scheme, iconHref()];
    changeScheme(false);
    const turnedLight = [fixed.scheme, iconHref()];
    fixed.dispose();

    // follow the scheme and reactive icons in a root
    const [icons, setIcons] = createSignal(
        { light: "/light.svg", dark: "/dark.svg" },
        { ownedWrite: true },
    );
    const reactive = createRoot((disposeRoot) => {
        const favicon = createFaviconScheme(icons);
        changeScheme(true);
        flush();
        const dark = [favicon.scheme(), iconHref()];
        setIcons({ light: "/light-2.svg", dark: "/dark-2.svg" });
        flush();
        const replaced = iconHref();
        disposeRoot();

        return [dark, replaced];
    });

    expect({ startedDark, turnedLight, reactive, after: iconHref() }).toEqual({
        startedDark: ["dark", "/dark.svg"],
        turnedLight: ["light", "/light.svg"],
        reactive: [["dark", "/dark.svg"], "/dark-2.svg"],
        after: null,
    });
});

test("refuse an animation without frames, which would point the favicon at the page", () => {
    expect(() => makeFaviconAnimation([])).toThrow(
        new RangeError("the favicon animation has no frame 0"),
    );
});
