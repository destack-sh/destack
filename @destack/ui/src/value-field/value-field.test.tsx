import {
    BoxSides,
    Color,
    Corners,
    Dimension,
    FilterStack,
    FontFeatures,
    Layout,
    Paint,
    Palette,
    type Shadow,
    ShadowStack,
    Size,
    Stroke,
    TextFormat,
    Transition,
    Opacity,
    Typography,
} from "@destack/style";
import type { ActionValue, Choice } from "@destack/object";
import {
    Duration,
    type JsonObject,
    JsonValue,
    Money,
    present,
    Rating,
    RecurrenceRule,
    schema,
    toJsonSchema,
    Url,
} from "@destack/schema";
import { expect, test } from "@destack/test";
import { createSignal, flush } from "@destack/view";
import { render, stubPopovers } from "@destack/view/test";
import { MIXED, sharedValue, ValueField, ValueFieldProvider } from "./index.ts";
import { preloadNamedInputs } from "./index.ts";

// load the kit's named inputs, so every field of a named schema renders at once
await preloadNamedInputs();

/** Draw a value's input, recording what it gives. */
function draw(type: ActionValue, start?: JsonValue, choices: readonly Choice[] = []) {
    const given: (JsonValue | undefined)[] = [];
    const [value, setValue] = createSignal(start);
    const { container } = render(() => (
        <ValueField
            type={type}
            value={value()}
            choices={choices}
            onValueChange={(next) => {
                given.push(next);
                setValue(() => next);
            }}
        />
    ));
    const write = (text: string) => {
        const input = present(
            container.querySelector<HTMLInputElement | HTMLTextAreaElement>("input, textarea"),
            "the input",
        );
        input.value = text;
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
        flush();
    };

    return { container, given, type: write };
}

test("give text, numbers and JSON", () => {
    const text = draw({ kind: "text" });
    text.type("Lake");
    text.type("");
    const number = draw({ kind: "number", format: { style: "percent" } });
    number.type("25%");
    const json = draw({ kind: "json", schema: describe(schema.json()) });
    json.type('{"count": 1}');
    json.type("{");

    expect([text.given, number.given, json.given]).toEqual([
        ["Lake", undefined],
        [0.25],
        [{ count: 1 }, undefined],
    ]);
});

test("give a yes or no from where it starts", () => {
    const { container, given } = draw({ kind: "boolean" }, true);
    container.querySelector<HTMLElement>("[role=switch], input[type=checkbox]")?.click();
    flush();

    expect(given).toEqual([false]);
});

test("take an input an app replaces", () => {
    const given: (JsonValue | undefined)[] = [];
    const { container } = render(() => (
        <ValueFieldProvider
            inputs={{
                number: (properties) => (
                    <button
                        type="button"
                        onClick={() => properties.onValueChange(properties.type.maximum)}
                    >
                        most
                    </button>
                ),
            }}
        >
            <ValueField
                type={{ kind: "number", maximum: 5 }}
                value={undefined}
                onValueChange={(value) => void given.push(value)}
            />
        </ValueFieldProvider>
    ));
    container.querySelector("button")?.click();

    expect(given).toEqual([5]);
});

test("take the input an app replaces by its schema's name, inside one it replaces further out", () => {
    // replace the paint's input, inside a provider that replaces the number's
    const given: (JsonValue | undefined)[] = [];
    const { container } = render(() => (
        <ValueFieldProvider inputs={{ number: () => <output>number</output> }}>
            <ValueFieldProvider
                named={{
                    "style.paint": (properties) => (
                        <button
                            type="button"
                            onClick={() =>
                                properties.onValueChange({ kind: "solid", color: "#ff0000" })
                            }
                        >
                            red
                        </button>
                    ),
                }}
            >
                <ValueField
                    type={{ kind: "json", schema: describe(Paint) }}
                    value={undefined}
                    onValueChange={(value) => void given.push(value)}
                />
                <ValueField type={{ kind: "number" }} value={undefined} onValueChange={() => {}} />
            </ValueFieldProvider>
        </ValueFieldProvider>
    ));
    present(container.querySelector("button"), "the app's paint input").click();

    // expect the paint given by the app's input, and the outer provider's number kept
    expect({ given, number: container.querySelector("output")?.textContent }).toEqual({
        given: [{ kind: "solid", color: "#ff0000" }],
        number: "number",
    });
});

