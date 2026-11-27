import { type Message, t } from "@destack/locale";
import { ValueGroup } from "./group.tsx";
import { DateRange, Duration, type PlainDate, RecurrenceRule, type Weekday } from "@destack/schema";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { For, type JSX, useLocale } from "@destack/view";
import { DatePicker } from "../date-picker/index.ts";
import { NumberField } from "../number-field/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import type { NamedValueField } from "./value-field.tsx";

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

/** The units of a duration an input shows, with what each is called, as estimates count time. */
const DURATION_UNITS = [
    ["days", t.context("label")`Days`],
    ["hours", t.context("label")`Hours`],
    ["minutes", t.context("label")`Minutes`],
] as const;

/** The rule an input starts at, every week. */
const WEEKLY: RecurrenceRule = { frequency: "weekly" };

/** The styles of the inputs of times. */
const styles = style.create({
    row: {
        display: "flex",
        flexWrap: "wrap",
        alignItems: "center",
        gap: space[2],
    },
    unit: {
        width: "6rem",
    },
});

/** The properties of the input of a named JSON value. */
type NamedProperties = Parameters<NamedValueField>[0];

/** Render the input of a range of dates: its first and last day, typed or picked. */
export function DateRangeInput(properties: NamedProperties): JSX.Element {
    // read the range, none while it is not one
    const range = (): DateRange | undefined => DateRange.safeParse(properties.value).data;

    return (
        <DatePicker
            mode="range"
            aria-label={properties["aria-label"]}
            disabled={properties.disabled}
            selected={range()}
            onSelect={(selected) => {
                // give a range once it has its last day
                if (selected?.to !== undefined) {
                    properties.onValueChange(DateRange.of(selected.from, selected.to));
                }
            }}
        />
    );
}

/** Render the input of a span of time: its days, hours and minutes, as an estimate counts them. */
export function DurationInput(properties: NamedProperties): JSX.Element {
    // read the duration, and write back only the units it has
    const locale = useLocale();
    const duration = (): Duration => Duration.safeParse(properties.value).data ?? {};
    const change = (unit: (typeof DURATION_UNITS)[number][0], amount: number | undefined): void => {
        const { [unit]: _left, ...rest } = duration();
        properties.onValueChange(
            amount === undefined || amount === 0 ? rest : { ...rest, [unit]: amount },
        );
    };

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={styles.row}>
            <For each={DURATION_UNITS}>
                {([unit, label]) => (
                    <NumberField
                        aria-label={locale.render(label)}
                        xstyle={styles.unit}
                        min={0}
                        disabled={properties.disabled}
                        value={duration()[unit] ?? 0}
                        onValueChange={(amount) => change(unit, amount)}
                    />
                )}
            </For>
        </ValueGroup>
    );
}

/** Render the input of a repeating rule: how often, every how many periods, and on which days. */
export function RecurrenceInput(properties: NamedProperties): JSX.Element {
    // read the rule, every week while none is given
    const locale = useLocale();
    const rule = (): RecurrenceRule => RecurrenceRule.safeParse(properties.value).data ?? WEEKLY;
    const change = (next: RecurrenceRule): void => properties.onValueChange(next);

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={styles.row}>
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
                xstyle={styles.unit}
                min={1}
                disabled={properties.disabled}
                value={rule().interval ?? 1}
                onValueChange={(interval) => {
                    // keep every one period as the rule's default
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
