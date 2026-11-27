import { JsonValue, toJsonSchema } from "@destack/schema";
import { Color } from "@destack/style";
import { expect, test } from "@destack/test";
import { render, stubPopovers } from "@destack/view/test";
import { ValueField } from "./index.ts";

test("show a named schema's input once its chunk loads", async () => {
    // draw a color's field, loading its input unless another test loaded it first
    stubPopovers();
    const { container } = render(() => (
        <ValueField
            type={{ kind: "json", schema: JsonValue.object(toJsonSchema(Color.schema)) }}
            value="#f76b15"
            onValueChange={() => undefined}
        />
    ));

    await expect.poll(() => container.querySelector("bdi")?.textContent).toBe("#f76b15");
});
