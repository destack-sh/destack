import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { Cover, CoverCenter } from "./index.ts";

test("fill a cover's least height and center the element a caller marks", () => {
    const { container } = render(() => (
        <Cover minHeight="32rem" space="6">
            <p>Destack</p>
            <CoverCenter render={(attributes) => <h1 {...attributes} />}>
                Take back your software
            </CoverCenter>
        </Cover>
    ));
    const cover = container.querySelector("[data-slot=cover]");
    const center = container.querySelector("[data-slot=cover-center]");

    // the cover carries its height and its space between and around, the centered element renders as the heading
    expect([cover?.getAttribute("style"), center?.tagName, center?.textContent]).toEqual([
        "--x-minBlockSize: 32rem; --x-gap: var(--destack-space-6); --x-padding: var(--destack-space-6);",
        "H1",
        "Take back your software",
    ]);
});

test("leave out a cover's padding", () => {
    const { container } = render(() => <Cover noPad />);

    expect(container.querySelector("[data-slot=cover]")?.getAttribute("style")).toBe(
        "--x-minBlockSize: 100svh; --x-gap: var(--destack-space-4); --x-padding: 0;",
    );
});
