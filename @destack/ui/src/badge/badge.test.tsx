import { expect, test } from "@destack/test";
import * as style from "@destack/style";
import { Badge, badgeVariants } from "./index.ts";
import { classes, markup, render } from "@destack/view/test";

test("render a span with its variant", () => {
    const { container } = render(() => <Badge variant="secondary">Draft</Badge>);
    expect(markup(container)).toBe('<span data-slot="badge" data-variant="secondary">Draft</span>');
});

test("style a link like a badge of the same variant", () => {
    const { container } = render(() => (
        <>
            <Badge variant="outline" />
            <a href="/tags/draft" {...style.attrs(badgeVariants({ variant: "outline" }))} />
        </>
    ));

    // the link takes the badge's classes exactly
    const [badge, link] = classes(container);
    expect(link).toBe(badge);
});

test("tint a badge in a swatch and report the swatch, apart from an untinted badge", () => {
    const { container } = render(() => (
        <>
            <Badge color="green">Site</Badge>
            <Badge>Site</Badge>
        </>
    ));

    // the tinted badge takes its swatch's tint and text, the plain one none
    expect(markup(container)).toBe(
        '<span data-slot="badge" style="--x-backgroundColor: var(--destack-swatch-green-tint); --x-color: var(--destack-swatch-green-tint-text);" data-variant="default" data-color="green">Site</span>' +
            '<span data-slot="badge" data-variant="default">Site</span>',
    );
});
