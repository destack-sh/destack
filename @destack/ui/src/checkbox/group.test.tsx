import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { Checkbox, CheckboxGroup } from "./index.ts";

/** Read each box as its state, the parent's first. */
function statesOf(container: Element): (string | null)[] {
    return [...container.querySelectorAll("[role=checkbox]")].map((box) =>
        box.getAttribute("aria-checked"),
    );
}

/** Click a box by its place. */
function click(container: Element, index: number): void {
    container.querySelectorAll<HTMLElement>("[role=checkbox]")[index]?.click();
    flush();
}

test("check values apart and all at once by the parent, leaving a disabled box as it is", () => {
    const given: (readonly string[])[] = [];
    const { container } = render(() => (
        <form>
            <CheckboxGroup
                name="by"
                defaultValue={["app", "other"]}
                onValueChange={(values) => void given.push(values)}
            >
                <Checkbox parent />
                <Checkbox value="app" disabled />
                <Checkbox value="mail" />
                <Checkbox value="push" />
            </CheckboxGroup>
        </form>
    ));
    const states = [statesOf(container)];
    for (const index of [2, 0, 0, 1]) {
        click(container, index);
        states.push(statesOf(container));
    }
    const form = container.querySelector("form");

    expect([states, given, form === null ? [] : new FormData(form).getAll("by")]).toEqual([
        [
            ["mixed", "true", "false", "false"],
            ["mixed", "true", "true", "false"],
            ["true", "true", "true", "true"],
            ["mixed", "true", "false", "false"],
            ["mixed", "true", "false", "false"],
        ],
        [
            ["app", "other", "mail"],
            ["other", "app", "mail", "push"],
            ["other", "app"],
        ],
        [],
    ]);
});

test("follow the owner's values, and leave a read-only group alone", () => {
    const [values, setValues] = createSignal<readonly string[]>(["a"]);
    const { container } = render(() => (
        <>
            <CheckboxGroup value={values()}>
                <Checkbox value="a" />
                <Checkbox value="b" />
            </CheckboxGroup>
            <CheckboxGroup defaultValue={["c"]} readOnly>
                <Checkbox parent />
                <Checkbox value="c" />
            </CheckboxGroup>
        </>
    ));
    for (const index of [1, 2, 3]) {
        click(container, index);
    }
    const before = statesOf(container);
    setValues(["b"]);
    flush();

    expect([before, statesOf(container)]).toEqual([
        ["true", "false", "true", "true"],
        ["false", "true", "true", "true"],
    ]);
});
