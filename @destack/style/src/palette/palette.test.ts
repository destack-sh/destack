import { Color } from "../color/index.ts";
import { present, toJsonSchema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Palette } from "./index.ts";

test("describe a palette sized by its user as a palette, keeping its size", () => {
    // the description names the palette, so an input knows it, and keeps the size beside it
    expect(toJsonSchema(Palette.min(1).max(10))).toEqual({
        $schema: "https://json-schema.org/draft/2020-12/schema",
        $ref: "#/$defs/palette",
        title: "Palette",
        minItems: 1,
        maxItems: 10,
        type: "array",
        items: { $ref: "#/$defs/color" },
        $defs: {
            palette: { type: "array", items: { $ref: "#/$defs/color" }, title: "Palette" },
            color: present(toJsonSchema(Color.schema).$defs?.["color"], "the color's definition"),
        },
    });
});
