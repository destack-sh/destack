import type { ObjectValue } from "@destack/object";
import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Value } from "./value.tsx";

test("draw each kind of value as people read it, an object by the choice it points to", () => {
    // draw one value of each kind, and a reference with and without what it points to
    const values: readonly ObjectValue[] = [
        { kind: "absent" },
        { kind: "text", value: "Ship it" },
        { kind: "number", value: 1200.5, format: undefined },
        { kind: "boolean", value: false },
        { kind: "choice", value: 1, title: "Urgent", color: "red" },
        { kind: "state", value: "done", title: "Done", stage: "final" },
        { kind: "reference", value: "list-1", target: "list" },
        { kind: "json", value: { count: 1 } },
    ];
    const drawn = values.map(
        (value) => render(() => <Value value={value} />).container.textContent,
    );
    const related = render(() => (
        <Value
            value={{ kind: "subject", value: "user-1" }}
            related={{ value: "user-1", title: "Ada", icon: { emoji: "🦊" } }}
        />
    )).container.textContent;

    expect({ drawn, related }).toEqual({
        drawn: ["–", "Ship it", "1,200.5", "–", "Urgent", "Done", "list-1", '{"count":1}'],
        related: "🦊Ada",
    });
});
