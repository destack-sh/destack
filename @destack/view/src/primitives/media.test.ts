import { afterEach, expect, test, vi } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createBreakpoints, createMediaQuery, sortBreakpoints } from "./media.ts";

/** The breakpoints of the tests. */
const BREAKPOINTS = { sm: "640px", lg: "1024px", xl: "1280px" };

/** The browser's media query function, restored after each test. */
const original = window.matchMedia;

afterEach(() => {
    window.matchMedia = original;
});

/** Answer media queries as matching when they name one of the widths, keeping each list to dispatch to. */
function stubMatchMedia(widths: readonly string[]): Map<string, MediaQueryList> {
    const lists = new Map<string, MediaQueryList>();
    window.matchMedia = (query: string) => {
        const list = original.call(window, query);
        Object.defineProperty(list, "matches", {
            configurable: true,
            value: widths.some((width) => query.includes(width)),
        });
        lists.set(query, list);

        return list;
    };

    return lists;
}

/** Make a query list match or stop matching, telling its listeners. */
function change(list: MediaQueryList | undefined, matches: boolean): void {
    if (list === undefined) {
        throw new Error("the query was never asked");
    }
    Object.defineProperty(list, "matches", { configurable: true, value: matches });
    list.dispatchEvent(new MediaQueryListEvent("change", { matches, media: list.media }));
}

test("match each breakpoint whose minimum width the viewport reaches, keyed by the last", () => {
    // match nothing, then small, then small and large
    const observed = [[], [BREAKPOINTS.sm], [BREAKPOINTS.sm, BREAKPOINTS.lg]].map((widths) => {
        stubMatchMedia(widths);

        return createRoot((disposeRoot) => {
            const matches = createBreakpoints(BREAKPOINTS);
            const result = { ...matches, key: matches.key };
            disposeRoot();

            return result;
        });
    });

    expect(observed).toEqual([
        { sm: false, lg: false, xl: false, key: undefined },
        { sm: true, lg: false, xl: false, key: "sm" },
        { sm: true, lg: true, xl: false, key: "lg" },
    ]);
});

test("query breakpoints as minimum widths by default, or as another feature", () => {
    const queries = [undefined, "max-width"].map((mediaFeature) => {
        const lists = stubMatchMedia([]);
        createRoot((disposeRoot) => {
            createBreakpoints(BREAKPOINTS, mediaFeature === undefined ? {} : { mediaFeature });
            disposeRoot();
        });

        return [...lists.keys()];
    });

    expect(queries).toEqual([
        ["(min-width: 640px)", "(min-width: 1024px)", "(min-width: 1280px)"],
        ["(max-width: 640px)", "(max-width: 1024px)", "(max-width: 1280px)"],
    ]);
});

test("follow a breakpoint as its query starts and stops matching", () => {
    const lists = stubMatchMedia([]);
    const { matches, dispose } = createRoot((disposeRoot) => ({
        matches: createBreakpoints(BREAKPOINTS),
        dispose: disposeRoot,
    }));
    change(lists.get("(min-width: 640px)"), true);
    flush();
    const widened = { ...matches, key: matches.key };
    change(lists.get("(min-width: 640px)"), false);
    flush();
    const narrowed = matches.sm;
    dispose();

    expect([widened, narrowed]).toEqual([{ sm: true, lg: false, xl: false, key: "sm" }, false]);
});

test("listen to no query when told not to watch changes", () => {
    const lists = stubMatchMedia([]);
    const listen = vi.spyOn(EventTarget.prototype, "addEventListener");
    createRoot((disposeRoot) => {
        createBreakpoints(BREAKPOINTS, { watchChange: false });
        disposeRoot();
    });
    const calls = listen.mock.calls.length;
    listen.mockRestore();

    expect([lists.size, calls]).toEqual([3, 0]);
});

test("follow whether a media query matches", () => {
    const lists = stubMatchMedia([]);
    const { matches, dispose } = createRoot((disposeRoot) => ({
        matches: createMediaQuery("(max-width: 767px)"),
        dispose: disposeRoot,
    }));
    const before = matches();
    change(lists.get("(max-width: 767px)"), true);
    flush();
    const after = matches();
    dispose();

    expect([before, after]).toEqual([false, true]);
});

test("sort breakpoints from the narrowest to the widest", () => {
    expect(Object.entries(sortBreakpoints({ fhd: "1920px", hd: "1280px", sd: "720px" }))).toEqual([
        ["sd", "720px"],
        ["hd", "1280px"],
        ["fhd", "1920px"],
    ]);
});
