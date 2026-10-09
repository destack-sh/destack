import { expect, test } from "@destack/test";
import { MOTION_VARIABLE } from "@destack/theme";
import { createRoot, flush } from "solid-js";
import { createReducedMotion } from "./index.ts";

test("hold motion still while the motion scale reads zero, and follow it as it changes", async () => {
    // pin the scale on the element, the test DOM resolving only an element's own custom properties
    const element = document.createElement("span");
    document.body.append(element);
    element.style.setProperty(MOTION_VARIABLE, "0");
    const states: boolean[] = [];

    // read the motion before and after the pin moves to full motion
    await createRoot(async (dispose) => {
        const isReduced = createReducedMotion(() => element);
        flush();
        states.push(isReduced());
        element.style.setProperty(MOTION_VARIABLE, "1");
        await new Promise((resolve) => {
            setTimeout(resolve, 0);
        });
        flush();
        states.push(isReduced());
        dispose();
    });
    element.remove();

    expect(states).toEqual([true, false]);
});