test("show the title of a person, and keep them as other text is typed", () => {
    stubPopovers();
    const people = [{ value: "a", title: "Ada" }];
    const { container, given, type } = draw({ kind: "person" }, "a", people);
    const shown = () => container.querySelector("input")?.value;
    const before = shown();
    type("Gr");
    const during = shown();
    type("");

    expect([before, during, shown(), given]).toEqual(["Ada", "Gr", "", [undefined]]);
});

test("keep JSON as it is typed", () => {
    const { container, type } = draw({ kind: "json", schema: describe(schema.json()) }, [1]);
    const shown = () => container.querySelector("textarea")?.value;
    const before = shown();
    type("[1,  2]");
    const during = shown();
    type("[1,");

    expect([before, during, shown()]).toEqual(["[\n  1\n]", "[1,  2]", "[1,"]);
});

test("give a color typed in its field, the field keeping the text", () => {
    stubPopovers();
    const { container, given } = draw({ kind: "json", schema: describe(Color.schema) }, "#f76b15");

    // type a color into the row's own text, without opening the picker
    const field = present(
        container.querySelector<HTMLInputElement>("input[data-slot=color-field]"),
        "the color's text",
    );
    field.value = "#0090ff";
    field.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", bubbles: true }));
    flush();

    expect([field.value, given.at(-1)]).toEqual(["#0090ff", "#0090ff"]);
});

test("add, move and remove the colors of a palette between the fewest and the most", () => {
    stubPopovers();
    const { container, given } = draw({ kind: "json", schema: describe(Palette.min(2).max(3)) }, [
        "#e0eaff",
        "#241d9a",
    ]);

    // adding repeats the last color up to the most, a swatch's popover moves its color and removes it
    press(container, "Add color");
    press(container, "Add color");
    press(container, "Color 1");
    press(document.body, "Move later", "Color 1");
    press(container, "Color 3");
    press(document.body, "Remove color", "Color 3");

    expect(given).toEqual([
        ["#e0eaff", "#241d9a", "#241d9a"],
        ["#241d9a", "#e0eaff", "#241d9a"],
        ["#241d9a", "#e0eaff"],
    ]);
});

test("give a length by its number, kept in its unit", () => {
    stubPopovers();
    const { given, type } = draw(typeOf(Dimension), px(12));
    type("16");

    expect(given).toEqual([px(16)]);
});

test("round every corner by one radius, then one corner on its own", () => {
    // set all four, then turn each one apart and change the top left
    stubPopovers();
    const { container, given } = draw(typeOf(Corners), Corners.all(px(8)));
    fill(container, "All corners", "12");
    press(container, "Each corner");
    fill(container, "Top left", "0");

    expect(given).toEqual([Corners.all(px(12)), { ...Corners.all(px(12)), topLeft: px(0) }]);
});

test("draw a stroke outside the box", () => {
    stubPopovers();
    const start: Stroke = { color: "#000000", width: px(1), style: "solid", align: "inside" };
    const { container, given } = draw(typeOf(Stroke), start);
    press(container, "Outside");

    expect(given).toEqual([{ ...start, align: "outside" }]);
});

