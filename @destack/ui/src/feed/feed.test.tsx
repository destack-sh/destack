import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { render } from "@destack/view/test";
import { layOutElements } from "../virtualizer/test/layout.ts";
import { Feed } from "./feed.tsx";

/** Send a key to the focused element. */
function page(key: string): void {
    document.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }),
    );
}

test("render a feed of labelled articles that say their place, and move between them by page keys", () => {
    // render three entries of five, then page down from the first and up again
    layOutElements({ client: 100, scroll: 2000 });
    const { container } = render(() => (
        <>
            <button type="button">Before</button>
            <Feed
                each={["one", "two", "three"]}
                label={(entry) => `Entry ${entry}`}
                estimateSize={() => 20}
                total={5}
                isLoading
                aria-label="Activity"
            >
                {(entry) => entry}
            </Feed>
            <button type="button">After</button>
        </>
    ));
    flush();
    const feed = container.querySelector("[data-slot=feed]");
    const articles = [...container.querySelectorAll<HTMLElement>("[role=article]")];
    articles[0]?.focus();
    page("PageDown");
    const down = document.activeElement?.textContent;
    page("PageUp");
    const up = document.activeElement?.textContent;
    document.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", {
            key: "End",
            ctrlKey: true,
            bubbles: true,
            cancelable: true,
        }),
    );

    expect({
        feed: [
            feed?.getAttribute("role"),
            feed?.getAttribute("aria-busy"),
            feed?.getAttribute("aria-label"),
        ],
        articles: articles.map((article) => [
            article.getAttribute("aria-label"),
            article.getAttribute("aria-posinset"),
            article.getAttribute("aria-setsize"),
            article.getAttribute("tabindex"),
        ]),
        focus: [down, up, document.activeElement?.textContent],
    }).toEqual({
        feed: ["feed", "true", "Activity"],
        articles: [
            ["Entry one", "1", "5", "0"],
            ["Entry two", "2", "5", "0"],
            ["Entry three", "3", "5", "0"],
        ],
        focus: ["two", "one", "After"],
    });
});
