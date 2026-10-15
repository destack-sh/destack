import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { mergeProperties, type PartAttributes, renderPart, useRender } from "./index.ts";

/** The properties of a test part: attributes any element takes, and options only the part reads. */
const PROPERTIES = {
    id: "save",
    "aria-label": "Save note",
    "data-testid": "save",
    variant: "outline",
    disabled: true,
    children: "Save",
};

/** The attributes a test part gives its element. */
const PART: PartAttributes = { "data-slot": "button", role: "link", tabindex: 0 };

test("render a part's own element with every attribute, or the caller's element with the forwarded ones", () => {
    const { container } = render(() => (
        <>
            {useRender({ render: undefined, defaultTagName: "button", part: PART, properties: {} })}
            {useRender({
                render: (attributes) => <a href="#notes" {...attributes} />,
                defaultTagName: "button",
                part: PART,
                properties: PROPERTIES,
            })}
        </>
    ));

    // leave the part's own options, such as its variant, off the caller's element
    expect(markup(container)).toBe(
        '<button data-slot="button" role="link" tabindex="0"></button>' +
            '<a href="#notes" data-slot="button" role="link" tabindex="0" id="save" aria-label="Save note" data-testid="save">Save</a>',
    );
});

test("run the caller's and the part's handlers on the part's own element", () => {
    const calls: string[] = [];
    const { container } = render(() =>
        useRender({
            render: undefined,
            defaultTagName: "button",
            part: { onClick: () => calls.push("part") },
            properties: { onClick: () => calls.push("caller") },
        }),
    );
    container.querySelector("button")?.click();

    expect(calls).toEqual(["caller", "part"]);
});

test("render an element part on its own tag or on the element its caller renders, with its slot and the caller's attributes", () => {
    const { container } = render(() => (
        <>
            {renderPart("div", "card-header", { id: "header", title: "Header" }, null)}
            {renderPart("div", "card-header", { render: (part) => <section {...part} /> }, null, {
                id: "own",
            })}
        </>
    ));

    expect(markup(container)).toBe(
        '<div data-slot="card-header" id="header" title="Header"></div>' +
            '<section data-slot="card-header" id="own"></section>',
    );
});

test("merge a part's attributes under its caller's, running both sides' handlers, the caller's first", () => {
    const calls: string[] = [];
    const merged = mergeProperties(
        { id: "own", title: "Own", onClick: () => calls.push("part") },
        { id: "theirs", title: undefined, onClick: () => calls.push("caller") },
    );
    merged.onClick();

    // the caller's id and undefined title win, and both handlers run
    expect({ id: merged.id, title: merged.title, calls }).toEqual({
        id: "theirs",
        title: undefined,
        calls: ["caller", "part"],
    });
});
