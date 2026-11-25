import { expect, test } from "@destack/test";
import { flush } from "@destack/view";
import { focused, markup, render } from "@destack/view/test";
import {
    TagField,
    TagFieldClear,
    TagFieldInput,
    TagFieldList,
    type TagFieldProperties,
} from "./index.ts";

/** Press keys where the focus is. */
function press(...keys: readonly string[]): void {
    for (const key of keys) {
        document.activeElement?.dispatchEvent(
            new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }),
        );
        flush();
    }
}

/** Render a field, and type and press keys in it. */
function field(properties: TagFieldProperties) {
    const told: string[] = [];
    const { container } = render(() => (
        <TagField
            onValueInvalid={(tag, refusal) => void told.push(`${tag} ${refusal}`)}
            {...properties}
        />
    ));
    const input = (): HTMLInputElement | null =>
        container.querySelector("[data-slot=tag-field-input]");
    const type = (words: string): void => {
        const element = input();
        if (element !== null) {
            element.focus();
            element.value = words;
            element.dispatchEvent(new Event("input", { bubbles: true }));
        }
        flush();
    };
    const tags = (): readonly string[] =>
        [...container.querySelectorAll("[data-slot=tag-field-item]")].map((tag) => tag.textContent);

    return { container, told, input, type, tags };
}

test("render tags as a list with buttons, the text of the next, and hidden inputs", () => {
    const { container } = field({ name: "tag", defaultValue: ["travel"] });

    expect(markup(container)).toBe(
        [
            '<div data-slot="tag-field" role="group"><div data-slot="tag-field-list" role="list" aria-label="Tags">',
            '<span data-slot="tag-field-item" role="listitem" tabindex="-1"><span>travel</span>',
            '<button data-slot="tag-field-remove" type="button" tabindex="-1" aria-label="Remove \u2068travel\u2069">',
            '<svg viewBox="0 0 256 256" fill="currentColor" width="0.75em" height="0.75em" aria-hidden="true" data-slot="icon"></svg>',
            "</button></span></div>",
            '<input data-slot="tag-field-input" autocomplete="off" aria-label="Add tag" enterkeyhint="done">',
            '<span data-slot="visually-hidden" role="status"></span>',
            '<input type="hidden" value="travel" name="tag"></div>',
        ].join(""),
    );
});

test("add by Enter, a delimiter and a paste, and leave the spaces out", () => {
    const tags = field({});
    tags.type(" travel ");
    press("Enter");
    tags.type("food, wine,che");
    const typed = tags.input()?.value;
    const paste = Object.assign(new Event("paste", { bubbles: true, cancelable: true }), {
        clipboardData: { getData: () => "ese\nbread" },
    });
    tags.input()?.dispatchEvent(paste);
    flush();

    expect({ typed, tags: tags.tags(), left: tags.input()?.value }).toEqual({
        typed: "che",
        tags: ["travel", "food", "wine", "cheese", "bread"],
        left: "",
    });
});

test("refuse a tag that is there, is not taken, or is one too many, its text kept", () => {
    const tags = field({
        defaultValue: ["a"],
        max: 2,
        maxLength: 4,
        validate: (tag) => tag !== "bad",
    });
    const left: (string | undefined)[] = [];
    for (const words of ["a", "bad", "longer", "b", "c"]) {
        tags.type(words);
        press("Enter");
        left.push(tags.input()?.value);
    }

    expect({
        told: tags.told,
        left,
        tags: tags.tags(),
        isInvalid: tags.input()?.getAttribute("aria-invalid"),
    }).toEqual({
        told: ["a duplicate", "bad invalid", "longer invalid", "c max"],
        left: ["a", "bad", "longer", "", "c"],
        tags: ["a", "b"],
        isInvalid: "true",
    });
});

test("take a tag twice and one too many where the owner allows it, then invalid", () => {
    const tags = field({ defaultValue: ["a"], max: 1, allowOverflow: true, allowDuplicates: true });
    tags.type("a");
    press("Enter");

    expect([tags.tags(), tags.told, tags.input()?.getAttribute("aria-invalid")]).toEqual([
        ["a", "a"],
        [],
        "true",
    ]);
});

test("move between the tags by the keys, change one in place and remove others", async () => {
    const tags = field({ defaultValue: ["a", "b", "c"] });
    const path: string[] = [];
    const step = (...keys: readonly string[]): void => {
        press(...keys);
        path.push(focused());
    };

    // go to the last, the first, past the last, and back
    tags.input()?.focus();
    step("Backspace");
    step("Home");
    step("End");
    step("ArrowLeft");
    step("ArrowLeft");

    // change the middle one, then leave a change
    press("Enter");
    const edit = (): HTMLInputElement | null =>
        tags.container.querySelector("[data-slot=tag-field-edit]");
    await Promise.resolve();
    const element = edit();
    if (element !== null) {
        element.value = "changed";
    }
    press("Enter");
    const changed = tags.tags();
    press("F2");
    await Promise.resolve();
    press("Escape");
    const same = tags.tags();

    // remove it, and the one before
    press("Delete");
    const deleted = tags.tags();
    press("Backspace");

    expect({
        path,
        changed,
        same,
        deleted,
        left: tags.tags(),
        status: tags.container.querySelector("[role=status]")?.textContent,
    }).toEqual({
        path: ["c", "a", "", "c", "b"],
        changed: ["a", "changed", "c"],
        same: ["a", "changed", "c"],
        deleted: ["a", "c"],
        left: ["a"],
        status: "Removed \u2068c\u2069",
    });
});

test.for([
    ["add", ["a", "typed"], ""],
    ["clear", ["a"], ""],
    ["keep", ["a"], "typed"],
] as const)("%s the text as a person leaves", ([blurBehavior, expected, left]) => {
    const tags = field({ defaultValue: ["a"], blurBehavior });
    tags.type("typed");
    tags.input()?.blur();
    flush();

    expect([tags.tags(), tags.input()?.value]).toEqual([expected, left]);
});

test("keep a tag whose change is refused, and tell of it", async () => {
    const tags = field({ defaultValue: ["a", "b"] });
    tags.input()?.focus();
    press("Backspace", "Enter");
    await Promise.resolve();
    const edit = tags.container.querySelector<HTMLInputElement>("[data-slot=tag-field-edit]");
    if (edit !== null) {
        edit.value = "a";
    }
    press("Enter");

    expect([tags.tags(), tags.told]).toEqual([["a", "b"], ["a duplicate"]]);
});

test("remove every tag by a button, and none of a field to read", () => {
    const tags = field({
        defaultValue: ["a", "b"],
        get children() {
            return (
                <>
                    <TagFieldList />
                    <TagFieldInput />
                    <TagFieldClear />
                </>
            );
        },
    });
    tags.container.querySelector<HTMLElement>("[data-slot=tag-field-clear]")?.click();
    flush();
    const readOnly = field({ readOnly: true, defaultValue: ["a", "b"] });

    expect([
        tags.tags(),
        readOnly.tags(),
        readOnly.container.querySelectorAll("button, input").length,
    ]).toEqual([[], ["a", "b"], 0]);
});