test("add a shadow on top, set it inside, and remove the one below", () => {
    // start from one shadow, add another, turn the new one inward, then remove the old one
    stubPopovers();
    const old: Shadow = {
        color: "#00000033",
        offsetX: px(0),
        offsetY: px(1),
        blur: px(2),
        spread: px(0),
        inset: false,
    };
    const { container, given } = draw(typeOf(ShadowStack), [old]);
    press(container, "Add shadow");
    const top = present(container.querySelector("[aria-label='Shadow 1']"), "the top shadow");
    press(top, "Inner shadow");
    const isKept = container.querySelector("[aria-label='Shadow 1']") === top;
    press(container, "Remove Shadow 2");
    const added: Shadow = { ...old, offsetY: px(4), blur: px(12) };

    // expect each change, the top shadow's row kept as it changed
    expect({ given, isKept }).toEqual({
        given: [[added, old], [{ ...added, inset: true }, old], [{ ...added, inset: true }]],
        isKept: true,
    });
});

test("pick a document's typeface after the generic fallbacks, list its weights, or use a typed one", () => {
    // declare two faces of one family in the document
    stubPopovers();
    const fonts = Object.getOwnPropertyDescriptor(document, "fonts");
    Object.defineProperty(document, "fonts", {
        configurable: true,
        value: new Set([
            { family: "Plex Mono", weight: "400", style: "normal", stretch: "normal" },
            { family: "Plex Mono", weight: "700", style: "normal", stretch: "normal" },
        ]),
    });
    const start: Typography = {
        fontFamily: ["sans-serif"],
        fontSize: { value: 1, unit: "rem" },
        fontWeight: 400,
        letterSpacing: px(0),
        lineHeight: 1.5,
    };
    const { container, given } = draw(typeOf(Typography), start);

    // pick the family, then read the weights its faces draw
    press(container, "Font family");
    present(document.querySelector<HTMLElement>('[data-value="Plex Mono"]'), "Plex Mono").click();
    flush();
    const weights = [...container.querySelectorAll('[aria-label="Weight"] [role=radio]')].map(
        (segment) => segment.textContent,
    );

    // search for a font the document lacks, then use it
    press(container, "Font family");
    const search = present(
        document.querySelector<HTMLInputElement>("[data-slot=command-input]"),
        "the font search",
    );
    search.value = "Inter Variable";
    search.dispatchEvent(new InputEvent("input", { bubbles: true }));
    flush();
    present(
        document.querySelector<HTMLElement>('[data-value="Inter Variable"]'),
        "the typed font",
    ).click();
    flush();
    Object.defineProperty(document, "fonts", present(fonts, "the document's fonts"));

    expect([given, weights]).toEqual([
        [
            { ...start, fontFamily: ["Plex Mono", "sans-serif"] },
            { ...start, fontFamily: ["Inter Variable"] },
        ],
        ["Regular", "Bold"],
    ]);
});

test("give a transition's duration, kept in its unit", () => {
    stubPopovers();
    const start: Transition = {
        duration: { milliseconds: 200 },
        delay: {},
        timingFunction: [0, 0, 0.58, 1],
    };
    const { container, given } = draw(typeOf(Transition), start);
    fill(container, "Duration", "300");

    expect(given).toEqual([{ ...start, duration: { milliseconds: 300 } }]);
});

test("lay children out in a column, with the padding its sides input gives", () => {
    // turn the layout into a column, then pad every side
    stubPopovers();
    const start: Layout = {
        direction: "row",
        gap: px(8),
        align: "start",
        justify: "start",
        isWrapping: false,
        padding: BoxSides.all(px(0)),
    };
    const { container, given } = draw(typeOf(Layout), start);
    press(container, "Column");
    fill(container, "All sides", "16");

    expect(given).toEqual([
        { ...start, direction: "column" },
        { ...start, direction: "column", padding: BoxSides.all(px(16)) },
    ]);
});

test("size a box across its container, then give it a least width", () => {
    stubPopovers();
    const none = { kind: "none" } as const;
    const start: Size = {
        width: { kind: "fit" },
        height: { kind: "fit" },
        minWidth: none,
        maxWidth: none,
        minHeight: none,
        maxHeight: none,
    };
    const { container, given } = draw(typeOf(Size), start);

    // pick fill in the width's menu, then turn on its min width there
    press(container, "Width sizing");
    choose("Fill");
    press(container, "Width sizing");
    choose("Min width");

    expect(given).toEqual([
        { ...start, width: { kind: "fill" } },
        { ...start, width: { kind: "fill" }, minWidth: { kind: "length", length: px(100) } },
    ]);
});

