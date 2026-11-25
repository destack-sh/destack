import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { flush, useEnvironment, useLocale } from "@destack/view";
import { render, testWorld } from "@destack/view/test";
import {
    DragAnnouncer,
    Draggable,
    DragManager,
    type DragOperation,
    DragProvider,
    Droppable,
    type DragStep,
    type DragTarget,
    useDragManager,
} from "./index.ts";

/** The milliseconds a finger holds an item before it lifts. */
const LIFT_DELAY = 500;

/** The days of two weeks a test grid shows, Monday first. */
const DAYS = Array.from(
    { length: 14 },
    (_, index) => `2026-11-${String(16 + index).padStart(2, "0")}`,
);

/** The pixels each day's cell takes, seven to a row. */
const CELL = 100;

/** The marks that isolate interpolated names in rendered messages. */
const ISOLATES = /[⁦-⁩]/gu;

/** Step a day: a day along a week, a week across weeks. */
function stepDay(target: DragTarget, step: DragStep): DragTarget | undefined {
    // move a day along the row, or a week up and down
    const index = target.kind === "cell" ? DAYS.indexOf(target.key) : -1;
    const offsets: Record<DragStep, number> = {
        "inline-start": -1,
        "inline-end": 1,
        "block-start": -7,
        "block-end": 7,
    };
    const key = DAYS[index + offsets[step]];

    return key === undefined ? undefined : { kind: "cell", droppable: "month", key };
}

/** Render a two-week grid with one draggable event on a day, recording its drops. */
function drawMonth(): {
    readonly container: HTMLElement;
    readonly event: HTMLElement;
    readonly drops: DragOperation[];
    readonly manager: () => DragManager;
} {
    // record the manager the grid creates and the drops it hands on
    const drops: DragOperation[] = [];
    let created: DragManager | undefined;
    let grid: HTMLElement | undefined;
    let event: HTMLElement | undefined;
    const Month = () => {
        // drag events between days, a cell per day
        const manager = new DragManager(useLocale(), useEnvironment());
        created = manager;
        void new Droppable(
            {
                id: "month",
                group: "",
                axis: "both",
                element: () => grid,
                accepts: () => true,
                measure: () => undefined,
                targetAt: (x, y) => {
                    const key = DAYS[Math.floor(y / CELL) * 7 + Math.floor(x / CELL)];

                    return key === undefined
                        ? undefined
                        : { kind: "cell", droppable: "month", key };
                },
                step: stepDay,
                enter: () => ({ kind: "cell", droppable: "month", key: "2026-11-16" }),
                describe: (target) => ({
                    kind: "label",
                    label: target.kind === "cell" ? target.key : "",
                }),
                drop: (operation) => drops.push(operation),
            },
            manager,
        );
        const draggable = new Draggable(
            {
                id: "launch",
                type: "event",
                kind: "move",
                label: () => "Launch",
                element: () => event,
                home: () => ({ kind: "cell", droppable: "month", key: "2026-11-19" }),
            },
            manager,
        ).attributes([" ", "Enter"]);

        return (
            <div ref={(element) => (grid = element)}>
                <div tabindex={0} ref={(element) => (event = element)} {...draggable}>
                    Launch
                </div>
                <DragAnnouncer manager={manager} />
            </div>
        );
    };
    const { container } = render(() => <Month />);

    // lay the grid out as two rows of seven cells at the origin
    if (grid === undefined || event === undefined) {
        throw new TypeError("the test grid did not render");
    }
    grid.getBoundingClientRect = () => new DOMRect(0, 0, 7 * CELL, 2 * CELL);

    return {
        container,
        event,
        drops,
        manager: () => {
            if (created === undefined) {
                throw new TypeError("the test grid made no drag manager");
            }

            return created;
        },
    };
}

/** Read the live region's announcement without the isolating marks. */
function announced(container: HTMLElement): string {
    return (container.querySelector("[data-slot=drag-announcer]")?.textContent ?? "").replaceAll(
        ISOLATES,
        "",
    );
}

/** Send a pointer event to an element. */
function pointer(
    target: HTMLElement,
    type: string,
    x: number,
    y: number,
    pointerType = "mouse",
): void {
    target.dispatchEvent(
        new PointerEvent(type, {
            bubbles: true,
            cancelable: true,
            clientX: x,
            clientY: y,
            pointerId: 1,
            pointerType,
            width: 1,
            height: 1,
        }),
    );
    flush();
}

/** Send a key to an element. */
function pressKey(target: HTMLElement, name: string): void {
    target.dispatchEvent(
        new KeyboardEvent("keydown", { bubbles: true, cancelable: true, key: name }),
    );
    flush();
}

