import { afterEach, beforeEach, expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createOrientation, makeOrientation, type OrientationState } from "./orientation.ts";

/** A stand-in screen orientation the test turns. */
class StubOrientation extends EventTarget {
    /** The angle in degrees. */
    angle = 0;
    /** The orientation's type. */
    type: OrientationType = "portrait-primary";

    /** Turn the screen, telling the listeners. */
    turn(angle: number, type: OrientationType): void {
        this.angle = angle;
        this.type = type;
        this.dispatchEvent(new Event("change"));
    }
}

/** The orientation of the current test. */
let orientation = new StubOrientation();

beforeEach(() => {
    orientation = new StubOrientation();
    Object.defineProperty(screen, "orientation", { configurable: true, get: () => orientation });
});

afterEach(() => {
    Reflect.deleteProperty(screen, "orientation");
});

test("call back with the orientation as the screen turns, until cleared", () => {
    const seen: OrientationState[] = [];
    const clear = makeOrientation((state) => seen.push(state));
    orientation.turn(90, "landscape-primary");
    clear();
    orientation.turn(0, "portrait-primary");

    expect(seen).toEqual([{ angle: 90, type: "landscape-primary" }]);
});

test("follow the orientation's angle and type", () => {
    const observed = createRoot((disposeRoot) => {
        const { angle, type } = createOrientation();
        const initial = [angle(), type()];
        orientation.turn(270, "landscape-secondary");
        flush();
        const turned = [angle(), type()];
        disposeRoot();

        return [initial, turned];
    });

    expect(observed).toEqual([
        [0, "portrait-primary"],
        [270, "landscape-secondary"],
    ]);
});
