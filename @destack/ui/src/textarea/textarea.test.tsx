import { expect, test } from "@destack/test";
import { Textarea } from "./index.ts";
import { draw, markup } from "@destack/view/test";

test("render a native textarea with its attributes", () => {
    const container = draw(() => <Textarea name="body" rows={4} placeholder="Write" />);
    expect(markup(container)).toBe(
        '<textarea data-slot="textarea" name="body" rows="4" placeholder="Write"></textarea>',
    );
});
