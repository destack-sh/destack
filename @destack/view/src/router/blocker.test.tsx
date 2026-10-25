import { expect, onTestFinished, test } from "@destack/test";
import { flush } from "solid-js";
import { type Blocker, type BlockerFunction, useBlocker } from "./blocker.ts";
import { createRouter, defineRoutes, memoryHistory, useLocation, useNavigate } from "./index.ts";
import { render, wait } from "../test/dom.ts";
import { testBrowser } from "../test/browser.ts";

/** Render a router over a memory history whose pages block leaving while a flag is set. */
function renderBlocked(shouldBlock: BlockerFunction, history: "memory" | "browser" = "memory") {
    // route two pages, the first blocking while dirty
    const exposed: {
        blocker?: Blocker;
        navigate?: (to: string | number, isReplace?: boolean) => void;
        path?: () => string;
    } = {};
    const Page = () => {
        exposed.blocker = useBlocker(shouldBlock);
        const navigate = useNavigate();
        exposed.navigate = (to, isReplace = false) => {
            if (typeof to === "number") {
                navigate(to);
            } else {
                navigate(to, { replace: isReplace });
            }
        };
        const location = useLocation();
        exposed.path = () => location.pathname;

        return <p>page</p>;
    };
    const routes = defineRoutes([
        { path: "/", component: Page },
        { path: "/next", component: Page },
    ]);
    testBrowser().navigation.navigate("/", { history: "replace" });
    const Router = createRouter(
        history === "memory" ? { routes, history: memoryHistory("/") } : { routes },
    );

    // mount in the test's tab, removed after the test
    const element = document.createElement("main");
    const { unmount } = render(() => <Router />, { container: element });
    onTestFinished(() => {
        unmount();
        element.remove();
    });

    return exposed;
}

test("hold a navigation while the condition holds, then let it through or drop it", () => {
    // navigate while dirty, then drop the held navigation
    let isDirty = true;
    const page = renderBlocked(() => isDirty);
    flush();
    page.navigate?.("/next");
    flush();
    const held = [page.blocker?.state(), page.blocker?.location(), page.path?.()];
    page.blocker?.reset();
    flush();
    const dropped = [page.blocker?.state(), page.path?.()];

    // navigate again and let it through, then navigate while clean
    page.navigate?.("/next");
    flush();
    page.blocker?.proceed();
    flush();
    const proceeded = [page.blocker?.state(), page.path?.()];
    isDirty = false;
    page.navigate?.("/");
    flush();

    expect([held, dropped, proceeded, [page.blocker?.state(), page.path?.()]]).toEqual([
        ["blocked", "/next", "/"],
        ["unblocked", "/"],
        ["unblocked", "/next"],
        ["unblocked", "/"],
    ]);
});

test("weigh each navigation by where the page is, where it goes and how it moves through history", async () => {
    // record every navigation the blocker weighs, holding none
    const weighed: [string, string | number, string][] = [];
    const page = renderBlocked(({ currentLocation, nextLocation, historyAction }) => {
        weighed.push([currentLocation.pathname, nextLocation, historyAction]);

        return false;
    }, "browser");
    flush();

    // push, replace and step back
    page.navigate?.("/next");
    flush();
    page.navigate?.("/next?step=2", true);
    flush();
    page.navigate?.(-1);
    await wait(0);
    flush();

    expect(weighed).toEqual([
        ["/", "/next", "PUSH"],
        ["/next", "/next?step=2", "REPLACE"],
        ["/next", -1, "POP"],
    ]);
});

test("undo a traversal of two entries taken outside the router while a guard holds it, in one move", () => {
    // move two entries on while clean
    let isDirty = false;
    const page = renderBlocked(() => isDirty, "browser");
    flush();
    page.navigate?.("/next");
    flush();
    page.navigate?.("/next?step=2");
    flush();

    // step back two entries on the tab while dirty, which a later listener sees undone, then again once clean
    const { navigation } = testBrowser();
    const moves: string[] = [];
    using _moves = navigation.on((entry) =>
        moves.push(new URL(entry.url).pathname + new URL(entry.url).search),
    );
    isDirty = true;
    navigation.go(-2);
    flush();
    const held = [page.path?.(), navigation.currentEntry.index];
    isDirty = false;
    navigation.go(-2);
    flush();
    const passed = [page.path?.(), navigation.currentEntry.index];

    expect({ held, passed, moves }).toEqual({
        held: ["/next", 2],
        passed: ["/", 0],
        moves: ["/next?step=2", "/"],
    });
});

test("undo a back step taken outside the router while a guard holds it, and follow it once let through", () => {
    // hold leaving while dirty, on the tab's session history, moving to the next page while clean
    let isDirty = false;
    const page = renderBlocked(() => isDirty, "browser");
    flush();
    page.navigate?.("/next");
    flush();

    // step back on the tab while dirty, then again once clean
    const { navigation } = testBrowser();
    isDirty = true;
    navigation.go(-1);
    flush();
    const held = [page.path?.(), new URL(navigation.currentEntry.url).pathname];
    isDirty = false;
    navigation.go(-1);
    flush();
    const passed = [page.path?.(), new URL(navigation.currentEntry.url).pathname];

    expect({ held, passed }).toEqual({ held: ["/next", "/next"], passed: ["/", "/"] });
});
