import { expect, test } from "@destack/test";
import { Transition } from "./index.ts";

test("play out changes of some properties, refusing a negative time", () => {
    const fade = {
        duration: { milliseconds: 200 },
        delay: {},
        timingFunction: [0.2, 0, 0, 1],
    } satisfies Transition;

    expect({
        css: Transition.css(fade, ["opacity", "translate"]),
        refused: Transition.safeParse({
            ...fade,
            duration: { milliseconds: -1 },
            delay: { milliseconds: 1 },
        }).error?.issues.map((issue) => issue.path),
    }).toEqual({
        css: "opacity 200ms cubic-bezier(0.2, 0, 0, 1) 0ms, translate 200ms cubic-bezier(0.2, 0, 0, 1) 0ms",
        refused: [["duration", "milliseconds"]],
    });
});

test("start a transition partway by a negative delay, as CSS does", () => {
    const partway = {
        duration: { seconds: 1 },
        delay: { milliseconds: -250 },
        timingFunction: [0, 0, 1, 1],
    };

    expect([
        Transition.safeParse(partway).success,
        Transition.css(Transition.parse(partway), ["opacity"]),
    ]).toEqual([true, "opacity 1000ms linear -250ms"]);
});
