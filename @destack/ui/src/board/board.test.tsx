import { expect, test } from "@destack/test";
import { Shortcut } from "@destack/locale";
import { flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import type { SortableMove } from "../sortable/index.ts";
import {
    Board,
    BoardColumn,
    BoardColumnContent,
    BoardColumnHandle,
    BoardColumnHeader,
    BoardItem,
} from "./index.ts";

/** The marks that isolate interpolated names in rendered messages. */
const ISOLATES = /[⁦-⁩]/gu;

/** The pixels a column takes across, with its gap. */
const COLUMN = 300;

/** Render a launch board of three columns, its card and column moves collected. */
function drawBoard(isAllowed?: (item: string, column: string) => boolean): {
    readonly container: HTMLElement;
    readonly items: SortableMove[];
    readonly columns: SortableMove[];
    readonly opened: string[];
} {
    // render the columns with their cards
    const items: SortableMove[] = [];
    const columns: SortableMove[] = [];
    const opened: string[] = [];
    const lists = [
        ["todo", "To do", ["Write the post", "Record the demo"]],
        ["doing", "Doing", ["Price the plans"]],
        ["done", "Done", []],
    ] as const;
    const { container } = render(() => (
        <Board
            label="Launch"
            onAction={(key) => opened.push(key)}
            {...(isAllowed === undefined ? {} : { isAllowed })}
            onItemMove={(move) => items.push(move)}
            onColumnMove={(move) => columns.push(move)}
        >
            {lists.map(([id, label, cards]) => (
                <BoardColumn id={id} label={label}>
                    <BoardColumnHeader>
                        <BoardColumnHandle /> {label} {cards.length}
                    </BoardColumnHeader>
                    <BoardColumnContent>
                        {cards.map((card) => (
                            <BoardItem id={card.toLowerCase().replaceAll(" ", "-")}>
                                {card}
                            </BoardItem>
                        ))}
                    </BoardColumnContent>
                </BoardColumn>
            ))}
        </Board>
    ));

    return { container, items, columns, opened };
}

/** Press and release a key on the focused element. */
function press(container: HTMLElement, key: string): void {
    for (const type of ["keydown", "keyup"]) {
        (container.ownerDocument.activeElement ?? container).dispatchEvent(
            new KeyboardEvent(type, { key, bubbles: true, cancelable: true }),
        );
    }
    flush();
}

/** Press and release a pointer on a card. */
function click(card: HTMLElement | undefined, options: PointerEventInit): void {
    for (const type of ["pointerdown", "pointerup"]) {
        card?.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                pointerId: 2,
                width: 1,
                height: 1,
                ...options,
            }),
        );
    }
    flush();
}

/** Read the live region's announcement without the isolating marks. */
function announced(container: HTMLElement): string {
    return (container.querySelector("[data-slot=drag-announcer]")?.textContent ?? "").replaceAll(
        ISOLATES,
        "",
    );
}

/** Find an element by its slot and text. */
function find(container: HTMLElement, slot: string, label: string): HTMLElement {
    const found = [...container.querySelectorAll<HTMLElement>(`[data-slot=${slot}]`)].find(
        (element) => element.textContent?.includes(label),
    );
    if (found === undefined) {
        throw new TypeError(`no '${slot}' with '${label}'`);
    }

    return found;
}

test("move a card into the next column with Space and the arrow keys, leaving Enter to the card", () => {
    // press Enter, which leaves the card alone, then lift, cross a column and drop
    const { container, items } = drawBoard();
    find(container, "board-item", "Record the demo").focus();
    press(container, "Enter");
    const afterEnter = announced(container);
    const announcements: string[] = [];
    for (const key of [" ", "ArrowRight", " "]) {
        press(container, key);
        announcements.push(announced(container));
    }

    expect({ afterEnter, announcements, items }).toEqual({
        afterEnter: "",
        announcements: [
            "Picked up Record the demo, position 2 of 2 in To do.",
            "Record the demo moved to position 2 of 2 in Doing.",
            "Record the demo dropped at position 2 of 2 in Doing.",
        ],
        items: [
            {
                id: "record-the-demo",
                container: "doing",
                previous: "price-the-plans",
                next: undefined,
            },
        ],
    });
});

