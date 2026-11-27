import { type Message, t } from "@destack/locale";
import { type PlainDate, RecurrenceRule, type Weekday } from "@destack/schema";
import * as style from "@destack/style";
import { For, type JSX, useLocale } from "@destack/view";
import { NumberField } from "../number-field/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import { ValueGroup, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** A week from Monday to Sunday, whose dates name the weekdays in the person's language. */
const WEEK: readonly (readonly [Weekday, PlainDate])[] = [
    [1, "2026-11-30"],
    [2, "2026-12-01"],
    [3, "2026-12-02"],
    [4, "2026-12-03"],
    [5, "2026-12-04"],
    [6, "2026-12-05"],
    [7, "2026-12-06"],
];

/** How often a rule repeats, titled as calendars offer it. */
const FREQUENCIES = {
    daily: t.context("option")`Daily`,
    weekly: t.context("option")`Weekly`,
    monthly: t.context("option")`Monthly`,
    yearly: t.context("option")`Yearly`,
} satisfies Record<RecurrenceRule["frequency"], Message>;

/** The rule an input starts at, every week. */
const WEEKLY: RecurrenceRule = { frequency: "weekly" };

/** The styles of a repeating rule's input. */
const styles = style.create({
    every: {
        width: "6rem",
    },
});

/** Render the input of a repeating rule: how often, every how many periods, and on which days. */
export function RecurrenceInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the rule, every week while none is given
    const locale = useLocale();
    const rule = (): RecurrenceRule => RecurrenceRule.safeParse(properties.value).data ?? WEEKLY;
    const change = (next: RecurrenceRule): void => properties.onValueChange(next);

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.row}>
            {/* How often, and every how many */}
            <Select
                aria-label={locale.render(t.context("label")`Repeat`)}
                disabled={properties.disabled}
                value={rule().frequency}
                onValueChange={(key) => {
                    // repeat at the new frequency, keeping the interval and dropping the days
                    const frequency = RecurrenceRule.shape.frequency.safeParse(key).data;
                    if (frequency !== undefined) {
                        change({
                            frequency,
                            ...(rule().interval === undefined ? {} : { interval: rule().interval }),
                        });
                    }
                }}
            >
                <For each={Object.entries(FREQUENCIES)}>
                    {([key, title]) => <SelectItem value={key}>{locale.render(title)}</SelectItem>}
                </For>
            </Select>
            <NumberField
                aria-label={locale.render(t.context("label")`Every`)}
                xstyle={styles.every}
                min={1}
                disabled={properties.disabled}
                value={rule().interval ?? 1}
                onValueChange={(interval) => {
                    // leave out an interval of one, the rule's default
                    const { interval: _left, ...rest } = rule();
                    change(interval === undefined || interval === 1 ? rest : { ...rest, interval });
                }}
            />

            {/* Weekdays of a weekly rule */}
            <ToggleGroup
                multiple
                size="sm"
                hidden={rule().frequency !== "weekly"}
                aria-label={locale.render(t.context("label")`On`)}
                disabled={properties.disabled}
                value={(rule().byDay ?? []).map((each) => String(each.day))}
                onValueChange={(keys) => {
                    // repeat on the days pressed, the first date's weekday once none is
                    const days = WEEK.filter(([day]) => [keys].flat().includes(String(day)));
                    const { byDay: _left, ...rest } = rule();
                    change(
                        days.length === 0
                            ? rest
                            : { ...rest, byDay: days.map(([day]) => ({ day })) },
                    );
                }}
            >
                <For each={WEEK}>
                    {([day, date]) => (
                        <ToggleGroupItem
                            value={String(day)}
                            aria-label={locale.plainDate(date, { weekday: "long" })}
                        >
                            {locale.plainDate(date, { weekday: "short" })}
                        </ToggleGroupItem>
                    )}
                </For>
            </ToggleGroup>
        </ValueGroup>
    );
}
