import { Color } from "../color/index.ts";
import { present, toJsonSchema } from "@destack/schema";
import { expect, test } from "@destack/test";
import { Palette } from "./index.ts";

test("describe a palette sized by its user as a palette, keeping its size", () => {
    // the description names the palette, so an input knows it, and keeps the size beside it
    expect(toJsonSchema(Palette.min(1).max(10))).toEqual({
        $schema: "https://json-schema.org/draft/2020-12/schema",
        $ref: "#/$defs/style.palette",
        title: "Palette",
        minItems: 1,
        maxItems: 10,
        type: "array",
        items: { $ref: "#/$defs/style.color" },
        $defs: {
            "style.palette": {
                type: "array",
                items: { $ref: "#/$defs/style.color" },
                title: "Palette",
            },
            "style.color": present(
                toJsonSchema(Color.schema).$defs?.["style.color"],
                "the color's definition",
            ),
        },
    });
});
