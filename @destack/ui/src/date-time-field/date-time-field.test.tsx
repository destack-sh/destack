import { expect, test } from "@destack/test";
import { Locale, Localization } from "@destack/locale";
import { createSignal, flush, LocaleContext } from "@destack/view";
import { render } from "@destack/view/test";
import { DateTimeField } from "./index.ts";

/** The morning of a day, in Zurich 10:30. */
const MORNING = Date.UTC(2026, 10, 24, 9, 30);

/** Read the field of the morning in a language, on a clock. */
function written(tag: string, hourCycle?: 12 | 24): string | null {
    return render(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
            <DateTimeField value={MORNING} timeZone="Europe/Zurich" hourCycle={hourCycle} />
        </LocaleContext>
    )).container.textContent;
}

/** Type a key into the day of the morning in a language, and read it. */
function type(tag: string, key: string): string | null | undefined {
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
            <DateTimeField defaultValue={MORNING} timeZone="Europe/Zurich" />
        </LocaleContext>
    ));
    const day = container.querySelector<HTMLElement>("[data-segment=day]");
    day?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    flush();

    return day?.getAttribute("aria-valuenow");
}

test("give the instant as a segment changes, and show one set from outside", () => {
    const given: (number | undefined)[] = [];
    const [value, setValue] = createSignal<number | undefined>(MORNING);
    const { container } = render(() => (
        <DateTimeField
            value={value()}
            timeZone="Europe/Zurich"
            onValueChange={(next) => {
                given.push(next);
                setValue(next);
            }}
        />
    ));
    const segments = () => [...container.querySelectorAll<HTMLElement>("[role=spinbutton]")];
    const shown = () => segments().map((segment) => segment.textContent);
    const before = shown();
    const press = (place: number, key: string) => {
        segments()[place]?.dispatchEvent(
            new KeyboardEvent("keydown", { key, bubbles: true, cancelable: true }),
        );
        flush();
    };
    press(1, "ArrowUp");
    const after = shown();
    setValue(MORNING + 2 * 60 * 60_000);
    flush();
    const outside = shown();
    press(3, "Backspace");

    expect({ before, after, outside, given }).toEqual({
        before: ["11", "24", "2026", "10", "30", "AM"],
        after: ["11", "25", "2026", "10", "30", "AM"],
        outside: ["11", "24", "2026", "12", "30", "PM"],
        given: [MORNING + 24 * 60 * 60_000, undefined],
    });
    expect(shown().slice(0, 3)).toEqual(["11", "24", "2026"]);
});

test("lay out the time where the language has it, on the clock given", () => {
    expect([written("vi"), written("de"), written("de", 12), written("en-US", 24)]).toEqual([
        "10:30 24/11/2026",
        "24.11.2026, 10:30",
        "24.11.2026, 10:30 AM",
        "11/24/2026, 10:30",
    ]);
});

test("refuse an instant outside the limits, and submit and reset with a form", () => {
    const { container } = render(() => (
        <form>
            <DateTimeField
                name="due"
                defaultValue={MORNING}
                max={MORNING}
                timeZone="Europe/Zurich"
            />
        </form>
    ));
    const form = container.querySelector("form");
    const input = container.querySelector("input");
    const read = () => [
        input?.value,
        input?.validationMessage,
        container.querySelector("[role=group]")?.getAttribute("aria-invalid"),
    ];
    const before = read();
    container
        .querySelector("[data-segment=day]")
        ?.dispatchEvent(new KeyboardEvent("keydown", { key: "ArrowUp", bubbles: true }));
    flush();
    const after = read();
    form?.reset();
    flush();

    expect([before, after, read()]).toEqual([
        ["2026-11-24T09:30:00.000Z", "", null],
        [
            "2026-11-25T09:30:00.000Z",
            "Choose \u2068Nov 24, 2026 at 10:30 AM\u2069 or earlier.",
            "true",
        ],
        ["2026-11-24T09:30:00.000Z", "", null],
    ]);
});

test("show a value alone, its segments in reach of the keys", () => {
    const given: unknown[] = [];
    const { container } = render(() => (
        <DateTimeField
            readOnly
            defaultValue={MORNING}
            onValueChange={(value) => void given.push(value)}
        />
    ));
    const day = container.querySelector<HTMLElement>("[data-segment=day]");
    for (const key of ["ArrowUp", "5", "Backspace", "Home", "PageUp"]) {
        day?.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
    }

    expect([given, day?.getAttribute("aria-readonly"), day?.tabIndex]).toEqual([[], "true", 0]);
});

test("type the digits of the language, and no others", () => {
    expect([type("ar-EG", "\u0667"), type("ar-EG", "7"), type("en", "\u0667")]).toEqual([
        "7",
        "7",
        "24",
    ]);
});