test("cut text after some lines", () => {
    stubPopovers();
    const start: TextFormat = {
        align: "start",
        case: "none",
        decoration: "none",
        wrap: "wrap",
        truncation: { kind: "none" },
    };
    const { container, given } = draw(typeOf(TextFormat), start);
    press(container, "Text settings");
    press(container, "Cut after lines");
    fill(container, "Lines", "3");

    expect(given).toEqual([
        { ...start, truncation: { kind: "lines", count: 1 } },
        { ...start, truncation: { kind: "lines", count: 3 } },
    ]);
});

test("add a filter last, and an OpenType feature by its tag", () => {
    stubPopovers();
    const filters = draw(typeOf(FilterStack), []);
    press(filters.container, "Add filter");
    const features = draw(typeOf(FontFeatures), {});
    const tag = present(
        features.container.querySelector<HTMLInputElement>("input[aria-label='New tag']"),
        "the new tag",
    );
    tag.value = "tnum";
    tag.dispatchEvent(new InputEvent("input", { bubbles: true }));
    flush();
    press(features.container, "Add");

    expect([filters.given, features.given]).toEqual([
        [[{ kind: "blur", radius: px(4) }]],
        [{ tnum: 1 }],
    ]);
});

test("give a duration as Temporal holds it, a number and a unit for each of its units", () => {
    // change the hours, add minutes below them, then remove the hours
    stubPopovers();
    const { container, given } = draw(typeOf(Duration), { hours: 1 });
    fill(container, "Amount", "2");
    press(container, "Add unit");
    const minutes = present(
        container.querySelectorAll<HTMLInputElement>("input[aria-label=Amount]")[1],
        "the minutes",
    );
    minutes.value = "30";
    minutes.dispatchEvent(new InputEvent("input", { bubbles: true }));
    flush();
    press(container, "Remove Hours");

    expect(given).toEqual([
        { hours: 2 },
        { hours: 2, minutes: 0 },
        { hours: 2, minutes: 30 },
        { minutes: 30 },
    ]);
});

test("carry a fraction of a unit typed into the smaller units, as 1.5 hours", () => {
    stubPopovers();
    const { container, given } = draw(typeOf(Duration), { hours: 1 });
    fill(container, "Amount", "1.5");

    expect(given).toEqual([{ hours: 1, minutes: 30 }]);
});

test("repeat every other week on the weekdays pressed, every week by default", () => {
    stubPopovers();
    const { container, given } = draw(typeOf(RecurrenceRule), { frequency: "weekly" });
    fill(container, "Every", "2");
    press(container, "Monday");
    press(container, "Friday");
    fill(container, "Every", "1");

    expect(given).toEqual([
        { frequency: "weekly", interval: 2 },
        { frequency: "weekly", interval: 2, byDay: [{ day: 1 }] },
        { frequency: "weekly", interval: 2, byDay: [{ day: 1 }, { day: 5 }] },
        { frequency: "weekly", byDay: [{ day: 1 }, { day: 5 }] },
    ]);
});

test("give money in its currency's minor unit from the amount typed in its major unit", () => {
    const start: Money = { amount: 1250, currency: "EUR" };
    stubPopovers();
    const { container, given } = draw(typeOf(Money), start);
    fill(container, "Amount", "19.99");

    expect(given).toEqual([{ amount: 1999, currency: "EUR" }]);
});

test("rate by the star pressed, out of the most the schema allows, and clear by pressing it again", () => {
    stubPopovers();
    const { container, given } = draw(typeOf(Rating.max(3)), 2);
    const stars = container.querySelectorAll("[data-slot=toggle-group-item], button");
    press(container, "3 stars");
    press(container, "3 stars");

    expect({ stars: stars.length, given }).toEqual({ stars: 3, given: [3, 0] });
});

