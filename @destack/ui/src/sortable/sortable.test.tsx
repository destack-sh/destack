import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { draw } from "@destack/view/test";
import {
    Sortable,
    SortableContainer,
    SortableHandle,
    SortableItem,
    type SortableMove,
} from "./index.ts";

/** Render a board of two columns, its moves collected. */
function drawBoard(moves: SortableMove[]): HTMLElement {
    return draw(() => (
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
    ));
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
    return (
        container.querySelector("[data-slot=sortable-announcer]")?.textContent ?? ""
    ).replaceAll(ISOLATES, "");
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
        moves: [{ id: "a", container: "done", previous: "d", next: undefined }],
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
        dragging: container.querySelector("[data-state=dragging]"),
    }).toEqual({
        announcement: "Moving Review was cancelled. It returned to position 2 of 3 in To do.",
        moves: [],
        dragging: null,
    });
});

test("drag an item with the pointer past the middle of another and drop it there", () => {
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
    const state = container.querySelector("[data-slot=sortable-item]")?.getAttribute("data-state");
    pointer("pointerup", 35);
    flush();

    expect({ state, moves }).toEqual({
        state: "dragging",
        moves: [{ id: "a", container: "todo", previous: "b", next: "c" }],
    });
});

test("nest an item under the one before it with the right arrow, and move a nested one back out with the left", () => {
    const moves: SortableMove[] = [];
    const container = draw(() => (
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
        { id: "faq", container: "guide", previous: "setup", next: undefined },
        { id: "setup", container: "root", previous: "guide", next: "faq" },
    ]);
});
