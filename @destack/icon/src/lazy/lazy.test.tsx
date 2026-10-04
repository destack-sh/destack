import { expect, onTestFinished, test } from "@destack/test";
import { createSignal } from "solid-js";
import { render, type JSX } from "@solidjs/web";
import acorn from "@destack/icon/phosphor/acorn";
import alien from "@destack/icon/phosphor/alien";
import type { IconName } from "@destack/icon";
import { LazyIcon } from "@destack/icon/lazy";

/** Render an element into a detached container, disposing it after the test. */
function draw(element: () => JSX.Element): HTMLElement {
    const container = document.createElement("div");
    onTestFinished(render(element, container));

    return container;
}

/** Wrap an icon body in the SVG element an icon renders. */
function svg(attributes: string, body: string): string {
    return `<svg viewBox="0 0 256 256" fill="currentColor" ${attributes}>${body}</svg>`;
}

test("draw an icon chosen at run time empty at its final size until its module arrives", async () => {
    // the icon renders its sized frame at once and its body once its module loads
    const container = draw(() => <LazyIcon name="acorn" size={32} label="Acorn" />);
    const loading = container.innerHTML;
    await expect.poll(() => container.innerHTML).not.toBe(loading);
    expect([loading, container.innerHTML]).toEqual([
        svg('width="32" height="32" role="img" aria-label="Acorn"', ""),
        svg(
            'width="32" height="32" role="img" aria-label="Acorn"',
            acorn.regular.replace("/>", "></path>"),
        ),
    ]);
});

test("load the module of each name a signal selects", async () => {
    // switching the name draws the newly selected icon in the selected weight
    const [name, setName] = createSignal<IconName>("acorn");
    const container = draw(() => <LazyIcon name={name()} weight="bold" />);
    setName("alien");
    await expect
        .poll(() => container.innerHTML)
        .toBe(
            svg(
                'width="1em" height="1em" aria-hidden="true"',
                alien.bold.replaceAll("/>", "></path>"),
            ),
        );
});
