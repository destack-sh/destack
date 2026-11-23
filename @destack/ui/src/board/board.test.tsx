import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Shortcut } from "@destack/locale";
import { createSignal, Errored, flush, For, Show } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { Draggable, DragProvider, useDragManager } from "../drag/index.ts";
import { VirtualList } from "../virtual-list/index.ts";
import { layOutElements } from "../virtualizer/test/layout.ts";
import { type BoardColumnMove, type BoardDraft, BoardMove, type BoardOrder } from "./index.ts";
import {
    Board,
    BoardColumn,
    BoardColumnContent,
    BoardComposer,
    BoardColumnMenu,
    BoardColumnAdd,
    BoardHiddenList,
    BoardColumnCount,
    BoardColumnList,
    BoardItemMenu,
    BoardLane,
    BoardLaneCell,
    BoardLaneRow,
    BoardColumnHandle,
    BoardColumnHeader,
    BoardItem,
} from "./index.ts";

/** The marks that isolate interpolated names in rendered messages. */
const ISOLATES = /[⁦-⁩]/gu;

/** The pixels a column takes across, with its gap. */
const COLUMN = 300;

/** The height and width every element measures in a long column, in pixels. */
const VIEWPORT = 100;

/** The height of each card in a long column, in pixels. */
const CARD = 20;

/** Render a launch board of three columns, its card and column moves collected. */
function drawBoard(isAllowed?: (item: string, column: string) => boolean): {
    readonly container: HTMLElement;
    readonly items: BoardMove[];
    readonly columns: BoardColumnMove[];
    readonly opened: string[];
} {
    // render the columns with their cards
    const items: BoardMove[] = [];
    const columns: BoardColumnMove[] = [];
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
            <BoardColumnList>
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
            </BoardColumnList>
        </Board>
    ));

    return { container, items, columns, opened };
}

/** Press and release a key on the focused element. */
function press(container: HTMLElement, key: string): void {
    // settle what came before, then each key event as the browser's own task
    for (const type of ["keydown", "keyup"]) {
        flush();
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
                type: "card",
                column: "doing",
                lane: undefined,
                previous: "price-the-plans",
                next: undefined,
            },
        ],
    });
});

test("drag a card with the pointer into an empty column, and leave it where it is short of the lift distance", () => {
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
    const marked = contents[2]?.getAttribute("data-drop");
    pointer("pointerup", 2 * COLUMN + 20, 20);

    // the empty Done marks the drop across itself while the card is over it
    expect({ nudged, marked, items }).toEqual({
        nudged: [],
        marked: "inside",
        items: [
            {
                id: "write-the-post",
                type: "card",
                column: "done",
                lane: undefined,
                previous: undefined,
                next: undefined,
            },
        ],
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
        columns: [
            {
                id: "doing",
                previous: undefined,
                next: "todo",
            },
        ],
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
    ].map((content) => [content.hasAttribute("data-over"), content.hasAttribute("data-refused")]);
    press(container, "ArrowRight");
    press(container, " ");

    expect({ states, items }).toEqual({
        states: [
            [true, false],
            [false, true],
            [false, false],
        ],
        items: [
            {
                id: "write-the-post",
                type: "card",
                column: "done",
                lane: undefined,
                previous: undefined,
                next: undefined,
            },
        ],
    });
});

test("move a card into another column and on again, its card remounting in each column it lands in", () => {
    // render a board whose columns re-render their cards from the moves
    const [cards, setCards] = createSignal<Record<string, readonly string[]>>({
        todo: ["Write the post"],
        doing: [],
        done: [],
    });
    const { container } = render(() => (
        <Board
            label="Launch"
            onItemMove={(move) =>
                setCards((lists) => {
                    const rest = Object.fromEntries(
                        Object.entries(lists).map(([id, list]) => [
                            id,
                            list.filter((card) => card !== move.id),
                        ]),
                    );

                    return {
                        ...rest,
                        [move.column]: [...(rest[move.column] ?? []), move.id],
                    };
                })
            }
        >
            <BoardColumnList>
                <For each={["todo", "doing", "done"]}>
                    {(id) => (
                        <BoardColumn id={id} label={id}>
                            <BoardColumnContent>
                                <For each={cards()[id]}>
                                    {(card) => <BoardItem id={card}>{card}</BoardItem>}
                                </For>
                            </BoardColumnContent>
                        </BoardColumn>
                    )}
                </For>
            </BoardColumnList>
        </Board>
    ));

    // lift it with Space, step right and drop, twice
    for (const key of [" ", "ArrowRight", " ", " ", "ArrowRight", " "]) {
        container.querySelector<HTMLElement>("[data-slot=board-item]")?.focus();
        press(container, key);
    }

    expect(cards()).toEqual({ todo: [], doing: [], done: ["Write the post"] });
});

/** What a team board reports: its moves, removals, new cards and items from other views. */
interface TeamBoard {
    /** The board's element. */
    readonly container: HTMLElement;
    /** The cards' moves in the order the board reported them. */
    readonly moves: BoardMove[];
    /** The cards removed, a list per removal. */
    readonly removed: (readonly string[])[];
    /** The cards written into composers. */
    readonly created: BoardDraft[];
    /** The items of other views dropped on the board. */
    readonly received: BoardMove[];
}

/** Draw a board of To do, Doing and Done, a lane per person where there are people. */
function drawTeam(
    cells: Readonly<Record<string, readonly string[]>>,
    options: {
        readonly people?: readonly string[];
        readonly order?: BoardOrder;
        readonly limit?: number;
        readonly hidden?: readonly string[];
        readonly collapsedLanes?: readonly string[];
        readonly isAllowed?: (item: string, column: string) => boolean;
    } = {},
): TeamBoard {
    // render the columns, then a lane of cells per person, every handler recording
    stubPopovers();
    const moves: BoardMove[] = [];
    const removed: (readonly string[])[] = [];
    const created: BoardDraft[] = [];
    const received: BoardMove[] = [];
    const columns = ["todo", "doing", "done"];
    const people = options.people ?? [];
    const cardsOf = (column: string, lane?: string) =>
        (cells[lane === undefined ? column : `${lane} ${column}`] ?? []).map((card) => (
            <BoardItem id={card}>{card}</BoardItem>
        ));
    const { container } = render(() => (
        <DragProvider>
            <Board
                label="Team"
                order={options.order ?? "manual"}
                defaultHiddenColumns={options.hidden ?? []}
                defaultCollapsedLanes={options.collapsedLanes ?? []}
                {...(options.isAllowed === undefined ? {} : { isAllowed: options.isAllowed })}
                onAction={() => undefined}
                onItemMove={(move) => moves.push(move)}
                onItemRemove={(keys) => removed.push(keys)}
                onCreate={(draft) => created.push(draft)}
                dropTypes={["event"]}
                onReceive={(drop) => received.push(drop)}
            >
                <BoardColumnList>
                    {columns.map((column) => (
                        <BoardColumn
                            id={column}
                            label={column}
                            {...(column === "doing" && options.limit !== undefined
                                ? { limit: options.limit }
                                : {})}
                        >
                            <BoardColumnHeader>
                                {column}
                                <BoardColumnCount />
                                <BoardColumnMenu />
                                <BoardColumnAdd />
                            </BoardColumnHeader>
                            <Show when={people.length === 0}>
                                <BoardComposer position="start" />
                                <BoardColumnContent>{cardsOf(column)}</BoardColumnContent>
                                <BoardComposer />
                            </Show>
                        </BoardColumn>
                    ))}
                    <BoardHiddenList />
                </BoardColumnList>
                {people.map((person) => (
                    <BoardLane id={person} label={person}>
                        <BoardLaneRow>
                            {columns.map((column) => (
                                <BoardLaneCell column={column}>
                                    <BoardColumnContent>
                                        {cardsOf(column, person)}
                                    </BoardColumnContent>
                                </BoardLaneCell>
                            ))}
                        </BoardLaneRow>
                    </BoardLane>
                ))}
                <BoardItemMenu />
            </Board>
        </DragProvider>
    ));

    return { container, moves, removed, created, received };
}

/** Press a key with modifiers on the focused element. */
function chord(container: HTMLElement, key: string, modifiers: KeyboardEventInit): void {
    (container.ownerDocument.activeElement ?? container).dispatchEvent(
        new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...modifiers }),
    );
    flush();
}

