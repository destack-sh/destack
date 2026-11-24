import { Locale, Localization } from "@destack/locale";
import { present } from "@destack/schema";
import { expect, test } from "@destack/test";
import { testSimulation } from "@destack/test/world";
import { BrowserSimulation } from "@destack/environment/browser/simulation";
import { flush, LocaleContext } from "@destack/view";
import { render } from "@destack/view/test";
import { NumberField, type NumberFieldProperties } from "./index.ts";

/** Draw a number field, recording what it gives. */
function draw(properties: NumberFieldProperties, tag = "en") {
    const changed: (number | undefined)[] = [];
    const committed: (number | undefined)[] = [];
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
            <NumberField
                name="count"
                onValueChange={(value) => void changed.push(value)}
                onValueCommitted={(value) => void committed.push(value)}
                {...properties}
            />
        </LocaleContext>
    ));
    const input = present(container.querySelector<HTMLInputElement>("input[type=text]"), "input");
    const type = (text: string) => {
        input.value = text;
        input.dispatchEvent(new InputEvent("input", { bubbles: true }));
        flush();

        return input.value;
    };
    const press = (key: string, modifiers: KeyboardEventInit = {}) => {
        input.dispatchEvent(
            new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true, ...modifiers }),
        );
        flush();

        return input.value;
    };
    const blur = () => {
        input.dispatchEvent(new FocusEvent("blur"));
        flush();

        return input.value;
    };
    const submitted = () => container.querySelector<HTMLInputElement>("input[name=count]")?.value;

    return { container, input, type, press, blur, changed, committed, submitted };
}

/** Settle on a text, and read what the field shows and gives. */
function settle(properties: NumberFieldProperties, text: string) {
    const { type, blur, committed } = draw(properties);
    type(text);

    return [blur(), committed];
}

test("step by keys, finer and coarser", () => {
    const { press } = draw({ defaultValue: 1, step: 0.1 });

    expect([
        press("ArrowUp"),
        press("ArrowUp"),
        press("ArrowUp", { shiftKey: true }),
        press("ArrowDown", { altKey: true }),
        press("PageDown"),
    ]).toEqual(["1.1", "1.2", "2.2", "2.19", "1.19"]);
});

test("stay within the limits and go to them", () => {
    const { press } = draw({ defaultValue: 4, min: 1, max: 5 });

    expect([
        press("ArrowUp"),
        press("ArrowUp"),
        press("Home"),
        press("ArrowDown"),
        press("End"),
    ]).toEqual(["5", "5", "1", "1", "5"]);
});

test("snap to multiples from the lowest", () => {
    const { press } = draw({ defaultValue: 6, min: 2, step: 3, snapOnStep: true });

    expect([press("ArrowUp"), press("ArrowUp"), press("ArrowDown"), press("ArrowDown")]).toEqual([
        "8",
        "11",
        "8",
        "5",
    ]);
});

test("take what is typed as it is typed, and settle within the limits", () => {
    const { type, blur, changed, committed, submitted } = draw({ min: 0, max: 50 });

    expect([type("7"), type("7."), type("72"), blur(), submitted()]).toEqual([
        "7",
        "7.",
        "72",
        "50",
        "50",
    ]);
    expect({ changed, committed }).toEqual({ changed: [7, 72, 50], committed: [50] });
});

test("leave text that is no number, and ask for one", () => {
    const { container, input, type, blur, committed } = draw({ defaultValue: 3 });
    const read = () => [
        input.value,
        input.getAttribute("aria-invalid"),
        container.querySelector<HTMLInputElement>("input[name=count]")?.validationMessage,
    ];
    type("abc");
    const typing = read();
    blur();
    const left = read();
    type("4");
    blur();
    const fixed = read();
    type("");
    blur();

    expect([typing, left, fixed, read(), committed]).toEqual([
        ["abc", null, ""],
        ["abc", "true", "Enter a number"],
        ["4", null, ""],
        ["", null, ""],
        [undefined, 4, undefined],
    ]);
});

test("read and write as the language does", () => {
    const german = draw(
        { defaultValue: 1234.5, format: { style: "currency", currency: "EUR" } },
        "de",
    );
    const before = german.input.value;
    german.type("2.500,75");

    expect([before, german.blur(), german.committed]).toEqual([
        "1.234,50\u00a0€",
        "2.500,75\u00a0€",
        [2500.75],
    ]);
});

test("leave a field that takes no input", () => {
    const { press, container, changed } = draw({ defaultValue: 2, disabled: true });
    const buttons = [...container.querySelectorAll("button")].map((button) => button.disabled);

    expect([press("ArrowUp"), buttons, changed]).toEqual(["2", [true, true], []]);
});

test("keep the digits it is written with, those of its steps included", () => {
    expect([
        settle({ format: { style: "percent" } }, "12.5%"),
        settle({ format: { style: "percent" }, step: 0.005 }, "12.5%"),
        settle({ format: { style: "currency", currency: "USD" } }, "2.50075"),
        settle({}, "2.50075"),
        settle({ format: { maximumFractionDigits: 0 } }, "2.6"),
    ]).toEqual([
        ["13%", [0.13]],
        ["12.5%", [0.125]],
        ["$2.50", [2.5]],
        ["2.50075", [2.50075]],
        ["3", [3]],
    ]);
});

test("stop stepping at the limit while the button is pressed", async () => {
    await using simulation = testSimulation();
    const tab = new BrowserSimulation(simulation).environment("tab");
    const committed: (number | undefined)[] = [];
    const { container } = render(
        () => (
            <NumberField
                defaultValue={1}
                max={3}
                onValueCommitted={(value) => void committed.push(value)}
            />
        ),
        { environment: tab },
    );
    const button = container.querySelector("button");
    button?.dispatchEvent(
        new PointerEvent("pointerdown", { button: 0, bubbles: true, cancelable: true }),
    );
    flush();
    await simulation.jump(simulation.clock.now() + 60_000);

    expect([committed, button?.disabled]).toEqual([[3], true]);
});