test("move an event across days with the keys, a week at a time up and down, announcing each day", () => {
    // lift, step a day right and a week down, then drop
    const { container, event, drops } = drawMonth();
    const announcements: string[] = [];
    for (const name of [" ", "ArrowRight", "ArrowDown", "Enter"]) {
        pressKey(event, name);
        announcements.push(announced(container));
    }

    expect({ announcements, drops: drops.map((drop) => [drop.pointer, drop.target]) }).toEqual({
        announcements: [
            "Picked up Launch, 2026-11-19.",
            "Launch moved to 2026-11-20.",
            "Launch moved to 2026-11-27.",
            "Launch dropped at 2026-11-27.",
        ],
        drops: [["keyboard", { kind: "cell", droppable: "month", key: "2026-11-27" }]],
    });
});

test("lift an event under a finger held still, follow it to another day, and leave a moving finger to scrolling", async () => {
    // swipe a finger across before the delay, which scrolls instead of lifting
    const { event, drops, manager } = drawMonth();
    const { simulation } = testWorld();
    pointer(event, "pointerdown", 350, 50, "touch");
    pointer(event, "pointermove", 380, 50, "touch");
    pointer(event, "pointercancel", 380, 50, "touch");
    const isSwipeLifted = manager().operation() !== undefined;

    // hold a finger still past the delay, then drag it to the next week's Tuesday and let go
    pointer(event, "pointerdown", 350, 50, "touch");
    await simulation.sleep(LIFT_DELAY);
    flush();
    const lifted = manager().operation()?.pointer;
    pointer(event, "pointermove", 150, 150, "touch");
    const target = manager().operation()?.target;
    pointer(event, "pointerup", 150, 150, "touch");

    expect({ isSwipeLifted, lifted, target, drops: drops.map((drop) => drop.target) }).toEqual({
        isSwipeLifted: false,
        lifted: "touch",
        target: { kind: "cell", droppable: "month", key: "2026-11-24" },
        drops: [{ kind: "cell", droppable: "month", key: "2026-11-24" }],
    });
});

test("drop nothing outside every droppable, announcing the event back on its day", () => {
    // drag the event with a mouse below the grid and let go there
    const { container, event, drops } = drawMonth();
    pointer(event, "pointerdown", 350, 50);
    pointer(event, "pointermove", 350, 500);
    pointer(event, "pointerup", 350, 500);

    expect({ announcement: announced(container), drops }).toEqual({
        announcement: "Moving Launch was cancelled. It returned to 2026-11-19.",
        drops: [],
    });
});

test("take an event by the day the pointer pressed, however far its first step goes", () => {
    // press the event's Sunday, three days after its home, and drag it a week on in one step
    const { event, manager } = drawMonth();
    pointer(event, "pointerdown", 650, 50);
    pointer(event, "pointermove", 650, 150);
    const operation = manager().operation();
    pointer(event, "pointerup", 650, 150);

    expect([operation?.from, operation?.grab, operation?.target]).toEqual([
        { kind: "cell", droppable: "month", key: "2026-11-19" },
        { kind: "cell", droppable: "month", key: "2026-11-22" },
        { kind: "cell", droppable: "month", key: "2026-11-29" },
    ]);
});

/** Render a shelf that takes any item from a provider's drags, a book on the left one. */
function Shelf(properties: {
    readonly id: string;
    readonly left: number;
    readonly drops: string[];
    readonly elements: Map<string, HTMLElement>;
}) {
    // take any item, landing at the shelf, and record each drop by the shelf it lands on
    const drags = useDragManager();
    let element: HTMLElement | undefined;
    const target: DragTarget = { kind: "cell", droppable: properties.id, key: properties.id };
    void new Droppable(
        {
            id: properties.id,
            group: properties.id,
            axis: "both",
            element: () => element,
            accepts: () => true,
            measure: () => undefined,
            targetAt: () => target,
            step: () => undefined,
            enter: () => target,
            describe: () => ({ kind: "label", label: properties.id }),
            drop: (operation) => properties.drops.push(`${operation.item} on ${properties.id}`),
        },
        drags.manager,
    );
    const item =
        properties.id === "left"
            ? new Draggable(
                  {
                      id: "book",
                      type: "book",
                      kind: "move",
                      label: () => "Book",
                      element: () => properties.elements.get("book"),
                      home: () => target,
                  },
                  drags.manager,
              ).attributes([" "])
            : undefined;

    return (
        <div
            ref={(created: HTMLElement) => {
                element = created;
                created.getBoundingClientRect = () => new DOMRect(properties.left, 0, 100, 100);
                properties.elements.set(properties.id, created);
            }}
        >
            {item === undefined ? null : (
                <div
                    ref={(created: HTMLElement) => properties.elements.set("book", created)}
                    {...item}
                >
                    Book
                </div>
            )}
        </div>
    );
}

