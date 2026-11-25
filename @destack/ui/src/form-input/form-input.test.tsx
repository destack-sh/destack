import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render } from "@destack/view/test";
import { FormInput, isFormControl } from "./index.ts";

test("submit a value under its name, refuse an invalid one, and go back as the form resets", () => {
    const [value, setValue] = createSignal("7");
    const [error, setError] = createSignal("");
    const { container } = render(() => (
        <form>
            <FormInput
                name="hours"
                value={value()}
                error={error()}
                disabled={false}
                onReset={() => setValue("0")}
                onFocus={() => undefined}
            />
        </form>
    ));
    flush();
    const form = container.querySelector("form");
    const input = container.querySelector("input");
    const read = () => [input?.name, input?.value];
    const first = [read(), form?.checkValidity()];
    setError("too many");
    flush();
    const refused = form?.checkValidity();
    form?.reset();
    flush();

    expect({ first, refused, after: read() }).toEqual({
        first: [["hours", "7"], true],
        refused: false,
        after: ["hours", "0"],
    });
});

test("take part in a form with a name, a form or a needed value", () => {
    expect(
        [{}, { name: "hours" }, { form: "sheet" }, { required: true }].map(isFormControl),
    ).toEqual([false, true, true, true]);
});
