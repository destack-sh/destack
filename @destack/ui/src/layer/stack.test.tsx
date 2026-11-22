import { expect, test } from "@destack/test";
import { LayerStack } from "./index.ts";

test("follow the topmost modal as modals open and close in any order", () => {
    // open two modals, close the lower one first, then the top one
    const [lower, upper] = [document.createElement("dialog"), document.createElement("dialog")];
    const stack = new LayerStack();
    const tops: (string | undefined)[] = [];
    lower.id = "lower";
    upper.id = "upper";
    const stop = stack.subscribe((top) => tops.push(top?.id));
    stack.open(lower);
    stack.open(upper);
    stack.close(lower);
    stack.close(lower);
    stack.close(upper);
    stop();
    stack.open(lower);

    // closing a modal twice tells nobody the second time, and an unsubscribed handler hears nothing
    expect([tops, stack.top()?.id]).toEqual([["lower", "upper", "upper", undefined], "lower"]);
});

test("share one stack per document", () => {
    const element = document.createElement("div");

    expect(LayerStack.of(element)).toBe(LayerStack.of(document.body));
});
