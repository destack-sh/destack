import { expect, test } from "@destack/test";
import { untouched } from "../test/server.ts";
import { createRoot } from "solid-js";
import { createMutationObserver, mutationObserver } from "./mutation-observer.ts";

test("observe nothing on the server", () => {
    const node = untouched<Node>();
    const observed = createRoot((disposeRoot) => {
        const [add, { start, stop, instance, isSupported }] = createMutationObserver(
            node,
            { childList: true },
            () => {},
        );
        start();
        stop();
        add(node);
        mutationObserver({ childList: true }, () => {})(untouched<Element>());
        disposeRoot();

        return { instance, isSupported };
    });

    expect(observed).toEqual({ instance: undefined, isSupported: false });
});
