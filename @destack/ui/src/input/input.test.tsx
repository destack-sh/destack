import { expect, test } from "@destack/test";
import { Input } from "./index.ts";
import { markup, render } from "@destack/view/test";

test("render a native input with its attributes", () => {
    const { container } = render(() => (
        <Input type="email" name="email" placeholder="Email" required />
    ));
    expect(markup(container)).toBe(
        '<input data-slot="input" type="email" name="email" placeholder="Email" required="">',
    );
});
