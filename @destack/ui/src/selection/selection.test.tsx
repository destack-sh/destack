import { expect, onTestFinished, test } from "@destack/test";
import { createEffect, createRoot, createSignal, flush } from "@destack/view";
import { Collection } from "../collection/index.ts";
import { Selection, type SelectionProperties, valuesOf } from "./selection.ts";

/** Create a selection disposed after the test. */
function createSelection(properties: SelectionProperties): Selection {
    return createRoot((dispose) => {
        onTestFinished(dispose);

        return new Selection(properties);
    });
}

test("store a single selection's own value as a list and report it as one value or none", () => {
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

test("store a multiple selection's own values and report the whole list", () => {
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

test("leave a controlled selection at its owner's value, an undefined value selecting none", () => {
    // ask for a value while the owner has none, then let the owner select it
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
    const before = selection.values();
    setOwned("date");
    flush();

    expect({ before, followed: selection.values(), reported }).toEqual({
        before: [],
        followed: ["date"],
        reported: ["date"],
    });
});

test("toggle a value alone in a single selection and beside the others in a multiple one, and add without dropping", () => {
    // toggle and select in turn, reading the values after each
    const single = createSelection({ defaultValue: "title" });
    const multiple = createSelection({ multiple: true, defaultValue: ["bold"] });
    const steps: (readonly string[])[] = [];
    for (const step of [
        () => single.toggle("date"),
        () => single.toggle("date"),
        () => single.add("title"),
        () => single.add("title"),
        () => multiple.toggle("italic"),
        () => multiple.toggle("bold"),
        () => multiple.add("italic"),
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

test("select by press: replace on a plain press, toggle with command, extend a range with shift", () => {
    // lay out six files, the fourth unavailable, and press through them
    const files = ["a", "b", "c", "d", "e", "f"];
    const collection = createRoot((dispose) => {
        onTestFinished(dispose);

        return new Collection<string>({
            sections: () => [{ key: "files", items: files }],
            key: (file) => file,
            text: (file) => file,
            isDisabled: (file) => file === "d",
        });
    });
    const selection = createSelection({ multiple: true, behavior: "replace" });
    const plain = { shiftKey: false, metaKey: false, ctrlKey: false };
    const shift = { ...plain, shiftKey: true };
    const command = { ...plain, metaKey: true };
    const steps: (readonly string[])[] = [];
    for (const step of [
        () => selection.select("b", plain, collection),
        () => selection.select("e", shift, collection),
        () => selection.select("c", shift, collection),
        () => selection.select("f", command, collection),
        () => selection.select("a", plain, collection),
        () => selection.selectAll(collection),
        () => selection.clear(),
    ]) {
        step();
        flush();
        steps.push(selection.values());
    }

    // shift ranges skip the unavailable file and replace the last range from the same anchor
    expect(steps).toEqual([
        ["b"],
        ["b", "c", "e"],
        ["b", "c"],
        ["b", "c", "f"],
        ["a"],
        ["a", "b", "c", "e", "f"],
        [],
    ]);
});

test("read one value, several or none as a list", () => {
    expect([valuesOf(undefined), valuesOf("title"), valuesOf(["bold", "italic"])]).toEqual([
        [],
        ["title"],
        ["bold", "italic"],
    ]);
});

test("start a range anew once the owner dropped the value it extended from", () => {
    // press b, let the owner clear the selection, then extend to d with shift
    const [value, setValue] = createSignal<readonly string[]>([]);
    const collection = createRoot((dispose) => {
        onTestFinished(dispose);

        return new Collection({
            sections: () => [{ key: "letters", items: ["a", "b", "c", "d"] }],
            key: (letter) => letter,
            text: (letter) => letter,
        });
    });
    const selection = createSelection({
        multiple: true,
        behavior: "replace",
        get value() {
            return value();
        },
        onValueChange: setValue,
    });
    const modifiers = { shiftKey: false, metaKey: false, ctrlKey: false };
    selection.select("b", modifiers, collection);
    flush();
    const anchored = selection.anchor();
    setValue([]);
    flush();
    const dropped = selection.anchor();
    selection.select("d", { ...modifiers, shiftKey: true }, collection);
    flush();

    expect({ anchored, dropped, values: selection.values() }).toEqual({
        anchored: "b",
        dropped: undefined,
        values: ["d"],
    });
});

test("extend by a step from the focused value, anchoring there when nothing anchors a range", () => {
    // step from b to c with nothing selected, then on to d, then back to c
    const collection = createRoot((dispose) => {
        onTestFinished(dispose);

        return new Collection({
            sections: () => [{ key: "letters", items: ["a", "b", "c", "d"] }],
            key: (letter) => letter,
            text: (letter) => letter,
        });
    });
    const selection = createSelection({ multiple: true, behavior: "replace" });
    const steps: (readonly string[])[] = [];
    for (const [from, to] of [
        ["b", "c"],
        ["c", "d"],
        ["d", "c"],
    ] as const) {
        selection.extendFrom(from, to, collection);
        flush();
        steps.push(selection.values());
    }

    expect(steps).toEqual([
        ["b", "c"],
        ["b", "c", "d"],
        ["b", "c"],
    ]);
});

test("leave the last selected value where the selection may not be empty, as a segmented control does", () => {
    // press the one selected value of a single selection, and drop both values of a multiple one
    const single = createSelection({ defaultValue: "week", disallowEmptySelection: true });
    const multiple = createSelection({
        multiple: true,
        defaultValue: ["bold", "italic"],
        disallowEmptySelection: true,
    });
    single.toggle("week");
    multiple.toggle("bold");
    flush();
    multiple.toggle("italic");
    flush();

    expect({ single: single.values(), multiple: multiple.values() }).toEqual({
        single: ["week"],
        multiple: ["italic"],
    });
});
