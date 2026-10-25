import { expect, test } from "@destack/test";
import { render, wait } from "../test/dom.ts";
import { renderOnServer } from "../test/hydration.ts";
import { NotesAtInvalidPage } from "./search.fixture.tsx";
import { testBrowser } from "../test/browser.ts";

/** The server's HTML of the fixture, which renders at an address with an invalid page. */
const { NotesAtInvalidPage: html = "" } = await renderOnServer("src/router/search.fixture.tsx");

test("render URL state on the server at its defaults, then hydrate without a mismatch and rewrite the invalid value away", async () => {
    // hydrate in the browser at the server's address, which canonicalises the query as hydration ends
    testBrowser().navigation.navigate("/notes?q=milk&page=x", { history: "replace" });
    const container = document.createElement("div");
    container.innerHTML = html;
    render(() => <NotesAtInvalidPage />, { container, hydrate: true });
    await wait(0);

    expect([
        html,
        container.innerHTML,
        new URL(testBrowser().navigation.currentEntry.url).search,
    ]).toEqual([
        "<p _hk=007041002><!--$-->milk<!--/--> <!--$-->1<!--/--> <!--$-->?q=milk&amp;page=x<!--/--> <!--$-->unblocked<!--/--></p>",
        '<p _hk="007041002"><!--$-->milk<!--/--> <!--$-->1<!--/--> <!--$-->?q=milk<!--/--> <!--$-->unblocked<!--/--></p>',
        "?q=milk",
    ]);
});
