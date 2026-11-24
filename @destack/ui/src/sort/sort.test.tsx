import type { OrderBy } from "@destack/db";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { Sort, SortAdd, SortControl, type SortFields, SortList } from "./index.ts";

/** The fields of a task. */
const FIELDS: SortFields = {
    title: { label: "Title", type: { kind: "text" } },
    due: { label: "Due", type: { kind: "time" } },
    state: { label: "State" },
};

test("add, turn around, move and remove fields in priority order", () => {
    const given: [string, unknown][][] = [];
    const [order, setOrder] = createSignal<OrderBy>({ title: "asc" });
    const control = new SortControl({
        fields: FIELDS,
        get value() {
            return order();
        },
        onValueChange: (next) => {
            given.push(Object.entries(next));
            setOrder(next);
        },
    });
    const steps = [
        () => control.set("due"),
        () => control.set("title", "desc"),
        () => control.move("due", "title"),
        () => control.move("due", undefined),
        () => control.replace("due", "state"),
        () => control.remove("title"),
        () => control.clear(),
    ];
    for (const step of steps) {
        step();
        flush();
    }

    expect(given).toEqual([
        [
            ["title", "asc"],
            ["due", "asc"],
        ],
        [
            ["title", "desc"],
            ["due", "asc"],
        ],
        [
            ["due", "asc"],
            ["title", "desc"],
        ],
        [
            ["title", "desc"],
            ["due", "asc"],
        ],
        [
            ["title", "desc"],
            ["state", "asc"],
        ],
        [["state", "asc"]],
        [],
    ]);
});

test("show each direction in the words of its kind, and change it by its parts", () => {
    stubPopovers();
    const given: OrderBy[] = [];
    const { container } = render(() => (
        <Sort
            fields={FIELDS}
            defaultValue={{ title: "asc", due: "desc", state: "asc" }}
            onValueChange={(next) => void given.push(next)}
        >
            <SortList />
            <SortAdd />
        </Sort>
    ));
    const shown = () =>
        [...container.querySelectorAll("[data-slot=sort-item]")].map((item) =>
            ["field", "direction"].map(
                (part) => item.querySelector(`[data-slot=sort-${part}]`)?.textContent,
            ),
        );
    const before = [shown(), container.querySelector("[data-slot=sort-add]")];
    container.querySelector<HTMLElement>("[data-slot=sort-direction]")?.click();
    flush();
    container.querySelectorAll<HTMLElement>("[data-slot=sort-remove]")[1]?.click();
    flush();

    expect([
        before,
        shown(),
        given,
        container.querySelector("[data-slot=sort-add]")?.textContent,
    ]).toEqual([
        [
            [
                ["Title", "A to Z"],
                ["Due", "Newest first"],
                ["State", "Ascending"],
            ],
            null,
        ],
        [
            ["Title", "Z to A"],
            ["State", "Ascending"],
        ],
        [
            { title: "desc", due: "desc", state: "asc" },
            { title: "desc", state: "asc" },
        ],
        "Sort",
    ]);
});
