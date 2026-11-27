import { Patch } from "@destack/code";
import typescript from "@destack/code/grammar/typescript";
import { expect, test, vi } from "@destack/test";
import { createSignal, flush, type JSX } from "@destack/view";
import { render } from "@destack/view/test";
import { Diff, type DiffAnnotation, type DiffSelection } from "./index.ts";

/** A text of numbered lines. */
const LINES = Array.from({ length: 60 }, (_, line) => `line ${String(line + 1)}\n`).join("");

/** Read the rows of a diff, each with its cells. */
function rowsOf(container: Element): string[] {
    return [...container.querySelectorAll("tr")].map((row) =>
        [...row.querySelectorAll("td")].map((cell) => cell.textContent.trim()).join("|"),
    );
}

/** Press a key where the focus is. */
function type(key: string): void {
    const options = { key, bubbles: true, cancelable: true };
    document.activeElement?.dispatchEvent(new KeyboardEvent("keydown", options));
    flush();
}

/** Press a line's number, with shift to extend the selection to it. */
function down(cell: Element | undefined, shiftKey: boolean): void {
    cell?.dispatchEvent(new PointerEvent("pointerdown", { bubbles: true, button: 0, shiftKey }));
    flush();
}

/** Render a diff, and read what is on the page. */
function show(view: () => JSX.Element): Element {
    // settle what renders later
    const { container } = render(view);
    flush();

    return container;
}

test("show changed lines with their numbers and words, and reveal what is folded away", () => {
    const container = show(() => (
        <Diff
            aria-label="Lines"
            context={1}
            before={LINES}
            after={LINES.replace("line 30\n", "row 30\n")}
        />
    ));
    const folded = rowsOf(container);
    const words = [...container.querySelectorAll("[data-word]")].map((word) => [
        word.getAttribute("data-word"),
        word.textContent,
    ]);
    const press = (label: string): void => {
        container.querySelector<HTMLElement>(`[aria-label="${label}"]`)?.click();
        flush();
    };
    press("Show 20 lines below the change above");
    const some = rowsOf(container);
    press("Show every line");

    expect({ folded, words, some: [some.length, some.at(-2), some.at(-1)] }).toEqual({
        folded: [
            "28 unchanged linesline 28",
            "29|29|line 29",
            "30||removedline 30",
            "|30|addedrow 30",
            "31|31|line 31",
            "29 unchanged lines",
        ],
        words: [
            ["removed", "line"],
            ["added", "row"],
        ],
        some: [26, "51|51|line 51", "9 unchanged lines"],
    });
    expect(rowsOf(container).slice(0, 2)).toEqual(["1|1|line 1", "2|2|line 2"]);
});

test("put each changed line beside the one in its place where it is split", () => {
    const container = show(() => (
        <Diff aria-label="Plans" layout="split" before={"a\nb\n"} after={"a\nc\nd\n"} />
    ));

    expect(rowsOf(container)).toEqual(["1|a|1|a", "2|removedb|2|addedc", "||3|addedd"]);
});

test("read a patch with its path, its counts and only the lines it has", () => {
    const { files } = Patch.read(
        [
            "diff --git a/old.ts b/new.ts",
            "similarity index 90%",
            "rename from old.ts",
            "rename to new.ts",
            "--- a/old.ts",
            "+++ b/new.ts",
            "@@ -5,3 +5,3 @@ function first() {",
            " one",
            "-two",
            "+too",
            " three",
            "@@ -20,2 +20,3 @@ function second() {",
            " four",
            "+five",
            " six",
            "",
        ].join("\n"),
    );
    const [file] = files;
    const container = show(() => (file === undefined ? null : <Diff patch={file} context={0} />));

    expect({
        header: container.querySelector("[data-slot=diff-header]")?.textContent,
        rows: rowsOf(container),
        reveals: container.querySelectorAll("[data-slot=diff-fold] button").length,
    }).toEqual({
        header: "old.ts → new.tsMoved+2−1",
        rows: [
            "4 unchanged linesfunction first() {",
            "5|5|one",
            "6||removedtwo",
            "|6|addedtoo",
            "7|7|three",
            "12 unchanged linesfunction second() {",
            "20|20|four",
            "|21|addedfive",
            "21|22|six",
        ],
        reveals: 0,
    });
});

