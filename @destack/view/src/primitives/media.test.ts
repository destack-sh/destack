import { TEST_START } from "@destack/test/world";
import { testWorld } from "../test/browser.ts";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { Simulation } from "@destack/environment/simulation";
import { expect, test, vi } from "@destack/test";
import { flush } from "solid-js";
import { createPageRoot } from "../test/dom.ts";
import { createBreakpoints, createMediaQuery, sortBreakpoints } from "./media.ts";

/** The breakpoints of the tests. */
const BREAKPOINTS = { sm: "640px", lg: "1024px", xl: "1280px" };

test("match each breakpoint whose minimum width the viewport reaches, keyed by the last", async () => {
    // match nothing, then small, then small and large
    await using simulation = new Simulation({ seed: 1, start: TEST_START });
    const media = [
        {},
        { "(min-width: 640px)": true },
        { "(min-width: 640px)": true, "(min-width: 1024px)": true },
    ];
    const observed = media.map((matching, index) => {
        const tab = new BrowserSimulation(simulation, { media: matching }).environment(`${index}`);

        return createPageRoot((disposeRoot) => {
            const matches = createBreakpoints(BREAKPOINTS);
            const result = { ...matches, key: matches.key };
            disposeRoot();

            return result;
        }, tab);
    });

    expect(observed).toEqual([
        { sm: false, lg: false, xl: false, key: undefined },
        { sm: true, lg: false, xl: false, key: "sm" },
        { sm: true, lg: true, xl: false, key: "lg" },
    ]);
});

test("query breakpoints as minimum widths by default, or as another feature", async () => {
    // record the queries each feature asks the display
    await using simulation = new Simulation({ seed: 1, start: TEST_START });
    const queries = [undefined, "max-width"].map((mediaFeature, index) => {
        const tab = new BrowserSimulation(simulation).environment(`${index}`);
        const media = vi.spyOn(tab.display, "media");
        createPageRoot((disposeRoot) => {
            createBreakpoints(BREAKPOINTS, mediaFeature === undefined ? {} : { mediaFeature });
            disposeRoot();
        }, tab);

        return media.mock.calls.map(([query]) => query);
    });

    expect(queries).toEqual([
        ["(min-width: 640px)", "(min-width: 1024px)", "(min-width: 1280px)"],
        ["(max-width: 640px)", "(max-width: 1024px)", "(max-width: 1280px)"],
    ]);
});

test("follow a breakpoint as its query starts and stops matching", async () => {
    // widen the viewport past the small breakpoint, then narrow it again
    const { tab } = testWorld();
    const { matches, dispose } = createPageRoot(
        (disposeRoot) => ({ matches: createBreakpoints(BREAKPOINTS), dispose: disposeRoot }),
        tab,
    );
    tab.display.emulate({ "(min-width: 640px)": true });
    flush();
    const widened = { ...matches, key: matches.key };
    tab.display.emulate({ "(min-width: 640px)": false });
    flush();
    const narrowed = matches.sm;
    dispose();

    expect([widened, narrowed]).toEqual([{ sm: true, lg: false, xl: false, key: "sm" }, false]);
});

test("keep the breakpoints read once when told not to watch changes", async () => {
    // read the breakpoints narrow, then widen the display past the small breakpoint
    const { tab } = testWorld();
    const { breakpoints, dispose } = createPageRoot(
        (disposeRoot) => ({
            breakpoints: createBreakpoints(BREAKPOINTS, { watchChange: false }),
            dispose: disposeRoot,
        }),
        tab,
    );
    const before = { ...breakpoints };
    tab.display.emulate({ "(min-width: 640px)": true });
    flush();
    const after = { ...breakpoints };
    dispose();

    expect([before, after]).toEqual([
        { sm: false, lg: false, xl: false, key: undefined },
        { sm: false, lg: false, xl: false, key: undefined },
    ]);
});

test("follow whether a media query matches", async () => {
    // start narrow, then match the query
    const { tab } = testWorld();
    const { matches, dispose } = createPageRoot(
        (disposeRoot) => ({
            matches: createMediaQuery("(max-width: 767px)"),
            dispose: disposeRoot,
        }),
        tab,
    );
    const before = matches();
    tab.display.emulate({ "(max-width: 767px)": true });
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
