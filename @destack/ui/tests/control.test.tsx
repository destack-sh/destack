import type { Example } from "@destack/package/declare";
import { expect, test, vi } from "@destack/test";
import { flush, type JSX } from "@destack/view";
import { accessibleName, render, stubPopovers } from "@destack/view/test";
import manifest from "../package.json" with { type: "json" };

/** The state units and part builders the package exports, which show no examples. */
const STATE = new Set([
    "action",
    "build",
    "collection",
    "drag",
    "focus",
    "interaction",
    "markdown",
    "motion",
    "part",
    "position",
    "selection",
    "toggle-state",
    "virtualizer",
]);

/** The components that draw on a canvas, which the test's page has none of. */
const CANVAS = "chart";

/** What a person operates, hidden inputs of a form left out. */
const CONTROLS =
    ":is(button, input, select, textarea, a[href], [role=checkbox], [role=switch], [role=radio], [role=slider], [role=spinbutton], [role=combobox], [role=textbox]):not([type=hidden], [aria-hidden=true])";

/** The attributes that list the ids of other elements on the page. */
const REFERENCES = ["aria-labelledby", "aria-describedby", "for"];

/** The examples of every component the package exports. */
const EXAMPLES: readonly (readonly [string, Example<object, JSX.Element>])[] = (
    await Promise.all(
        Object.keys(manifest.exports)
            .filter((path) => !path.includes("*") && !path.endsWith(".css"))
            .map((path) => path.slice(2))
            .filter((name) => !STATE.has(name) && !name.startsWith(CANVAS))
            .map(async (name) => {
                // read the component's module as its exports
                const module: unknown = await import(`../src/${name}/${name}.example.tsx`);

                return Object.entries(module ?? {})
                    .filter((entry): entry is [string, Example<object, JSX.Element>] =>
                        isExample(entry[1]),
                    )
                    .map(([exported, example]) => [exported, example] as const);
            }),
    )
).flat();

test("give every control of every example a name, a type and references that lead somewhere", () => {
    stubPopovers();
    const problems = EXAMPLES.flatMap(([name, example]) => {
        // render the example alone
        const { container, unmount } = render(() => example.render?.());
        flush();
        const found = problemsOf(container).map((problem) => `${name}: ${problem}`);
        unmount();

        return found;
    });

    expect([...new Set(problems)]).toEqual([]);
});

test("render every example without an error or a warning", () => {
    // listen to the console while each example renders and goes
    stubPopovers();
    const said: string[] = [];
    const listen =
        (kind: string) =>
        (...values: unknown[]) =>
            void said.push(`${kind}: ${String(values[0]).split("\n")[0] ?? ""}`);
    const warn = vi.spyOn(console, "warn").mockImplementation(listen("warning"));
    const error = vi.spyOn(console, "error").mockImplementation(listen("error"));
    const problems = EXAMPLES.flatMap(([name, example]) => {
        said.length = 0;
        const { unmount } = render(() => example.render?.());
        flush();
        unmount();

        return said.map((each) => `${name} ${each}`);
    });
    warn.mockRestore();
    error.mockRestore();

    expect([...new Set(problems)]).toEqual([]);
});

/** List what is wrong with the controls in an element. */
function problemsOf(container: HTMLElement): string[] {
    // name every control, and tell every button what it does in a form
    const page = container.ownerDocument;
    const controls = [...page.body.querySelectorAll<HTMLElement>(CONTROLS)].filter(
        (control) =>
            !control.hasAttribute("hidden") &&
            control.closest("[aria-hidden=true], [inert], select :scope") === null,
    );
    const unnamed = controls
        .filter((control) => accessibleName(control) === "")
        .map((control) => `unnamed ${describe(control)}`);
    const untyped = controls
        .filter((control) => control.localName === "button" && !control.hasAttribute("type"))
        .map((control) => `untyped ${describe(control)}`);
    const ordered = controls
        .filter((control) => control.tabIndex > 0)
        .map((control) => `ordered ${describe(control)}`);

    // lead references to elements there are, each with an id of its own
    const all = [...page.body.querySelectorAll<HTMLElement>("*")];
    const ids = all.map((element) => element.id).filter((id) => id !== "");
    const doubled = ids
        .filter((id, index) => ids.indexOf(id) !== index)
        .map((id) => `doubled id of ${describe(page.getElementById(id))}`);
    const lost = all.flatMap((element) =>
        REFERENCES.flatMap((attribute) =>
            (element.getAttribute(attribute)?.split(" ") ?? [])
                .filter((id) => id !== "" && page.getElementById(id) === null)
                .map(() => `lost ${attribute} of ${describe(element)}`),
        ),
    );

    return [...unnamed, ...untyped, ...ordered, ...doubled, ...lost];
}

/** Write an element by its slot, else its tag. */
function describe(element: Element | null): string {
    return element?.getAttribute("data-slot") ?? element?.localName ?? "nothing";
}

/** Report whether an export is an example. */
function isExample(value: unknown): value is Example<object, JSX.Element> {
    return typeof value === "object" && value !== null && "of" in value && "render" in value;
}
