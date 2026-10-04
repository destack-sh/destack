import { Locale, Localization } from "@destack/locale";
import { PlainDate } from "@destack/schema";
import { expect, test } from "@destack/test";
import { LocaleContext } from "@destack/locale/solid";
import type { JSX } from "@solidjs/web";
import { flush } from "solid-js";
import { Calendar, type DateRange, Day } from "./index.ts";
import { draw, press } from "@destack/view/test";

/** Read a `YYYY-MM-DD` date. */
function day(text: string): PlainDate {
    return Day.parse(text);
}

/** Render an element in a locale. */
function drawIn(tag: string, element: () => JSX.Element): HTMLElement {
    return draw(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>{element()}</LocaleContext>
    ));
}

/** Read the column headers of a calendar's grid. */
function weekdays(container: Element): string[] {
    return [...container.querySelectorAll("th")].map((header) => header.textContent ?? "");
}

/** Read the key of the focused day. */
function focusedDay(): string | undefined {
    return document.activeElement?.getAttribute("data-day") ?? undefined;
}

/** Click a day's button by its key. */
function click(container: Element, key: string): void {
    container.querySelector<HTMLElement>(`[data-day="${key}"]`)?.click();
    flush();
}

/** Read each selected day's key with its place in a range. */
function selected(container: Element): string[] {
    return [...container.querySelectorAll("[aria-selected=true] button")].map(
        (button) =>
            `${button.getAttribute("data-day")} ${button.getAttribute("data-range") ?? "selected"}`,
    );
}

test("start weeks on the locale's first day and name the month in its language", () => {
    const american = drawIn("en-US", () => (
        <Calendar defaultValue={day("2026-10-04")} today={day("2026-10-04")} />
    ));
    const austrian = drawIn("de-AT", () => (
        <Calendar defaultValue={day("2026-10-04")} today={day("2026-10-04")} />
    ));
    expect([
        american.querySelector("h2")?.textContent,
        weekdays(american),
        austrian.querySelector("h2")?.textContent,
        weekdays(austrian),
    ]).toEqual([
        "October 2026",
        ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"],
        "Oktober 2026",
        ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"],
    ]);
});

test("mark today and the selected day, naming each day by its full date", () => {
    const container = drawIn("en-US", () => (
        <Calendar defaultValue={day("2026-10-12")} today={day("2026-10-04")} />
    ));
    const cell = (key: string) => container.querySelector(`[data-day="${key}"]`);
    expect([
        cell("2026-10-04")?.getAttribute("aria-current"),
        selected(container),
        cell("2026-10-12")?.getAttribute("aria-label"),
        cell("2026-10-12")?.getAttribute("tabindex"),
        cell("2026-09-27")?.getAttribute("data-outside"),
    ]).toEqual(["date", ["2026-10-12 selected"], "Monday, October 12, 2026", "0", "true"]);
});

test("move the focused day with arrows, Home, End, Page Up and Page Down, crossing months", () => {
    const container = drawIn("de-AT", () => (
        <Calendar defaultValue={day("2026-10-30")} today={day("2026-10-04")} />
    ));
    container.querySelector<HTMLElement>("[tabindex='0']")?.focus();
    const visited = [
        "ArrowRight",
        "ArrowRight",
        "ArrowDown",
        "Home",
        "End",
        "PageUp",
        "ArrowUp",
        "ArrowLeft",
    ].map((key) => {
        press(key);
        flush();

        return focusedDay();
    });

    // weeks run Monday to Sunday in Austria, and Page Up keeps the day of month where it can
    expect(visited).toEqual([
        "2026-10-31",
        "2026-11-01",
        "2026-11-08",
        "2026-11-02",
        "2026-11-08",
        "2026-10-08",
        "2026-10-01",
        "2026-09-30",
    ]);
    expect(container.querySelector("h2")?.textContent).toBe("September 2026");
});

test("select a day on click, ignoring disabled days, and report it", () => {
    const changes: PlainDate[] = [];
    const container = drawIn("en-US", () => (
        <Calendar
            defaultMonth={day("2026-10-01")}
            isDisabled={(entry) => Day.key(entry) === "2026-10-02"}
            onValueChange={(entry) => changes.push(entry)}
        />
    ));
    click(container, "2026-10-02");
    click(container, "2026-10-05");
    expect([changes.map((entry) => Day.key(entry)), selected(container)]).toEqual([
        ["2026-10-05"],
        ["2026-10-05 selected"],
    ]);
});

test("select a range from its start to a later end, marking the start, middle and end", () => {
    const changes: DateRange[] = [];
    const container = drawIn("en-US", () => (
        <Calendar
            mode="range"
            defaultMonth={day("2026-10-01")}
            onValueChange={(range) => changes.push(range)}
        />
    ));
    click(container, "2026-10-05");
    const open = selected(container);
    click(container, "2026-10-08");
    const closed = selected(container);
    click(container, "2026-10-03");
    expect([
        open,
        closed,
        selected(container),
        changes.map(
            (range) => `${Day.key(range.from)}..${range.to === undefined ? "" : Day.key(range.to)}`,
        ),
    ]).toEqual([
        ["2026-10-05 start"],
        ["2026-10-05 start", "2026-10-06 middle", "2026-10-07 middle", "2026-10-08 end"],
        ["2026-10-03 start"],
        ["2026-10-05..", "2026-10-05..2026-10-08", "2026-10-03.."],
    ]);
});

test("toggle several days in multiple mode", () => {
    const changes: string[][] = [];
    const container = drawIn("en-US", () => (
        <Calendar
            mode="multiple"
            defaultMonth={day("2026-10-01")}
            onValueChange={(days) => changes.push(days.map((entry) => Day.key(entry)))}
        />
    ));
    click(container, "2026-10-05");
    click(container, "2026-10-09");
    click(container, "2026-10-05");
    expect([changes, selected(container)]).toEqual([
        [["2026-10-05"], ["2026-10-05", "2026-10-09"], ["2026-10-09"]],
        ["2026-10-09 selected"],
    ]);
});

test("count days and months across year ends without time zones", () => {
    expect([
        Day.key(Day.of(2026, 12, 32)),
        Day.key(Day.addMonths(day("2026-01-31"), 1)),
        Day.key(Day.addMonths(day("2024-03-31"), -1)),
        Day.weeks(day("2021-02-01"), 1).length,
        PlainDate.compare(day("2026-10-04"), day("2026-10-05")) < 0,
    ]).toEqual(["2027-01-01", "2026-02-28", "2024-02-29", 4, true]);
});
