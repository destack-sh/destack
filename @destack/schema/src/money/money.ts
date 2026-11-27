import * as schema from "../validate/index.ts";

/** A currency by its ISO 4217 code, or a custom one such as `x-credits`. */
export const Currency = schema.string().regex(/^(?:[A-Z]{3}|x-[a-z0-9]+(?:-[a-z0-9]+)*)$/u, {
    message: "a currency is an ISO 4217 code such as EUR, or a custom one such as x-credits",
});
/** A currency's code. */
export type Currency = schema.Infer<typeof Currency>;

/** An amount of money in a currency, in its minor unit such as cents, as Stripe counts it. */
export const Money = schema
    .object({
        /** The amount in the currency's minor unit, such as 1250 for 12.50 euros. */
        amount: schema.number().int(),
        /** The currency. */
        currency: Currency,
    })
    .meta({ id: "money", title: "Money" });
/** An amount of money in a currency. */
export type Money = schema.Infer<typeof Money>;