/** Write a move as its card, cell and neighbours. */
function moveOf(move: BoardMove): string {
    return `${move.id} → ${move.lane ?? "-"}/${move.column} after ${move.previous ?? "-"} before ${move.next ?? "-"}`;
}

test("move every selected card with the one lifted, in the board's order, one after another", () => {
    // select a and b in To do with x, then lift b and drop it into Doing before c
    const board = drawTeam({ todo: ["a", "b"], doing: ["c"] });
    find(board.container, "board-item", "a").focus();
    press(board.container, "x");
    find(board.container, "board-item", "b").focus();
    press(board.container, "x");
    for (const key of [" ", "ArrowRight", " "]) {
        press(board.container, key);
    }

    // the keys take b to the end of Doing, after c, and a goes ahead of it
    expect(board.moves.map(moveOf)).toEqual([
        "a → -/doing after c before -",
        "b → -/doing after a before -",
    ]);
});

test("move the focused card a column over with Shift+Alt and the arrows, at its row, not past the edge", () => {
    // the second card in To do, beside two cards in Doing
    const board = drawTeam({ todo: ["a", "b"], doing: ["c", "d"] });
    find(board.container, "board-item", "b").focus();
    chord(board.container, "ArrowRight", { shiftKey: true, altKey: true });
    chord(board.container, "ArrowLeft", { shiftKey: true, altKey: true });

    // b lands at its row in Doing, and To do is the first column, so the left move does nothing
    expect(board.moves.map(moveOf)).toEqual(["b → -/doing after c before d"]);
});

test("open a card's menu with a right click: open, move to each other column, and delete", () => {
    // right-click c in Doing, then choose to move it to Done
    const board = drawTeam({ todo: ["a"], doing: ["c"] });
    find(board.container, "board-item", "c").dispatchEvent(
        new MouseEvent("contextmenu", {
            bubbles: true,
            cancelable: true,
            clientX: 10,
            clientY: 10,
        }),
    );
    flush();
    const items = [
        ...document.querySelectorAll<HTMLElement>("[data-slot=board-item-menu] [role=menuitem]"),
    ];
    const titles = items.map((item) => (item.textContent ?? "").replaceAll(ISOLATES, ""));
    items.find((item) => item.textContent?.includes("done"))?.click();
    flush();
    document.querySelectorAll("[data-slot=board-item-menu]").forEach((menu) => menu.remove());

    expect({ titles, moves: board.moves.map(moveOf) }).toEqual({
        titles: ["OpenEnter", "Move to todo", "Move to done", "DeleteDelete"],
        moves: ["c → -/done after - before -"],
    });
});

