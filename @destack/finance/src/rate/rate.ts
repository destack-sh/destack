import type { Charge } from "../object/charge.ts";
import type { PriceTerms } from "../object/price.ts";
import { present } from "@destack/schema";
import { type Sku } from "../sku/sku.ts";
import { Cost, type DecimalAmount } from "./amount.ts";
import { CatalogReference } from "../catalog/reference.ts";

/** The fractions of a pricing unit a quantity keeps. */
const QUANTITY_SCALE = 1_000_000;

/** The rating engine: usage priced at its SKU's list price in the billing currency. */
export const Rate = {
    /** Price a quantity of a SKU's meter in the billing currency, rounded up. */
    usage(
        sku: Sku,
        quantity: number,
        isBilled: boolean,
        conversion: DecimalAmount,
    ): Pick<
        Charge,
        "pricingQuantity" | "skuPriceId" | "listUnitPrice" | "listCost" | "billedCost"
    > {
        // price the quantity in pricing units at the list price in the billing currency
        const { unitSize, observedAt } = sku.definition;
        const unitAmount = Cost.of(
            ceilingDivide(
                Cost.scaled(sku.definition.unitAmount) * Cost.scaled(conversion),
                Cost.whole(1),
            ),
        );
        const pricingQuantity = quantity / unitSize;
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

    /** Price a quantity at a seller's price terms, per unit or through graduated tiers, rounded up. */
    price(terms: PriceTerms, quantity: number): Cost {
        // price each unit alike
        const scaled = BigInt(Math.round(quantity * QUANTITY_SCALE));
        const each = BigInt(QUANTITY_SCALE);
        if (terms.billingScheme === "per_unit") {
            const unitAmount = present(
                terms.unitAmountDecimal ?? terms.unitAmount,
                "a per-unit price's amount",
            );

            return Cost.of(ceilingDivide(Cost.scaled(String(unitAmount)) * scaled, each));
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
