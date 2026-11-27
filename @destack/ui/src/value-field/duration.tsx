import minusIcon from "@destack/icon/phosphor/minus";
import plusIcon from "@destack/icon/phosphor/plus";
import { type Message, t } from "@destack/locale";
import { DURATION_UNITS, Duration, type DurationUnit } from "@destack/schema";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { For, type JSX, useLocale } from "@destack/view";
import { Button } from "../button/index.ts";
import { UnitParts } from "../dimension-field/index.ts";
import { Icon } from "../icon/index.ts";
import { ValueSet, valueStyles } from "./group.tsx";
import { type NamedValueInputProperties, readValue } from "./value-field.tsx";

/** What each unit of a duration is called. */
const UNIT_TITLES = {
    years: t.context("option")`Years`,
    months: t.context("option")`Months`,
    weeks: t.context("option")`Weeks`,
    days: t.context("option")`Days`,
    hours: t.context("option")`Hours`,
    minutes: t.context("option")`Minutes`,
    seconds: t.context("option")`Seconds`,
    milliseconds: t.context("option")`Milliseconds`,
    microseconds: t.context("option")`Microseconds`,
    nanoseconds: t.context("option")`Nanoseconds`,
} satisfies Record<DurationUnit, Message>;

/** The styles of a duration's rows: its number, its unit and its remove button in columns. */
const styles = style.create({
    menu: {
        width: "100%",
    },
    unit: {
        display: "grid",
        gridTemplateColumns: "minmax(0, 1fr) 9rem auto",
        alignItems: "center",
        gap: space[2],
    },
});

/** The unit a duration of nothing shows. */
const START_UNIT: DurationUnit = "seconds";

/** Render the input of a duration: a number and a unit for each of its units. */
export function DurationInput(properties: NamedValueInputProperties): JSX.Element {
    // read the duration, and its units largest first, one unit for a duration of nothing
    const locale = useLocale();
    const duration = (): Duration => readValue(properties.value, Duration) ?? {};
    const units = (): readonly DurationUnit[] => {
        const present = DURATION_UNITS.filter((unit) => duration()[unit] !== undefined);

        return present.length === 0 ? [START_UNIT] : present;
    };
    const give = (next: Readonly<Partial<Record<DurationUnit, number>>>): void => {
        // give back only a duration Temporal takes, its units of one sign
        const read = Duration.safeParse(next);
        if (read.success) {
            properties.onValueChange(read.data);
        }
    };
    const change = (from: DurationUnit, to: DurationUnit, amount: number): void => {
        // move the amount to the unit picked, a fraction carried into the smaller units
        const total = (to === from ? 0 : (duration()[to] ?? 0)) + amount;
        if (to === "months" && !Number.isInteger(total)) {
            return;
        }

        // add what it carries to the units the duration has besides
        const rest = Object.entries(duration()).filter(([unit]) => unit !== from && unit !== to);
        const merged = new Map<string, number>(rest);
        for (const [unit, value] of Object.entries(Duration.of(total, to))) {
            merged.set(unit, (merged.get(unit) ?? 0) + value);
        }
        give(Object.fromEntries(merged));
    };
    const next = (): DurationUnit | undefined => {
        // add the largest unit below the smallest shown, else the largest left
        const smallest = Math.max(...units().map((unit) => DURATION_UNITS.indexOf(unit)));
        const left = DURATION_UNITS.filter((unit) => !units().includes(unit));

        return left.find((unit) => DURATION_UNITS.indexOf(unit) > smallest) ?? left[0];
    };

    return (
        <ValueSet
            label={properties.label}
            aria-label={properties["aria-label"]}
            xstyle={valueStyles.group}
        >
            {/* Each unit's number, kept by its place as it changes */}
            <For each={units()} keyed={false}>
                {(unit) => (
                    <div {...style.attrs(styles.unit)}>
                        <UnitParts
                            units={DURATION_UNITS}
                            title={(each) => locale.render(UNIT_TITLES[each])}
                            unitStyle={styles.menu}
                            value={{ value: duration()[unit()] ?? 0, unit: unit() }}
                            disabled={properties.disabled === true}
                            onValueChange={(changed) => change(unit(), changed.unit, changed.value)}
                        />
                        <Button
                            variant="ghost"
                            isIconOnly
                            hidden={units().length === 1}
                            disabled={properties.disabled}
                            aria-label={locale.render(
                                t.context("action")`Remove ${locale.render(UNIT_TITLES[unit()])}`,
                            )}
                            onClick={() => {
                                const { [unit()]: _left, ...rest } = duration();
                                give(rest);
                            }}
                        >
                            <Icon icon={minusIcon} />
                        </Button>
                    </div>
                )}
            </For>

            {/* Add the next smaller unit it has none of */}
            <Button
                variant="ghost"
                disabled={properties.disabled === true || next() === undefined}
                onClick={() => {
                    const unit = next();
                    // store the units shown, a duration of nothing's too, and the one added
                    const shown = Object.fromEntries(units().map((each) => [each, 0]));
                    if (unit !== undefined) {
                        give({ ...shown, ...duration(), [unit]: 0 });
                    }
                }}
            >
                <Icon icon={plusIcon} />
                {locale.render(t.context("action")`Add unit`)}
            </Button>
        </ValueSet>
    );
}
