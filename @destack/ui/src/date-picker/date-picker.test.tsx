import { Locale, Localization } from "@destack/locale";
import { LocaleContext } from "@destack/locale/solid";
import type { PlainDate } from "@destack/schema";
import { expect, test } from "@destack/test";
import { draw, stubPopovers } from "@destack/view/test";
import type { JSX } from "@solidjs/web";
import { flush } from "solid-js";
import { Day } from "../calendar/index.ts";
import { DatePicker } from "./index.ts";

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

/** Click a day's button by its key. */
function click(container: Element, key: string): void {
    container.querySelector<HTMLElement>(`[data-day="${key}"]`)?.click();
    flush();
}

/** Read an element's accessible name from the elements its `aria-labelledby` names, in order. */
function labelledName(element: Element | null | undefined): string {
    return (element?.getAttribute("aria-labelledby") ?? "")
        .split(" ")
        .map((id) => document.getElementById(id)?.textContent)
        .join(" ");
}

test("show the chosen day in the locale's medium date after its name and close the date picker's popover", () => {
    stubPopovers();
    const changes: PlainDate[] = [];
    const container = drawIn("en-US", () => (
        <DatePicker aria-label="Due date" onValueChange={(entry) => changes.push(entry)} />
    ));
    const trigger = container.querySelector("[data-slot=date-picker-trigger]");
    const placeholder = labelledName(trigger);
    const content = container.querySelector<HTMLElement>("[data-slot=date-picker-content]");
    content?.setAttribute("data-popover-open", "date-picker-trigger");
    container.querySelector<HTMLElement>("[data-day]:not([data-outside])")?.click();
    flush();
    const chosen = changes[0];
    expect([
        placeholder,
        trigger?.getAttribute("popovertarget") === content?.id,
        content?.getAttribute("aria-labelledby") === trigger?.id,
        changes.length,
        content?.hasAttribute("data-popover-open"),
    ]).toEqual(["Due date Pick a date", true, true, 1, false]);

    // the trigger keeps the chosen day after its name
    expect(labelledName(trigger)).toBe(
        chosen === undefined
            ? ""
            : `Due date ${new Intl.DateTimeFormat("en-US", { dateStyle: "medium", timeZone: "UTC" }).format(Day.time(chosen))}`,
    );
});

test("keep a range date picker open until the range closes, then show the span", () => {
    stubPopovers();
    const container = drawIn("en-US", () => (
        <DatePicker mode="range" defaultValue={{ from: day("2026-10-05") }} aria-label="Trip" />
    ));
    const content = container.querySelector<HTMLElement>("[data-slot=date-picker-content]");
    content?.setAttribute("data-popover-open", "date-picker-trigger");
    click(container, "2026-10-04");
    const reopened = content?.hasAttribute("data-popover-open");
    click(container, "2026-10-09");
    expect([
        reopened,
        content?.hasAttribute("data-popover-open"),
        labelledName(container.querySelector("[data-slot=date-picker-trigger]")),
    ]).toEqual([true, false, "Trip Oct 4 – 9, 2026"]);
});
