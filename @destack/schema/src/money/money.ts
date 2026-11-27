import { present } from "../presence/index.ts";
import * as schema from "../validate/index.ts";

/** The prefix of a custom currency's code, as private use codes start in BCP 47. */
const CUSTOM = "x-";

/** A currency by its ISO 4217 code, or a custom one such as `x-credits`. */
const currencySchema = schema.string().regex(/^(?:[A-Z]{3}|x-[a-z0-9]+(?:-[a-z0-9]+)*)$/u, {
    message: "a currency is an ISO 4217 code such as EUR, or a custom one such as x-credits",
});
/** A currency's code. */
export type Currency = schema.Infer<typeof currencySchema>;

/** A currency by its ISO 4217 code, or a custom one, and the digits it writes. */
export const Currency = Object.assign(currencySchema, {
    /** Tell whether a currency is a custom one, which ISO 4217 and Intl do not know. */
    isCustom(currency: Currency): boolean {
        return currency.startsWith(CUSTOM);
    },

    /** Read the digits a currency writes past its major unit, none for a custom one. */
    digits(currency: Currency): number {
        // ask Intl for an ISO currency's digits, which no locale changes
        if (Currency.isCustom(currency)) {
            return 0;
        }
        const format = new Intl.NumberFormat("en", { style: "currency", currency });

        return present(format.resolvedOptions().maximumFractionDigits, "a currency's digits");
    },
});

/** An amount of money in a currency, in its minor unit such as cents, as Stripe counts it. */
export const Money = schema
    .object({
        /** The amount in the currency's minor unit, such as 1250 for 12.50 euros. */
        amount: schema.number().int(),
        /** The currency. */
        currency: Currency,
    })
    .meta({ id: "schema.money", title: "Money" });
/** An amount of money in a currency. */
export type Money = schema.Infer<typeof Money>;