test("write cards into a column by its composer and by c, closing on Escape", () => {
    // open To do's composer by its button, write two cards, then c in Doing and Escape
    const board = drawTeam({ todo: ["a"], doing: ["c"] });
    const add = board.container.querySelectorAll<HTMLElement>("[data-slot=board-composer-open]");
    add[0]?.click();
    flush();
    const type = (title: string) => {
        const field = board.container.querySelector<HTMLTextAreaElement>(
            "[data-slot=board-composer]",
        );
        if (field === null) {
            throw new TypeError("the composer did not open");
        }
        field.value = title;
        field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
        flush();
    };
    type("Ship it");
    type("Tell people");
    find(board.container, "board-item", "c").focus();
    press(board.container, "c");
    const opened = (
        board.container.querySelector("[data-slot=board-composer]")?.getAttribute("aria-label") ??
        ""
    ).replaceAll(ISOLATES, "");
    board.container
        .querySelector("[data-slot=board-composer]")
        ?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    flush();

    expect({
        created: board.created,
        opened,
        composers: board.container.querySelectorAll("[data-slot=board-composer]").length,
    }).toEqual({
        created: [
            { column: "todo", lane: undefined, title: "Ship it", position: "end" },
            { column: "todo", lane: undefined, title: "Tell people", position: "end" },
        ],
        opened: "New card in doing",
        composers: 0,
    });
});

test("count each column's cards against its limit, and list a hidden column at the row's end", () => {
    // three cards in Doing against a limit of two, To do hidden
    const board = drawTeam({ todo: ["a"], doing: ["b", "c", "d"] }, { limit: 2, hidden: ["todo"] });
    const counts = [...board.container.querySelectorAll("[data-slot=board-column-count]")].map(
        (count) => [
            (count.textContent ?? "").replaceAll(ISOLATES, ""),
            count.hasAttribute("data-over"),
        ],
    );
    const listed = [...board.container.querySelectorAll("[data-slot=board-hidden-column]")].map(
        (row) => (row.textContent ?? "").replaceAll(ISOLATES, ""),
    );
    find(board.container, "board-item", "b").focus();
    press(board.container, "ArrowLeft");
    const focused = board.container.ownerDocument.activeElement?.textContent;
    board.container.querySelector<HTMLElement>("[data-slot=board-column-show]")?.click();
    flush();

    expect({
        counts,
        listed,
        hidden: [...board.container.querySelectorAll("[data-slot=board-column][data-hidden]")]
            .length,
        focused,
    }).toEqual({
        counts: [
            ["1", false],
            ["3 of 2", true],
            ["0", false],
        ],
        // the hidden To do is listed by name and count, unfocused, its button showing it
        listed: ["todo1"],
        hidden: 0,
        focused: "b",
    });
});

test("walk the focus down into the next lane's cell, and move a card a lane over by Shift+Alt", () => {
    // two people's lanes, each with cards in To do
    const board = drawTeam(
        { "ada todo": ["a1", "a2"], "grace todo": ["g1"], "grace doing": ["g2"] },
        { people: ["ada", "grace"] },
    );
    find(board.container, "board-item", "a2").focus();
    flush();
    press(board.container, "ArrowDown");
    const below = board.container.ownerDocument.activeElement?.textContent;
    find(board.container, "board-item", "a1").focus();
    flush();
    chord(board.container, "ArrowDown", { shiftKey: true, altKey: true });

    expect({ below, moves: board.moves.map(moveOf) }).toEqual({
        below: "g1",
        moves: ["a1 → grace/todo after - before g1"],
    });
});

test("leave a sorted cell's order to its owner, reporting only moves into another cell", () => {
    // lift a in a sorted board and step it below b, then on into Doing
    const board = drawTeam({ todo: ["a", "b"], doing: [] }, { order: "sorted" });
    find(board.container, "board-item", "a").focus();
    for (const key of [" ", "ArrowDown", " "]) {
        press(board.container, key);
    }
    find(board.container, "board-item", "a").focus();
    for (const key of [" ", "ArrowRight", " "]) {
        press(board.container, key);
    }

    expect(board.moves.map(moveOf)).toEqual(["a → -/doing after - before -"]);
});

