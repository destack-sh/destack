import { expect, test } from "@destack/test";
import { Separator } from "./index.ts";
import { classes, draw, markup } from "@destack/view/test";

test("hide a decorative separator from assistive technology", () => {
    const container = draw(() => <Separator />);
    expect(markup(container)).toBe(
        '<div data-slot="separator" data-orientation="horizontal" role="none"></div>',
    );
});

test("expose a semantic separator with its orientation", () => {
    const container = draw(() => (
        <>
            <Separator decorative={false} />
            <Separator decorative={false} orientation="vertical" />
        </>
    ));

    // horizontal is the separator role's implicit orientation, so only vertical states it
    expect(markup(container)).toBe(
        '<div data-slot="separator" data-orientation="horizontal" role="separator"></div>' +
            '<div data-slot="separator" data-orientation="vertical" role="separator" aria-orientation="vertical"></div>',
    );
    expect(new Set(classes(container)).size).toBe(2);
});
