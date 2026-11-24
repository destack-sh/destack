import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { CollageLayout, Gallery, GalleryItem, GridLayout, type Layout } from "./index.ts";

/** Render numbered items in a layout, and collect those opened. */
function draw(layout: Layout, count: number) {
    const opened: number[] = [];
    const [numbers, setNumbers] = createSignal(Array.from({ length: count }, (_each, i) => i + 1));
    const { container } = render(() => (
        <Gallery each={numbers()} layout={layout}>
            {(number) => (
                <GalleryItem
                    label={`Photo ${String(number())}`}
                    onOpen={() => void opened.push(number())}
                />
            )}
        </Gallery>
    ));
    flush();
    const items = () => [...container.querySelectorAll<HTMLElement>("[data-slot=gallery-open]")];

    return { container, opened, items, setNumbers };
}

test("show as many as fit and count the others", () => {
    const { container, items, setNumbers } = draw(new CollageLayout({ count: 3 }), 8);
    const more = () => container.querySelector("[data-slot=gallery-more]")?.textContent;
    const many = [items().length, more(), items().at(-1)?.getAttribute("aria-label")];
    setNumbers([1, 2]);
    flush();

    expect({ many, few: [items().length, more()] }).toEqual({
        many: [3, "+5", "Photo 3 +5"],
        few: [2, undefined],
    });
});

test("open items from one Tab stop", () => {
    const { items, opened } = draw(new GridLayout({ minimum: 100 }), 3);
    items()[0]?.focus();
    flush();
    for (const key of ["ArrowRight", "End"]) {
        document.activeElement?.dispatchEvent(
            new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }),
        );
        flush();
    }
    items()[2]?.click();

    expect({
        focused: document.activeElement?.getAttribute("aria-label"),
        stops: items().map(({ tabIndex }) => tabIndex),
        opened,
    }).toEqual({ focused: "Photo 3", stops: [-1, -1, 0], opened: [3] });
});

test("place items a browser cannot", () => {
    const layout: Layout = {
        gap: 0,
        place: (ratios) => ratios.map((_ratio, i) => ({ x: i, y: 0, width: 1, height: 1 })),
    };
    const { container, items } = draw(layout, 2);

    // the page measures nothing, so the items wait in a grid
    expect([
        container.querySelector("[data-slot=gallery-items]")?.getAttribute("data-flow"),
        items().length,
    ]).toEqual(["placed", 2]);
});
