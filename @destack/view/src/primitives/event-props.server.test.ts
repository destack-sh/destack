import { expect, test } from "@destack/test";
import { createEventProps } from "./event-props.ts";

test("make event properties that record nothing yet on the server", () => {
    const [store, properties] = createEventProps("click", "mousemove");

    expect([typeof properties.onclick, store.click]).toEqual(["function", undefined]);
});
