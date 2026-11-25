import { expect, test } from "@destack/test";
import { Locale, Localization } from "@destack/locale";
import { Errored, For, flush, LocaleContext } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { VirtualList } from "../virtual-list/index.ts";
import { layOutElements } from "../virtualizer/test/layout.ts";
import {
    Sortable,
    SortableContainer,
    SortableHandle,
    SortableItem,
    SortableMove,
} from "./index.ts";

/** Render a board of two columns, its moves collected. */
function drawBoard(moves: SortableMove[]): HTMLElement {
    return render(() => (
        <Sortable onMove={(move) => moves.push(move)}>
            <SortableContainer id="todo" label="To do">
                <SortableItem id="a">
                    <SortableHandle /> Write
                </SortableItem>
                <SortableItem id="b">
                    <SortableHandle /> Review
                </SortableItem>
                <SortableItem id="c">
                    <SortableHandle /> Ship
                </SortableItem>
            </SortableContainer>
            <SortableContainer id="done" label="Done">
                <SortableItem id="d">
                    <SortableHandle /> Plan
                </SortableItem>
            </SortableContainer>
        </Sortable>
    )).container;
}

/** Give an element a box at a height in a column two hundred pixels wide. */
function place(element: Element, top: number, height: number): void {
    Object.defineProperty(element, "getBoundingClientRect", {
        value: () => new DOMRect(0, top, 200, height),
    });
}

/** Press a key on the focused element. */
function press(container: HTMLElement, key: string): void {
    (container.ownerDocument.activeElement ?? container).dispatchEvent(
        new KeyboardEvent("keydown", { key, bubbles: true }),
    );
    flush();
}

/** The marks that isolate interpolated names in rendered messages. */
const ISOLATES = /[\u2066-\u2069]/gu;

/** Read the live region's announcement without the isolating marks. */
function announced(container: HTMLElement): string {
    return (container.querySelector("[data-slot=drag-announcer]")?.textContent ?? "").replaceAll(
        ISOLATES,
        "",
    );
}

test("lift an item with Space, move it down and across with the arrow keys, announce each step and report the drop", () => {
    const moves: SortableMove[] = [];
    const container = drawBoard(moves);
    container.querySelector<HTMLElement>("[data-slot=sortable-handle]")?.focus();
    press(container, " ");
    const lifted = announced(container);
    press(container, "ArrowDown");
    const moved = announced(container);
    const marked = container.querySelector("[data-drop]")?.textContent?.trim();
    press(container, "ArrowRight");
    press(container, " ");

    expect({ lifted, moved, marked, dropped: announced(container), moves }).toEqual({
        lifted: "Picked up Write, position 1 of 3 in To do.",
        moved: "Write moved to position 2 of 3 in To do.",
        marked: "Ship",
        dropped: "Write dropped at position 2 of 2 in Done.",
        moves: [
            { id: "a", type: "item", container: "done", previous: "d", next: undefined, items: [] },
        ],
    });
});

test("put a lifted item back on Escape without reporting a move", () => {
    const moves: SortableMove[] = [];
    const container = drawBoard(moves);
    container.querySelectorAll<HTMLElement>("[data-slot=sortable-handle]")[1]?.focus();
    press(container, "Enter");
    press(container, "ArrowUp");
    press(container, "Escape");

    expect({
        announcement: announced(container),
        moves,
        dragging: container.querySelector("[data-dragging]"),
    }).toEqual({
        announcement: "Moving Review was cancelled. It returned to position 2 of 3 in To do.",
        moves: [],
        dragging: null,
    });
});

