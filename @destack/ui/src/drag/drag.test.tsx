import { expect, test } from "@destack/test";
import { flush, useEnvironment, useLocale } from "@destack/view";
import { render, testWorld } from "@destack/view/test";
import {
    DragAnnouncer,
    Draggable,
    DragManager,
    type DragOperation,
    Droppable,
    type DragStep,
    type DragTarget,
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
        const manager = new DragManager(
            { onDrop: (operation) => drops.push(operation) },
            useLocale(),
            useEnvironment(),
        );
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
