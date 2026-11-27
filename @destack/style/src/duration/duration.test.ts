import { expect, test } from "@destack/test";
import { Duration } from "./index.ts";

test("write a duration as a CSS time, and read it in milliseconds", () => {
    const durations = [
        { value: 200, unit: "ms" },
        { value: 0.25, unit: "s" },
    ] as const;

    expect({
        css: durations.map((duration) => Duration.css(duration)),
        milliseconds: durations.map((duration) => Duration.milliseconds(duration)),
        isNegativeValid: Duration.safeParse({ value: -1, unit: "ms" }).success,
    }).toEqual({ css: ["200ms", "0.25s"], milliseconds: [200, 250], isNegativeValid: false });
});