test("take an item of another view dropped into a column, with the cards it lands between", () => {
    // an event dragged by the pointer from beside the board into Doing, above its card
    stubPopovers();
    let element: HTMLElement | undefined;
    const Event = () => {
        const drags = useDragManager();
        const attributes = new Draggable(
            {
                id: "event-standup",
                item: "standup",
                type: "event",
                kind: "move",
                label: () => "Standup",
                element: () => element,
                home: () => ({ kind: "list", droppable: "calendar", index: 0 }),
            },
            drags.manager,
        ).attributes([" "]);

        return (
            <div ref={(created: HTMLDivElement) => (element = created)} {...attributes}>
                Standup
            </div>
        );
    };
    const received: BoardMove[] = [];
    const { container } = render(() => (
        <DragProvider>
            <Event />
            <Board dropTypes={["event"]} onReceive={(drop) => received.push(drop)}>
                <BoardColumnList>
                    {["todo", "doing"].map((column) => (
                        <BoardColumn id={column} label={column}>
                            <BoardColumnContent>
                                {column === "doing" ? <BoardItem id="c">c</BoardItem> : undefined}
                            </BoardColumnContent>
                        </BoardColumn>
                    ))}
                </BoardColumnList>
            </Board>
        </DragProvider>
    ));

    // lay out the columns side by side and c at the top of Doing
    container
        .querySelectorAll<HTMLElement>("[data-slot=board-column-content]")
        .forEach((content, index) => {
            content.getBoundingClientRect = () => new DOMRect(index * COLUMN, 0, 280, 400);
        });
    const card = find(container, "board-item", "c");
    card.getBoundingClientRect = () => new DOMRect(COLUMN, 0, 280, 40);
    for (const [type, x, y] of [
        ["pointerdown", 10, 600],
        ["pointermove", 100, 500],
        ["pointermove", COLUMN + 20, 8],
        ["pointerup", COLUMN + 20, 8],
    ] as const) {
        element?.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: y,
                pointerId: 6,
            }),
        );
        flush();
    }

    expect(received).toEqual([
        {
            id: "standup",
            type: "event",
            column: "doing",
            lane: undefined,
            previous: undefined,
            next: "c",
        },
    ]);
});

test("collapse a lane to its header, its cards out of the focus walk and the steps", () => {
    // the first person's lane collapsed above the second's
    const board = drawTeam(
        { "ada todo": ["a1"], "grace todo": ["g1"] },
        { people: ["ada", "grace"], collapsedLanes: ["ada"] },
    );
    find(board.container, "board-item", "g1").focus();
    press(board.container, "ArrowUp");
    const focused = board.container.ownerDocument.activeElement?.textContent;
    chord(board.container, "ArrowUp", { shiftKey: true, altKey: true });

    expect({
        collapsed: [
            ...board.container.querySelectorAll("[data-slot=board-lane][data-collapsed]"),
        ].map((lane) => lane.getAttribute("aria-label")),
        focused,
        moves: board.moves.map(moveOf),
    }).toEqual({ collapsed: ["ada"], focused: "g1", moves: [] });
});

test("follow the hidden columns the owner controls, reporting each toggle", () => {
    // the owner leaves To do hidden until it hears of a toggle
    stubPopovers();
    const [hidden, setHidden] = createSignal<readonly string[]>(["todo"]);
    const changes: (readonly string[])[] = [];
    const { container } = render(() => (
        <Board
            hiddenColumns={hidden()}
            onHiddenColumnsChange={(columns) => {
                changes.push(columns);
                setHidden(columns);
            }}
        >
            <BoardColumnList>
                {["todo", "doing"].map((column) => (
                    <BoardColumn id={column} label={column}>
                        <BoardColumnHeader>
                            <BoardColumnMenu />
                        </BoardColumnHeader>
                    </BoardColumn>
                ))}
                <BoardHiddenList />
            </BoardColumnList>
        </Board>
    ));
    const hiddenNow = () =>
        [...container.querySelectorAll<HTMLElement>("[data-slot=board-column][data-hidden]")]
            .length;
    const before = hiddenNow();
    chooseInColumnMenu(container, 1, "Hide");

    expect({ before, after: hiddenNow(), changes }).toEqual({
        before: 1,
        after: 2,
        changes: [["todo", "doing"]],
    });
});

test("refuse a board whose columns hide with no list of hidden columns", () => {
    // a column menu, whose Hide needs a hidden list the columns row leaves out
    stubPopovers();
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Board>
                <BoardColumnList>
                    <BoardColumn id="todo" label="todo">
                        <BoardColumnHeader>
                            <BoardColumnMenu />
                        </BoardColumnHeader>
                    </BoardColumn>
                </BoardColumnList>
            </Board>
        </Errored>
    ));
    flush();

    expect(container.textContent).toBe(
        "TypeError: board columns hide into a BoardHiddenList, and the board has none",
    );
});

test("refuse a board whose owner hides a column once it shows, with no list of hidden columns", () => {
    // show every column, then hide To do by the owner's state
    const [hidden, setHidden] = createSignal<readonly string[]>([]);
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Board hiddenColumns={hidden()}>
                <BoardColumnList>
                    <BoardColumn id="todo" label="todo">
                        <BoardColumnHeader>todo</BoardColumnHeader>
                    </BoardColumn>
                </BoardColumnList>
            </Board>
        </Errored>
    ));
    flush();
    const isShown = container.querySelector("[data-slot=board-column]") !== null;
    setHidden(["todo"]);
    flush();

    expect([isShown, container.textContent]).toEqual([
        true,
        "TypeError: board columns hide into a BoardHiddenList, and the board has none",
    ]);
});

/** Open a column's menu by its place among the columns and choose an item of it. */
function chooseInColumnMenu(container: HTMLElement, column: number, item: string): void {
    // press the column's menu button, then the item in the menu it controls
    const trigger = container.querySelectorAll<HTMLElement>("[data-slot=board-column-menu]")[
        column
    ];
    trigger?.click();
    flush();
    const menu = document.getElementById(trigger?.getAttribute("aria-controls") ?? "");
    [...(menu?.querySelectorAll<HTMLElement>("[role=menuitem]") ?? [])]
        .find((each) => each.textContent === item)
        ?.click();
    flush();
}

