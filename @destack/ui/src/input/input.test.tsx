import { expect, test } from "@destack/test";
import { Input } from "./index.ts";
import { classes, draw, markup } from "@destack/view/test";

test("render a native input with its attributes", () => {
    const container = draw(() => <Input type="email" name="email" placeholder="Email" required />);
    expect(markup(container)).toBe(
        '<input data-slot="input" type="email" name="email" placeholder="Email" required="">',
    );
});

test("mark an invalid input with a different class", () => {
    const container = draw(() => (
        <>
            <Input />
            <Input aria-invalid="true" />
        </>
    ));

    // the invalid input keeps aria-invalid and styles differently
    expect(markup(container)).toBe(
        '<input data-slot="input"><input data-slot="input" aria-invalid="true">',
    );
    expect(new Set(classes(container)).size).toBe(2);
});

test("disable a native input", () => {
    const container = draw(() => <Input disabled />);
    expect(markup(container)).toBe('<input data-slot="input" disabled="">');
});
