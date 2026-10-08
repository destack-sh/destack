import { expect, test } from "@destack/test";
import { createRoot, flush } from "solid-js";
import { createEventProps } from "./event-props.ts";

test("record the latest event of each name through its on property", () => {
    const [[store, properties], dispose] = createRoot(
        (disposeRoot) => [createEventProps("keydown", "keyup"), disposeRoot] as const,
    );
    properties.onkeydown(new KeyboardEvent("keydown", { key: "A" }));
    properties.onkeyup(new KeyboardEvent("keyup", { key: "B" }));
    flush();
    const keys = [store.keydown?.key, store.keyup?.key];
    dispose();

    expect(keys).toEqual(["A", "B"]);
});
