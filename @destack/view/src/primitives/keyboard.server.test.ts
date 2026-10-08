import { expect, test } from "@destack/test";
import { createRoot } from "solid-js";
import {
    createKeyHold,
    createShortcut,
    useCurrentlyHeldKey,
    useKeyDownEvent,
    useKeyDownList,
    useKeyDownSequence,
} from "./keyboard.ts";

test("follow no keys on the server", () => {
    const observed = createRoot((disposeRoot) => {
        createShortcut(["Control", "A"], () => {});
        const values = [
            useKeyDownEvent()(),
            useKeyDownList()(),
            useCurrentlyHeldKey()(),
            useKeyDownSequence()(),
            createKeyHold("Alt")(),
        ];
        disposeRoot();

        return values;
    });

    expect(observed).toEqual([null, [], null, [], false]);
});