test("open a new card at a column's top from its header's new button, focusing it again on Escape", async () => {
    // press Doing's new button, type a title and press Enter, then Escape
    const board = drawTeam({ todo: ["a"], doing: ["b"] });
    board.container.querySelectorAll<HTMLElement>("[data-slot=board-column-add]")[1]?.click();
    flush();
    const field = board.container.querySelector<HTMLTextAreaElement>("[data-slot=board-composer]");
    if (field === null) {
        throw new TypeError("the new button opened no composer");
    }
    field.value = "c";
    field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    field.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
    );
    flush();
    await Promise.resolve();
    const focused = board.container.ownerDocument.activeElement;

    expect({
        created: board.created,
        focused: [
            focused?.getAttribute("data-slot"),
            focused?.getAttribute("aria-label")?.replace(ISOLATES, ""),
        ],
    }).toEqual({
        created: [{ column: "doing", lane: undefined, title: "c", position: "start" }],
        focused: ["board-column-add", "New card in doing"],
    });
});

test("open a new card from a column's header in the first shown lane of a board with lanes", () => {
    // collapse Ada's lane, then press Doing's new button and write a title
    stubPopovers();
    const created: BoardDraft[] = [];
    const { container } = render(() => (
        <Board
            label="Team"
            defaultCollapsedLanes={["ada"]}
            onCreate={(draft) => created.push(draft)}
        >
            <BoardColumnList>
                {["todo", "doing"].map((column) => (
                    <BoardColumn id={column} label={column}>
                        <BoardColumnHeader>
                            {column}
                            <BoardColumnAdd />
                        </BoardColumnHeader>
                    </BoardColumn>
                ))}
            </BoardColumnList>
            {["ada", "grace"].map((person) => (
                <BoardLane id={person} label={person}>
                    <BoardLaneRow>
                        {["todo", "doing"].map((column) => (
                            <BoardLaneCell column={column}>
                                <BoardComposer position="start" />
                                <BoardColumnContent />
                            </BoardLaneCell>
                        ))}
                    </BoardLaneRow>
                </BoardLane>
            ))}
        </Board>
    ));
    container.querySelectorAll<HTMLElement>("[data-slot=board-column-add]")[1]?.click();
    flush();
    const field = container.querySelector<HTMLTextAreaElement>("[data-slot=board-composer]");
    if (field === null) {
        throw new TypeError("the new button opened no composer");
    }
    field.value = "c";
    field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    flush();

    expect(created).toEqual([{ column: "doing", lane: "grace", title: "c", position: "start" }]);
});

test("land a card dropped on a hidden column's row at the column's end", () => {
    // lift a in To do and step it right past Done onto the hidden Doing's row at the row's end
    const board = drawTeam({ todo: ["a"], doing: ["b", "c"] }, { hidden: ["doing"] });
    find(board.container, "board-item", "a").focus();
    for (const key of [" ", "ArrowRight", "ArrowRight", " "]) {
        press(board.container, key);
    }

    expect(board.moves.map(moveOf)).toEqual(["a → -/doing after c before -"]);
});

test("land a card dropped on a hidden column's row in its lane on a board with lanes", () => {
    // drag g in Grace's To do onto the hidden Doing's row, which every lane's Doing aims at
    const board = drawTeam(
        { "ada todo": ["a"], "ada doing": ["d"], "grace todo": ["g"], "grace doing": ["x"] },
        { people: ["ada", "grace"], hidden: ["doing"] },
    );
    const card = find(board.container, "board-item", "g");
    card.getBoundingClientRect = () => new DOMRect(0, 100, 280, 40);
    const row = board.container.querySelector<HTMLElement>(
        "[data-slot=board-hidden-column][data-column=doing]",
    );
    if (row === null) {
        throw new TypeError("the hidden Doing has no row");
    }
    row.getBoundingClientRect = () => new DOMRect(2 * COLUMN, 0, 200, 32);
    for (const [type, x, y] of [
        ["pointerdown", 20, 120],
        ["pointermove", 2 * COLUMN + 20, 16],
        ["pointerup", 2 * COLUMN + 20, 16],
    ] as const) {
        card.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: y,
                pointerId: 1,
            }),
        );
        flush();
    }

    expect(board.moves.map(moveOf)).toEqual(["g → grace/doing after x before -"]);
});

test("refuse a second list of hidden columns on one board", () => {
    // mount two hidden lists in the columns row
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Board>
                <BoardColumnList>
                    <BoardColumn id="todo" label="todo">
                        <BoardColumnHeader>todo</BoardColumnHeader>
                    </BoardColumn>
                    <BoardHiddenList />
                    <BoardHiddenList />
                </BoardColumnList>
            </Board>
        </Errored>
    ));
    flush();

    expect(container.textContent).toBe(
        "TypeError: a board has one BoardHiddenList, and this board has two",
    );
});

test("land a card dragged by the pointer onto a hidden column's row at the column's end", () => {
    // lay out To do's card and the hidden Doing's row apart, then drag the card onto the row
    const board = drawTeam({ todo: ["a"], doing: ["b", "c"] }, { hidden: ["doing"] });
    const card = find(board.container, "board-item", "a");
    card.getBoundingClientRect = () => new DOMRect(0, 0, 280, 40);
    const row = board.container.querySelector<HTMLElement>(
        "[data-slot=board-hidden-column][data-column=doing]",
    );
    if (row === null) {
        throw new TypeError("the hidden Doing has no row");
    }
    row.getBoundingClientRect = () => new DOMRect(2 * COLUMN, 0, 200, 32);
    for (const [type, x] of [
        ["pointerdown", 20],
        ["pointermove", 2 * COLUMN + 20],
        ["pointerup", 2 * COLUMN + 20],
    ] as const) {
        card.dispatchEvent(
            new PointerEvent(type, {
                bubbles: true,
                cancelable: true,
                clientX: x,
                clientY: 16,
                pointerId: 1,
            }),
        );
        flush();
    }

    expect(board.moves.map(moveOf)).toEqual(["a → -/doing after c before -"]);
});

