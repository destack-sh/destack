import { Address, type Comparator } from "@destack/resource";
import { PlanError } from "@destack/resource/error";
import { canonicalize, type JsonValue } from "@destack/schema";
import type { Sku } from "../sku/sku.ts";
import { SkuDescription } from "../sku/description.ts";

/** The fields charges rated under a SKU depend on, which its prices do not change. */
const RATED = ["provider", "meter", "pricingUnit", "unitSize", "currency"] as const;

/** Describe a SKU with its provider, meter, unit and prices. */
export function describeSku(sku: Sku): SkuDescription {
    return { ...sku.definition, package: sku.package };
}

/** Plan a SKU's change between releases: new prices and sources, the same provider, meter and unit. */
export const compareSku: Comparator = (before, after) => {
    // refuse a change of what the SKU's units measure
    const earlier = SkuDescription.parse(before.description);
    const later = SkuDescription.parse(after.description);
    const target = Address.join("sku", later.name);
    const problems = RATED.filter(
        (field) => canonicalize(earlier[field]) !== canonicalize(later[field]),
    ).map((field) => ({
        target,
        detail: `keep ${field} ${canonicalize(earlier[field])}, or declare another sku`,
    }));
    if (problems.length > 0) {
        throw new PlanError(problems);
    }

    return { steps: [] };
};

/** List a SKU's term: what it prices and in which unit. */
export function skuVocabulary(input: Record<string, JsonValue>): Record<string, JsonValue> {
    const sku = SkuDescription.parse(input);

    return {
        [Address.join("sku", sku.name)]: {
            provider: sku.provider,
            service: sku.service,
            pricingUnit: sku.pricingUnit,
        },
    };
}
