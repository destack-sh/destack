import { expect, test } from "@destack/test";
import { Transition } from "./index.ts";

test("play out changes of some properties, refusing times and easings CSS does not read", () => {
    const fade = { duration: "200ms", delay: "0s", easing: "cubic-bezier(0.2, 0, 0, 1)" };

    expect({
        css: Transition.css(fade, ["opacity", "translate"]),
        refused: Transition.safeParse({
            ...fade,
            duration: "fast",
            easing: "bounce",
        }).error?.issues.map((issue) => issue.path),
    }).toEqual({
        css: "opacity 200ms cubic-bezier(0.2, 0, 0, 1) 0s, translate 200ms cubic-bezier(0.2, 0, 0, 1) 0s",
        refused: [["duration"], ["easing"]],
    });
});
