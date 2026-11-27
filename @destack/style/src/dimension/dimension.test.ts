import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Dimension } from "./index.ts";

test("read a dimension's number and unit, and refuse text that is none", () => {
    // read lengths, a percentage and a bare zero, then write one back
    const texts = ["12px", "-1.5rem", ".25em", "50%", "0"];

    expect({
        valid: ["12px", "0", "1.5", "px", "12 px"].map((text) => Dimension.safeParse(text).success),
        split: texts.map((text) => Dimension.split(text)),
        written: Dimension.of(2.5, "rem"),
    }).toEqual({
        valid: [true, true, false, false, false],
        split: [
            { value: 12, unit: "px" },
            { value: -1.5, unit: "rem" },
            { value: 0.25, unit: "em" },
            { value: 50, unit: "%" },
            { value: 0, unit: "px" },
        ],
        written: "2.5rem",
    });
});

test("refuse to split text that is no dimension, with the form it takes", () => {
    // read what splitting a word fails with
    const messages = (() => {
        try {
            Dimension.split("wide");

            return [];
        } catch (error) {
            return error instanceof schema.Error
                ? error.issues.map((issue) => issue.message)
                : [error];
        }
    })();

    expect(messages).toEqual(["a dimension is a number and a unit, such as 12px"]);
});