test("color each text by its grammar, with changed words marked", () => {
    const container = show(() => (
        <Diff
            aria-label="Port"
            grammar={typescript}
            before={"const port = 1;\n"}
            after={"const port = 2;\n"}
        />
    ));
    const tokens = [...container.querySelectorAll("[data-token], [data-word]")].map((element) => [
        element.getAttribute("data-token") ?? element.getAttribute("data-word"),
        element.textContent,
    ]);

    expect(tokens).toEqual([
        ["keyword", "const"],
        ["operator", "="],
        ["removed", "1"],
        ["punctuation", ";"],
        ["keyword", "const"],
        ["operator", "="],
        ["added", "2"],
        ["punctuation", ";"],
    ]);
});

test("go between changes, accept, reject and comment on them by their actions", () => {
    const done: string[] = [];
    const container = show(() => (
        <Diff
            aria-label="Plans"
            before={LINES}
            after={LINES.replace("line 3\n", "row 3\n").replace("line 50\n", "row 50\n")}
            onAccept={(change) => void done.push(`accept ${String(change.after.start)}`)}
            onReject={(change) => void done.push(`reject ${String(change.after.start)}`)}
            onComment={(side, line) => void done.push(`comment ${side} ${String(line)}`)}
        />
    ));
    container.querySelector<HTMLElement>("[data-slot=diff-content]")?.focus();
    const places = ["j", "j", "y", "k", "n", "c"].map((key) => {
        type(key);

        return document.activeElement?.getAttribute("data-change") ?? null;
    });

    expect({ places, done }).toEqual({
        places: ["0", "1", "1", "0", "0", "0"],
        done: ["accept 49", "reject 2", "comment after 2"],
    });
});

test("select a line, extend to another and leave it, and show what stands under a line", () => {
    const selections: (string | undefined)[] = [];
    const [annotations, setAnnotations] = createSignal<readonly DiffAnnotation[]>([]);
    const container = show(() => (
        <Diff
            aria-label="Plans"
            before={"a\nb\nc\n"}
            after={"a\nx\nc\n"}
            annotations={annotations()}
            onSelectionChange={(selection: DiffSelection | undefined) =>
                void selections.push(
                    selection &&
                        `${selection.side} ${String(selection.start)}-${String(selection.end)}`,
                )
            }
        />
    ));
    const numbers = [...container.querySelectorAll("td[data-line-number]")];
    const number = (line: number): Element | undefined =>
        numbers.find((cell) => cell.matches(`[data-side=after][data-line="${String(line)}"]`));
    down(number(1), false);
    down(number(2), true);
    down(number(0), true);
    container
        .querySelector("[data-slot=diff-content]")
        ?.dispatchEvent(new KeyboardEvent("keydown", { key: "Escape", bubbles: true }));
    flush();
    setAnnotations([{ side: "after", line: 1, content: () => "Why x?" }]);
    flush();

    expect({ selections, rows: rowsOf(container) }).toEqual({
        selections: ["after 1-2", "after 1-3", "after 0-2", undefined],
        rows: ["1|1|a", "2||removedb", "|2|addedx", "Why x?", "3|3|c"],
    });
});

test("show lines far ahead at once, and color them as they are parsed", async () => {
    // take lines enough that the last one is many slices ahead
    const lines = Array.from(
        { length: 4000 },
        (_, line) => `const value${String(line)} = ${String(line)};\n`,
    );
    const before = lines.join("");
    const container = show(() => (
        <Diff
            aria-label="Values"
            grammar={typescript}
            before={before}
            after={before.replace("const value3999", "let value3999")}
        />
    ));
    const last = (): (string | null)[] =>
        [
            ...(container.querySelector("tr[data-kind=added]")?.querySelectorAll("[data-token]") ??
                []),
        ].map((token) => token.getAttribute("data-token"));
    const [text, plain] = [rowsOf(container).at(-1), last()];
    await vi.waitFor(() => {
        flush();
        expect(last()).not.toEqual([]);
    });

    expect({ text, plain, colored: last() }).toEqual({
        text: "|4000|addedlet value3999 = 3999;",
        plain: [],
        colored: ["operator", "number", "punctuation"],
    });
});

test("mark a last line whose break only one text has, and say why nothing else shows", () => {
    const container = show(() => <Diff aria-label="End" before={"a\nb"} after={"a\nb\nc\n"} />);
    const marked = [...container.querySelectorAll("[title]")].map((mark) => [
        mark.getAttribute("title"),
        mark.closest("tr")?.dataset["kind"],
    ]);
    const [file] = Patch.read(
        "diff --git a/old.ts b/new.ts\nsimilarity index 100%\nrename from old.ts\nrename to new.ts\n",
    ).files;
    const moved = show(() => (file === undefined ? null : <Diff patch={file} />));

    expect({ marked, moved: moved.querySelector("p")?.textContent }).toEqual({
        marked: [["No line break at the end", "removed"]],
        moved: "Moved without changes",
    });
});
