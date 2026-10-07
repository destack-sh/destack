import { Locale, Localization } from "@destack/locale";
import type { PlainTime } from "@destack/schema";
import { expect, test } from "@destack/test";
import { flush, type JSX, LocaleContext } from "@destack/view";
import { draw, press } from "@destack/view/test";
import { TimeField } from "./index.ts";

/** Render an element in a locale. */
function drawIn(tag: string, element: () => JSX.Element): HTMLElement {
    return draw(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>{element()}</LocaleContext>
    ));
}

/** Press keys on the focused element in turn. */
function type(...keys: string[]): void {
    for (const key of keys) {
        press(key);
        flush();
    }
}

/** List the parts and names of a container's spinbuttons, in order. */
function segments(container: Element): (string | null)[][] {
    return [...container.querySelectorAll("[role=spinbutton]")].map((segment) => [
        segment.getAttribute("data-segment"),
        segment.getAttribute("aria-label"),
        segment.getAttribute("aria-valuemin"),
        segment.getAttribute("aria-valuemax"),
    ]);
}

test("type a time on a 12-hour clock, choosing the day period by its letter", () => {
    const changes: (PlainTime | undefined)[] = [];
    const container = drawIn("en-US", () => (
        <TimeField aria-label="Starts" onValueChange={(value) => changes.push(value)} />
    ));
    container.querySelector<HTMLElement>("[data-segment=hour]")?.focus();
    type("9", "3", "0", "p");
    expect([segments(container), changes, container.textContent]).toEqual([
        [
            ["hour", "Hour", "1", "12"],
            ["minute", "Minute", "0", "59"],
            ["dayPeriod", "AM/PM", "0", "1"],
        ],
        ["21:30"],
        "09:30 PM",
    ]);
});

test("type a time on a 24-hour clock without a day period, and wrap a stepped minute", () => {
    const changes: (PlainTime | undefined)[] = [];
    const container = drawIn("de-AT", () => (
        <TimeField aria-label="Beginn" onValueChange={(value) => changes.push(value)} />
    ));
    container.querySelector<HTMLElement>("[data-segment=hour]")?.focus();
    type("2", "3", "5", "9", "ArrowUp", "Backspace");
    expect([segments(container), changes, container.textContent]).toEqual([
        [
            ["hour", "Hour", "0", "23"],
            ["minute", "Minute", "0", "59"],
        ],
        ["23:59", "23:00", undefined],
        "23:––",
    ]);
});

test("show a controlled time set from outside in its segments", () => {
    const container = drawIn("en-US", () => <TimeField aria-label="Starts" value="00:05" />);
    expect(container.textContent).toBe("12:05 AM");
});
