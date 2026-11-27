import { t } from "@destack/locale";
import { schema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { field } from "../field/index.ts";
import { ActionInput } from "./input.ts";

test("read the value in force while none is given, from a field, a schema and a description", () => {
    // declare a default on a field and on a schema, and none on another
    const inputs = [
        ActionInput.of(
            "size",
            field.enum({ sm: { title: t`Small` }, md: { title: t`Medium` } }).default("md"),
        ),
        ActionInput.of("ratio", schema.number().default(0.5)),
        ActionInput.of("count", field.integer()),
    ];
    const back = inputs.map((input) =>
        ActionInput.from(input.describe(), () => {
            throw new TypeError("no input takes objects");
        }),
    );

    // keep each default through a description and back
    expect([inputs, back].map((each) => each.map((input) => input.default))).toEqual([
        ["md", 0.5, undefined],
        ["md", 0.5, undefined],
    ]);
});