test("give a web address once it parses, and none once the field is cleared", () => {
    stubPopovers();
    const { container, given } = draw(typeOf(Url), undefined);
    const input = present(
        container.querySelector<HTMLInputElement>("input[type=url]"),
        "the address",
    );
    for (const text of ["example", " https://example.com/notes ", ""]) {
        input.value = text;
        input.dispatchEvent(new Event("change", { bubbles: true }));
        flush();
    }

    expect(given).toEqual(["https://example.com/notes", undefined]);
});

test("show the inherited value muted until one is given, and give it back by reset", () => {
    // start unset, with half in force
    const { container, given, type } = drawLabelled({
        label: "Opacity",
        type: { kind: "number", format: { style: "percent" } },
        start: undefined,
        inherited: 0.5,
    });
    const seen = () => ({
        state: container.querySelector("[data-slot=field]")?.getAttribute("data-state"),
        shown: container.querySelector("input")?.value,
        reset: buttonNames(container).includes("Reset Opacity"),
    });
    const unset = seen();

    // give a quarter, then reset it to the half in force, the press taking the focus
    type("25%");
    const set = seen();
    container.querySelector("input")?.blur();
    press(container, "Reset Opacity");

    expect([unset, set, seen(), given]).toEqual([
        { state: "unset", shown: "50%", reset: false },
        { state: "set", shown: "25%", reset: true },
        { state: "unset", shown: "50%", reset: false },
        [0.25, undefined],
    ]);
});

test("show values several objects hold apart as one mark, and give them all one from it", () => {
    // press the mark, which the label names and which hands the focus to the input
    const { container, given, type } = drawLabelled({
        label: "Title",
        type: { kind: "text" },
        start: MIXED,
        inherited: undefined,
    });
    const mark = present(
        [...container.querySelectorAll("button")].find((button) => nameOf(button) === "Mixed"),
        "the mark of mixed values",
    );
    const label = container.querySelector("label")?.getAttribute("for");
    const before = { text: mark.textContent, isNamed: label === mark.id };
    mark.click();
    flush();
    const isFocused = document.activeElement === container.querySelector("input");
    type("Lake");

    expect([before, isFocused, given]).toEqual([{ text: "Mixed", isNamed: true }, true, ["Lake"]]);
});

test("share a value all objects hold alike, whatever its key order, and mark one they hold apart", () => {
    expect(
        [
            [
                { x: 1, y: 2 },
                { y: 2, x: 1 },
            ],
            [undefined, undefined],
            [1, 2],
            [undefined, 1],
        ].map((values) => sharedValue(values)),
    ).toEqual([{ x: 1, y: 2 }, undefined, MIXED, MIXED]);
});

test("show no stroke as its legend and a button to add one, which starts a stroke", () => {
    const { container, given } = drawLabelled({
        label: "Stroke",
        type: typeOf(Stroke),
        start: undefined,
        inherited: undefined,
    });
    const before = [...container.querySelectorAll("input, select")].length;
    press(container, "Add Stroke");

    expect([before, given]).toEqual([
        0,
        [{ color: "#000000", width: px(1), style: "solid", align: "inside" }],
    ]);
});

test("show no bindable type as one legend, its binding and adding side by side", () => {
    const { container } = drawLabelled({
        label: "Type",
        type: typeOf(Typography.bindable),
        start: undefined,
        inherited: undefined,
    });

    // the value inside hands its add action to the binding's legend, which draws the only legend
    expect(
        [...container.querySelectorAll("legend")].map((legend) =>
            [...legend.querySelectorAll("button")].map((button) =>
                button.getAttribute("aria-label")?.replaceAll(/[\u2068\u2069]/gu, ""),
            ),
        ),
    ).toEqual([["Bind to token", "Add Type"]]);
});

