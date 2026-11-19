import { afterEach, expect, test } from "@destack/test";
import { createInputMask, createMaskPattern } from "./input-mask.ts";

afterEach(() => {
    document.body.replaceChildren();
});

/** Type values into a masked input one after another, reading each masked value. */
function typeInto(handler: (event: Event) => string, values: readonly string[]): string[] {
    const input = document.createElement("input");
    input.addEventListener("input", (event) => handler(event));
    document.body.append(input);

    return values.map((value) => {
        input.value = value;
        input.dispatchEvent(new Event("input", { bubbles: true }));

        return input.value;
    });
}

test("insert a string mask's literals as the value grows, and stop at its length", () => {
    expect(
        typeInto(createInputMask("9999-99-99"), [
            "11111",
            "1111-11",
            "1111-111",
            "1111-11-11",
            "1111-11-111",
        ]),
    ).toEqual(["1111-1", "1111-11", "1111-11-1", "1111-11-11", "1111-11-11"]);
});

test("drop characters no pattern takes", () => {
    expect(typeInto(createInputMask("9999-99-99"), ["a", "-"])).toEqual(["", ""]);
});

test("replace a pattern mask's matches", () => {
    const mask = createInputMask([
        /[^0-9a-zäöüß\-_/]|^(https?:\/\/|)(meet\.goto\.com|gotomeet\.me|)\/?/giu,
        () => "",
    ]);

    expect(typeInto(mask, ["https://meet.goto.com/test"])).toEqual(["test"]);
});

test("write the value and the rest of the pattern onto the element before the field", () => {
    // mask a date with a label before the field
    const label = document.createElement("label");
    const input = document.createElement("input");
    document.body.append(label, input);
    input.addEventListener(
        "input",
        createMaskPattern(createInputMask("9999-99-99"), () => "YYYY-MM-DD"),
    );
    const attributes = (): (string | null)[] => [
        label.getAttribute("data-mask-value"),
        label.getAttribute("data-mask-pattern"),
    ];
    const steps = [attributes()];
    for (const value of ["1", "20771", ""]) {
        input.value = value;
        input.dispatchEvent(new Event("input"));
        steps.push(attributes());
    }

    expect(steps).toEqual([
        [null, null],
        ["1", "YYY-MM-DD"],
        ["2077-1", "M-DD"],
        [null, null],
    ]);
});
