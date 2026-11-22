import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { layOutElements } from "../virtualizer/test/layout.ts";
import { useVirtualizer, VirtualList } from "./virtual-list.tsx";
import { Virtualizer } from "../virtualizer/index.ts";

/** The height and width every element measures, in pixels. */
const VIEWPORT = 100;

/** The size of each item, in pixels. */
const ITEM = 20;

/** Read the rendered items of a list as their index and text. */
function rendered(container: Element): string[] {
    return [...container.querySelectorAll("[data-slot=virtual-item]")].map(
        (item) => `${item.getAttribute("data-index") ?? ""} ${item.textContent}`,
    );
}

test("render only the items in view with some to spare, and others as the list scrolls", () => {
    // render a thousand rows, then scroll fifty rows down
    layOutElements({ client: VIEWPORT, scroll: 1000 * ITEM });
    const each = Array.from({ length: 1000 }, (_, index) => `row ${String(index)}`);
    const { container } = render(() => (
        <VirtualList each={each} estimateSize={() => ITEM} overscan={1}>
            {(row) => row}
        </VirtualList>
    ));
    flush();
    const top = rendered(container);
    const list = container.querySelector("[data-slot=virtual-list]");
    if (list === null) {
        throw new TypeError("the test list did not render");
    }
    list.scrollTop = 50 * ITEM;
    list.dispatchEvent(new Event("scroll"));
    flush();

    expect({
        top,
        scrolled: rendered(container),
        orientation: list.getAttribute("data-orientation"),
    }).toEqual({
        top: ["0 row 0", "1 row 1", "2 row 2", "3 row 3", "4 row 4", "5 row 5"],
        scrolled: [
            "49 row 49",
            "50 row 50",
            "51 row 51",
            "52 row 52",
            "53 row 53",
            "54 row 54",
            "55 row 55",
        ],
        orientation: "vertical",
    });
});

test("follow the items as they change, reuse an item's content while its place changes, and hand out the virtualizer", () => {
    // render three rows that count their renders, then add one in front
    layOutElements({ client: VIEWPORT, scroll: 1000 * ITEM });
    const [each, setEach] = createSignal([{ id: "a" }, { id: "b" }, { id: "c" }]);
    const renders: string[] = [];
    let virtualizer: Virtualizer | undefined;
    const Row = (properties: { readonly id: string }) => {
        virtualizer = useVirtualizer();
        renders.push(properties.id);

        return <>{properties.id}</>;
    };
    const { container } = render(() => (
        <VirtualList
            each={each()}
            estimateSize={() => ITEM}
            itemKey={(row) => row.id}
            itemProperties={(row) => ({ "aria-label": row.id })}
        >
            {(row) => <Row id={row.id} />}
        </VirtualList>
    ));
    flush();
    setEach((current) => [{ id: "z" }, ...current]);
    flush();

    expect({
        rendered: rendered(container),
        renders,
        labels: [...container.querySelectorAll("[data-slot=virtual-item]")].map((item) =>
            item.getAttribute("aria-label"),
        ),
        isVirtualizer: virtualizer instanceof Virtualizer,
    }).toEqual({
        rendered: ["0 z", "1 a", "2 b", "3 c"],
        renders: ["a", "b", "c", "z"],
        labels: ["z", "a", "b", "c"],
        isVirtualizer: true,
    });
});
