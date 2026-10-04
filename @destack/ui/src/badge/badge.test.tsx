import { expect, test } from "@destack/test";
import * as style from "@destack/style";
import { Badge, badgeStyle } from "./index.ts";
import { classes, draw, markup } from "@destack/view/test";

test("render a span with its variant", () => {
    const container = draw(() => <Badge variant="secondary">Draft</Badge>);
    expect(markup(container)).toBe('<span data-slot="badge" data-variant="secondary">Draft</span>');
});

test("select different classes for each variant", () => {
    const container = draw(() => (
        <>
            <Badge variant="default" />
            <Badge variant="secondary" />
            <Badge variant="destructive" />
            <Badge variant="outline" />
            <Badge variant="ghost" />
            <Badge variant="link" />
        </>
    ));

    // six variants style six different ways
    expect(new Set(classes(container)).size).toBe(6);
});

test("style a link like a badge of the same variant", () => {
    const container = draw(() => (
        <>
            <Badge variant="outline" />
            <a href="/tags/draft" {...style.attrs(badgeStyle({ variant: "outline" }))} />
        </>
    ));

    // the link takes the badge's classes exactly
    const [badge, link] = classes(container);
    expect(link).toBe(badge);
});
