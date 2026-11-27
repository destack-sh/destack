import starIcon from "@destack/icon/phosphor/star";
import { ValueGroup } from "./group.tsx";
import { plural, t } from "@destack/locale";
import { Money, Rating } from "@destack/schema";
import * as style from "@destack/style";
import { space } from "@destack/theme/tokens.stylex";
import { For, type JSX, useLocale } from "@destack/view";
import { Icon } from "../icon/index.ts";
import { NumberField } from "../number-field/index.ts";
import { Select, SelectItem } from "../select/index.ts";
import { ToggleGroup, ToggleGroupItem } from "../toggle-group/index.ts";
import type { NamedValueField } from "./value-field.tsx";

/** The currencies a money input offers beside its own, the most traded ones. */
const CURRENCIES = ["EUR", "USD", "GBP", "CHF", "JPY"] as const;

/** The money an input starts at, none in euros. */
const NO_MONEY: Money = { amount: 0, currency: "EUR" };

/** The most stars a rating shows where its schema sets no most. */
const STARS = 5;

/** The styles of the inputs of amounts. */
const styles = style.create({
    row: {
        display: "flex",
        alignItems: "center",
        gap: space[2],
    },
    amount: {
        flex: 1,
        minWidth: "6rem",
    },
});

/** The properties of the input of a named JSON value. */
type NamedProperties = Parameters<NamedValueField>[0];

/** Render the input of money: its amount in the currency's major unit, and its currency. */
export function MoneyInput(properties: NamedProperties): JSX.Element {
    // read the money, and the digits its currency writes past the major unit
    const locale = useLocale();
    const money = (): Money => Money.safeParse(properties.value).data ?? NO_MONEY;
    const scale = (): number => 10 ** digitsOf(money().currency);
    const currencies = (): readonly string[] =>
        CURRENCIES.some((each) => each === money().currency)
            ? CURRENCIES
            : [...CURRENCIES, money().currency];
    const isCustom = (): boolean => money().currency.startsWith("x-");

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={styles.row}>
            {/* Amount */}
            <NumberField
                aria-label={locale.render(t.context("label")`Amount`)}
                xstyle={styles.amount}
                disabled={properties.disabled}
                format={isCustom() ? {} : { style: "currency", currency: money().currency }}
                value={money().amount / scale()}
                onValueChange={(amount) =>
                    amount !== undefined &&
                    properties.onValueChange({ ...money(), amount: Math.round(amount * scale()) })
                }
            />

            {/* Currency */}
            <Select
                aria-label={locale.render(t.context("label")`Currency`)}
                disabled={properties.disabled}
                value={money().currency}
                onValueChange={(currency) =>
                    currency !== undefined && properties.onValueChange({ ...money(), currency })
                }
            >
                <For each={currencies()}>
                    {(code) => <SelectItem value={code}>{code}</SelectItem>}
                </For>
            </Select>
        </ValueGroup>
    );
}

/** Render the input of a rating: a row of stars, pressing the one given again clearing it. */
export function RatingInput(properties: NamedProperties): JSX.Element {
    // read the rating and the most stars its schema allows
    const locale = useLocale();
    const rating = (): number => Rating.safeParse(properties.value).data ?? 0;
    const most = (): number => {
        const maximum = properties.type.schema["maximum"];

        return typeof maximum === "number" ? maximum : STARS;
    };
    const stars = (): readonly number[] => Array.from({ length: most() }, (_, index) => index + 1);

    return (
        <ToggleGroup
            size="sm"
            aria-label={properties["aria-label"]}
            disabled={properties.disabled}
            value={rating() === 0 ? undefined : String(rating())}
            onValueChange={(key) => properties.onValueChange(key === undefined ? 0 : Number(key))}
        >
            <For each={stars()}>
                {(star) => (
                    <ToggleGroupItem
                        value={String(star)}
                        aria-label={locale.render(
                            t.context(
                                "option",
                            )`${plural(star, { one: "# star", other: "# stars" })}`,
                        )}
                    >
                        <Icon icon={starIcon} weight={star <= rating() ? "fill" : "regular"} />
                    </ToggleGroupItem>
                )}
            </For>
        </ToggleGroup>
    );
}

/** Read the digits a currency writes past its major unit, none for a custom one. */
function digitsOf(currency: string): number {
    return currency.startsWith("x-")
        ? 0
        : (new Intl.NumberFormat("en", { style: "currency", currency }).resolvedOptions()
              .maximumFractionDigits ?? 2);
}