test("drag an item with the pointer past the middle of another, previewed above the page, and drop it there", () => {
    stubPopovers();
    const moves: SortableMove[] = [];
    const container = drawBoard(moves);

    // lay the items out in a column twenty pixels apart
    const [todo, done] = container.querySelectorAll("[data-slot=sortable-container]");
    const items = container.querySelectorAll("[data-slot=sortable-item]");
    place(todo ?? container, 0, 60);
    place(done ?? container, 100, 20);
    items.forEach((item, index) => place(item, index < 3 ? index * 20 : 100, 20));

    // press the first handle and drag it below the second item
    const handle = container.querySelector<HTMLElement>("[data-slot=sortable-handle]");
    const pointer = (type: string, y: number) =>
        handle?.dispatchEvent(
            new PointerEvent(type, { clientX: 10, clientY: y, button: 0, bubbles: true }),
        );
    pointer("pointerdown", 10);
    pointer("pointermove", 35);
    flush();
    const isDragging = container
        .querySelector("[data-slot=sortable-item]")
        ?.hasAttribute("data-dragging");
    const reached = announced(container);
    const preview = container.querySelector<HTMLElement>("[data-slot=drag-preview]");
    const previewed = [
        preview?.hasAttribute("data-popover-open"),
        preview?.firstElementChild?.getAttribute("data-slot"),
        preview?.getAttribute("style"),
    ];
    pointer("pointerup", 35);
    flush();

    // the copy stands where the item stood, moved by the pointer's 25 pixels
    expect({
        isDragging,
        reached,
        previewed,
        isPreviewedAfter: container.querySelector("[data-slot=drag-preview]") !== null,
        moves,
    }).toEqual({
        isDragging: true,
        reached: "Write moved to position 2 of 3 in To do.",
        previewed: [
            true,
            "sortable-item",
            "--x-width: 200px; --x-height: 20px; --x-translate: 0px 25px;",
        ],
        isPreviewedAfter: false,
        moves: [{ id: "a", type: "item", container: "todo", previous: "b", next: "c", items: [] }],
    });
});

test("nest an item under the one before it with the right arrow, and move a nested one back out with the left", () => {
    const moves: SortableMove[] = [];
    const { container } = render(() => (
        <Sortable nesting onMove={(move) => moves.push(move)}>
            <SortableContainer id="root" label="Pages">
                <SortableItem id="guide">
                    <SortableHandle /> Guide
                    <SortableContainer id="guide" label="Guide">
                        <SortableItem id="setup">
                            <SortableHandle /> Setup
                        </SortableItem>
                    </SortableContainer>
                </SortableItem>
                <SortableItem id="faq">
                    <SortableHandle /> FAQ
                </SortableItem>
            </SortableContainer>
        </Sortable>
    ));
    const handles = container.querySelectorAll<HTMLElement>("[data-slot=sortable-handle]");
    handles[2]?.focus();
    press(container, " ");
    press(container, "ArrowRight");
    press(container, " ");
    handles[1]?.focus();
    press(container, " ");
    press(container, "ArrowLeft");
    press(container, " ");

    expect(moves).toEqual([
        {
            id: "faq",
            type: "item",
            container: "guide",
            previous: "setup",
            next: undefined,
            items: [],
        },
        { id: "setup", type: "item", container: "root", previous: "guide", next: "faq", items: [] },
    ]);
});

/** Place an item before a neighbour its list lacks. */
function placeBeforeMissing(): string[] {
    const move = { id: "a", type: "item", container: "second", previous: undefined, next: "q" };

    return SortableMove.place(["x"], "a", move, (id) => id);
}

test("apply a move across keyed lists and within one list, refusing a neighbour the list lacks", () => {
    // move b to the second list before y, move z to the start of its list, and aim at a missing one
    const lists = { first: ["a", "b"], second: ["x", "y"] };
    const across = SortableMove.apply(
        lists,
        { id: "b", container: "second", next: "y" },
        (id) => id,
    );
    const within = SortableMove.place(["x", "y", "z"], "z", { id: "z", next: "x" }, (id) => id);
    expect({ across, within }).toEqual({
        across: { first: ["a"], second: ["x", "b", "y"] },
        within: ["z", "x", "y"],
    });
    expect(placeBeforeMissing).toThrow(new RangeError("sortable neighbour 'q' not found"));
});

test("refuse an item whose id is none of its virtual list's keys", () => {
    layOutElements({ client: 100, scroll: 1000 });
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Sortable>
                <SortableContainer id="tasks">
                    <VirtualList
                        each={["Pack"]}
                        estimateSize={() => 20}
                        itemKey={(task) => `task ${task}`}
                    >
                        {(task) => <SortableItem id={task}>{task}</SortableItem>}
                    </VirtualList>
                </SortableContainer>
            </Sortable>
        </Errored>
    ));
    flush();

    expect(container.textContent).toBe(
        "TypeError: sortable item 'Pack' is not among its virtual list's keys",
    );
});

