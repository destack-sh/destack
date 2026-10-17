import type { Charge } from "../object/charge.ts";
import type { Price } from "../object/price.ts";
import { present } from "@destack/schema";
import { type Sku } from "../sku/sku.ts";
import { Cost, type DecimalAmount } from "./amount.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** The fractions of a pricing unit a quantity has. */
const QUANTITY_SCALE = 1_000_000;

/** The rating engine: usage priced at its SKU's list price in the billing currency. */
export const Rate = {
    /** Price a line's quantity of a SKU's meter in the billing currency, capped and rounded up. */
    usage(
        sku: Sku,
        quantity: number,
        isBilled: boolean,
        conversion: DecimalAmount,
    ): Pick<
        Charge,
        "pricingQuantity" | "skuPriceId" | "listUnitPrice" | "listCost" | "billedCost"
    > {
        // price the quantity in pricing units, up to the SKU's cap, at the list price in the billing currency
        const { unitSize, maximumQuantity, observedAt } = sku.definition;
        const unitAmount = Cost.of(
            ceilingDivide(
                Cost.scaled(sku.definition.unitAmount) * Cost.scaled(conversion),
                Cost.whole(1),
            ),
        );
        const pricingQuantity = Math.min(quantity / unitSize, maximumQuantity ?? Infinity);
        const scaledQuantity = BigInt(Math.round(pricingQuantity * QUANTITY_SCALE));
        const listCost = ceilingDivide(
            Cost.scaled(unitAmount) * scaledQuantity,
            BigInt(QUANTITY_SCALE),
        );

        return {
            pricingQuantity,
            skuPriceId: `${CatalogReference.key(sku.reference)}@${observedAt}`,
            listUnitPrice: unitAmount,
            listCost: Cost.of(listCost),
            billedCost: Cost.of(isBilled ? listCost : 0n),
        };
    },

    /** Read a per-unit price's amount per unit in fractional minor units, null if tiered. */
    unitPrice(terms: Pick<Price, "unitAmount" | "unitAmountDecimal">): DecimalAmount | null {
        // read the per-unit price's decimal amount, else its whole one
        if (terms.unitAmountDecimal !== null) {
            return terms.unitAmountDecimal;
        } else if (terms.unitAmount !== null) {
            return String(terms.unitAmount);
        }

        return null;
    },

    /** Price a quantity at a seller's terms, per unit or through graduated tiers, rounded up. */
    price(
        terms: Pick<Price, "billingScheme" | "unitAmount" | "unitAmountDecimal" | "tiers">,
        quantity: number,
    ): Cost {
        // price each unit alike
        const scaled = BigInt(Math.round(quantity * QUANTITY_SCALE));
        const each = BigInt(QUANTITY_SCALE);
        if (terms.billingScheme === "per_unit") {
            const unitAmount = present(Rate.unitPrice(terms), "a per-unit price's amount");

            return Cost.of(ceilingDivide(Cost.scaled(unitAmount) * scaled, each));
        }

        // price each tier's units at its amounts, the tiers in rising order
        let charged = 0n;
        let floor = 0n;
        for (const tier of present(terms.tiers, "a tiered price's tiers")) {
            const ceiling = tier.upTo === "inf" ? scaled : BigInt(tier.upTo) * each;
            const units = (scaled < ceiling ? scaled : ceiling) - floor;
            if (units > 0n) {
                charged += ceilingDivide(Cost.whole(tier.unitAmount ?? 0) * units, each);
                charged += Cost.whole(tier.flatAmount ?? 0);
            }
            floor = ceiling;
        }

        return Cost.of(charged);
    },
};

/** Divide non-negative whole numbers, rounding up. */
function ceilingDivide(numerator: bigint, denominator: bigint): bigint {
    return (numerator + denominator - 1n) / denominator;
}
