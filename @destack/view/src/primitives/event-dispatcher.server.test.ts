import { expect, test } from "@destack/test";
import { createEventDispatcher } from "./event-dispatcher.ts";

test("dispatch nothing on the server", () => {
    let isCalled = false;
    const dispatch = createEventDispatcher<{ onFoo: (event: CustomEvent<string>) => void }>({
        onFoo: () => {
            isCalled = true;
        },
    });

    expect([dispatch("foo", "payload"), isCalled]).toEqual([true, false]);
});
