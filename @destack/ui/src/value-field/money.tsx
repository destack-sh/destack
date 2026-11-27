import { t } from "@destack/locale";
import { Currency, Money, present } from "@destack/schema";
import * as style from "@destack/style";
import { For, type JSX, useLocale } from "@destack/view";
import { NumberField } from "../number-field/index.ts";
import {
    Combobox,
    ComboboxContent,
    ComboboxEmpty,
    ComboboxInput,
    ComboboxItem,
} from "../combobox/index.ts";
import { ValueGroup, valueStyles } from "./group.tsx";
import type { NamedValueFieldProperties } from "./value-field.tsx";

/** The currencies of ISO 4217 the platform knows, as Intl lists them. */
const CURRENCIES: readonly string[] = Intl.supportedValuesOf("currency");

/** The money an input starts at, none in euros. */
const NO_MONEY: Money = { amount: 0, currency: "EUR" };

/** The styles of a money input. */
const styles = style.create({
    amount: {
        flex: 1,
        minWidth: "6rem",
    },
    currency: {
        width: "12rem",
    },
});

/** Render the input of money: its amount in the currency's major unit, and its currency. */
export function MoneyInput(properties: NamedValueFieldProperties): JSX.Element {
    // read the money, and the digits its currency writes past the major unit
    const locale = useLocale();
    const money = (): Money => Money.safeParse(properties.value).data ?? NO_MONEY;
    const scale = (): number => 10 ** Currency.digits(money().currency);
    const names = new Intl.DisplayNames([locale.tag], { type: "currency" });
    const titleOf = (code: string): string =>
        Currency.isCustom(code)
            ? code
            : `${present(names.of(code), "a currency's name")} (${code})`;
    const currencies = (): readonly string[] =>
        CURRENCIES.includes(money().currency) ? CURRENCIES : [...CURRENCIES, money().currency];

    return (
        <ValueGroup aria-label={properties["aria-label"]} xstyle={valueStyles.row}>
            {/* Amount */}
            <NumberField
                aria-label={locale.render(t.context("label")`Amount`)}
                xstyle={styles.amount}
                disabled={properties.disabled}
                format={
                    Currency.isCustom(money().currency)
                        ? {}
                        : { style: "currency", currency: money().currency }
                }
                value={money().amount / scale()}
                onValueChange={(amount) =>
                    amount !== undefined &&
                    properties.onValueChange({ ...money(), amount: Math.round(amount * scale()) })
                }
            />

            {/* Currency, found by its code or name */}
            <Combobox
                value={money().currency}
                onValueChange={(key: string | undefined) => {
                    const currency = Currency.safeParse(key).data;
                    if (currency !== undefined) {
                        properties.onValueChange({ ...money(), currency });
                    }
                }}
            >
                <ComboboxInput
                    aria-label={locale.render(t.context("label")`Currency`)}
                    disabled={properties.disabled}
                    xstyle={styles.currency}
                />
                <ComboboxContent>
                    <ComboboxEmpty>{locale.render(t.context("title")`No results.`)}</ComboboxEmpty>
                    <For each={currencies()}>
                        {(code) => (
                            <ComboboxItem value={code} textValue={`${titleOf(code)} ${code}`}>
                                {titleOf(code)}
                            </ComboboxItem>
                        )}
                    </For>
                </ComboboxContent>
            </Combobox>
        </ValueGroup>
    );
}
