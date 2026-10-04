import { expect, test } from "@destack/test";
import { Textarea } from "./index.ts";
import { classes, draw, markup } from "@destack/view/test";

test("render a native textarea with its attributes", () => {
    const container = draw(() => <Textarea name="body" rows={4} placeholder="Write" />);
    expect(markup(container)).toBe(
        '<textarea data-slot="textarea" name="body" rows="4" placeholder="Write"></textarea>',
    );
});

test("mark an invalid textarea with a different class", () => {
    const container = draw(() => (
        <>
            <Textarea />
            <Textarea aria-invalid="true" />
        </>
    ));

    // the invalid textarea keeps aria-invalid and styles differently
    expect(markup(container)).toBe(
        '<textarea data-slot="textarea"></textarea><textarea data-slot="textarea" aria-invalid="true"></textarea>',
    );
    expect(new Set(classes(container)).size).toBe(2);
});

test("disable a native textarea", () => {
    const container = draw(() => <Textarea disabled />);
    expect(markup(container)).toBe('<textarea data-slot="textarea" disabled=""></textarea>');
});
