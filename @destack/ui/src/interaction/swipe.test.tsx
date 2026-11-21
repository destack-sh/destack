import { expect, onTestFinished, test } from "@destack/test";
import { createRoot, flush } from "@destack/view";
import { render } from "@destack/view/test";
import type { Side } from "../position/index.ts";
import { Swipe, type SwipeEvent } from "./index.ts";

/** The width and height of the swiped test element, in pixels. */
const EXTENT = 200;

/** A swipe on a test element and the ends it reported. */
interface Swiped {
    /** The swipe. */
    readonly swipe: Swipe;
    /** The swiped element. */
    readonly element: HTMLElement;
    /** The control inside the element, where no swipe starts. */
    readonly control: HTMLElement;
    /** The ends the swipe reported. */
    readonly ends: SwipeEvent[];
}

/** Render an element that a swipe toward a side follows. */
function drawSwipe(side: Side, canStart: () => boolean = () => true): Swiped {
    // render the element with the swipe's handlers, on a clock that never runs
    const ends: SwipeEvent[] = [];
    const clock = { setTimeout: () => 0, clearTimeout: () => undefined };
    const swipe = createRoot((dispose) => {
        onTestFinished(dispose);

        return new Swipe(
            { side: () => side, onSwipeEnd: (event) => ends.push(event), canStart },
            clock,
        );
    });
    const { container } = render(() => (
        <div {...swipe.attributes()}>
            <button>Close</button>
        </div>
    ));
    const element = container.firstElementChild;
    const control = container.querySelector("button");
    if (!(element instanceof HTMLElement) || control === null) {
        throw new TypeError("the swiped element did not render");
    }
    element.getBoundingClientRect = () => DOMRect.fromRect({ width: EXTENT, height: EXTENT });

    return { swipe, element, control, ends };
}

/** Send a pointer event at a position and a time to an element. */
function pointer(target: HTMLElement, type: string, x: number, y: number, time = 0): void {
    const event = new PointerEvent(type, { bubbles: true, clientX: x, clientY: y, pointerId: 1 });
    Object.defineProperty(event, "timeStamp", { value: time });
    target.dispatchEvent(event);
    flush();
}

/** Swipe an element toward the right by a distance over a time, and read whether it dismisses. */
function isDismissal(distance: number, time: number): boolean | undefined {
    const { element, ends } = drawSwipe("right");
    pointer(element, "pointerdown", 0, 0, 0);
    pointer(element, "pointermove", distance, 0, time);
    pointer(element, "pointerup", distance, 0, time);

    return ends[0]?.isDismissal;
}

test("follow a pointer toward the side and report its travel and speed as it lets go", () => {
    // press, move right twice and let go
    const { swipe, element, ends } = drawSwipe("right");
    pointer(element, "pointerdown", 10, 0, 0);
    pointer(element, "pointermove", 40, 0, 10);
    pointer(element, "pointermove", 70, 0, 20);
    const following = [swipe.offset(), swipe.isSwiping(), element.dataset["swiping"]];
    pointer(element, "pointerup", 70, 0, 20);

    // the end reports the last move's speed and springs back
    expect({
        following,
        released: [swipe.offset(), swipe.isSwiping(), swipe.style()],
        ends: ends.map((end) => [end.travel, end.speed, end.isDismissal]),
    }).toEqual({
        following: [60, true, ""],
        released: [undefined, false, null],
        ends: [[60, 3, true]],
    });
});

test("report travel toward an upward side, negative while the pointer goes the other way", () => {
    // swipe up past the start, then down below it
    const { swipe, element } = drawSwipe("top");
    pointer(element, "pointerdown", 0, 100);
    pointer(element, "pointermove", 0, 40);
    const toward = swipe.offset();
    pointer(element, "pointermove", 0, 130);

    expect({ toward, away: swipe.offset() }).toEqual({ toward: 60, away: -30 });
});

test("spring back and dismiss nothing when the browser takes a swipe over", () => {
    const { swipe, element, ends } = drawSwipe("right");
    pointer(element, "pointerdown", 0, 0);
    pointer(element, "pointermove", 150, 0);
    pointer(element, "pointercancel", 150, 0);

    expect({
        offset: swipe.offset(),
        ends: ends.map((end) => [end.isCancelled, end.isDismissal]),
    }).toEqual({ offset: undefined, ends: [[true, false]] });
});

test("start no swipe on a control inside the element or where the element refuses one", () => {
    // press on the button, then on an element that refuses swipes
    const onControl = drawSwipe("right");
    pointer(onControl.control, "pointerdown", 0, 0);
    pointer(onControl.element, "pointermove", 50, 0);
    const refused = drawSwipe("right", () => false);
    pointer(refused.element, "pointerdown", 0, 0);
    pointer(refused.element, "pointermove", 50, 0);

    expect([onControl.swipe.offset(), refused.swipe.offset()]).toEqual([undefined, undefined]);
});

test("dismiss on a release past a quarter of the element or 48 pixels, or on a fast enough flick", () => {
    // swipe far slowly, short slowly, and short fast, on an element 200 pixels wide
    expect([
        isDismissal(60, 1000),
        isDismissal(50, 1000),
        isDismissal(20, 10),
        isDismissal(10, 10),
        isDismissal(-80, 10),
    ]).toEqual([true, false, true, false, false]);
});
