import { expect, onTestFinished, test } from "@destack/test";
import { render } from "@solidjs/web";
import { ObjectReference } from "@destack/sync";
import { createRouter, defineRoutes, memoryHistory } from "./index.ts";
import { Link } from "./link.tsx";

/** A note in a space, opened through the open path. */
const note = ObjectReference.parse({
    packageId: "package-01996ab0-0000-7000-8000-000000000001",
    type: "note",
    scope: "space-01996ab0-0000-7000-8000-000000000002",
    id: "note-01996ab0-0000-7000-8000-000000000003",
});

/** Render links on the start page of a router over a memory history. */
function renderLinks(): HTMLElement {
    // route the start page to the links under test
    const routes = defineRoutes([
        {
            path: "/",
            component: () => (
                <>
                    <Link href="/board" replace prefetch={false}>
                        Board
                    </Link>
                    <Link href={note}>Note</Link>
                    <Link href={note} view="editor">
                        Note in the editor
                    </Link>
                </>
            ),
        },
    ]);
    const Router = createRouter({ routes, history: memoryHistory("/") });

    // mount the router and remove it after the test
    const element = document.createElement("main");
    document.body.append(element);
    const dispose = render(() => <Router />, element);
    onTestFinished(() => {
        dispose();
        element.remove();
    });

    return element;
}

test("link to paths through the router and to objects through the open path", () => {
    // read each rendered anchor's address and router attributes
    const anchors = [...renderLinks().querySelectorAll("a")].map((anchor) => ({
        href: anchor.getAttribute("href"),
        rel: anchor.getAttribute("rel"),
        replace: anchor.hasAttribute("replace"),
        preload: anchor.getAttribute("preload"),
    }));
    const opened = new URLSearchParams({ object: JSON.stringify(note) });

    // keep the path for the router, and open objects outside it in the presenting view
    expect(anchors).toEqual([
        { href: "/board", rel: null, replace: true, preload: "false" },
        {
            href: `/.destack/open?${opened.toString()}`,
            rel: "external",
            replace: false,
            preload: null,
        },
        {
            href: `/.destack/open?${opened.toString()}&view=editor`,
            rel: "external",
            replace: false,
            preload: null,
        },
    ]);
});
