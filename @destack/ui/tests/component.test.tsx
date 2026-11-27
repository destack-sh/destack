import { ActionInput } from "@destack/object";
import type { Example } from "@destack/package/declare";
import type { JsonValue } from "@destack/schema";
import { expect, onTestFinished, test } from "@destack/test";
import { flush, type JSX } from "@destack/view";
import type { Component } from "@destack/view/declare";
import { markup, render, stubPopovers } from "@destack/view/test";
import { layOutElements } from "../src/virtualizer/test/layout.ts";
import { COMPONENTS, EXAMPLES } from "./fixture/examples.ts";

/** The size every image loads at, a photo's. */
const PHOTO = { width: 1600, height: 1200 };

/** The size of every element, a phone's screen, which the test DOM does not lay out. */
const SCREEN = 640;

/** The components that declare properties a person changes, by their exported names. */
const DECLARING = COMPONENTS.filter(
    ([, component]) => Object.keys(component.properties).length > 0,
);

test("show every component that declares properties in an example of its own", () => {
    // list each declaring component no example is of
    const shown = new Set(EXAMPLES.map(([, example]) => example.of));
    const missing = DECLARING.filter(([, component]) => !shown.has(component)).map(
        ([name]) => name,
    );

    expect(missing).toEqual([]);
});

test("label and describe every declared property, and title each of its choices in words", () => {
    // read each property as the inspector does, where only choices in words need a title
    const problems = DECLARING.flatMap(([name, component]) =>
        Object.entries(component.properties).flatMap(([key, definition]) => {
            const input = ActionInput.of(key, definition);
            const untitled =
                input.value.kind === "choice"
                    ? input.value.choices.filter(
                          (choice) => choice.title === undefined && /\p{L}/u.test(choice.value),
                      )
                    : [];

            return [
                ...(input.label === undefined ? [`${name}.${key}: label it`] : []),
                ...(input.description === undefined ? [`${name}.${key}: describe it`] : []),
                ...untitled.map((choice) => `${name}.${key}.${choice.value}: title it`),
            ];
        }),
    );

    expect(problems).toEqual([]);
});

test("change every example by its component's properties, and every property in an example", () => {
    // render each example of a declaring component with each property changed, noting what responds
    stubPopovers();
    loadImages();
    layOutElements({ client: SCREEN, scroll: SCREEN });
    const responsive = new Set<string>();
    const ignoring: string[] = [];
    for (const [exported, example] of EXAMPLES) {
        const component = DECLARING.find(([, each]) => each === example.of);
        if (component === undefined) {
            continue;
        }
        const [name, declared] = component;
        const base = { ...declared.defaults, ...example.properties };
        const before = drawn(example, base);
        const changed = Object.keys(declared.properties).filter((key) =>
            otherValuesOf(declared, key, base[key]).some(
                (value) => drawn(example, { ...base, [key]: value }) !== before,
            ),
        );

        // note an example no property changes, and each property that changed this one
        if (changed.length === 0) {
            ignoring.push(`${exported}: pass its properties to ${name}`);
        }
        for (const key of changed) {
            responsive.add(`${name}.${key}`);
        }
    }

    // list each declared property no example responds to
    const unused = DECLARING.flatMap(([name, component]) =>
        Object.keys(component.properties)
            .map((key) => `${name}.${key}`)
            .filter((property) => !responsive.has(property))
            .map((property) => `${property}: change an example or stop declaring it`),
    );

    expect({ ignoring, unused }).toEqual({ ignoring: [], unused: [] });
});

/** Render an example with some properties, reading its markup and classes, popovers included. */
function drawn(example: Example<object, JSX.Element>, properties: Record<string, unknown>): string {
    // render the example, then read the whole document, where popovers and dialogs portal to
    const { container, unmount } = render(() => example.render?.(properties));
    flush();
    const page = container.ownerDocument.body;
    const styled = [...page.querySelectorAll("*")].map((element) => element.getAttribute("class"));
    const read = `${markup(page)}\n${styled.join("\n")}`;
    unmount();

    return read;
}

/** List other values a property takes than its current one, as a person would set them. */
function otherValuesOf(
    component: Component,
    key: string,
    current: unknown,
): (JsonValue | undefined)[] {
    // read the values its input takes: each other choice, the other boolean, another number or text, or none
    const definition = component.properties[key];
    if (definition === undefined) {
        return [];
    }
    const { value, presence } = ActionInput.of(key, definition);
    if (value.kind === "choice") {
        return value.choices.map((choice) => choice.value).filter((each) => each !== current);
    } else if (value.kind === "boolean") {
        return [current !== true];
    } else if (value.kind === "number") {
        const number = typeof current === "number" ? current : (value.minimum ?? 0);

        return [
            value.maximum !== undefined && number + 1 > value.maximum ? number - 1 : number + 1,
        ];
    } else if (value.kind === "text") {
        return [current === "Changed" ? "Changed again" : "Changed"];
    }
    // clear an optional JSON value given, such as a paint, which no other value of its schema stands in for
    else if (value.kind === "json" && presence === "optional" && current !== undefined) {
        return [undefined];
    }

    return [];
}

/** Load every image at a photo's size, which the test DOM never fetches. */
function loadImages(): void {
    // stand in for a loaded image until the test ends
    const prototype = HTMLImageElement.prototype;
    const sizes = [
        ["complete", true],
        ["naturalWidth", PHOTO.width],
        ["naturalHeight", PHOTO.height],
    ] as const;
    const before = sizes.map(([name]) => Object.getOwnPropertyDescriptor(prototype, name));
    for (const [name, value] of sizes) {
        Object.defineProperty(prototype, name, { configurable: true, get: () => value });
    }

    // restore each property once the test finishes
    onTestFinished(() => {
        sizes.forEach(([name], index) => {
            const descriptor = before[index];
            if (descriptor !== undefined) {
                Object.defineProperty(prototype, name, descriptor);
            }
        });
    });
}
