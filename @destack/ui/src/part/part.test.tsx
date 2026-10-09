import { expect, test } from "@destack/test";
import { markup, render } from "@destack/view/test";
import { forwarded, mergeProperties, type PartAttributes, rendered, renderPart } from "./index.ts";

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

test("forward the attributes any element takes and leave the part's own options behind", () => {
    const attributes = forwarded(PROPERTIES);
    const variant: unknown = Reflect.get(attributes, "variant");

    expect({
        keys: Object.keys(attributes),
        spread: { ...attributes },
        hasVariant: "variant" in attributes,
        variant,
    }).toEqual({
        keys: ["id", "aria-label", "data-testid", "children"],
        spread: { id: "save", "aria-label": "Save note", "data-testid": "save", children: "Save" },
        hasVariant: false,
        variant: undefined,
    });
});

test("render a part's own element, or the caller's element with the part's and the forwarded attributes", () => {
    const { container } = render(() => (
        <>
            {rendered(undefined, PART, PROPERTIES, () => (
                <button {...PART}>Own</button>
            ))}
            {rendered(
                (attributes) => (
                    <a href="#notes" {...attributes} />
                ),
                PART,
                PROPERTIES,
                () => (
                    <button>Own</button>
                ),
            )}
        </>
    ));

    expect(markup(container)).toBe(
        '<button data-slot="button" role="link" tabindex="0">Own</button>' +
            '<a href="#notes" data-slot="button" role="link" tabindex="0" id="save" aria-label="Save note" data-testid="save">Save</a>',
    );
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
