import { expect, test } from "@destack/test";
import { Skeleton } from "./index.ts";
import { draw, markup } from "@destack/view/test";

test("render a placeholder hidden from assistive technology", () => {
    const container = draw(() => <Skeleton aria-hidden="true" />);
    expect(markup(container)).toBe('<div data-slot="skeleton" aria-hidden="true"></div>');
});
