import { Locale, Localization } from "@destack/locale";
import { expect, test } from "@destack/test";
import { flush, LocaleContext } from "@destack/view";
import { render, testWorld } from "@destack/view/test";
import { RelativeTime, type RelativeTimeProperties } from "./index.ts";

/** A minute, in milliseconds. */
const MINUTE = 60_000;

/** Read how a moment some minutes from now is written. */
function written(
    minutes: number,
    properties: Omit<RelativeTimeProperties, "value"> = {},
    tag = "en",
): string | null {
    const value = testWorld().tab.clock.now() + minutes * MINUTE;
    const { container } = render(() => (
        <LocaleContext value={Localization.of(Locale.parse(tag), [])}>
            <RelativeTime {...properties} value={value} timeZone="UTC" />
        </LocaleContext>
    ));

    return container.textContent;
}

test("write a moment by its distance, and its date past the threshold", () => {
    const date = new Date(testWorld().tab.clock.now() - 90 * 24 * 60 * MINUTE);
    const medium = new Intl.DateTimeFormat("en", { dateStyle: "medium", timeZone: "UTC" });

    expect([
        written(-0.5),
        written(-0.5, { precision: "second" }),
        written(-5),
        written(-5, { length: "narrow" }),
        written(60 * 3),
        written(-60 * 24, {}, "de"),
        written(-60 * 24 * 90),
        written(-60 * 24 * 90, { threshold: { days: 365 } }),
        written(-5, { children: (words) => <b>{`seen ${words}`}</b> }),
    ]).toEqual([
        "now",
        "30 seconds ago",
        "5 minutes ago",
        "5m ago",
        "in 3 hours",
        "gestern",
        medium.format(date),
        "3 months ago",
        "seen 5 minutes ago",
    ]);
});

test("move on with the clock", async () => {
    const { simulation, tab } = testWorld();
    const value = tab.clock.now() - 5 * MINUTE;
    const { container } = render(() => <RelativeTime value={value} timeZone="UTC" />);
    const before = container.textContent;
    await simulation.jump(tab.clock.now() + 3 * MINUTE);
    flush();

    expect([before, container.textContent, container.querySelector("time")?.dateTime]).toEqual([
        "5 minutes ago",
        "8 minutes ago",
        new Date(value).toISOString(),
    ]);
});
