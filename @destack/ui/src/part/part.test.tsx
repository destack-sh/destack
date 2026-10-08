import { expect, test } from "@destack/test";
import { draw, markup } from "@destack/view/test";
import { forwarded, type PartAttributes, rendered } from "./index.ts";

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
    const container = draw(() => (
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
            '<a href="#notes" id="save" aria-label="Save note" data-testid="save" data-slot="button" role="link" tabindex="0">Save</a>',
    );
});
