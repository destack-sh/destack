import { expect, test } from "@destack/test";
import { render } from "@destack/view/test";
import { createSignal } from "@destack/view";
import acorn from "@destack/icon/phosphor/acorn";
import alien from "@destack/icon/phosphor/alien";
import { IconBodies, type IconName } from "@destack/icon";
import { LazyIcon } from "@destack/ui/icon";

/** Wrap an icon body in the SVG element an icon renders, with its style's class. */
function svg(attributes: string, body: string): string {
    return `<svg viewBox="0 0 256 256" fill="currentColor" ${attributes} data-slot="icon" class="x2lah0s">${body}</svg>`;
}

test("draw an icon chosen at run time empty at its final size until its module arrives", async () => {
    // load the module first, so the icon's body arrives on the next turn, then draw its sized frame at once
    await IconBodies.load("acorn");
    const { container } = render(() => <LazyIcon name="acorn" size={32} label="Acorn" />);
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
    const { container } = render(() => <LazyIcon name={name()} weight="bold" />);
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
