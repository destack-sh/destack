import { isServer } from "@solidjs/web";
import type { Accessor } from "solid-js";
import { makeEventListener } from "./event-listener.ts";
import type { Display } from "@destack/environment/browser";
import { useEnvironment } from "../page/view.ts";
import { createHydratableStaticStore } from "./static-store.ts";
import { createHydratableSignal } from "./utils.ts";

/** Named minimum or maximum widths, such as `{ sm: "640px", lg: "1024px" }`. */
export type Breakpoints = Record<string, string>;

/** Whether each breakpoint's query matches, and the last one that does. */
export type Matches<Names extends Breakpoints> = {
    readonly [Name in keyof Names]: Name extends "key" ? never : boolean;
} & {
    /** The last breakpoint whose query matches, undefined while none does. */
    readonly key: keyof Names | undefined;
};

/** How breakpoints are watched. */
export interface BreakpointOptions<Names extends Breakpoints> {
    /** Whether the matches follow changes, true by default. */
    readonly watchChange?: boolean;
    /** The matches on the server, none by default. */
    readonly fallbackState?: { readonly [Name in keyof Names]: boolean };
    /** The feature each width is queried as, `min-width` for mobile first by default. */
    readonly mediaFeature?: string;
}

/** Call back whenever a media query starts or stops matching, until the owner is cleaned up. */
export function makeMediaQueryListener(
    query: string | MediaQueryList,
    callback: (event: MediaQueryListEvent) => void,
    display: Display,
): () => void {
    // listen nowhere on the server
    if (isServer) {
        return () => {};
    }
    const list = typeof query === "string" ? display.media(query) : query;

    return makeEventListener(list, "change", callback);
}

/** Follow whether a media query matches, answering the server fallback on the server. */
export function createMediaQuery(query: string, serverFallback = false): Accessor<boolean> {
    // answer the fallback on the server
    if (isServer) {
        return () => serverFallback;
    }

    // follow the query on the page's display from the server's answer through hydration
    const list = useEnvironment().display.media(query);
    const [matches, setMatches] = createHydratableSignal(serverFallback, () => list.matches);
    makeEventListener(list, "change", () => setMatches(list.matches));

    return matches;
}

/** Follow whether the device prefers a dark appearance. */
export function createPrefersDark(serverFallback?: boolean): Accessor<boolean> {
    return createMediaQuery("(prefers-color-scheme: dark)", serverFallback);
}

/** Follow which breakpoints a viewport reaches. */
export function createBreakpoints<Names extends Breakpoints>(
    breakpoints: Names,
    options?: BreakpointOptions<Names>,
): Matches<Names>;
/**
 * Follow each breakpoint's query in a static store.
 *
 * NOTE #Architecture: the stated store of matches by name, as solid-primitives' createBreakpoints.
 */
export function createBreakpoints(
    breakpoints: Breakpoints,
    options: BreakpointOptions<Breakpoints> = {},
): Readonly<Record<string, boolean>> {
    // match nothing on the server unless the fallback says otherwise
    const names = Object.keys(breakpoints);
    const fallback =
        options.fallbackState ?? Object.fromEntries(names.map((name) => [name, false]));
    if (isServer) {
        return withKey({ ...fallback }, () => names.findLast((name) => fallback[name] === true));
    }

    // read each query on the page's display from the server's fallback through hydration
    const { mediaFeature = "min-width", watchChange = true } = options;
    const { display } = useEnvironment();
    const lists = Object.entries(breakpoints).map(
        ([name, width]) => [name, display.media(`(${mediaFeature}: ${width})`)] as const,
    );
    const [matches, setMatches] = createHydratableStaticStore<Record<string, boolean>>(
        fallback,
        () => Object.fromEntries(lists.map(([name, list]) => [name, list.matches])),
    );

    // follow each query's changes unless told otherwise
    if (watchChange) {
        for (const [name, list] of lists) {
            makeEventListener(list, "change", (event) => setMatches(name, event.matches));
        }
    }

    return withKey(matches, () => names.findLast((name) => matches[name] === true));
}

/** Copy breakpoints sorted from the narrowest width to the widest. */
export function sortBreakpoints(breakpoints: Breakpoints): Breakpoints {
    const sorted = Object.entries(breakpoints).toSorted(
        ([, first], [, second]) => Number.parseInt(first, 10) - Number.parseInt(second, 10),
    );

    return Object.fromEntries(sorted);
}

/** Add the hidden `key` getter with the last matching breakpoint. */
function withKey(
    matches: Readonly<Record<string, boolean>>,
    key: () => string | undefined,
): Readonly<Record<string, boolean>> {
    return Object.defineProperty(matches, "key", { enumerable: false, get: key });
}
