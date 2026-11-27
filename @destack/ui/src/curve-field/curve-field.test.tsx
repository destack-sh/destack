import type { CubicBezier } from "@destack/style";
import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { render } from "@destack/view/test";
import { CurveField } from "./index.ts";

test("move a control point by arrows, further with Shift, within the curve's time", () => {
    const given: CubicBezier[] = [];
    const { container } = render(() => (
        <CurveField
            aria-label="Easing"
            defaultValue={[0.42, 0, 0.58, 1]}
            onValueChange={(next) => given.push(next)}
        />
    ));
    const [first, second] = container.querySelectorAll<HTMLElement>(
        "[data-slot=curve-field-point]",
    );
    press(first, "ArrowRight");
    press(first, "ArrowUp", true);
    press(second, "ArrowRight", true);
    press(second, "ArrowRight", true);
    press(second, "ArrowRight", true);
    press(second, "ArrowRight", true);
    press(second, "ArrowRight", true);

    expect(given.at(-1)).toEqual([0.43, 0.1, 1, 1]);
});

/** Press a key on a control point, with Shift held where asked. */
function press(point: HTMLElement | undefined, key: string, shiftKey = false): void {
    point?.dispatchEvent(new KeyboardEvent("keydown", { key, shiftKey, bubbles: true }));
    flush();
}