test("show a value of another shape as unreadable, rather than a value it does not hold", () => {
    // store text where an opacity belongs, as data from before a schema changed would
    const { container } = render(() => (
        <>
            <ValueField
                label="Opacity"
                type={typeOf(Opacity)}
                value="half"
                onValueChange={() => {}}
            />
            <ValueField
                label="Name"
                type={{ kind: "text" }}
                value="Card"
                onValueChange={() => {}}
            />
        </>
    ));
    const inputs = [...container.querySelectorAll("input")];

    // the opacity says it cannot be read, and the field beside it keeps working
    expect(inputs.map((input) => [input.value, input.getAttribute("aria-invalid")])).toEqual([
        ["This value cannot be read", "true"],
        ["Card", null],
    ]);
});

test("reset a value of several parts from its legend", () => {
    const { container, given } = drawLabelled({
        label: "Corners",
        type: typeOf(Corners),
        start: Corners.all(px(8)),
        inherited: undefined,
    });
    const legend = present(container.querySelector("legend"), "the legend");
    const isInLegend = buttonNames(legend).includes("Reset Corners");
    press(container, "Reset Corners");

    expect([isInLegend, given]).toEqual([true, [undefined]]);
});

/** Draw a labelled value's field from a start and the value in force, recording what it gives. */
function drawLabelled(options: {
    readonly label: string;
    readonly type: ActionValue;
    readonly start: JsonValue | undefined | typeof MIXED;
    readonly inherited: JsonValue | undefined;
}) {
    stubPopovers();
    const given: (JsonValue | undefined)[] = [];
    const [value, setValue] = createSignal<JsonValue | undefined | typeof MIXED>(options.start);
    const { container } = render(() => (
        <ValueField
            label={options.label}
            type={options.type}
            value={value()}
            inherited={options.inherited}
            onValueChange={(next) => {
                given.push(next);
                setValue(() => next);
            }}
        />
    ));
    const type = (text: string) => {
        const input = present(container.querySelector("input"), "the input");
        input.focus();
        input.value = text;
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
        flush();
    };

    return { container, given, type };
}

/** List the names of the buttons inside an element. */
function buttonNames(root: Element): string[] {
    return [...root.querySelectorAll("button")].map((button) => nameOf(button));
}

/** A length in pixels. */
function px(value: number): Dimension {
    return { value, unit: "px" };
}

/** Type a value's schema as the JSON a JSON input takes. */
function typeOf(definition: schema.Schema): ActionValue {
    return { kind: "json", schema: describe(definition) };
}

/** Type a number into the number field of a name, or into the input inside it. */
function fill(root: Element, label: string, text: string): void {
    const field = present(root.querySelector(`[aria-label="${label}"]`), `the field ${label}`);
    const input = present(
        field instanceof HTMLInputElement ? field : field.querySelector("input"),
        `the input ${label}`,
    );
    input.value = text;
    input.dispatchEvent(new InputEvent("input", { bubbles: true }));
    flush();
}

/** Choose an item of the open menu by its text. */
function choose(text: string): void {
    const items = [...document.querySelectorAll<HTMLElement>("[role^=menuitem]")];
    present(
        items.find((item) => item.textContent.trim() === text),
        `the menu item ${text}`,
    ).click();
    flush();
}

/** Read a button's name without the marks that isolate a number in it. */
function nameOf(button: HTMLButtonElement): string {
    return (button.getAttribute("aria-label") ?? button.textContent).replaceAll(
        /[\u2066-\u2069]/gu,
        "",
    );
}

/** Press the button of a name, inside the popover of another name where one is given. */
function press(root: Element, label: string, popover?: string): void {
    const scope =
        popover === undefined
            ? root
            : present(
                  root.querySelector(`[data-slot=popover-content][aria-label="${popover}"]`),
                  `the popover of ${popover}`,
              );
    const buttons = [...scope.querySelectorAll<HTMLButtonElement>("button")];
    present(
        buttons.find((each) => nameOf(each) === label),
        `the button ${label}`,
    ).click();
    flush();
}

/** Describe a schema as the JSON Schema a JSON input carries. */
function describe(definition: schema.Schema): JsonObject {
    return JsonValue.object(toJsonSchema(definition));
}
