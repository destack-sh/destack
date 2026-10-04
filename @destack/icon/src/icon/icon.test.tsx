import { expect, test } from "@destack/test";
import { draw } from "@destack/view/test";
import { createSignal, flush } from "solid-js";
import { Icon } from "@destack/icon";
import acorn from "@destack/icon/phosphor/acorn";
import trash from "@destack/icon/phosphor/trash";

/** The regular trash icon's body as Phosphor draws it. */
const TRASH =
    '<path d="M216,48H176V40a24,24,0,0,0-24-24H104A24,24,0,0,0,80,40v8H40a8,8,0,0,0,0,16h8V208a16,16,0,0,0,16,16H192a16,16,0,0,0,16-16V64h8a8,8,0,0,0,0-16ZM96,40a8,8,0,0,1,8-8h48a8,8,0,0,1,8,8v8H96Zm96,168H64V64H192ZM112,104v64a8,8,0,0,1-16,0V104a8,8,0,0,1,16,0Zm48,0v64a8,8,0,0,1-16,0V104a8,8,0,0,1,16,0Z"></path>';

/** Wrap an icon body in the SVG element an icon renders. */
function svg(attributes: string, body: string): string {
    return `<svg viewBox="0 0 256 256" fill="currentColor" ${attributes}>${body}</svg>`;
}

test("draw the regular weight at 1em, hidden from assistive technology", async () => {
    // an unlabeled icon draws the regular body at 1em, hidden from assistive technology
    const container = draw(() => <Icon name="trash" />);
    await expect
        .poll(() => container.innerHTML)
        .toBe(svg('width="1em" height="1em" aria-hidden="true"', TRASH));
});

test("expose a labeled icon as an image named by its label", async () => {
    // a labeled icon is an image with the label as its accessible name
    const container = draw(() => <Icon name="trash" label="Delete" />);
    await expect
        .poll(() => container.innerHTML)
        .toBe(svg('width="1em" height="1em" role="img" aria-label="Delete"', TRASH));
});

test("draw the body of the selected weight", async () => {
    // the bold weight draws its own body, which differs from the regular one
    const container = draw(() => <Icon name="trash" weight="bold" />);
    await expect
        .poll(() => container.innerHTML)
        .toBe(
            svg(
                'width="1em" height="1em" aria-hidden="true"',
                trash.bold.replace("/>", "></path>"),
            ),
        );
    expect(trash.bold).not.toBe(trash.regular);
});

test("size both dimensions as a CSS length or in user units", async () => {
    // a size sets both dimensions, as a CSS length or in user units
    const container = draw(() => (
        <>
            <Icon name="trash" size="2rem" />
            <Icon name="trash" size={24} />
        </>
    ));
    await expect
        .poll(() => container.innerHTML)
        .toBe(
            svg('width="2rem" height="2rem" aria-hidden="true"', TRASH) +
                svg('width="24" height="24" aria-hidden="true"', TRASH),
        );
});

test("draw the bodies passed for a choice between known icons", () => {
    // passed bodies draw the icon chosen at run time
    const [isAcorn, setAcorn] = createSignal(true);
    const container = draw(() => <Icon icon={isAcorn() ? acorn : trash} size={16} />);
    const drawn = container.innerHTML;
    setAcorn(false);
    flush();
    expect([drawn, container.innerHTML]).toEqual([
        svg('width="16" height="16" aria-hidden="true"', acorn.regular.replace("/>", "></path>")),
        svg('width="16" height="16" aria-hidden="true"', TRASH),
    ]);
});
