import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { Mention } from "./index.ts";

/** The accent tint of a mention without a color of its own. */
const TINT =
    'style="--x-backgroundColor: var(--destack-swatch-accent-tint); --x-color: var(--destack-swatch-accent-tint-text);"';

test("mention a person by an at sign and a page by its icon, in a sentence", () => {
    const { container } = render(() => (
        <p>
            Ask <Mention choice={{ value: "ada", title: "Ada" }} /> about{" "}
            <Mention choice={{ value: "plan", title: "Plan", icon: { icon: "file-text" } }} />
        </p>
    ));

    expect(markup(container)).toBe(
        [
            `<p>Ask <span data-slot="mention" ${TINT}>@Ada</span> about `,
            `<span data-slot="mention" ${TINT}><span>`,
            '<span data-slot="glyph" data-size="default" aria-hidden="true">',
            '<svg viewBox="0 0 256 256" fill="currentColor" width="100%" height="100%" aria-hidden="true" data-slot="icon"></svg>',
            "</span></span>Plan</span></p>",
        ].join(""),
    );
});
