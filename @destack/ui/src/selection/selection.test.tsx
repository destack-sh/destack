import { expect, onTestFinished, test } from "@destack/test";
import { createEffect, createRoot, createSignal, flush } from "@destack/view";
import { Selection, type SelectionProperties, valuesOf } from "./selection.ts";

/** Create a selection disposed after the test. */
function createSelection(properties: SelectionProperties): Selection {
    return createRoot((dispose) => {
        onTestFinished(dispose);

        return new Selection(properties);
    });
}

test("keep a single selection's own value as a list and report it as one value or none", () => {
    // start with one value, replace it and clear it
    const reported: (string | undefined)[] = [];
    const selection = createSelection({
        defaultValue: "title",
        onValueChange: (value) => reported.push(value),
    });
    const lists = [selection.values()];
    selection.replace(["date"]);
    flush();
    lists.push(selection.values());
    selection.replace([]);
    flush();
    lists.push(selection.values());

    expect({ lists, reported }).toEqual({
        lists: [["title"], ["date"], []],
        reported: ["date", undefined],
    });
});

test("keep a multiple selection's own values and report the whole list", () => {
    const reported: (readonly string[])[] = [];
    const selection = createSelection({
        multiple: true,
        defaultValue: ["bold"],
        onValueChange: (value) => reported.push(value),
    });
    selection.replace(["bold", "italic"]);
    flush();

    expect({ values: selection.values(), reported }).toEqual({
        values: ["bold", "italic"],
        reported: [["bold", "italic"]],
    });
});

test("hold a controlled selection at its owner's value, an undefined value selecting none", () => {
    // ask for a value while the owner keeps none, then let the owner select it
    const [owned, setOwned] = createSignal<string | undefined>(undefined);
    const reported: (string | undefined)[] = [];
    const selection = createSelection({
        get value() {
            return owned();
        },
        onValueChange: (value) => reported.push(value),
    });
    selection.replace(["date"]);
    flush();
    const held = selection.values();
    setOwned("date");
    flush();

    expect({ held, followed: selection.values(), reported }).toEqual({
        held: [],
        followed: ["date"],
        reported: ["date"],
    });
});

test("toggle a value alone in a single selection and beside the others in a multiple one, and select without dropping", () => {
    // toggle and select in turn, reading the values after each
    const single = createSelection({ defaultValue: "title" });
    const multiple = createSelection({ multiple: true, defaultValue: ["bold"] });
    const steps: (readonly string[])[] = [];
    for (const step of [
        () => single.toggle("date"),
        () => single.toggle("date"),
        () => single.select("title"),
        () => single.select("title"),
        () => multiple.toggle("italic"),
        () => multiple.toggle("bold"),
        () => multiple.select("italic"),
    ]) {
        step();
        flush();
        steps.push([...single.values(), "|", ...multiple.values()]);
    }

    expect(steps).toEqual([
        ["date", "|", "bold"],
        ["|", "bold"],
        ["title", "|", "bold"],
        ["title", "|", "bold"],
        ["title", "|", "bold", "italic"],
        ["title", "|", "italic"],
        ["title", "|", "italic"],
    ]);
});

test("rerun only the readers of the values a change selects or drops", () => {
    // follow how often each value's reader runs while the selection changes
    const selection = createSelection({ multiple: true, defaultValue: ["bold"] });
    const runs: Record<string, number> = { bold: 0, italic: 0, code: 0 };
    createRoot((dispose) => {
        onTestFinished(dispose);
        for (const value of Object.keys(runs)) {
            createEffect(
                () => selection.isSelected(value),
                () => {
                    runs[value] = (runs[value] ?? 0) + 1;
                },
            );
        }
    });
    flush();
    selection.toggle("italic");
    flush();

    expect({
        runs,
        selected: [selection.isSelected("bold"), selection.isSelected("code")],
    }).toEqual({ runs: { bold: 1, italic: 2, code: 1 }, selected: [true, false] });
});

test("read one value, several or none as a list", () => {
    expect([valuesOf(undefined), valuesOf("title"), valuesOf(["bold", "italic"])]).toEqual([
        [],
        ["title"],
        ["bold", "italic"],
    ]);
});
