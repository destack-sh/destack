import { DeclarationReference } from "@destack/package/declare";
import { Rate, type Sku } from "@destack/finance";
import { Fee } from "./book.ts";

/** A SKU with its quantity in one unit of a priced meter, in the units of the SKU's own meter. */
export type Component = readonly [sku: Sku, quantity: number];

/** A provider's pricing model: the SKUs and quantities one unit of each generic meter costs. */
export class PricingModel {
    /** The components of each meter the model prices, by meter key. */
    readonly #components: ReadonlyMap<string, readonly Component[]>;

    /** Keep each meter's components. */
    constructor(meters: readonly (readonly [DeclarationReference, readonly Component[]])[]) {
        this.#components = new Map(
            meters.map(([meter, components]) => [DeclarationReference.key(meter), components]),
        );
    }

    /** Read the components one unit of a meter costs, absent for a meter the model leaves out. */
    components(meter: DeclarationReference): readonly Component[] | undefined {
        return this.#components.get(DeclarationReference.key(meter));
    }

    /** Price a meter's quantity at each component's list price, in US cents. */
    price(meter: DeclarationReference, quantity: number): number {
        // require the meter's components
        const components = this.components(meter);
        if (components === undefined) {
            throw new TypeError(`a pricing model has no price for meter ${meter.name}`);
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
