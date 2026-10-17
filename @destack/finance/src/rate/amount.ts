import { defineSchema, schema } from "@destack/schema";

/** The decimals of a fractional amount, the most a payment provider stores. */
const DECIMALS = 12;

/** The fractional minor units in one minor unit at those decimals. */
const SCALE = 10n ** BigInt(DECIMALS);

/** A currency by its ISO 4217 code, or a custom one such as x-credits. */
export const Currency = defineSchema(
    schema.string().regex(/^(?:[A-Z]{3}|x-[a-z0-9]+(?:-[a-z0-9]+)*)$/u),
);

/** The currency of credits, converted through each seller's price book. */
export const CREDITS = "x-credits";

/** An amount in the minor unit of its currency, such as cents. */
export const Amount = defineSchema(schema.number().int().min(0));

/** An amount in minor units of its currency with up to 12 decimals, such as 0.000000024 cents. */
export const DecimalAmount = defineSchema(
    schema.string().regex(/^(?:0|[1-9]\d*)(?:\.\d{1,12})?$/u),
);
/** An amount in fractional minor units. */
export type DecimalAmount = schema.Infer<typeof DecimalAmount>;

/** A cost in fractional minor units as FOCUS cost columns state it, negative for a credit. */
export const Cost = Object.assign(
    defineSchema(schema.string().regex(/^-?(?:0|[1-9]\d*)(?:\.\d{1,12})?$/u)),
    {
        /** Read a cost or an amount as whole fractional minor units at 12 decimals. */
        scaled(text: string): bigint {
            // read the sign, the whole part and the fraction padded to the stored decimals
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

        /** Round costs to whole minor units adding up to their total rounded once, half up. */
        apportion(costs: readonly bigint[]): number[] {
            // round the total once, and take each cost's whole part
            const total = costs.reduce((sum, cost) => sum + cost, 0n);
            const rounded = (total + SCALE / 2n) / SCALE;
            const wholes = costs.map((cost) => cost / SCALE);
            const remaining = Number(rounded - wholes.reduce((sum, whole) => sum + whole, 0n));

            // give the minor units left to the largest remainders, the earlier cost first on a tie
            const ranked = new Set(
                costs
                    .map((cost, position) => ({ position, remainder: cost % SCALE }))
                    .toSorted((left, right) => {
                        // rank the larger remainder first, and the earlier cost on a tie
                        if (left.remainder !== right.remainder) {
                            return left.remainder > right.remainder ? -1 : 1;
                        }

                        return left.position - right.position;
                    })
                    .slice(0, remaining)
                    .map((entry) => entry.position),
            );

            return wholes.map((whole, position) =>
                Number(ranked.has(position) ? whole + 1n : whole),
            );
        },

        /** Read whole minor units as fractional minor units. */
        whole(amount: number): bigint {
            return BigInt(amount) * SCALE;
        },
    },
);
/** A cost in fractional minor units, negative for a credit. */
export type Cost = schema.Infer<typeof Cost>;
