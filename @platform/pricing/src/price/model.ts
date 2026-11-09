import type { Meter, MeterAttributes } from "@destack/telemetry/meter";
import { Rate, type Sku } from "@destack/finance/declare";
import { Fee } from "./book.ts";

/** A SKU with its quantity in one unit of a priced meter, in the units of the SKU's own meter. */
export type Component = readonly [sku: Sku, quantity: number];

/** A provider's pricing model: the SKUs and quantities one unit of each generic meter costs. */
export class PricingModel {
    /** The components of each meter and attribute values the model prices, by their key. */
    readonly #components: ReadonlyMap<string, readonly Component[]>;

    /** Keep each meter's components at its attribute values, none where nothing is billed. */
    constructor(meters: readonly (readonly [Meter, MeterAttributes, readonly Component[]])[]) {
        this.#components = new Map(
            meters.map(([meter, attributes, components]) => [meter.key(attributes), components]),
        );
    }

    /** Read the components one unit of a meter's key with attributes costs, absent if unpriced. */
    components(key: string): readonly Component[] | undefined {
        return this.#components.get(key);
    }

    /** Price a quantity of a meter at some attribute values, by their key, in US cents. */
    price(key: string, quantity: number): number {
        // require the meter's components
        const components = this.components(key);
        if (components === undefined) {
            throw new TypeError(`a pricing model has no price for ${key}`);
        }

        // rate each component's share of the quantity at its list price in US cents
        let total = 0;
        for (const [sku, share] of components) {
            const conversion = Fee.conversion(sku.definition.currency);
            total += Number(Rate.usage(sku, quantity * share, true, conversion).listCost);
        }

        return total;
    }
}
