import { expect, test } from "@destack/test";
import { createRoot } from "solid-js";
import {
    createKeyHold,
    createShortcut,
    createCurrentlyHeldKey,
    createKeyDownEvent,
    createKeyDownList,
    createKeyDownSequence,
} from "./keyboard.ts";

test("follow no keys on the server", () => {
    const observed = createRoot((disposeRoot) => {
        createShortcut(["Control", "A"], () => {});
        const values = [
            createKeyDownEvent()(),
            createKeyDownList()(),
            createCurrentlyHeldKey()(),
            createKeyDownSequence()(),
            createKeyHold("Alt")(),
        ];
        disposeRoot();

        return values;
    });

    expect(observed).toEqual([null, [], null, [], false]);
});