test("drag a card with the pointer into an empty column, and keep it where it is short of the lift distance", () => {
    stubPopovers();
    // lay out three columns side by side, each card forty pixels high
    const { container, items } = drawBoard();
    const contents = container.querySelectorAll<HTMLElement>("[data-slot=board-column-content]");
    contents.forEach((content, index) => {
        content.getBoundingClientRect = () => new DOMRect(index * COLUMN, 0, 280, 400);
    });
    container.querySelectorAll<HTMLElement>("[data-slot=board-item]").forEach((item, index) => {
        const column = item.closest("[data-slot=board-column-content]") === contents[0] ? 0 : 1;
        item.getBoundingClientRect = () => new DOMRect(column * COLUMN, (index % 2) * 40, 280, 40);
    });

    // nudge a card by two pixels, then drag it into Done and let go
    const card = find(container, "board-item", "Write the post");
    const pointer = (type: string, x: number, y: number) => {
        card.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: y,
                pointerId: 1,
                width: 1,
                height: 1,
            }),
        );
        flush();
    };
    pointer("pointerdown", 20, 20);
    pointer("pointermove", 22, 20);
    pointer("pointerup", 22, 20);
    const nudged = [...items];
    pointer("pointerdown", 20, 20);
    pointer("pointermove", 2 * COLUMN + 20, 20);
    pointer("pointerup", 2 * COLUMN + 20, 20);

    expect({ nudged, items }).toEqual({
        nudged: [],
        items: [{ id: "write-the-post", container: "done", previous: undefined, next: undefined }],
    });
});

test("move a column along the row by its handle, reported apart from card moves", () => {
    // lift Doing by its handle and move it before To do
    const { container, items, columns } = drawBoard();
    find(container, "board-column", "Doing")
        .querySelector<HTMLElement>("[data-slot=board-column-handle]")
        ?.focus();
    for (const key of ["Enter", "ArrowLeft", "Enter"]) {
        press(container, key);
    }

    expect({ columns, items }).toEqual({
        columns: [{ id: "doing", container: "board-columns", previous: undefined, next: "todo" }],
        items: [],
    });
});

test("move the focus across columns with the arrows, skipping empty ones, and open a card with Enter", () => {
    // focus the first card, step right twice and down, then open the focused card
    const { container, opened } = drawBoard();
    const cards = container.querySelectorAll<HTMLElement>("[data-slot=board-item]");
    cards[0]?.focus();
    const visits: string[] = [];
    for (const key of ["ArrowRight", "ArrowRight", "ArrowLeft", "ArrowDown", "Enter"]) {
        press(container, key);
        visits.push(container.ownerDocument.activeElement?.textContent ?? "");
    }

    // one card at a time takes the tab stop
    expect({
        visits,
        tabStops: [...cards].filter((card) => card.tabIndex === 0).map((card) => card.textContent),
        opened,
    }).toEqual({
        visits: [
            "Price the plans",
            "Price the plans",
            "Write the post",
            "Record the demo",
            "Record the demo",
        ],
        tabStops: ["Record the demo"],
        opened: ["record-the-demo"],
    });
});

test("select cards with x, shift and the arrows and command-click, clear them with Escape, and select all", () => {
    // read the selected cards after each step
    const { container, opened } = drawBoard();
    const cards = [...container.querySelectorAll<HTMLElement>("[data-slot=board-item]")];
    const selected = () =>
        cards
            .filter((card) => card.getAttribute("aria-selected") === "true")
            .map((card) => card.textContent);
    const steps: (string | null)[][] = [];
    cards[0]?.focus();
    press(container, "x");
    steps.push(selected());
    container.ownerDocument.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", {
            key: "ArrowDown",
            shiftKey: true,
            bubbles: true,
            cancelable: true,
        }),
    );
    flush();
    steps.push(selected());
    click(cards[2], { metaKey: true });
    steps.push(selected());
    press(container, "Escape");
    steps.push(selected());

    // select every card with the platform's mod key and A
    const isCommand = Shortcut.isCommand(navigator.userAgent);
    container.ownerDocument.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", {
            key: "a",
            metaKey: isCommand,
            ctrlKey: !isCommand,
            bubbles: true,
            cancelable: true,
        }),
    );
    flush();
    steps.push(selected());

    expect({ steps, opened }).toEqual({
        steps: [
            ["Write the post"],
            ["Write the post", "Record the demo"],
            ["Write the post", "Record the demo", "Price the plans"],
            [],
            ["Write the post", "Record the demo", "Price the plans"],
        ],
        opened: [],
    });
});

test("skip a column a card may not enter, which the drag marks as refusing it", () => {
    // refuse Doing to every card, then move a card right from To do
    const { container, items } = drawBoard((_item, column) => column !== "doing");
    container.querySelector<HTMLElement>("[data-slot=board-item]")?.focus();
    press(container, " ");
    const states = [
        ...container.querySelectorAll<HTMLElement>("[data-slot=board-column-content]"),
    ].map((content) => content.dataset["state"]);
    press(container, "ArrowRight");
    press(container, " ");

    expect({ states, items }).toEqual({
        states: ["over", "refused", "idle"],
        items: [{ id: "write-the-post", container: "done", previous: undefined, next: undefined }],
    });
});
