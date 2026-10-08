import { expect, onTestFinished, test } from "@destack/test";
import { render } from "@solidjs/web";
import { flush } from "solid-js";
import { type Blocker, type BlockerFunction, useBlocker } from "./blocker.ts";
import { createRouter, defineRoutes, memoryHistory, useLocation, useNavigate } from "./index.ts";

/** Render a router over a memory history whose pages block leaving while a flag holds, exposing the blocker, navigation and location. */
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
    window.history.replaceState(null, "", "/");
    const Router = createRouter(
        history === "memory" ? { routes, history: memoryHistory("/") } : { routes },
    );

    // mount and remove after the test
    const element = document.createElement("main");
    document.body.append(element);
    const dispose = render(() => <Router />, element);
    onTestFinished(() => {
        dispose();
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
    await new Promise((resolve) => {
        setTimeout(resolve);
    });
    flush();

    expect(weighed).toEqual([
        ["/", "/next", "PUSH"],
        ["/next", "/next?step=2", "REPLACE"],
        ["/next", -1, "POP"],
    ]);
});
