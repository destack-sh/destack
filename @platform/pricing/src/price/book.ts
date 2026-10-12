import { CREDITS, type DecimalAmount } from "@destack/finance";

/** The currencies Destack bills in: North America's and Europe's. */
export const CURRENCIES = ["USD", "CAD", "EUR", "CHF", "GBP"] as const;

/** A currency Destack bills in. */
export type BillingCurrency = (typeof CURRENCIES)[number];

/** A credit's price in each currency Destack bills, in minor units per minor unit of credits. */
export const CREDIT_PRICES = {
    USD: "1",
    CAD: "1.4",
    EUR: "1",
    CHF: "1",
    GBP: "1",
} as const satisfies Readonly<Record<BillingCurrency, DecimalAmount>>;

/** A monthly fee in minor units of each currency Destack bills. */
export type Fee = Readonly<Record<BillingCurrency, number>>;

/** Monthly fees, the licensed prices they set, and the conversion list prices compare at. */
export const Fee = {
    /** Read the US cents a minor unit of a list price's currency is worth. */
    conversion(currency: string): DecimalAmount {
        // convert credits at the dollar's credit price
        if (currency === CREDITS) {
            return CREDIT_PRICES.USD;
        }
        // keep US cents at par
        else if (currency === "USD") {
            return "1";
        }
        // refuse any other currency
        else {
            throw new TypeError(`currency ${currency} converts to no US cents`);
        }
    },

    /** List a fee's licensed monthly prices by currency code, each including as much usage. */
    prices(fee: Fee) {
        return Object.fromEntries(
            CURRENCIES.map((currency) => [
                currency.toLowerCase(),
                {
                    currency,
                    unitAmount: fee[currency],
                    includedUsage: fee[currency],
                    recurring: {
                        interval: "month" as const,
                        intervalCount: 1,
                        usage: "licensed" as const,
                    },
                },
            ]),
        );
    },
};
