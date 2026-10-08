import { expect, onTestFinished, test } from "@destack/test";
import { createRoot, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { isDismissal, Swipe, type SwipeDirection, type SwipeRelease, swipeStyle } from "./index.ts";

/** The width and height of the swiped test element, in pixels. */
const EXTENT = 200;

/** A swipe on a test element, the releases it reported and a clock the test moves. */
interface Swiped {
    /** The swipe. */
    readonly swipe: Swipe;
    /** The swiped element. */
    readonly element: HTMLElement;
    /** The control inside the element, where no swipe starts. */
    readonly control: HTMLElement;
    /** The releases the swipe handed on. */
    readonly releases: SwipeRelease[];
    /** Move the clock the swipe reads to a time, in milliseconds. */
    readonly at: (time: number) => void;
}

/** Render an element that a swipe toward a direction follows, reading a clock the test moves. */
function drawSwipe(direction: SwipeDirection, canStart: () => boolean = () => true): Swiped {
    // read the time from a clock the test moves, restored after the test
    let time = 0;
    const now = Object.getOwnPropertyDescriptor(performance, "now");
    Object.defineProperty(performance, "now", { configurable: true, value: () => time });
    onTestFinished(() => {
        if (now === undefined) {
            Reflect.deleteProperty(performance, "now");
        } else {
            Object.defineProperty(performance, "now", now);
        }
    });

    // render the element with the swipe on its pointer events
    const releases: SwipeRelease[] = [];
    const swipe = createRoot((dispose) => {
        onTestFinished(dispose);

        return new Swipe(
            () => direction,
            (release) => releases.push(release),
            canStart,
        );
    });
    const { container } = render(() => (
        <div
            onPointerDown={(event) => swipe.start(event)}
            onPointerMove={(event) => swipe.follow(event)}
            onPointerUp={() => swipe.release()}
        >
            <button>Close</button>
        </div>
    ));
    const element = container.firstElementChild;
    const control = container.querySelector("button");
    if (!(element instanceof HTMLElement) || control === null) {
        throw new TypeError("the swiped element did not render");
    }
    element.getBoundingClientRect = () => DOMRect.fromRect({ width: EXTENT, height: EXTENT });

    return { swipe, element, control, releases, at: (next) => (time = next) };
}

/** Send a pointer event at a position to an element. */
function pointer(target: HTMLElement, type: string, x: number, y: number): void {
    target.dispatchEvent(
        new PointerEvent(type, { bubbles: true, clientX: x, clientY: y, pointerId: 1 }),
    );
    flush();
}

test("follow a pointer toward the edge and hand its travel, speed and extent on as it lets go", () => {
    // press, move right twice and let go
    const { swipe, element, releases, at } = drawSwipe("right");
    pointer(element, "pointerdown", 10, 0);
    at(10);
    pointer(element, "pointermove", 40, 0);
    at(20);
    pointer(element, "pointermove", 70, 0);
    const following = [swipe.offset(), swipe.translate()];
    pointer(element, "pointerup", 70, 0);

    // the release reports the last move's speed and springs back
    expect({ following, released: [swipe.offset(), swipe.translate()], releases }).toEqual({
        following: [60, "60px 0"],
        released: [undefined, undefined],
        releases: [{ travel: 60, speed: 3, extent: EXTENT }],
    });
});

test("move toward an upward edge only, holding the element in place while the pointer goes the other way", () => {
    // swipe up past the start, then down below it
    const { swipe, element } = drawSwipe("up");
    pointer(element, "pointerdown", 0, 100);
    pointer(element, "pointermove", 0, 40);
    const toward = [swipe.offset(), swipe.translate()];
    pointer(element, "pointermove", 0, 130);

    expect({ toward, away: [swipe.offset(), swipe.translate()] }).toEqual({
        toward: [60, "0 -60px"],
        away: [-30, "0 0px"],
    });
});

test("spring back without a release when a swipe is cancelled", () => {
    const { swipe, element, releases } = drawSwipe("right");
    pointer(element, "pointerdown", 0, 0);
    pointer(element, "pointermove", 50, 0);
    swipe.cancel();
    flush();
    pointer(element, "pointerup", 50, 0);

    expect({ offset: swipe.offset(), releases }).toEqual({ offset: undefined, releases: [] });
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
    expect([
        isDismissal({ travel: 60, speed: 0, extent: 200 }),
        isDismissal({ travel: 40, speed: 0, extent: 100 }),
        isDismissal({ travel: 49, speed: 0, extent: 100 }),
        isDismissal({ travel: 20, speed: 0.6, extent: 200 }),
        isDismissal({ travel: 10, speed: 0.6, extent: 200 }),
        isDismissal({ travel: -80, speed: -1, extent: 200 }),
    ]).toEqual([true, false, true, true, false, false]);
});

test("style an element only while a swipe moves it", () => {
    expect([swipeStyle(undefined), swipeStyle("10px 0") === null]).toEqual([null, false]);
});
