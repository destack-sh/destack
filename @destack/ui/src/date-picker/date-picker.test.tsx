import { Locale, Localization } from "@destack/locale";
import type { PlainDate } from "@destack/schema";
import { expect, test } from "@destack/test";
import { press, render, stubPopovers } from "@destack/view/test";
import { flush, type JSX, LocaleContext } from "@destack/view";
import { Day, type DateRange } from "../calendar/index.ts";
import { Field, FieldLabel } from "../field/index.ts";
import { DatePicker } from "./index.ts";

/** Read a `YYYY-MM-DD` date. */
function day(text: string): PlainDate {
    return Day.parse(text);
}

/** Render an element in a locale. */
function drawIn(tag: string, element: () => JSX.Element): HTMLElement {
    return render(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>{element()}</LocaleContext>
    )).container;
}

/** Click an element of a container by a selector. */
function click(container: Element, selector: string): void {
    container.querySelector<HTMLElement>(selector)?.click();
    flush();
}

/** Focus a segment of a container by a selector and press keys on it in turn. */
function type(container: Element, selector: string, ...keys: string[]): void {
    container.querySelector<HTMLElement>(selector)?.focus();
    for (const key of keys) {
        press(key);
        flush();
    }
}

/** Read whether the date picker's calendar popover in a container is open. */
function isOpen(container: Element): boolean | undefined {
    return container
        .querySelector("[data-slot=date-picker-content]")
        ?.hasAttribute("data-popover-open");
}

/** Read the text of each date field of a container. */
function fields(container: Element): (string | null)[] {
    return [...container.querySelectorAll("[data-slot=date-field]")].map(
        (field) => field.textContent,
    );
}

/** Read an element's accessible name from the elements its `aria-labelledby` names, in order. */
function labelledName(element: Element | null | undefined): string {
    return (element?.getAttribute("aria-labelledby") ?? "")
        .split(" ")
        .map((id) => document.getElementById(id)?.textContent)
        .join(" ");
}

test("type a day into a date picker's field, then pick another from its calendar shown at the typed month", () => {
    stubPopovers();
    const changes: (PlainDate | undefined)[] = [];
    const container = drawIn("en-US", () => (
        <DatePicker aria-label="Due date" onSelect={(value) => changes.push(value)} />
    ));
    const group = container.querySelector("[data-slot=date-picker]");
    type(container, "[data-segment=month]", "0", "3", "0", "7", "2", "0", "2", "7");
    click(container, "[data-slot=date-picker-trigger]");
    const opened = [
        isOpen(container),
        container.querySelector("[data-slot=calendar-caption]")?.textContent,
    ];
    click(container, '[data-day="2027-03-20"]');
    expect([
        group?.getAttribute("role"),
        group?.getAttribute("aria-label"),
        opened,
        changes,
        isOpen(container),
        fields(container),
    ]).toEqual([
        "group",
        "Due date",
        [true, "March 2027"],
        [day("2027-03-07"), day("2027-03-20")],
        false,
        ["03/20/2027"],
    ]);
});

test("type a range's start and end, then pick a range from the calendar, keeping the popover open until it closes", () => {
    stubPopovers();
    const changes: (DateRange | undefined)[] = [];
    const container = drawIn("en-US", () => (
        <DatePicker mode="range" aria-label="Trip" onSelect={(value) => changes.push(value)} />
    ));
    const [start, end] = container.querySelectorAll("[data-slot=date-field]");
    type(
        container,
        "[aria-label='End date'] [data-segment=month]",
        "1",
        "0",
        "0",
        "9",
        "2",
        "0",
        "2",
        "6",
    );
    const typedEnd = [...changes];
    type(
        container,
        "[aria-label='Start date'] [data-segment=month]",
        "1",
        "0",
        "0",
        "4",
        "2",
        "0",
        "2",
        "6",
    );
    click(container, "[data-slot=date-picker-trigger]");
    click(container, '[data-day="2026-10-12"]');
    const isHalfOpen = isOpen(container);
    click(container, '[data-day="2026-10-16"]');
    expect([
        [start?.getAttribute("aria-label"), end?.getAttribute("aria-label")],
        typedEnd,
        changes,
        isHalfOpen,
        isOpen(container),
        fields(container),
    ]).toEqual([
        ["Start date", "End date"],
        [],
        [
            { from: day("2026-10-04"), to: day("2026-10-09") },
            { from: day("2026-10-12") },
            { from: day("2026-10-12"), to: day("2026-10-16") },
        ],
        true,
        false,
        ["10/12/2026", "10/16/2026"],
    ]);
});

test("choose a preset beside the calendar, closing the popover and showing the day in the field", () => {
    stubPopovers();
    const container = drawIn("en-US", () => (
        <DatePicker
            aria-label="Due date"
            presets={[{ label: "Christmas", value: day("2026-12-25") }]}
        />
    ));
    click(container, "[data-slot=date-picker-trigger]");
    [...container.querySelectorAll<HTMLElement>("[data-slot=date-picker-presets] button")]
        .find((button) => button.textContent === "Christmas")
        ?.click();
    flush();
    expect([fields(container), isOpen(container)]).toEqual([["12/25/2026"], false]);
});

test("choose several days from a multiple date picker's trigger, named by the picker's name and the count", () => {
    stubPopovers();
    const container = drawIn("en-US", () => (
        <DatePicker mode="multiple" aria-label="Off days" defaultSelected={[day("2026-10-05")]} />
    ));
    const trigger = container.querySelector("[data-slot=date-picker-trigger]");
    const one = labelledName(trigger);
    click(container, "[data-slot=date-picker-trigger]");
    click(container, '[data-day="2026-10-06"]');
    expect([one, labelledName(trigger), isOpen(container)]).toEqual([
        "Off days Oct 5, 2026",
        "Off days 2 dates",
        true,
    ]);
});

test("name a date picker's group by its field's label", () => {
    const container = drawIn("en-US", () => (
        <Field>
            <FieldLabel>Due</FieldLabel>
            <DatePicker />
        </Field>
    ));
    const label = container.querySelector("[data-slot=field-label]");
    const group = container.querySelector("[data-slot=date-picker]");
    expect([
        group?.getAttribute("aria-labelledby") === label?.id,
        container.querySelector("[data-slot=date-field]")?.hasAttribute("aria-labelledby"),
    ]).toEqual([true, false]);
});
