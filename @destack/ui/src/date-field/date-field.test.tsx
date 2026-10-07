import { Locale, Localization } from "@destack/locale";
import type { PlainDate } from "@destack/schema";
import { expect, test } from "@destack/test";
import { flush, type JSX, LocaleContext } from "@destack/view";
import { draw, markup, press } from "@destack/view/test";
import { Field, FieldDescription, FieldLabel } from "../field/index.ts";
import { DateField } from "./index.ts";

/** Render an element in a locale. */
function drawIn(tag: string, element: () => JSX.Element): HTMLElement {
    return draw(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>{element()}</LocaleContext>
    ));
}

/** Focus the segment of a part in a container. */
function focus(container: Element, kind: string): void {
    container.querySelector<HTMLElement>(`[data-segment=${kind}]`)?.focus();
}

/** Press keys on the focused element in turn. */
function type(...keys: string[]): void {
    for (const key of keys) {
        press(key);
        flush();
    }
}

/** Read the part the focused segment edits. */
function focusedSegment(): string | undefined {
    return document.activeElement instanceof HTMLElement
        ? document.activeElement.dataset["segment"]
        : undefined;
}

/** The markup of an empty segment of a part, named and bounded. */
function emptySegment(kind: string, name: string, max: number, placeholder: string): string {
    return (
        `<span role="spinbutton" tabindex="0" contenteditable="true" inputmode="numeric" enterkeyhint="next" aria-label="${name}" aria-valuemin="1" aria-valuemax="${String(max)}" aria-valuetext="Empty" ` +
        `data-slot="segment" data-segment="${kind}" data-placeholder="">${placeholder}</span>`
    );
}

/** The markup of a literal between segments. */
function literal(text: string): string {
    return `<span aria-hidden="true" data-slot="segment-literal">${text}</span>`;
}

test("lay out an empty date as a group of spinbuttons in the locale's order, the literals hidden", () => {
    const american = drawIn("en-US", () => <DateField aria-label="Due" />);
    const austrian = drawIn("de-AT", () => <DateField aria-label="Due" />);
    expect([markup(american), markup(austrian)]).toEqual([
        '<div role="group" data-slot="date-field" aria-label="Due">' +
            emptySegment("month", "Month", 12, "––") +
            literal("/") +
            emptySegment("day", "Day", 31, "––") +
            literal("/") +
            emptySegment("year", "Year", 9999, "––––") +
            "</div>",
        '<div role="group" data-slot="date-field" aria-label="Due">' +
            emptySegment("day", "Day", 31, "––") +
            literal(".") +
            emptySegment("month", "Month", 12, "––") +
            literal(".") +
            emptySegment("year", "Year", 9999, "––––") +
            "</div>",
    ]);
});

test("type a date digit by digit, moving on once a segment takes no more digits, and report it once complete", () => {
    const changes: (PlainDate | undefined)[] = [];
    const container = drawIn("en-US", () => (
        <DateField aria-label="Due" onValueChange={(value) => changes.push(value)} />
    ));
    focus(container, "month");
    type("1");
    const afterOne = focusedSegment();
    type("0", "7", "2", "0", "2");
    const beforeLast = [...changes];
    type("6");
    const month = container.querySelector("[data-segment=month]");
    expect([
        afterOne,
        beforeLast,
        changes,
        focusedSegment(),
        month?.getAttribute("aria-valuenow"),
        month?.getAttribute("aria-valuetext"),
        container.textContent,
    ]).toEqual([
        "month",
        [],
        [{ year: 2026, month: 10, day: 7 }],
        "year",
        "10",
        "October",
        "10/07/2026",
    ]);
});

test("step segments with the arrow keys, wrapping, and report undefined for an impossible date", () => {
    const changes: (PlainDate | undefined)[] = [];
    const container = drawIn("en-US", () => (
        <DateField
            aria-label="Due"
            defaultValue={{ year: 2026, month: 12, day: 31 }}
            onValueChange={(value) => changes.push(value)}
        />
    ));
    focus(container, "month");
    type("ArrowUp", "ArrowDown", "ArrowDown");
    const impossible = container.textContent;
    focus(container, "day");
    type("Backspace");
    expect([changes, impossible, container.textContent]).toEqual([
        [{ year: 2026, month: 1, day: 31 }, { year: 2026, month: 12, day: 31 }, undefined],
        "11/31/2026",
        "11/––/2026",
    ]);
});

test("move between segments along the writing direction, right to left in Hebrew", () => {
    const container = drawIn("he-IL", () => <DateField aria-label="Due" />);
    const order = [...container.querySelectorAll<HTMLElement>("[role=spinbutton]")].map(
        (segment) => segment.dataset["segment"],
    );
    focus(container, "day");
    type("ArrowLeft");
    const left = focusedSegment();
    type("ArrowLeft", "ArrowRight", "ArrowRight");
    expect([order, left, focusedSegment()]).toEqual([["day", "month", "year"], "month", "day"]);
});

test("name the group by its field's label and describe it by the field's description", () => {
    const container = drawIn("en-US", () => (
        <Field invalid>
            <FieldLabel>Due</FieldLabel>
            <DateField />
            <FieldDescription>The day the task is due.</FieldDescription>
        </Field>
    ));
    const label = container.querySelector("[data-slot=field-label]");
    const group = container.querySelector("[role=group][data-slot=date-field]");
    const description = container.querySelector("[data-slot=field-description]");
    expect([
        group?.getAttribute("aria-labelledby") === label?.id,
        label?.hasAttribute("for"),
        group?.getAttribute("aria-describedby") === description?.id,
        group?.getAttribute("aria-invalid"),
    ]).toEqual([true, false, true, "true"]);
});

test("finish a segment typed short of its digits once the focus leaves it", () => {
    const changes: (PlainDate | undefined)[] = [];
    const container = drawIn("en-US", () => (
        <DateField
            aria-label="Due"
            defaultValue={{ year: 2026, month: 10, day: 7 }}
            onValueChange={(value) => changes.push(value)}
        />
    ));
    focus(container, "month");
    type("1");
    const typing = [...changes];
    focus(container, "day");
    flush();
    expect([typing, changes]).toEqual([[], [{ year: 2026, month: 1, day: 7 }]]);
});
