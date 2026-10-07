import { defineSchema, schema } from "@destack/schema";

/** The decimals of a fractional amount, the most a payment provider keeps. */
const DECIMALS = 12;

/** The fractional minor units in one minor unit at the kept decimals. */
const SCALE = 10n ** BigInt(DECIMALS);

/** A currency by its ISO 4217 code, or a custom one such as x-credits. */
export const Currency = defineSchema(
    schema.string().regex(/^(?:[A-Z]{3}|x-[a-z0-9]+(?:-[a-z0-9]+)*)$(?![\s\S])/u),
);

/** The currency of credits, converted through each seller's price book. */
export const CREDITS = "x-credits";

/** An amount in the minor unit of its currency, such as cents. */
export const Amount = defineSchema(schema.number().int().min(0));

/** An amount in the minor unit of its currency with up to 12 decimals, such as 0.000000024 cents per byte. */
export const DecimalAmount = defineSchema(
    schema.string().regex(/^(?:0|[1-9]\d*)(?:\.\d{1,12})?$(?![\s\S])/u),
);
/** An amount in fractional minor units. */
export type DecimalAmount = schema.Infer<typeof DecimalAmount>;

/** A cost in fractional minor units as FOCUS cost columns keep it, negative for a credit. */
export const Cost = Object.assign(
    defineSchema(schema.string().regex(/^-?(?:0|[1-9]\d*)(?:\.\d{1,12})?$(?![\s\S])/u)),
    {
        /** Read a cost or an amount as whole fractional minor units at 12 decimals. */
        scaled(text: string): bigint {
            // read the sign, the whole part and the fraction padded to the kept decimals
            const isNegative = text.startsWith("-");
            const [whole = "0", fraction = ""] = (isNegative ? text.slice(1) : text).split(".");
            const scaled = BigInt(whole) * SCALE + BigInt(fraction.padEnd(DECIMALS, "0"));

            return isNegative ? -scaled : scaled;
        },

        /** Write whole fractional minor units as a cost without trailing zeros. */
        of(scaled: bigint): string {
            // write the digits around the decimal point, trimming the fraction
            const sign = scaled < 0n ? "-" : "";
            const digits = (scaled < 0n ? -scaled : scaled).toString().padStart(DECIMALS + 1, "0");
            const whole = digits.slice(0, -DECIMALS);
            const fraction = digits.slice(-DECIMALS).replace(/0+$/u, "");

            return `${sign}${fraction === "" ? whole : `${whole}.${fraction}`}`;
        },

        /** Round fractional minor units up to whole ones. */
        ceiling(scaled: bigint): number {
            const whole = scaled / SCALE;

            return Number(scaled > whole * SCALE ? whole + 1n : whole);
        },

        /** Read whole minor units as fractional minor units. */
        whole(amount: number): bigint {
            return BigInt(amount) * SCALE;
        },
    },
);
/** A cost in fractional minor units, negative for a credit. */
export type Cost = schema.Infer<typeof Cost>;
