import { expect, test } from "@destack/test";
import { render } from "../test/dom.ts";
import { renderOnServer } from "../test/hydration.ts";
import { NotesAtInvalidPage } from "./search.fixture.tsx";

/** The server's HTML of the fixture, which renders at an address with an invalid page. */
const { NotesAtInvalidPage: html = "" } = await renderOnServer("src/router/search.fixture.tsx");

test("render URL state on the server at its defaults, then hydrate without a mismatch and rewrite the invalid value away", () => {
    // hydrate in the browser at the server's address, which canonicalises the query as hydration ends
    window.history.replaceState(null, "", "/notes?q=milk&page=x");
    const container = document.createElement("div");
    container.innerHTML = html;
    render(() => <NotesAtInvalidPage />, { container, hydrate: true });

    expect([html, container.innerHTML, location.search]).toEqual([
        "<p _hk=7041002><!--$-->milk<!--/--> <!--$-->1<!--/--> <!--$-->?q=milk&amp;page=x<!--/--> <!--$-->unblocked<!--/--></p>",
        '<p _hk="7041002"><!--$-->milk<!--/--> <!--$-->1<!--/--> <!--$-->?q=milk<!--/--> <!--$-->unblocked<!--/--></p>',
        "?q=milk",
    ]);
});
