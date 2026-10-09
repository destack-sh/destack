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