test("select every shown card with Mod+A and delete them, leaving a hidden column's cards out", () => {
    // the hidden Done column, whose card stays out of the selection
    const board = drawTeam({ todo: ["a"], doing: ["b"], done: ["c"] }, { hidden: ["done"] });
    find(board.container, "board-item", "a").focus();
    chord(
        board.container,
        "a",
        Shortcut.isCommand(navigator.userAgent) ? { metaKey: true } : { ctrlKey: true },
    );
    press(board.container, "Delete");

    expect(board.removed).toEqual([["a", "b"]]);
});

test("offer a selection's menu no Open, and only the columns that let every card in", () => {
    // a and b selected, Done refusing a
    stubPopovers();
    const board = drawTeam(
        { todo: ["a", "b"], doing: [] },
        { isAllowed: (item, column) => !(item === "a" && column === "done") },
    );
    find(board.container, "board-item", "a").focus();
    press(board.container, "x");
    find(board.container, "board-item", "b").focus();
    press(board.container, "x");
    find(board.container, "board-item", "b").dispatchEvent(
        new MouseEvent("contextmenu", {
            bubbles: true,
            cancelable: true,
            clientX: 10,
            clientY: 10,
        }),
    );
    flush();
    const items = [
        ...document.querySelectorAll<HTMLElement>("[data-slot=board-item-menu] [role=menuitem]"),
    ];
    const titles = items.map((item) => (item.textContent ?? "").replaceAll(ISOLATES, ""));
    items.find((item) => item.textContent?.includes("doing"))?.click();
    flush();
    document.querySelectorAll("[data-slot=board-item-menu]").forEach((menu) => menu.remove());

    expect({ titles, moves: board.moves.map(moveOf) }).toEqual({
        titles: ["Move to doing", "DeleteDelete"],
        moves: ["a → -/doing after - before -", "b → -/doing after a before -"],
    });
});

/** Read the key a test owner files a cell's cards under: "column", or "lane column". */
function keyOf(cell: { readonly column: string; readonly lane: string | undefined }): string {
    return cell.lane === undefined ? cell.column : `${cell.lane} ${cell.column}`;
}

/** Draw a board whose owner applies every move and removal, a lane per person where given. */
function drawLive(
    initial: Readonly<Record<string, readonly string[]>>,
    people: readonly string[] = [],
): { readonly container: HTMLElement; readonly cards: () => Record<string, readonly string[]> } {
    // file the cards by cell as the owner, under "column" or "lane column"
    stubPopovers();
    const [cards, setCards] = createSignal<Record<string, readonly string[]>>({ ...initial });
    const columns = ["todo", "doing", "done"];
    const cell = (column: string, lane?: string) => (
        <BoardColumnContent>
            <For each={cards()[keyOf({ column, lane })] ?? []}>
                {(card) => <BoardItem id={card}>{card}</BoardItem>}
            </For>
        </BoardColumnContent>
    );
    const { container } = render(() => (
        <Board
            onItemMove={(move) =>
                setCards((current) => BoardMove.apply(current, move, keyOf, (card) => card))
            }
            onItemRemove={(keys) =>
                setCards((current) =>
                    Object.fromEntries(
                        Object.entries(current).map(([key, list]) => [
                            key,
                            list.filter((card) => !keys.includes(card)),
                        ]),
                    ),
                )
            }
        >
            <BoardColumnList>
                {columns.map((column) => (
                    <BoardColumn id={column} label={column}>
                        {people.length === 0 ? cell(column) : undefined}
                    </BoardColumn>
                ))}
            </BoardColumnList>
            {people.map((person) => (
                <BoardLane id={person} label={person}>
                    <BoardLaneRow>
                        {columns.map((column) => (
                            <BoardLaneCell column={column}>{cell(column, person)}</BoardLaneCell>
                        ))}
                    </BoardLaneRow>
                </BoardLane>
            ))}
            <BoardItemMenu />
        </Board>
    ));

    return { container, cards };
}

/** Open a card's menu with a right click and choose an item by its text. */
async function choose(container: HTMLElement, card: string, item: string): Promise<void> {
    // right-click the card, then press the item and let the focus settle
    find(container, "board-item", card).dispatchEvent(
        new MouseEvent("contextmenu", {
            bubbles: true,
            cancelable: true,
            clientX: 10,
            clientY: 10,
        }),
    );
    flush();
    [...document.querySelectorAll<HTMLElement>("[role=menuitem]")]
        .find((each) => each.textContent?.includes(item))
        ?.click();
    flush();
    await Promise.resolve();
    flush();
}

test("delete a card from its menu, the focus moving to the card after it, the menu closing cleanly", async () => {
    // b deleted from its menu, between a and c
    const board = drawLive({ todo: ["a", "b", "c"] });
    await choose(board.container, "b", "Delete");

    expect({
        cards: board.cards(),
        focused: board.container.ownerDocument.activeElement?.textContent,
    }).toEqual({ cards: { todo: ["a", "c"] }, focused: "c" });
});

