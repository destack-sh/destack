import { expect, test } from "@destack/test";
import { Transition } from "./index.ts";

test("play out changes of some properties, refusing a negative time", () => {
    const fade = {
        duration: { value: 200, unit: "ms" },
        delay: { value: 0, unit: "s" },
        timingFunction: [0.2, 0, 0, 1],
    } satisfies Transition;

    expect({
        css: Transition.css(fade, ["opacity", "translate"]),
        refused: Transition.safeParse({
            ...fade,
            duration: { value: -1, unit: "ms" },
        }).error?.issues.map((issue) => issue.path),
    }).toEqual({
        css: "opacity 200ms cubic-bezier(0.2, 0, 0, 1) 0s, translate 200ms cubic-bezier(0.2, 0, 0, 1) 0s",
        refused: [["duration", "value"]],
    });
});