test("move an item between two components that share a provider's drags, announcing once", () => {
    // drag the book from the left shelf onto the right one, each shelf its own component
    const drops: string[] = [];
    const elements = new Map<string, HTMLElement>();
    const { container } = render(() => (
        <DragProvider>
            <Shelf id="left" left={0} drops={drops} elements={elements} />
            <Shelf id="right" left={100} drops={drops} elements={elements} />
        </DragProvider>
    ));
    const book = elements.get("book");
    if (book === undefined) {
        throw new TypeError("the left shelf did not render its book");
    }
    pointer(book, "pointerdown", 50, 50);
    pointer(book, "pointermove", 100, 50);
    pointer(book, "pointermove", 150, 50);
    pointer(book, "pointerup", 150, 50);

    // the right shelf takes the drop, and the provider's one live region announced it
    expect({
        drops,
        announcers: container.querySelectorAll("[data-slot=drag-announcer]").length,
        announced: announced(container),
    }).toEqual({ drops: ["book on right"], announcers: 1, announced: "Book dropped at right." });
});

/** Send a platform's drag event over an element, with what it brings. */
function send(
    element: Element,
    type: "dragover" | "dragleave" | "drop",
    at: { readonly x: number; readonly y: number },
    transfer: Readonly<Record<string, unknown>>,
): Event {
    // add what the test's page leaves out of the event
    const event = new Event(type, { bubbles: true, cancelable: true });
    Object.assign(event, {
        clientX: at.x,
        clientY: at.y,
        relatedTarget: null,
        view: element.ownerDocument.defaultView,
        dataTransfer: { effectAllowed: "all", items: [], types: [], ...transfer },
    });
    element.dispatchEvent(event);
    flush();

    return event;
}

/** A place that takes files on its left half alone, recording its drops. */
function Desk(properties: { readonly drops: DragOperation[] }) {
    const { manager } = useDragManager();
    let element: HTMLElement | undefined;
    void new Droppable(
        {
            id: "desk",
            group: "desk",
            axis: "both",
            element: () => element,
            accepts: ({ type }) => type === "Files",
            measure: () => undefined,
            targetAt: (x) =>
                x < 50 ? { kind: "cell", droppable: "desk", key: "left" } : undefined,
            step: () => undefined,
            enter: () => ({ kind: "cell", droppable: "desk", key: "left" }),
            describe: () => ({ kind: "label", label: "the desk" }),
            drop: (operation) => properties.drops.push(operation),
        },
        manager,
    );

    return (
        <div
            data-aim={manager.operation()?.target === undefined ? "none" : "left"}
            ref={(created: HTMLElement) => {
                element = created;
                created.getBoundingClientRect = () => new DOMRect(0, 0, 100, 100);
            }}
        >
            <input />
        </div>
    );
}

/** Render a desk, and read where a drag over it aims. */
function drawDesk() {
    const drops: DragOperation[] = [];
    const { container } = render(() => <Desk drops={drops} />);
    flush();
    const desk = present(container.firstElementChild, "the desk");

    return { desk, drops, aim: () => desk.getAttribute("data-aim") };
}

test("take files from outside where a droppable accepts them, and hand them on", () => {
    const { desk, drops, aim } = drawDesk();
    const file = new File(["hello"], "hello.txt", { type: "text/plain" });
    const files = { types: ["Files"] };
    const over = send(desk, "dragover", { x: 10, y: 10 }, files);
    const aims = [aim()];
    const beside = send(desk, "dragover", { x: 80, y: 10 }, files);
    aims.push(aim());
    send(desk, "dragover", { x: 20, y: 10 }, files);
    send(
        desk,
        "drop",
        { x: 20, y: 10 },
        {
            ...files,
            items: [{ kind: "file", getAsFile: () => file, webkitGetAsEntry: () => null }],
        },
    );

    expect({
        aims,
        isTaken: [over.defaultPrevented, beside.defaultPrevented],
        drops: drops.map(({ type, effect, items, target }) => ({ type, effect, items, target })),
        after: aim(),
    }).toEqual({
        aims: ["left", "none"],
        isTaken: [true, false],
        drops: [
            {
                type: "Files",
                effect: "copy",
                items: [{ kind: "file", file }],
                target: { kind: "cell", droppable: "desk", key: "left" },
            },
        ],
        after: "none",
    });
});

test("leave what nothing takes, text over a field, and a drag that leaves", () => {
    const { desk, drops, aim } = drawDesk();
    const at = { x: 10, y: 10 };
    const link = send(desk, "dragover", at, { types: ["text/uri-list", "text/plain"] });
    const aims = [aim()];
    send(desk, "dragover", at, { types: ["Files"] });
    aims.push(aim());
    send(desk, "dragleave", { x: 0, y: 0 }, { types: ["Files"] });
    aims.push(aim());
    const drop = send(desk, "drop", at, { types: ["Files"] });
    const field = present(desk.querySelector("input"), "the field");
    const text = send(field, "dragover", at, { types: ["text/plain"] });

    expect({
        aims,
        isTaken: [link, drop, text].map(({ defaultPrevented }) => defaultPrevented),
        drops,
    }).toEqual({ aims: ["none", "left", "none"], isTaken: [false, false, false], drops: [] });
});
