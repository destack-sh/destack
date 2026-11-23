import { expect, onTestFinished, test } from "@destack/test";
import { createSignal, Errored, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { layOutElements } from "../virtualizer/test/layout.ts";
import {
    useVirtualizer,
    type VirtualHost,
    VirtualHostContext,
    VirtualList,
    type VirtualWindow,
} from "./virtual-list.tsx";
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

test("hand the list around its keys and places, and mount the item it persists out of view", () => {
    // render a thousand rows inside a list that persists the five hundredth
    layOutElements({ client: VIEWPORT, scroll: 1000 * ITEM });
    const each = Array.from({ length: 1000 }, (_, index) => `row ${String(index)}`);
    const windows: VirtualWindow[] = [];
    const host: VirtualHost = {
        add: (window) => void windows.push(window),
        persisted: () => ["row 500"],
    };
    const { container } = render(() => (
        <VirtualHostContext value={host}>
            <VirtualList each={each} estimateSize={() => ITEM} overscan={1} itemKey={(row) => row}>
                {(row) => row}
            </VirtualList>
        </VirtualHostContext>
    ));
    flush();

    // the list around takes the focus in its items, so the list is no tab stop
    expect({
        keys: windows.map((window) => window.keys().slice(0, 2)),
        place: windows[0]?.positions().get("row 500"),
        rendered: rendered(container),
        tabindex: container.querySelector("[data-slot=virtual-list]")?.getAttribute("tabindex"),
    }).toEqual({
        keys: [["row 0", "row 1"]],
        place: 500,
        rendered: ["0 row 0", "1 row 1", "2 row 2", "3 row 3", "4 row 4", "5 row 5", "500 row 500"],
        tabindex: null,
    });
});

test("refuse a virtual list without item keys inside a list that reads its keys", () => {
    const host: VirtualHost = { add: () => undefined, persisted: () => [] };
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <VirtualHostContext value={host}>
                <VirtualList each={["row"]} estimateSize={() => ITEM}>
                    {(row) => row}
                </VirtualList>
            </VirtualHostContext>
        </Errored>
    ));

    expect(container.textContent).toBe(
        "TypeError: virtual list inside a list that reads it needs an item key",
    );
});

test("reveal an item by its key, scrolling it into view, and refuse a key the list lacks", () => {
    // render a thousand rows inside a list that reads them, then reveal the five hundredth
    layOutElements({ client: VIEWPORT, scroll: 1000 * ITEM });
    const each = Array.from({ length: 1000 }, (_, index) => `row ${String(index)}`);
    const windows: VirtualWindow[] = [];
    const host: VirtualHost = { add: (window) => void windows.push(window), persisted: () => [] };
    const { container } = render(() => (
        <VirtualHostContext value={host}>
            <VirtualList each={each} estimateSize={() => ITEM} overscan={1} itemKey={(row) => row}>
                {(row) => row}
            </VirtualList>
        </VirtualHostContext>
    ));
    flush();
    const window = windows[0];
    window?.reveal("row 500");
    flush();
    const scroller = container.querySelector<HTMLElement>("[data-slot=virtual-list]");

    // the row's end meets the view's end, and a missing key fails by its name
    expect(scroller?.scrollTop).toBe(501 * ITEM - VIEWPORT);
    expect(() => window?.reveal("row 1000")).toThrow(
        new RangeError("virtual list reveal of a missing key 'row 1000'"),
    );
});

test("estimate each item with the gap after it, revealing an item past the gaps before it", () => {
    // render a thousand rows apart by ten pixels, then reveal the five hundredth
    layOutElements({ client: VIEWPORT, scroll: 1000 * ITEM });
    const each = Array.from({ length: 1000 }, (_, index) => `row ${String(index)}`);
    const windows: VirtualWindow[] = [];
    const host: VirtualHost = { add: (window) => void windows.push(window), persisted: () => [] };
    const { container } = render(() => (
        <VirtualHostContext value={host}>
            <VirtualList
                each={each}
                estimateSize={() => ITEM}
                gap={10}
                overscan={1}
                itemKey={(row) => row}
            >
                {(row) => row}
            </VirtualList>
        </VirtualHostContext>
    ));
    flush();
    windows[0]?.reveal("row 500");
    flush();
    const scroller = container.querySelector<HTMLElement>("[data-slot=virtual-list]");

    // the row ends a gap past its text, each row before it a row and a gap tall
    expect(scroller?.scrollTop).toBe(501 * (ITEM + 10) - VIEWPORT);
});

test("read a gap given as a CSS length in pixels, estimating each item with it", () => {
    // render a thousand rows apart by a length measured at ten pixels
    layOutElements({ client: VIEWPORT, scroll: 1000 * ITEM });
    Object.defineProperty(HTMLElement.prototype, "getBoundingClientRect", {
        configurable: true,
        value(this: HTMLElement) {
            return new DOMRect(0, 0, this.dataset["slot"] === "length" ? 10 : 0, 0);
        },
    });
    onTestFinished(() => {
        Reflect.deleteProperty(HTMLElement.prototype, "getBoundingClientRect");
    });
    const each = Array.from({ length: 1000 }, (_, index) => `row ${String(index)}`);
    const windows: VirtualWindow[] = [];
    const host: VirtualHost = { add: (window) => void windows.push(window), persisted: () => [] };
    const { container } = render(() => (
        <VirtualHostContext value={host}>
            <VirtualList
                each={each}
                estimateSize={() => ITEM}
                gap="0.625rem"
                overscan={1}
                itemKey={(row) => row}
            >
                {(row) => row}
            </VirtualList>
        </VirtualHostContext>
    ));
    flush();
    windows[0]?.reveal("row 500");
    flush();
    const scroller = container.querySelector<HTMLElement>("[data-slot=virtual-list]");

    // each row before it a row and a gap tall, past the ring's reach the stub also measures
    expect(scroller?.scrollTop).toBe(10 + 501 * (ITEM + 10) - VIEWPORT);
});
