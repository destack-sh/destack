import { expect, test } from "@destack/test";
import { Label } from "./index.ts";
import { markup, render } from "@destack/view/test";

test("render a native label pointing at its control", () => {
    const { container } = render(() => <Label for="email">Email</Label>);
    expect(markup(container)).toBe('<label data-slot="label" for="email">Email</label>');
});