test("move a card to another column from its menu, the focus following it there", async () => {
    // a moved to Done from its menu
    const board = drawLive({ todo: ["a"], done: ["d"] });
    await choose(board.container, "a", "done");

    expect({
        cards: board.cards(),
        focused: board.container.ownerDocument.activeElement?.textContent,
    }).toEqual({ cards: { todo: [], done: ["d", "a"] }, focused: "a" });
});

test("confine the arrow keys of an open card menu to the menu, away from the cards", () => {
    // the menu opened on a, then the right arrow pressed in it
    const board = drawLive({ todo: ["a"], doing: ["b"] });
    find(board.container, "board-item", "a").dispatchEvent(
        new MouseEvent("contextmenu", {
            bubbles: true,
            cancelable: true,
            clientX: 10,
            clientY: 10,
        }),
    );
    flush();
    const item = document.querySelector<HTMLElement>("[role=menuitem]");
    item?.focus();
    item?.dispatchEvent(
        new KeyboardEvent("keydown", { key: "ArrowRight", bubbles: true, cancelable: true }),
    );
    flush();
    const focused = board.container.ownerDocument.activeElement?.getAttribute("role");
    document.querySelectorAll("[data-slot=board-item-menu]").forEach((menu) => menu.remove());

    expect(focused).toBe("menuitem");
});

test("step a lifted card across its lane's columns by keys, never into the next lane", () => {
    // a1 lifted in Ada's To do and stepped right three times
    const board = drawLive(
        { "ada todo": ["a1"], "ada doing": [], "ada done": [], "grace todo": ["g1"] },
        ["ada", "grace"],
    );
    find(board.container, "board-item", "a1").focus();
    for (const key of [" ", "ArrowRight", "ArrowRight", "ArrowRight", " "]) {
        press(board.container, key);
    }

    // the last column is Ada's Done, where a1 lands, Grace's lane untouched
    expect(board.cards()).toEqual({
        "ada todo": [],
        "ada doing": [],
        "ada done": ["a1"],
        "grace todo": ["g1"],
    });
});

test("move the selection by Shift+Alt from the focused card, whichever card was selected first", () => {
    // a in To do selected first, then c in Doing, focused, then Shift+Alt+Right
    const board = drawLive({ todo: ["a"], doing: ["c"], done: [] });
    find(board.container, "board-item", "a").focus();
    press(board.container, "x");
    find(board.container, "board-item", "c").focus();
    press(board.container, "x");
    chord(board.container, "ArrowRight", { shiftKey: true, altKey: true });

    expect(board.cards()).toEqual({ todo: [], doing: [], done: ["a", "c"] });
});

test("focus the composer c opens from a card, and hand the focus back to that card on Escape", async () => {
    // c pressed on b in To do
    const board = drawTeam({ todo: ["a", "b"] });
    find(board.container, "board-item", "b").focus();
    press(board.container, "c");
    await Promise.resolve();
    const opened = board.container.ownerDocument.activeElement?.getAttribute("data-slot");
    board.container.ownerDocument.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
    );
    flush();
    await Promise.resolve();

    expect({ opened, closed: board.container.ownerDocument.activeElement?.textContent }).toEqual({
        opened: "board-composer",
        closed: "b",
    });
});

/** Render a board whose owner deletes cards, To do and Doing each with a composer. */
function drawDeleting(initial: Readonly<Record<string, readonly string[]>>): {
    readonly container: HTMLElement;
    readonly remove: (key: string) => void;
} {
    stubPopovers();
    const [cards, setCards] = createSignal(initial);
    const { container } = render(() => (
        <Board label="Team" onCreate={() => undefined}>
            <BoardColumnList>
                {["todo", "doing"].map((column) => (
                    <BoardColumn id={column} label={column}>
                        <BoardColumnContent>
                            <For each={cards()[column] ?? []}>
                                {(card) => <BoardItem id={card}>{card}</BoardItem>}
                            </For>
                        </BoardColumnContent>
                        <BoardComposer />
                    </BoardColumn>
                ))}
            </BoardColumnList>
        </Board>
    ));
    const remove = (key: string) =>
        setCards((current) =>
            Object.fromEntries(
                Object.entries(current).map(([column, keys]) => [
                    column,
                    keys.filter((each) => each !== key),
                ]),
            ),
        );

    return { container, remove };
}

/** Press c on the first card, delete the cards, close the composer, and read the focus. */
async function closeAfterDeleting(
    cells: Readonly<Record<string, readonly string[]>>,
    keys: readonly string[],
): Promise<string | undefined> {
    const [opener] = keys;
    const { container, remove } = drawDeleting(cells);
    find(container, "board-item", present(opener, "the opener")).focus();
    press(container, "c");
    await Promise.resolve();
    for (const key of keys) {
        remove(key);
    }
    flush();
    container.ownerDocument.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", { key: "Escape", bubbles: true, cancelable: true }),
    );
    flush();
    await Promise.resolve();

    return container.ownerDocument.activeElement?.textContent;
}

test("focus the card nearest a deleted opener on Escape, else the board's first card", async () => {
    // delete Doing's d beside c and e, then d and the c nearest it
    const nearest = await closeAfterDeleting({ todo: ["a"], doing: ["c", "d", "e"] }, ["d"]);
    const first = await closeAfterDeleting({ todo: ["a"], doing: ["c", "d"] }, ["d", "c"]);

    expect([nearest, first]).toEqual(["e", "a"]);
});