test("leave a virtual list inside an item to the item, apart from the container's items", () => {
    // render an item whose content scrolls a list inside it, then lift it by the keyboard
    layOutElements({ client: 100, scroll: 1000 });
    const { container } = render(() => (
        <Sortable>
            <SortableContainer id="steps" label="Steps">
                <SortableItem id="Pack" label="Pack">
                    <SortableHandle /> Pack
                    <VirtualList each={["socks", "map"]} estimateSize={() => 20}>
                        {(thing) => thing}
                    </VirtualList>
                </SortableItem>
                <SortableItem id="Board">
                    <SortableHandle /> Board
                </SortableItem>
            </SortableContainer>
        </Sortable>
    ));
    flush();
    container.querySelector<HTMLElement>("[data-slot=sortable-handle]")?.focus();
    press(container, " ");

    expect(announced(container)).toBe("Picked up Pack, position 1 of 2 in Steps.");
});

test("refuse two virtual lists in one container", () => {
    layOutElements({ client: 100, scroll: 1000 });
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Sortable>
                <SortableContainer id="tasks">
                    <VirtualList each={["Pack"]} estimateSize={() => 20} itemKey={(task) => task}>
                        {(task) => <SortableItem id={task}>{task}</SortableItem>}
                    </VirtualList>
                    <VirtualList each={["Board"]} estimateSize={() => 20} itemKey={(task) => task}>
                        {(task) => <SortableItem id={task}>{task}</SortableItem>}
                    </VirtualList>
                </SortableContainer>
            </Sortable>
        </Errored>
    ));
    flush();

    expect(container.textContent).toBe(
        "TypeError: sortable container 'tasks' has several virtual lists",
    );
});

/** Drag the first of three items in a row past the second, and read what each does meanwhile. */
function dragInRow(tag: string, indicator: "line" | "gap") {
    stubPopovers();
    layOutElements({ client: 100, scroll: 100 });
    const moves: SortableMove[] = [];
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
            <Sortable indicator={indicator} onMove={(move) => moves.push(move)}>
                <SortableContainer id="row" orientation="horizontal">
                    <For each={["a", "b", "c"]}>
                        {(id) => (
                            <SortableItem id={id} activator="item">
                                {id}
                            </SortableItem>
                        )}
                    </For>
                </SortableContainer>
            </Sortable>
        </LocaleContext>
    ));
    const isBackward = tag === "ar";
    const items = [...container.querySelectorAll<HTMLElement>("[data-slot=sortable-item]")];
    const row = container.querySelector("[data-slot=sortable-container]") ?? container;
    Object.defineProperty(row, "getBoundingClientRect", {
        value: () => new DOMRect(0, 0, 300, 20),
    });
    items.forEach((item, index) => {
        const left = isBackward ? 200 - index * 100 : index * 100;
        Object.defineProperty(item, "getBoundingClientRect", {
            value: () => new DOMRect(left, 0, 100, 20),
        });
        Object.defineProperty(item, "offsetWidth", { value: 100 });
    });
    const x = (at: number): number => (isBackward ? 300 - at : at);
    for (const [type, at] of [
        ["pointerdown", 50],
        ["pointermove", 60],
        ["pointermove", 190],
    ] as const) {
        items[0]?.dispatchEvent(
            new PointerEvent(type, { clientX: x(at), clientY: 10, button: 0, bubbles: true }),
        );
        flush();
    }
    const rooms = items.map(
        (item) => /: ([^;]*);/u.exec(item.getAttribute("style") ?? "")?.[1] ?? "none",
    );
    items[0]?.dispatchEvent(
        new PointerEvent("pointerup", { clientX: x(190), clientY: 10, button: 0, bubbles: true }),
    );
    flush();

    return { rooms, moves: moves.map(({ previous, next }) => [previous, next]) };
}

test("land between the same items in either direction of the text", () => {
    expect([dragInRow("en", "line").moves, dragInRow("ar", "line").moves]).toEqual([
        [["b", "c"]],
        [["b", "c"]],
    ]);
});

test("make room for a dragged item where it lands", () => {
    expect([dragInRow("en", "gap").rooms, dragInRow("en", "line").rooms]).toEqual([
        ["0px 0px", "-100px 0px", "0px 0px"],
        ["none", "none", "none"],
    ]);
});
