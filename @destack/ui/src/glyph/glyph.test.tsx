import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { Glyph } from "./index.ts";

test("draw an emoji, an image and a named glyph, hiding an unnamed one from assistive technology", () => {
    const emoji = draw(() => <Glyph glyph={{ emoji: "📘" }} />);
    const image = draw(() => (
        <Glyph glyph={{ source: "/files/cover.webp" }} size="lg" label="Roadmap" />
    ));

    expect([markup(emoji), markup(image)]).toEqual([
        '<span data-slot="glyph" data-size="default" aria-hidden="true">📘</span>',
        '<span data-slot="glyph" data-size="lg" role="img" aria-label="Roadmap"><img src="/files/cover.webp" alt=""></span>',
    ]);
});