test("leave fixed cards and the cards of a board without a move handler where they are", () => {
    // a fixed card, and a board that hands no moves on
    stubPopovers();
    const { container } = render(() => (
        <>
            <Board onItemMove={() => undefined}>
                <BoardColumnList>
                    <BoardColumn id="todo" label="todo">
                        <BoardColumnContent>
                            <BoardItem id="fixed" disabled>
                                fixed
                            </BoardItem>
                        </BoardColumnContent>
                    </BoardColumn>
                </BoardColumnList>
            </Board>
            <Board>
                <BoardColumnList>
                    <BoardColumn id="todo" label="todo">
                        <BoardColumnContent>
                            <BoardItem id="still">still</BoardItem>
                        </BoardColumnContent>
                    </BoardColumn>
                </BoardColumnList>
            </Board>
        </>
    ));
    const lifted = ["fixed", "still"].map((card) => {
        // press Space on the card, and read whether a drag lifted it
        find(container, "board-item", card).focus();
        press(container, " ");
        const isLifted = container.querySelector("[data-dragging]") !== null;
        press(container, "Escape");

        return [
            card,
            isLifted,
            find(container, "board-item", card).getAttribute("aria-roledescription"),
        ];
    });

    expect(lifted).toEqual([
        ["fixed", false, null],
        ["still", false, null],
    ]);
});

test("extend a selection with shift across a hidden column, leaving its cards out", () => {
    // a in To do, the hidden Doing's b, and c in Done
    const board = drawTeam({ todo: ["a"], doing: ["b"], done: ["c"] }, { hidden: ["doing"] });
    click(find(board.container, "board-item", "a"), { metaKey: true });
    click(find(board.container, "board-item", "c"), { shiftKey: true });

    expect(
        [
            ...board.container.querySelectorAll<HTMLElement>(
                "[data-slot=board-item][data-selected]",
            ),
        ].map((card) => card.textContent),
    ).toEqual(["a", "c"]);
});

test("focus the nearest shown card once a cell's last card goes", async () => {
    // b, Doing's only card, deleted from its menu, a waiting in To do
    const board = drawLive({ todo: ["a"], doing: ["b"], done: [] });
    await choose(board.container, "b", "Delete");

    expect(board.container.ownerDocument.activeElement?.textContent).toBe("a");
});

test("refuse a composer on a board that takes no new cards", () => {
    // a composer on a board without onCreate
    stubPopovers();
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Board>
                <BoardColumnList>
                    <BoardColumn id="todo" label="todo">
                        <BoardComposer />
                    </BoardColumn>
                </BoardColumnList>
            </Board>
        </Errored>
    ));

    expect(container.textContent).toBe("TypeError: a board composer needs the board's onCreate");
});

/** Render a board whose backlog of five hundred cards mounts only the cards in view. */
function drawLongBoard(textOf?: (card: string) => string): HTMLElement {
    layOutElements({ client: VIEWPORT, scroll: 500 * CARD });
    const cards = Array.from({ length: 500 }, (_, index) => `Ticket ${String(index + 1)}`);
    const { container } = render(() => (
        <Errored fallback={(error) => String(error())}>
            <Board label="Backlog" onItemMove={() => undefined}>
                <BoardColumnList>
                    <BoardColumn id="backlog" label="Backlog">
                        <BoardColumnContent {...(textOf === undefined ? {} : { textOf })}>
                            <VirtualList
                                each={cards}
                                estimateSize={() => CARD}
                                overscan={1}
                                itemKey={(card) => card}
                            >
                                {(card) => <BoardItem id={card}>{card}</BoardItem>}
                            </VirtualList>
                        </BoardColumnContent>
                    </BoardColumn>
                </BoardColumnList>
            </Board>
        </Errored>
    ));
    flush();

    return container;
}

/** Read the mounted cards as their text, place and set size. */
function mountedCards(container: HTMLElement): string[] {
    return [...container.querySelectorAll("[data-slot=board-item]")].map(
        (card) =>
            `${card.textContent} ${card.getAttribute("aria-posinset") ?? ""}/${card.getAttribute("aria-setsize") ?? ""}`,
    );
}

test("mount a long column's cards in view, each placed among all its cards, and the focus past them", () => {
    // focus the first card, then jump to the last, which mounts as the column reveals it
    const container = drawLongBoard((card) => card);
    const before = mountedCards(container);
    container.querySelector<HTMLElement>("[data-slot=board-item]")?.focus();
    flush();
    document.activeElement?.dispatchEvent(
        new KeyboardEvent("keydown", { key: "End", bubbles: true, cancelable: true }),
    );
    flush();

    expect({ before, focused: document.activeElement?.textContent }).toEqual({
        before: [
            "Ticket 1 1/500",
            "Ticket 2 2/500",
            "Ticket 3 3/500",
            "Ticket 4 4/500",
            "Ticket 5 5/500",
            "Ticket 6 6/500",
        ],
        focused: "Ticket 500",
    });
});

test("refuse a virtual list in a board column that reads no text of its cards", () => {
    const container = drawLongBoard();

    expect(container.textContent).toBe(
        "TypeError: board column 'backlog' is invalid: its virtual list needs textOf",
    );
});
