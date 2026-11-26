import { expect, test } from "@destack/test";
import * as style from "@destack/style";
import { Badge } from "./index.ts";
import { classes, markup, render } from "@destack/view/test";

test("render a span with its variant", () => {
    const { container } = render(() => <Badge variant="outline">Draft</Badge>);
    expect(markup(container)).toBe('<span data-slot="badge" data-variant="outline">Draft</span>');
});

test("style a link like a badge of the same variant", () => {
    const { container } = render(() => (
        <>
            <Badge variant="outline" />
            <a href="/tags/draft" {...style.attrs(Badge.styles({ variant: "outline" }))} />
        </>
    ));

    // the link takes the badge's classes exactly
    const [badge, link] = classes(container);
    expect(link).toBe(badge);
});

test("paint a badge in its hue and report the hue, apart from an accent badge", () => {
    const { container } = render(() => (
        <>
            <Badge variant="soft" color="green">
                Site
            </Badge>
            <Badge>Site</Badge>
        </>
    ));

    // the green badge takes its hue's tint and text, the accent one its classes alone
    expect(markup(container)).toBe(
        '<span data-slot="badge" style="--x-backgroundColor: var(--destack-swatch-green-tint); --x-color: var(--destack-swatch-green-tint-text); --x-borderColor: transparent;" data-variant="soft" data-color="green">Site</span>' +
            '<span data-slot="badge" data-variant="solid">Site</span>',
    );
});
