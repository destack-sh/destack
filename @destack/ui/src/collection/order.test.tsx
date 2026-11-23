import { expect, onTestFinished, test } from "@destack/test";
import { inDocumentOrder } from "./index.ts";

/** Append a list of named elements to the document, the last nested in the one before it. */
function listOf(names: readonly string[]): ReadonlyMap<string, Element> {
    // append an element per name
    const list = document.createElement("ul");
    document.body.append(list);
    onTestFinished(() => list.remove());

    return new Map(names.map((name) => [name, list.appendChild(document.createElement("li"))]));
}

test("order items as their elements stand, however they were ordered before", () => {
    // reverse, rotate and shuffle the items of a list, one element nested in another list
    const names = ["a", "b", "c", "d", "e", "f"];
    const elements = listOf(names);
    const nested = document.createElement("ol");
    elements.get("c")?.append(nested);
    const inner = nested.appendChild(document.createElement("li"));
    const of = (name: string) => (name === "inner" ? inner : elements.get(name));
    const orders = [
        ["f", "e", "d", "inner", "c", "b", "a"],
        ["f", "a", "b", "c", "inner", "d", "e"],
        ["d", "inner", "a", "f", "c", "b", "e"],
    ];

    expect(orders.map((order) => inDocumentOrder(order, of).join(" "))).toEqual(
        orders.map(() => "a b c inner d e f"),
    );
});

test("leave items without an element after the item before them, and items in order as they are", () => {
    // order two items without elements among three with, then items that stand in order
    const elements = listOf(["a", "b", "c"]);
    const items = ["c", "lost", "a", "b", "late"];
    const inOrder = ["a", "b", "c"];
    const of = (name: string) => elements.get(name);

    expect([inDocumentOrder(items, of), inDocumentOrder(inOrder, of) === inOrder]).toEqual([
        ["a", "b", "late", "c", "lost"],
        true,
    ]);
});
